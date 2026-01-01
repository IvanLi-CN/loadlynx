use core::sync::atomic::{AtomicU32, Ordering};

use embassy_futures::yield_now;
use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::I2c as EhI2c;
use esp_hal::gpio::{Input, Output};
use heapless::Vec;

use crate::i2c0;

pub const FT6336U_ADDR_7BIT: u8 = 0x38;
const REG_TOUCH_DATA_START: u8 = 0x02;
const TOUCH_DATA_LEN: usize = 13;

// Coordinate mapping defaults. These MUST be verified on hardware by tapping
// screen corners; adjust if the marker is mirrored/rotated.
const TOUCH_RAW_MAX_X: i32 = 239;
const TOUCH_RAW_MAX_Y: i32 = 319;
const TOUCH_SWAP_XY: bool = true;
const TOUCH_INVERT_X: bool = true;
const TOUCH_INVERT_Y: bool = false;

// INT wait strategy:
// - Default: falling edge (pulse/edge-trigger behaviour).
// - Fallback: low-level until cleared (INT stays low until report is read).
const TOUCH_WAIT_LEVEL_LOW_MODE: bool = false;
const TOUCH_LEVEL_LOW_MAX_RETRIES: u8 = 6;

pub static TOUCH_INT_COUNT: AtomicU32 = AtomicU32::new(0);
pub static TOUCH_I2C_READ_COUNT: AtomicU32 = AtomicU32::new(0);
pub static TOUCH_PARSE_FAIL_COUNT: AtomicU32 = AtomicU32::new(0);

// Latest touch marker packed for lock-free UI overlay.
// Layout:
// - bits 0..9   : y (0..1023)
// - bits 10..19 : x (0..1023)
// - bits 20..23 : id (0..15)
// - bits 24..25 : event (0..3)
// - bit  26     : valid
static TOUCH_MARKER_PACKED: AtomicU32 = AtomicU32::new(0);
static TOUCH_MARKER_SEQ: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, Debug)]
pub struct TouchMarker {
    pub x: i32,
    pub y: i32,
    pub id: u8,
    pub event: u8,
}

pub fn touch_marker_seq() -> u32 {
    TOUCH_MARKER_SEQ.load(Ordering::Relaxed)
}

pub fn load_touch_marker() -> Option<TouchMarker> {
    let packed = TOUCH_MARKER_PACKED.load(Ordering::Relaxed);
    let valid = (packed >> 26) & 1;
    if valid == 0 {
        return None;
    }
    let y = (packed & 0x3ff) as i32;
    let x = ((packed >> 10) & 0x3ff) as i32;
    let id = ((packed >> 20) & 0x0f) as u8;
    let event = ((packed >> 24) & 0x03) as u8;
    Some(TouchMarker { x, y, id, event })
}

fn store_touch_marker(marker: TouchMarker) {
    let x = marker.x.clamp(0, 1023) as u32;
    let y = marker.y.clamp(0, 1023) as u32;
    let id = (marker.id & 0x0f) as u32;
    let event = (marker.event & 0x03) as u32;
    let packed = y | (x << 10) | (id << 20) | (event << 24) | (1 << 26);
    TOUCH_MARKER_PACKED.store(packed, Ordering::Relaxed);
    TOUCH_MARKER_SEQ.fetch_add(1, Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
    pub id: u8,
    pub event: u8,
    pub weight: u8,
    pub misc: u8,
}

#[derive(Clone, Debug)]
pub struct TouchReport {
    pub raw_touch_count: u8,
    pub points: Vec<TouchPoint, 2>,
}

pub enum TouchParseResult {
    Ok(TouchReport),
    CountTooHigh(TouchReport),
}

pub fn parse_raw_touch_report(raw: &[u8; TOUCH_DATA_LEN]) -> TouchParseResult {
    let raw_touch_count = raw[0] & 0x0f;
    let clamped_count = raw_touch_count.min(2);
    let mut points: Vec<TouchPoint, 2> = Vec::new();

    if clamped_count >= 1 {
        let _ = points.push(parse_point(&raw[1..7]));
    }
    if clamped_count >= 2 {
        let _ = points.push(parse_point(&raw[7..13]));
    }

    let report = TouchReport {
        raw_touch_count,
        points,
    };
    if raw_touch_count > 2 {
        TouchParseResult::CountTooHigh(report)
    } else {
        TouchParseResult::Ok(report)
    }
}

fn parse_point(buf: &[u8]) -> TouchPoint {
    // buf = [XH, XL, YH, YL, WEIGHT, MISC]
    let xh = buf[0];
    let xl = buf[1];
    let yh = buf[2];
    let yl = buf[3];
    let x = (((xh & 0x0f) as u16) << 8) | (xl as u16);
    let y = (((yh & 0x0f) as u16) << 8) | (yl as u16);
    let event = (xh >> 6) & 0x03;
    let id = (yh >> 4) & 0x0f;
    TouchPoint {
        x,
        y,
        id,
        event,
        weight: buf[4],
        misc: buf[5],
    }
}

fn map_to_logical(raw_x: u16, raw_y: u16) -> (i32, i32) {
    let mut x = raw_x as i32;
    let mut y = raw_y as i32;
    let mut max_x = TOUCH_RAW_MAX_X;
    let mut max_y = TOUCH_RAW_MAX_Y;

    if TOUCH_SWAP_XY {
        core::mem::swap(&mut x, &mut y);
        core::mem::swap(&mut max_x, &mut max_y);
    }
    if TOUCH_INVERT_X {
        x = max_x - x;
    }
    if TOUCH_INVERT_Y {
        y = max_y - y;
    }

    let logical_w = crate::DISPLAY_HEIGHT as i32;
    let logical_h = crate::DISPLAY_WIDTH as i32;
    (x.clamp(0, logical_w - 1), y.clamp(0, logical_h - 1))
}

async fn burst_read_0x02_0x0e<I2C: EhI2c>(
    i2c: &mut I2C,
    out: &mut [u8; TOUCH_DATA_LEN],
) -> Result<(), ()> {
    i2c.write_read(FT6336U_ADDR_7BIT, &[REG_TOUCH_DATA_START], out)
        .await
        .map_err(|_| ())
}

async fn read_touch_report_raw() -> Result<[u8; TOUCH_DATA_LEN], ()> {
    let mut buf = [0u8; TOUCH_DATA_LEN];
    TOUCH_I2C_READ_COUNT.fetch_add(1, Ordering::Relaxed);
    {
        let bus = i2c0::bus();
        let mut guard = bus.lock().await;
        burst_read_0x02_0x0e(&mut *guard, &mut buf).await?;
    }
    Ok(buf)
}

#[embassy_executor::task]
pub async fn touch_task(mut ctp_int: Input<'static>, mut ctp_rst: Output<'static>) {
    defmt::info!(
        "touch: task starting (addr=0x{:02x}, int_wait_mode={}, raw_max=({},{}) swap_xy={} inv_x={} inv_y={})",
        FT6336U_ADDR_7BIT,
        if TOUCH_WAIT_LEVEL_LOW_MODE {
            "level-low"
        } else {
            "falling-edge"
        },
        TOUCH_RAW_MAX_X,
        TOUCH_RAW_MAX_Y,
        TOUCH_SWAP_XY,
        TOUCH_INVERT_X,
        TOUCH_INVERT_Y
    );

    // Reset timing (minimum):
    // - RST low >= 5ms
    // - wait >= 300ms before relying on INT
    ctp_rst.set_low();
    embassy_time::Timer::after_millis(6).await;
    ctp_rst.set_high();
    embassy_time::Timer::after_millis(300).await;
    defmt::info!("touch: reset done; waiting for INT");

    loop {
        if TOUCH_WAIT_LEVEL_LOW_MODE {
            let _ = Wait::wait_for_low(&mut ctp_int).await;
        } else {
            let _ = Wait::wait_for_falling_edge(&mut ctp_int).await;
        }
        TOUCH_INT_COUNT.fetch_add(1, Ordering::Relaxed);

        match read_touch_report_raw().await {
            Ok(raw) => match parse_raw_touch_report(&raw) {
                TouchParseResult::Ok(report) => {
                    handle_report(&report);
                }
                TouchParseResult::CountTooHigh(report) => {
                    TOUCH_PARSE_FAIL_COUNT.fetch_add(1, Ordering::Relaxed);
                    defmt::warn!(
                        "touch: td_status too high (count={}); clamped to {}",
                        report.raw_touch_count,
                        report.points.len()
                    );
                    handle_report(&report);
                }
            },
            Err(()) => {
                defmt::warn!("touch: I2C read failed");
            }
        }

        // Low-level until cleared compatibility: if INT stays low after the
        // first read, allow limited retries to drain pending reports.
        if ctp_int.is_low() {
            for _ in 0..TOUCH_LEVEL_LOW_MAX_RETRIES {
                if !ctp_int.is_low() {
                    break;
                }
                match read_touch_report_raw().await {
                    Ok(raw) => match parse_raw_touch_report(&raw) {
                        TouchParseResult::Ok(report) => handle_report(&report),
                        TouchParseResult::CountTooHigh(report) => {
                            TOUCH_PARSE_FAIL_COUNT.fetch_add(1, Ordering::Relaxed);
                            handle_report(&report);
                        }
                    },
                    Err(()) => {
                        defmt::warn!("touch: I2C read failed (level-low retry)");
                    }
                }
                yield_now().await;
            }

            // If still low, wait for it to return high so we can observe a
            // future falling edge again (prevents tight stuck-low loops).
            if ctp_int.is_low() {
                defmt::warn!(
                    "touch: INT stuck low after retries (n={}); waiting for high",
                    TOUCH_LEVEL_LOW_MAX_RETRIES
                );
                let _ = Wait::wait_for_high(&mut ctp_int).await;
            }
        }

        yield_now().await;
    }
}

fn handle_report(report: &TouchReport) {
    // Some FT6336U configurations report "release" as TD_STATUS=0 (no points)
    // without emitting an explicit per-point event=1. Convert that into a
    // synthetic "up" marker at the last known coordinates so the UI can
    // reliably detect taps.
    if report.points.is_empty() {
        if let Some(last) = load_touch_marker() {
            store_touch_marker(TouchMarker {
                x: last.x,
                y: last.y,
                id: last.id,
                event: 1,
            });
        }
        return;
    }

    for p in report.points.iter() {
        let (x, y) = map_to_logical(p.x, p.y);
        defmt::info!(
            "touch: id={} event={} x={} y={} (raw_x={} raw_y={})",
            p.id,
            p.event,
            x,
            y,
            p.x,
            p.y
        );
        store_touch_marker(TouchMarker {
            x,
            y,
            id: p.id,
            event: p.event,
        });
    }
}

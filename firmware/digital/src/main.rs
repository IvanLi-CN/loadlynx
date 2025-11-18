#![no_std]
#![no_main]

use core::convert::Infallible;
use defmt::*;
use embassy_executor::Executor;
use embassy_futures::yield_now;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::ErrorType as SpiErrorType;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
use embedded_hal_async::spi::{Operation, SpiBus, SpiDevice};
use esp_hal::uart::{Config as UartConfig, DataBits, Parity, StopBits, Uart};
use esp_hal::{
    self as hal, Async,
    delay::Delay,
    gpio::{DriveMode, Level, NoPin, Output, OutputConfig},
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self as ledc_channel, ChannelIFace as _},
        timer::{self as ledc_timer, TimerIFace as _},
    },
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
// Async is already in scope via `use esp_hal::{ self as hal, Async, ... }`
// non-blocking读取通过 `uart.read_ready()` + `uart.read()` 实现
use lcd_async::{
    Builder, interface::SpiInterface, models::ST7789, options::Orientation,
    raw_framebuf::RawFrameBuf,
};
use static_cell::StaticCell;
use embassy_sync::blocking_mutex::CriticalSectionMutex;
use {esp_backtrace as _, esp_println as _}; // panic handler + defmt logger over espflash

mod ui;

esp_bootloader_esp_idf::esp_app_desc!();

const DISPLAY_WIDTH: usize = 240;
const DISPLAY_HEIGHT: usize = 320;
const FRAMEBUFFER_LEN: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * 2;
const TPS82130_ENABLE_DELAY_MS: u32 = 10;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();
static FRAMEBUFFER: StaticCell<[u8; FRAMEBUFFER_LEN]> = StaticCell::new();
static DISPLAY_RESOURCES: StaticCell<DisplayResources> = StaticCell::new();
static BACKLIGHT_TIMER: StaticCell<ledc_timer::Timer<'static, LowSpeed>> = StaticCell::new();
static BACKLIGHT_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> = StaticCell::new();
static UART1_CELL: StaticCell<Uart<'static, Async>> = StaticCell::new();

fn crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

fn process_frame(frame: &[u8], ui_state: &CriticalSectionMutex<ui::UiSnapshot>) {
    // Minimal implementation of docs/interfaces/uart-link.md:
    // ver:u8, flags:u8, seq:u8, msg:u8, len:u16LE, payload[len-2], crc16
    if frame.len() < 8 {
        return;
    }
    let ver = frame[0];
    let _flags = frame[1];
    let _seq = frame[2];
    let msg = frame[3];
    let len = (frame[4] as u16) | ((frame[5] as u16) << 8);
    if ver != 0x01 {
        return;
    }
    if msg != 0x10 {
        // Only Status for now
        return;
    }
    if frame.len() != 6 + len as usize {
        return;
    }
    if len < 2 {
        return;
    }

    let (hdr_and_payload, crc_bytes) = frame.split_at(frame.len() - 2);
    let crc_expected = (crc_bytes[0] as u16) | ((crc_bytes[1] as u16) << 8);
    let crc_calc = crc16_ccitt_false(hdr_and_payload);
    if crc_calc != crc_expected {
        warn!("uart-link: CRC mismatch (calc={} expected={})", crc_calc, crc_expected);
        return;
    }

    let payload = &frame[6..frame.len() - 2];
    if payload.is_empty() || payload[0] != 0x86 {
        // Expect CBOR array(6)
        return;
    }
    let mut idx = 1usize;
    let mut read_u32 = |buf: &[u8], idx: &mut usize| -> Option<u32> {
        if *idx >= buf.len() {
            return None;
        }
        if buf[*idx] != 0x1A {
            return None;
        }
        if *idx + 5 > buf.len() {
            return None;
        }
        let b0 = buf[*idx + 1] as u32;
        let b1 = buf[*idx + 2] as u32;
        let b2 = buf[*idx + 3] as u32;
        let b3 = buf[*idx + 4] as u32;
        *idx += 5;
        Some((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
    };

    let _ts_ms = match read_u32(payload, &mut idx) {
        Some(v) => v,
        None => return,
    };
    let ch1_i_mA = match read_u32(payload, &mut idx) {
        Some(v) => v,
        None => return,
    };
    let ch2_i_mA = match read_u32(payload, &mut idx) {
        Some(v) => v,
        None => return,
    };
    let vnr_sp_mV = match read_u32(payload, &mut idx) {
        Some(v) => v,
        None => return,
    };
    let vrmt_sp_mV = match read_u32(payload, &mut idx) {
        Some(v) => v,
        None => return,
    };
    let v5sns_mV = match read_u32(payload, &mut idx) {
        Some(v) => v,
        None => return,
    };

    unsafe {
        ui_state.lock_mut(|state| {
            state.update_from_status(ch1_i_mA, ch2_i_mA, vnr_sp_mV, vrmt_sp_mV, v5sns_mV);
        });
    }
}

// 简单异步任务（未启用时间驱动，使用合作式让出）
#[embassy_executor::task]
async fn ticker() {
    loop {
        // suppress noisy periodic tick log on ESP
        // trace!("LoadLynx digital tick");
        for _ in 0..1000 {
            yield_now().await;
        }
    }
}

struct DisplayResources {
    spi: Option<Spi<'static, Async>>,
    cs: Option<Output<'static>>,
    dc: Option<Output<'static>>,
    rst: Option<Output<'static>>,
    framebuffer: &'static mut [u8; FRAMEBUFFER_LEN],
}

struct SimpleSpiDevice<BUS, CS> {
    bus: BUS,
    cs: CS,
}

impl<BUS, CS> SimpleSpiDevice<BUS, CS> {
    const fn new(bus: BUS, cs: CS) -> Self {
        Self { bus, cs }
    }
}

impl<BUS, CS> SpiErrorType for SimpleSpiDevice<BUS, CS>
where
    BUS: SpiBus<u8>,
    CS: OutputPin<Error = Infallible>,
{
    type Error = BUS::Error;
}

impl<BUS, CS> SpiDevice for SimpleSpiDevice<BUS, CS>
where
    BUS: SpiBus<u8>,
    CS: OutputPin<Error = Infallible>,
{
    async fn transaction(
        &mut self,
        operations: &mut [Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        self.cs.set_low().unwrap();

        for op in operations.iter_mut() {
            match op {
                Operation::Read(buf) => self.bus.read(buf).await?,
                Operation::Write(buf) => self.bus.write(buf).await?,
                Operation::Transfer(read, write) => self.bus.transfer(read, write).await?,
                Operation::TransferInPlace(buf) => self.bus.transfer_in_place(buf).await?,
                Operation::DelayNs(_) => {
                    // No async-friendly delay available; treat as a no-op.
                }
            }
        }

        self.bus.flush().await?;
        self.cs.set_high().unwrap();

        Ok(())
    }
}

struct AsyncDelay {
    delay: Delay,
}

impl AsyncDelay {
    const fn new() -> Self {
        Self {
            delay: Delay::new(),
        }
    }
}

impl AsyncDelayNs for AsyncDelay {
    async fn delay_ns(&mut self, ns: u32) {
        self.delay.delay_nanos(ns);
    }
}

#[embassy_executor::task]
async fn display_task(
    ctx: &'static mut DisplayResources,
    ui_state: &'static CriticalSectionMutex<ui::UiSnapshot>,
) {
    info!("Display task starting");

    let spi = ctx.spi.take().expect("SPI bus unavailable");
    let cs = ctx.cs.take().expect("CS pin unavailable");
    let dc = ctx.dc.take().expect("DC pin unavailable");
    let rst = ctx.rst.take().expect("RST pin unavailable");
    let spi_device = SimpleSpiDevice::new(spi, cs);
    let interface = SpiInterface::new(spi_device, dc);
    let mut delay = AsyncDelay::new();

    // ST7789 color tuning:
    // Many ST7789-based IPS panels ship with RGB subpixel order and require
    // color inversion enabled for correct appearance. The previous configuration
    // used BGR without inversion which produced incorrect hues on this module
    // (e.g. cyan rendered as deep blue, dark backgrounds appeared too bright).
    //
    // If colors still look off on other panels, try toggling these options
    // per lcd-async's troubleshooting guide:
    //   - .color_order(lcd_async::options::ColorOrder::Bgr)
    //   - .invert_colors(lcd_async::options::ColorInversion::Normal)
    let mut display = Builder::new(ST7789, interface)
        .invert_colors(lcd_async::options::ColorInversion::Inverted)
        .color_order(lcd_async::options::ColorOrder::Rgb)
        .display_size(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16)
        .orientation(Orientation::new())
        .reset_pin(rst)
        .init(&mut delay)
        .await
        .expect("display init");

    loop {
        {
            let mut frame = RawFrameBuf::<Rgb565, _>::new(
                &mut ctx.framebuffer[..],
                DISPLAY_WIDTH,
                DISPLAY_HEIGHT,
            );
            ui_state.lock(|snapshot| {
                ui::render(&mut frame, snapshot);
            });
        }

        display
            .show_raw_data(
                0,
                0,
                DISPLAY_WIDTH as u16,
                DISPLAY_HEIGHT as u16,
                &ctx.framebuffer[..],
            )
            .await
            .expect("frame push");

        // 简单节流：约 10 Hz 重绘
        for _ in 0..500 {
            yield_now().await;
        }
    }
}

#[embassy_executor::task]
async fn uart_link_task(
    uart: &'static mut Uart<'static, Async>,
    ui_state: &'static CriticalSectionMutex<ui::UiSnapshot>,
) {
    info!("UART link task starting");

    const END: u8 = 0xC0;
    const ESC: u8 = 0xDB;
    const ESC_END: u8 = 0xDC;
    const ESC_ESC: u8 = 0xDD;

    let mut buf = [0u8; 256];
    let mut len = 0usize;
    let mut in_frame = false;
    let mut esc_next = false;
    let mut byte = [0u8; 1];

    loop {
        while uart.read_ready() {
            if uart.read(&mut byte).is_err() {
                break;
            }
            let b = byte[0];

            if !in_frame {
                if b == END {
                    in_frame = true;
                    len = 0;
                    esc_next = false;
                }
                continue;
            }

            if esc_next {
                let decoded = match b {
                    ESC_END => END,
                    ESC_ESC => ESC,
                    _ => b,
                };
                esc_next = false;
                if len < buf.len() {
                    buf[len] = decoded;
                    len += 1;
                }
                continue;
            }

            match b {
                END => {
                    if len >= 8 {
                        process_frame(&buf[..len], ui_state);
                    }
                    in_frame = false;
                    len = 0;
                }
                ESC => {
                    esc_next = true;
                }
                _ => {
                    if len < buf.len() {
                        buf[len] = b;
                        len += 1;
                    }
                }
            }
        }
        yield_now().await;
    }
}

#[main]
fn main() -> ! {
    let peripherals = hal::init(hal::Config::default());

    info!("LoadLynx digital alive; initializing local peripherals");

    // GPIO34 → FPC → 5V_EN, which drives the TPS82130SILR buck (docs/power/netlists/analog-board-netlist.enet).
    let alg_en_pin = peripherals.GPIO34;
    let mut alg_en = Output::new(alg_en_pin, Level::Low, OutputConfig::default());

    // SPI2 provides the high-speed channel for the TFT.
    let spi_peripheral = peripherals.SPI2;
    let sck = peripherals.GPIO12;
    let mosi = peripherals.GPIO11;
    let cs_pin = peripherals.GPIO13;
    let dc_pin = peripherals.GPIO10;
    let rst_pin = peripherals.GPIO6;
    let backlight_pin = peripherals.GPIO15;
    let ledc_peripheral = peripherals.LEDC;

    let spi = Spi::new(
        spi_peripheral,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .expect("spi init")
    .with_sck(sck)
    .with_mosi(mosi)
    .with_cs(NoPin)
    .into_async();

    let cs = Output::new(cs_pin, Level::High, OutputConfig::default());
    let dc = Output::new(dc_pin, Level::High, OutputConfig::default());
    let rst = Output::new(rst_pin, Level::High, OutputConfig::default());

    let mut ledc = Ledc::new(ledc_peripheral);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut backlight_timer = ledc.timer::<LowSpeed>(ledc_timer::Number::Timer0);
    backlight_timer
        .configure(ledc_timer::config::Config {
            duty: ledc_timer::config::Duty::Duty10Bit,
            clock_source: ledc_timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(20),
        })
        .expect("backlight timer");
    let backlight_timer = BACKLIGHT_TIMER.init(backlight_timer);

    let mut backlight_channel =
        ledc.channel::<LowSpeed>(ledc_channel::Number::Channel0, backlight_pin);
    backlight_channel
        .configure(ledc_channel::config::Config {
            timer: &*backlight_timer,
            duty_pct: 20,
            drive_mode: DriveMode::PushPull,
        })
        .expect("backlight channel");
    let backlight_channel = BACKLIGHT_CHANNEL.init(backlight_channel);
    backlight_channel.set_duty(20).expect("backlight duty set");

    let framebuffer = FRAMEBUFFER.init([0; FRAMEBUFFER_LEN]);

    let resources = DISPLAY_RESOURCES.init(DisplayResources {
        spi: Some(spi),
        cs: Some(cs),
        dc: Some(dc),
        rst: Some(rst),
        framebuffer,
    });

    const ENABLE_ANALOG_5V_ON_BOOT: bool = cfg!(feature = "enable_analog_5v_on_boot");
    if ENABLE_ANALOG_5V_ON_BOOT {
        info!("Digital peripherals ready; enabling TPS82130 5V rail after delay");
        let startup_delay = Delay::new();
        startup_delay.delay_millis(TPS82130_ENABLE_DELAY_MS);
        alg_en.set_high();
        info!("TPS82130 enabled; analog +5V rail requested");
    } else {
        info!(
            "Digital peripherals ready; SKIP enabling TPS82130 5V (debug mode). Build with feature 'enable_analog_5v_on_boot' to enable."
        );
    }

    // UART1 cross-link on GPIO17 (TX) / GPIO18 (RX)
    let uart_cfg = UartConfig::default()
        .with_baudrate(115_200)
        .with_data_bits(DataBits::_8)
        .with_parity(Parity::None)
        .with_stop_bits(StopBits::_1);

    info!("UART1 cross-link: GPIO17=TX / GPIO18=RX");

    let uart = Uart::new(peripherals.UART1, uart_cfg)
        .expect("uart1 init")
        .with_tx(peripherals.GPIO17)
        .with_rx(peripherals.GPIO18)
        .into_async();

    let uart1 = UART1_CELL.init(uart);
    static UI_STATE_CELL: StaticCell<CriticalSectionMutex<ui::UiSnapshot>> = StaticCell::new();
    let ui_state = UI_STATE_CELL.init(CriticalSectionMutex::new(ui::UiSnapshot::demo_const()));

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(ticker()).ok();
        spawner.spawn(display_task(resources, ui_state)).ok();
        spawner.spawn(uart_link_task(uart1, ui_state)).ok();
    })
}

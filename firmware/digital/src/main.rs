#![no_std]
#![no_main]

use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use defmt::*;
use embassy_executor::Executor;
use embassy_futures::yield_now;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::ErrorType as SpiErrorType;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
use embedded_hal_async::spi::{Operation, SpiBus, SpiDevice};
use embedded_io_async::Read as AsyncRead;
use esp_hal::time::Instant as HalInstant;
use esp_hal::uart::uhci::{self, RxConfig as UhciRxConfig, TxConfig as UhciTxConfig, Uhci};
use esp_hal::uart::{Config as UartConfig, DataBits, Parity, RxConfig, StopBits, Uart};
use esp_hal::{
    self as hal, Async,
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    gpio::{DriveMode, Input, InputConfig, Level, NoPin, Output, OutputConfig, Pull},
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self as ledc_channel, ChannelIFace as _},
        timer::{self as ledc_timer, TimerIFace as _},
    },
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi, SpiDmaBus},
    },
    time::Rate,
};

#[cfg(not(feature = "mock_setpoint"))]
use esp_hal::pcnt::{self, Pcnt, channel};
// Async is already in scope via `use esp_hal::{ self as hal, Async, ... }`
// UART async API (`embedded-io`) provides awaitable reads; leveraged below
use lcd_async::{
    Builder, interface::SpiInterface, models::ST7789, options::Orientation,
    raw_framebuf::RawFrameBuf,
};
use loadlynx_protocol::{
    FastStatus, MSG_SET_POINT, SetPoint, SlipDecoder, decode_fast_status_frame,
    encode_set_point_frame, slip_encode,
};
use static_cell::StaticCell;
use {esp_backtrace as _, esp_println as _}; // panic handler + defmt logger over espflash

mod ui;
use ui::UiSnapshot;

esp_bootloader_esp_idf::esp_app_desc!();

const FW_VERSION: &str = env!("LOADLYNX_FW_VERSION");

const DISPLAY_WIDTH: usize = 240;
const DISPLAY_HEIGHT: usize = 320;
const FRAMEBUFFER_LEN: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * 2;
const TPS82130_ENABLE_DELAY_MS: u32 = 10;
// 显示最小帧间隔（毫秒）：33ms ≈ 30FPS，与 analog 侧 30Hz FAST_STATUS 节奏对齐。
const DISPLAY_MIN_FRAME_INTERVAL_MS: u32 = 33;
// 将整帧分块推送到 LCD，以缩短单次 SPI 事务时间并为其它任务让出执行机会。
// 每块按行数分割：240 像素宽 × CHUNK 行 × RGB565(2B)，在 60MHz SPI 下能快速完成。
const DISPLAY_CHUNK_ROWS: usize = 4; // 再缩短单事务，降低单次 SPI 占用
const DISPLAY_CHUNK_YIELD_LOOPS: usize = 6; // 增大让出次数
const DISPLAY_DIRTY_MERGE_GAP_ROWS: usize = 8; // 适度扩大合并间隙，减少 SPI 往返
const DISPLAY_DIRTY_SPAN_FALLBACK: usize = 12; // 脏区 span 过多时退回整帧推送
const FRAME_SAMPLE_FRAMES: u32 = 3; // 仅记录前几帧像素统计，避免日志过多
const FRAME_LOG_POINTS: [(usize, usize); 3] = [
    (0, 0),
    (DISPLAY_WIDTH / 2, DISPLAY_HEIGHT / 2),
    (DISPLAY_WIDTH - 1, DISPLAY_HEIGHT - 1),
];
// 控制是否实际通过 SPI 推送到 LCD：DMA 验证阶段恢复开启以评估串口影响。
const ENABLE_DISPLAY_SPI_UPDATES: bool = true;
// 调试开关：正常运行应为 true，仅在单独验证 UI 或其它外设时才临时关闭 UART 链路任务。
const ENABLE_UART_LINK_TASK: bool = true;
const ENCODER_COUNTS_PER_STEP: i16 = 4; // quadrature: four edges per detent
const ENCODER_POLL_YIELD_LOOPS: usize = 200; // cooperative delay between polls
const ENCODER_DEBOUNCE_POLLS: u8 = 3; // simple stable-change debounce for button
const ENCODER_FILTER_CYCLES: u16 = 800; // ≈10 µs @ 80 MHz APB, filters encoder bounce

// UART + 协议相关的关键参数，用于日志自描述与 A/B 对比
const UART_BAUD: u32 = 115_200;
const UART_RX_FIFO_FULL_THRESHOLD: u16 = 120;
const UART_RX_TIMEOUT_SYMS: u8 = 12;
const FAST_STATUS_SLIP_CAPACITY: usize = 1536; // 更大 SLIP 缓冲降低分段/截断
// UART DMA 环形缓冲长度（同时作为 UHCI chunk_limit），与 SLIP 容量对齐以减少分段。
const UART_DMA_BUF_LEN: usize = 1536;
// SetPoint 发送频率：20Hz（50ms）
const SETPOINT_TX_PERIOD_MS: u32 = 100; // 降低到 10Hz，减轻模拟侧 UART 压力
const ENCODER_STEP_MA: i32 = 100; // 每个编码器步进 100mA
const ENABLE_UART_UHCI_DMA: bool = true;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();
#[repr(align(32))]
struct Align32<T>(T);

static FRAMEBUFFER: StaticCell<Align32<[u8; FRAMEBUFFER_LEN]>> = StaticCell::new();
static PREVIOUS_FRAMEBUFFER: StaticCell<Align32<[u8; FRAMEBUFFER_LEN]>> = StaticCell::new();
static DISPLAY_RESOURCES: StaticCell<DisplayResources> = StaticCell::new();
static BACKLIGHT_TIMER: StaticCell<ledc_timer::Timer<'static, LowSpeed>> = StaticCell::new();
static BACKLIGHT_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> = StaticCell::new();
static UART1_CELL: StaticCell<Uart<'static, Async>> = StaticCell::new();
static UART_DMA_DECODER: StaticCell<SlipDecoder<FAST_STATUS_SLIP_CAPACITY>> = StaticCell::new();
#[cfg(not(feature = "mock_setpoint"))]
static PCNT: StaticCell<Pcnt<'static>> = StaticCell::new();
type TelemetryMutex = Mutex<CriticalSectionRawMutex, TelemetryModel>;
static TELEMETRY: StaticCell<TelemetryMutex> = StaticCell::new();

#[cfg(not(feature = "mock_setpoint"))]
struct EncoderPins {
    a: Input<'static>,
    b: Input<'static>,
}

#[cfg(not(feature = "mock_setpoint"))]
static ENCODER_PINS: StaticCell<EncoderPins> = StaticCell::new();

// --- Telemetry & diagnostics -------------------------------------------------
static UART_RX_ERR_TOTAL: AtomicU32 = AtomicU32::new(0);
static PROTO_DECODE_ERRS: AtomicU32 = AtomicU32::new(0);
static FAST_STATUS_OK_COUNT: AtomicU32 = AtomicU32::new(0);
static LAST_UART_WARN_MS: AtomicU32 = AtomicU32::new(0);
static LAST_PROTO_WARN_MS: AtomicU32 = AtomicU32::new(0);
static DISPLAY_FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static DISPLAY_TASK_RUNNING: AtomicBool = AtomicBool::new(false);
static ENCODER_VALUE: AtomicI32 = AtomicI32::new(0);

#[inline]
fn now_ms32() -> u32 {
    timestamp_ms() as u32
}

fn timestamp_ms() -> u64 {
    HalInstant::now().duration_since_epoch().as_millis() as u64
}

defmt::timestamp!("{=u64:ms}", timestamp_ms());

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

#[embassy_executor::task]
async fn diag_task() {
    info!("Display diag task alive");
    loop {
        for _ in 0..2000 {
            yield_now().await;
        }
    }
}

#[cfg(feature = "mock_setpoint")]
const MOCK_SETPOINT_SCRIPT: &[(u32, i32)] =
    &[(0, 0), (1000, 500), (2000, 0), (3000, -500), (4000, 0)];

#[cfg(feature = "mock_setpoint")]
const MOCK_SCRIPT_LOOP: bool = true;

#[cfg(feature = "mock_setpoint")]
async fn mock_wait_ms(ms: u32) {
    for _ in 0..ms {
        yield_now().await;
    }
}

#[cfg(feature = "mock_setpoint")]
#[embassy_executor::task]
async fn mock_setpoint_task() {
    info!(
        "mock setpoint task running (script len={}, loop={})",
        MOCK_SETPOINT_SCRIPT.len(),
        MOCK_SCRIPT_LOOP
    );

    loop {
        let mut last_t = 0u32;
        for &(t_ms, target_ma) in MOCK_SETPOINT_SCRIPT.iter() {
            let delta = t_ms.saturating_sub(last_t);
            if delta > 0 {
                mock_wait_ms(delta).await;
            }
            last_t = t_ms;
            let steps = target_ma / ENCODER_STEP_MA;
            ENCODER_VALUE.store(steps, Ordering::SeqCst);
            info!(
                "mock setpoint script: t={} ms, target={} mA (steps={})",
                t_ms, target_ma, steps
            );
        }

        if !MOCK_SCRIPT_LOOP {
            break;
        }
    }
}

#[cfg(not(feature = "mock_setpoint"))]
#[embassy_executor::task]
async fn encoder_task(
    _unit: &'static pcnt::unit::Unit<'static, 0>,
    counter: pcnt::unit::Counter<'static, 0>,
    button: Input<'static>,
) {
    info!(
        "encoder task starting (GPIO1=ENC_A, GPIO2=ENC_B, GPIO0=ENC_SW active-low, counts_per_step={})",
        ENCODER_COUNTS_PER_STEP
    );

    let mut last_count = counter.get();
    let mut residual: i16 = 0;
    let mut last_button = button.is_low();
    let mut debounce: u8 = 0;

    loop {
        let count = counter.get();
        let delta = count.wrapping_sub(last_count);
        if delta != 0 {
            last_count = count;
            residual = residual.wrapping_add(delta);

            while residual >= ENCODER_COUNTS_PER_STEP || residual <= -ENCODER_COUNTS_PER_STEP {
                let phys_step = if residual > 0 { 1 } else { -1 };
                residual -= phys_step * ENCODER_COUNTS_PER_STEP;

                // Reverse logical direction to match panel orientation (CW increments).
                let logical_step = -phys_step;
                let new_val = ENCODER_VALUE
                    .fetch_add(logical_step as i32, Ordering::SeqCst)
                    .wrapping_add(logical_step as i32);

                if logical_step > 0 {
                    info!("encoder cw: value={}", new_val);
                } else {
                    info!("encoder ccw: value={}", new_val);
                }
            }
        }

        let pressed = button.is_low();
        if pressed != last_button {
            debounce = debounce.saturating_add(1);
            if debounce >= ENCODER_DEBOUNCE_POLLS {
                last_button = pressed;
                debounce = 0;
                if pressed {
                    ENCODER_VALUE.store(0, Ordering::SeqCst);
                    info!("encoder button pressed: value reset to 0");
                }
            }
        } else {
            debounce = 0;
        }

        for _ in 0..ENCODER_POLL_YIELD_LOOPS {
            yield_now().await;
        }
    }
}

struct DisplayResources {
    spi: Option<SpiDmaBus<'static, Async>>,
    cs: Option<Output<'static>>,
    dc: Option<Output<'static>>,
    rst: Option<Output<'static>>,
    framebuffer: &'static mut [u8; FRAMEBUFFER_LEN],
    previous_framebuffer: &'static mut [u8; FRAMEBUFFER_LEN],
}

struct TelemetryModel {
    snapshot: UiSnapshot,
    last_uptime_ms: Option<u32>,
    last_rendered: Option<UiSnapshot>,
}

impl TelemetryModel {
    fn new() -> Self {
        Self {
            snapshot: UiSnapshot::demo(),
            last_uptime_ms: None,
            last_rendered: None,
        }
    }

    fn update_from_status(&mut self, status: &FastStatus) {
        // 主电压使用本地 sense（v_local_mv），右侧列则分别显示 remote/local，
        // 以避免左侧大卡片和右侧“REMOTE”列完全重复。
        let main_voltage = status.v_local_mv as f32 / 1000.0;
        let remote_voltage = status.v_remote_mv as f32 / 1000.0;
        let local_voltage = status.v_local_mv as f32 / 1000.0;
        let i_local = status.i_local_ma as f32 / 1000.0;
        let i_remote = status.i_remote_ma as f32 / 1000.0;
        let power_w = status.calc_p_mw as f32 / 1000.0;

        self.snapshot.main_voltage = main_voltage;
        self.snapshot.remote_voltage = remote_voltage;
        self.snapshot.local_voltage = local_voltage;
        self.snapshot.main_current = i_local;
        self.snapshot.ch1_current = i_local;
        self.snapshot.ch2_current = i_remote;
        self.snapshot.main_power = power_w;
        self.snapshot.sink_core_temp = status.sink_core_temp_mc as f32 / 1000.0;
        self.snapshot.sink_exhaust_temp = status.sink_exhaust_temp_mc as f32 / 1000.0;
        self.snapshot.mcu_temp = status.mcu_temp_mc as f32 / 1000.0;

        write_runtime(&mut self.snapshot.run_time, status.uptime_ms);

        if let Some(prev) = self.last_uptime_ms {
            let delta_ms = status.uptime_ms.wrapping_sub(prev);
            if delta_ms < 60_000 {
                let delta_hours = delta_ms as f32 / 3_600_000.0;
                self.snapshot.energy_wh += power_w * delta_hours;
            }
        }
        self.last_uptime_ms = Some(status.uptime_ms);
    }

    /// Compute a change mask between the last rendered snapshot and the current
    /// one,返回当前快照副本与变化掩码。
    ///
    /// This is used by the display task to drive partial, character-aware
    /// updates on top of the existing framebuffer diff logic.
    fn diff_for_render(&mut self) -> (UiSnapshot, ui::UiChangeMask) {
        // Keep all display strings in sync with the latest numeric values so
        // the UI layer can render based purely on preformatted text. This is
        // intentionally called from the display task (UI context), not from the
        // UART link task, to avoid doing floating-point formatting work in the
        // UART path on ProCpu.
        self.snapshot.update_strings();

        let prev_snapshot = self.last_rendered.as_ref();
        let current = &self.snapshot;
        let mut mask = ui::UiChangeMask::default();

        if let Some(prev) = prev_snapshot {
            if prev.main_voltage_text != current.main_voltage_text
                || prev.main_current_text != current.main_current_text
                || prev.main_power_text != current.main_power_text
            {
                mask.main_metrics = true;
            }

            if prev.remote_voltage_text != current.remote_voltage_text
                || prev.local_voltage_text != current.local_voltage_text
            {
                mask.voltage_pair = true;
            }

            if prev.ch1_current_text != current.ch1_current_text
                || prev.ch2_current_text != current.ch2_current_text
            {
                mask.current_pair = true;
            }

            if prev.status_lines != current.status_lines {
                mask.telemetry_lines = true;
            }

            if mask.voltage_pair || mask.current_pair {
                mask.bars = true;
            }
        } else {
            // First-frame render: everything is considered dirty so that the
            // initial layout is fully drawn.
            mask.main_metrics = true;
            mask.voltage_pair = true;
            mask.current_pair = true;
            mask.telemetry_lines = true;
            mask.bars = true;
        }

        // 记录当前快照用于下一次 diff；只在这里 clone 一次，避免在栈上持有多份大对象。
        self.last_rendered = Some(self.snapshot.clone());
        (self.snapshot.clone(), mask)
    }
}

fn write_runtime(target: &mut heapless::String<16>, uptime_ms: u32) {
    target.clear();
    let total_seconds = uptime_ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let _ = core::fmt::write(target, format_args!("{hours:02}:{minutes:02}:{seconds:02}"));
}

async fn apply_fast_status(telemetry: &'static TelemetryMutex, status: &FastStatus) {
    let mut guard = telemetry.lock().await;
    guard.update_from_status(status);
}

fn protocol_error_str(err: &loadlynx_protocol::Error) -> &'static str {
    use loadlynx_protocol::Error::*;
    match err {
        BufferTooSmall => "buffer too small",
        PayloadTooLarge => "payload too large",
        InvalidVersion(_) => "invalid version",
        LengthMismatch => "length mismatch",
        InvalidPayloadLength => "payload length mismatch",
        UnsupportedMessage(_) => "unsupported message",
        CborEncode => "cbor encode",
        CborDecode => "cbor decode",
        InvalidCrc => "crc mismatch",
        SlipFrameTooLarge => "slip frame too large",
        SlipInvalidEscape(_) => "slip invalid escape",
    }
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
async fn display_task(ctx: &'static mut DisplayResources, telemetry: &'static TelemetryMutex) {
    DISPLAY_TASK_RUNNING.store(true, Ordering::Relaxed);
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

    {
        let mut frame =
            RawFrameBuf::<Rgb565, _>::new(&mut ctx.framebuffer[..], DISPLAY_WIDTH, DISPLAY_HEIGHT);
        // 首帧改为整屏高亮测试图，便于快速确认 LCD/背光是否工作正常。
        let bytes = frame.as_mut_bytes();
        for chunk in bytes.chunks_mut(2) {
            chunk[0] = 0xFF;
            chunk[1] = 0xFF;
        }
    }

    log_framebuffer_span("color-bars-fill", &ctx.framebuffer[..]);
    log_framebuffer_samples("color-bars-fill", &ctx.framebuffer[..]);

    // 首帧采用分块推送，降低长事务对调度的影响；可在调试时整体禁止 SPI 更新，以隔离 UART 问题。
    if ENABLE_DISPLAY_SPI_UPDATES {
        let bytes_per_row = DISPLAY_WIDTH * 2;
        let mut y = 0usize;
        while y < DISPLAY_HEIGHT {
            let rows = core::cmp::min(DISPLAY_CHUNK_ROWS, DISPLAY_HEIGHT - y);
            let start = y * bytes_per_row;
            let end = start + rows * bytes_per_row;
            info!("display: init chunk y={} rows={}", y, rows);
            display
                .show_raw_data(
                    0,
                    y as u16,
                    DISPLAY_WIDTH as u16,
                    rows as u16,
                    &ctx.framebuffer[start..end],
                )
                .await
                .expect("frame push (chunked init)");
            info!("display: init chunk done y={} rows={}", y, rows);

            for _ in 0..DISPLAY_CHUNK_YIELD_LOOPS {
                yield_now().await;
            }
            y += rows;
        }
        info!("Color bars rendered");
        ctx.previous_framebuffer
            .copy_from_slice(&ctx.framebuffer[..]);
    } else {
        info!("Color bars rendering skipped: display SPI updates disabled for UART A/B test");
    }

    let mut last_push_ms = timestamp_ms() as u32;
    // 为 FPS 统计维护一个滑动窗口：每个窗口至少 500ms，统计窗口内的帧数并据此估算 FPS。
    let mut fps_window_start_ms = last_push_ms;
    let mut fps_window_frames: u32 = 0;
    let mut last_fps: u32 = 0;
    loop {
        let now = timestamp_ms() as u32;
        let dt_ms = now.wrapping_sub(last_push_ms);
        if dt_ms >= DISPLAY_MIN_FRAME_INTERVAL_MS {
            let frame_idx = DISPLAY_FRAME_COUNT
                .fetch_add(1, Ordering::Relaxed)
                .wrapping_add(1);
            let log_this_frame = frame_idx <= FRAME_SAMPLE_FRAMES || frame_idx % 32 == 0;
            if log_this_frame {
                // 短期内每帧打印，之后按固定间隔抽样。
                info!("display: rendering frame {} (dt_ms={})", frame_idx, dt_ms);
            }

            // 进入本分辨率周期内的有效一帧，计入 FPS 统计窗口。
            fps_window_frames = fps_window_frames.wrapping_add(1);

            let (snapshot, mask) = {
                let mut guard = telemetry.lock().await;
                guard.diff_for_render()
            };

            if mask.is_empty() {
                if log_this_frame {
                    info!(
                        "display: frame {} skipped (no UI changes, dt_ms={})",
                        frame_idx, dt_ms
                    );
                }
            } else {
                {
                    let mut frame = RawFrameBuf::<Rgb565, _>::new(
                        &mut ctx.framebuffer[..],
                        DISPLAY_WIDTH,
                        DISPLAY_HEIGHT,
                    );
                    if frame_idx == 1 {
                        // 首帧：完整绘制静态布局 + 动态内容。
                        ui::render(&mut frame, &snapshot);
                    } else {
                        // 后续帧：仅按掩码重绘受影响区域。
                        ui::render_partial(&mut frame, &snapshot, &mask);
                    }
                    // 在左上角叠加 FPS 信息，使用上一统计窗口得到的整数 FPS。
                    ui::render_fps_overlay(&mut frame, last_fps);
                }

                if frame_idx <= FRAME_SAMPLE_FRAMES {
                    log_framebuffer_span("rendered-frame", &ctx.framebuffer[..]);
                    log_framebuffer_samples("rendered-frame", &ctx.framebuffer[..]);
                }
            }

            let mut dirty_rows = 0usize;
            let mut dirty_spans = 0usize;

            if ENABLE_DISPLAY_SPI_UPDATES {
                let bytes_per_row = DISPLAY_WIDTH * 2;
                let mut change_map = [false; DISPLAY_HEIGHT];
                for row in 0..DISPLAY_HEIGHT {
                    let offset = row * bytes_per_row;
                    change_map[row] = ctx.framebuffer[offset..offset + bytes_per_row]
                        != ctx.previous_framebuffer[offset..offset + bytes_per_row];
                }

                let mut row = 0usize;
                while row < DISPLAY_HEIGHT {
                    if !change_map[row] {
                        row += 1;
                        continue;
                    }

                    let start_row = row;
                    row += 1;
                    let mut gap_rows = 0usize;
                    while row < DISPLAY_HEIGHT {
                        if change_map[row] {
                            gap_rows = 0;
                            row += 1;
                        } else if gap_rows < DISPLAY_DIRTY_MERGE_GAP_ROWS {
                            gap_rows += 1;
                            row += 1;
                        } else {
                            break;
                        }
                    }

                    let rows_changed = row - start_row;
                    let start_idx = start_row * bytes_per_row;
                    let end_idx = start_idx + rows_changed * bytes_per_row;
                    display
                        .show_raw_data(
                            0,
                            start_row as u16,
                            DISPLAY_WIDTH as u16,
                            rows_changed as u16,
                            &ctx.framebuffer[start_idx..end_idx],
                        )
                        .await
                        .expect("frame push (dirty)");
                    ctx.previous_framebuffer[start_idx..end_idx]
                        .copy_from_slice(&ctx.framebuffer[start_idx..end_idx]);
                    dirty_rows += rows_changed;
                    dirty_spans += 1;
                }

                if dirty_spans >= DISPLAY_DIRTY_SPAN_FALLBACK {
                    // 如果脏区 span 过多，则退回整帧推送；否则保持行级增量更新。
                    display
                        .show_raw_data(
                            0,
                            0,
                            DISPLAY_WIDTH as u16,
                            DISPLAY_HEIGHT as u16,
                            &ctx.framebuffer[..],
                        )
                        .await
                        .expect("frame push (full fallback)");
                    ctx.previous_framebuffer
                        .copy_from_slice(&ctx.framebuffer[..]);
                    dirty_rows = DISPLAY_HEIGHT;
                    dirty_spans = 1;
                }
            } else {
                dirty_rows = 0;
                dirty_spans = 0;
            }

            if log_this_frame {
                info!(
                    "display: frame {} push complete (dirty_rows={} dirty_spans={})",
                    frame_idx, dirty_rows, dirty_spans
                );
            }

            // 每当统计窗口达到 ≥500ms 时，计算一次 FPS 并打印日志。
            let window_elapsed = now.wrapping_sub(fps_window_start_ms);
            if window_elapsed >= 500 {
                let fps = if window_elapsed > 0 {
                    (fps_window_frames.saturating_mul(1000)) / window_elapsed
                } else {
                    0
                };
                last_fps = fps;
                info!(
                    "display: fps window_ms={} frames={} fps={}",
                    window_elapsed, fps_window_frames, fps
                );
                fps_window_frames = 0;
                fps_window_start_ms = now;
            }

            last_push_ms = now;
        } else {
            // 未到下一帧的最小间隔，主动让出避免忙等占用整个 Core。
            yield_now().await;
        }
    }
}

fn log_framebuffer_span(label: &'static str, framebuffer: &[u8]) {
    if framebuffer.len() < 2 {
        return;
    }

    let mut min = u16::MAX;
    let mut max = 0u16;
    let mut checksum = 0u32;

    for chunk in framebuffer.chunks_exact(2) {
        let px = u16::from_be_bytes([chunk[0], chunk[1]]);
        if px < min {
            min = px;
        }
        if px > max {
            max = px;
        }
        checksum = checksum.wrapping_add(px as u32);
    }

    info!(
        "display framebuffer stats {}: min_pixel={} max_pixel={} checksum={}",
        label, min, max, checksum
    );
}

fn log_framebuffer_samples(label: &'static str, framebuffer: &[u8]) {
    for &(x, y) in FRAME_LOG_POINTS.iter() {
        if x >= DISPLAY_WIDTH || y >= DISPLAY_HEIGHT {
            continue;
        }
        let idx = (y * DISPLAY_WIDTH + x) * 2;
        if idx + 1 >= framebuffer.len() {
            continue;
        }
        let px = u16::from_be_bytes([framebuffer[idx], framebuffer[idx + 1]]);
        info!(
            "display framebuffer sample {} idx=({}, {} ) pixel={} (hi={}, lo={})",
            label,
            x,
            y,
            px,
            framebuffer[idx],
            framebuffer[idx + 1]
        );
    }
}

#[embassy_executor::task]
async fn uart_link_task(
    uart: &'static mut Uart<'static, Async>,
    telemetry: &'static TelemetryMutex,
) {
    info!(
        "UART link task starting (await read, no DMA): baud={} rx_fifo_thresh={} rx_timeout_syms={} slip_cap={} display_min_frame_interval_ms={} display_chunk_rows={} display_chunk_yield_loops={} display_spi_updates={}",
        UART_BAUD,
        UART_RX_FIFO_FULL_THRESHOLD,
        UART_RX_TIMEOUT_SYMS,
        FAST_STATUS_SLIP_CAPACITY,
        DISPLAY_MIN_FRAME_INTERVAL_MS,
        DISPLAY_CHUNK_ROWS,
        DISPLAY_CHUNK_YIELD_LOOPS,
        ENABLE_DISPLAY_SPI_UPDATES,
    );

    let mut decoder: SlipDecoder<FAST_STATUS_SLIP_CAPACITY> = SlipDecoder::new();
    let mut chunk = [0u8; 512];

    loop {
        match AsyncRead::read(uart, &mut chunk).await {
            Ok(n) if n > 0 => {
                feed_decoder(&chunk[..n], &mut decoder, telemetry).await;
            }
            Ok(_) => {
                continue;
            }
            Err(err) => {
                record_uart_error();
                rate_limited_uart_warn(&err);
                decoder.reset();
                continue;
            }
        }

        yield_now().await;
    }
}

#[embassy_executor::task]
async fn uart_link_task_dma(
    mut uhci_rx: uhci::UhciRx<'static, Async>,
    mut dma_rx: DmaRxBuf,
    telemetry: &'static TelemetryMutex,
) {
    info!(
        "UART link task starting (UHCI DMA): baud={} rx_fifo_thresh={} rx_timeout_syms={} dma_buf={} slip_cap={} display_min_frame_interval_ms={} display_chunk_rows={} display_chunk_yield_loops={} display_spi_updates={}",
        UART_BAUD,
        UART_RX_FIFO_FULL_THRESHOLD,
        UART_RX_TIMEOUT_SYMS,
        UART_DMA_BUF_LEN,
        FAST_STATUS_SLIP_CAPACITY,
        DISPLAY_MIN_FRAME_INTERVAL_MS,
        DISPLAY_CHUNK_ROWS,
        DISPLAY_CHUNK_YIELD_LOOPS,
        ENABLE_DISPLAY_SPI_UPDATES,
    );

    let decoder = UART_DMA_DECODER.init(SlipDecoder::new());

    loop {
        let mut transfer = match uhci_rx.read(dma_rx) {
            Ok(t) => t,
            Err((err, rx, buf)) => {
                record_uart_error();
                rate_limited_uart_warn(&err);
                uhci_rx = rx;
                dma_rx = buf;
                continue;
            }
        };

        transfer.wait_for_done().await;
        let (res, rx_back, buf_back) = transfer.wait();
        uhci_rx = rx_back;
        match res {
            Ok(()) => {
                let received = buf_back.number_of_received_bytes();
                let slice_len = received.min(buf_back.as_slice().len());
                feed_decoder(&buf_back.as_slice()[..slice_len], decoder, telemetry).await;
                dma_rx = buf_back;
            }
            Err(err) => {
                dma_rx = buf_back;
                record_uart_error();
                rate_limited_uart_warn(&err);
                decoder.reset();
                continue;
            }
        }

        yield_now().await;
    }
}

async fn feed_decoder(
    bytes: &[u8],
    decoder: &mut SlipDecoder<FAST_STATUS_SLIP_CAPACITY>,
    telemetry: &'static TelemetryMutex,
) {
    for &byte in bytes {
        match decoder.push(byte) {
            Ok(Some(frame)) => match decode_fast_status_frame(&frame) {
                Ok((header, status)) => {
                    apply_fast_status(telemetry, &status).await;
                    let total = FAST_STATUS_OK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                    // 默认每 32 帧打印一次成功节奏，用于长时间验收；
                    // 同时打印关键测量值，方便观察电流/电压是否在变化。
                    if total % 32 == 0 {
                        let display_running = DISPLAY_TASK_RUNNING.load(Ordering::Relaxed);
                        info!(
                            "fast_status ok (count={}, display_running={}, i_local_ma={} mA, target_value={} mA)",
                            total, display_running, status.i_local_ma, status.target_value
                        );
                    }
                    let _ = header; // keep header verified even if unused further
                }
                Err(err) => {
                    PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                    rate_limited_proto_warn(protocol_error_str(&err), frame.len());
                    decoder.reset();
                }
            },
            Ok(None) => {}
            Err(err) => {
                PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                rate_limited_proto_warn(protocol_error_str(&err), 0);
                decoder.reset();
            }
        }
    }
}

fn record_uart_error() {
    UART_RX_ERR_TOTAL.fetch_add(1, Ordering::Relaxed);
}

fn rate_limited_uart_warn<E: defmt::Format>(err: &E) {
    let now = now_ms32();
    let last = LAST_UART_WARN_MS.load(Ordering::Relaxed);
    if now.wrapping_sub(last) >= 2000 {
        LAST_UART_WARN_MS.store(now, Ordering::Relaxed);
        let total = UART_RX_ERR_TOTAL.load(Ordering::Relaxed);
        warn!("UART RX error: {:?} (total={})", err, total);
    }
}

fn rate_limited_proto_warn(kind: &str, len: usize) {
    let now = now_ms32();
    let last = LAST_PROTO_WARN_MS.load(Ordering::Relaxed);
    if now.wrapping_sub(last) >= 2000 {
        LAST_PROTO_WARN_MS.store(now, Ordering::Relaxed);
        let cnt = PROTO_DECODE_ERRS.load(Ordering::Relaxed);
        warn!(
            "protocol decode error ({}), frame_len={} [total={}]; resetting",
            kind, len, cnt
        );
    }
}

#[main]
fn main() -> ! {
    let peripherals = hal::init(hal::Config::default());

    info!("LoadLynx digital firmware version: {}", FW_VERSION);
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

    // 配置 SPI2 并启用 DMA：收缩 DMA 缓冲区以降低一次搬运的负载。
    // 4 行（4*240*2=1920B）以内的块可以覆盖单次传输，DMA 缓冲 2048B 足够。
    let (rx_buf, rx_desc, tx_buf, tx_desc) = esp_hal::dma_buffers!(2048);
    let dma_rx_buf = DmaRxBuf::new(rx_desc, rx_buf).expect("dma rx buf");
    let dma_tx_buf = DmaTxBuf::new(tx_desc, tx_buf).expect("dma tx buf");

    let spi = Spi::new(
        spi_peripheral,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40)) // 降低 SPI 频率以减少总线/栈压力
            .with_mode(Mode::_0),
    )
    .expect("spi init")
    .with_sck(sck)
    .with_mosi(mosi)
    .with_cs(NoPin)
    // 启用 SPI DMA 并绑定 DMA 缓冲区，然后切换到异步总线
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf)
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
            // 调试阶段提升背光亮度，避免“有画面但看起来近似黑屏”。
            duty_pct: 80,
            drive_mode: DriveMode::PushPull,
        })
        .expect("backlight channel");
    let backlight_channel = BACKLIGHT_CHANNEL.init(backlight_channel);
    backlight_channel.set_duty(80).expect("backlight duty set");

    let framebuffer = &mut FRAMEBUFFER.init_with(|| Align32([0; FRAMEBUFFER_LEN])).0;
    let prev_framebuffer = &mut PREVIOUS_FRAMEBUFFER
        .init_with(|| Align32([0; FRAMEBUFFER_LEN]))
        .0;

    let resources = DISPLAY_RESOURCES.init(DisplayResources {
        spi: Some(spi),
        cs: Some(cs),
        dc: Some(dc),
        rst: Some(rst),
        framebuffer,
        previous_framebuffer: prev_framebuffer,
    });

    let telemetry = TELEMETRY.init(Mutex::new(TelemetryModel::new()));

    #[cfg(not(feature = "mock_setpoint"))]
    let (encoder_button, encoder_unit, encoder_counter) = {
        let encoder_cfg = InputConfig::default().with_pull(Pull::Up);
        let encoder_pins = ENCODER_PINS.init(EncoderPins {
            a: Input::new(peripherals.GPIO1, encoder_cfg),
            b: Input::new(peripherals.GPIO2, encoder_cfg),
        });
        let encoder_button = Input::new(peripherals.GPIO0, encoder_cfg);

        // Hardware quadrature decoding via PCNT unit0.
        let pcnt = PCNT.init(Pcnt::new(peripherals.PCNT));
        let encoder_unit = &pcnt.unit0;

        let filter_cycles = ENCODER_FILTER_CYCLES.min(1023u16);
        encoder_unit
            .set_filter(Some(filter_cycles))
            .expect("encoder filter");
        encoder_unit.clear();

        let enc_a = encoder_pins.a.peripheral_input();
        let enc_b = encoder_pins.b.peripheral_input();

        let ch0 = &encoder_unit.channel0;
        ch0.set_ctrl_signal(enc_a.clone());
        ch0.set_edge_signal(enc_b.clone());
        ch0.set_ctrl_mode(channel::CtrlMode::Reverse, channel::CtrlMode::Keep);
        ch0.set_input_mode(channel::EdgeMode::Increment, channel::EdgeMode::Decrement);

        let ch1 = &encoder_unit.channel1;
        ch1.set_ctrl_signal(enc_b);
        ch1.set_edge_signal(enc_a);
        ch1.set_ctrl_mode(channel::CtrlMode::Reverse, channel::CtrlMode::Keep);
        ch1.set_input_mode(channel::EdgeMode::Decrement, channel::EdgeMode::Increment);

        encoder_unit.resume();
        let encoder_counter = encoder_unit.counter.clone();
        info!(
            "encoder pcnt configured (unit0, filter_cycles={}, counts_per_step={})",
            filter_cycles, ENCODER_COUNTS_PER_STEP
        );

        (encoder_button, encoder_unit, encoder_counter)
    };

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
    // NOTE: esp-hal 默认的 RxConfig 在大多数场景下更稳定：
    //   fifo_full_threshold ≈ 120, timeout ≈ 10 符号。
    // 之前我们调得太敏感（16 / 2），会放大中断压力；这里先回到接近默认的安全值。
    let uart_cfg = UartConfig::default()
        // Match analog MCU; 230400 baud is sufficient for 30 Hz FAST_STATUS traffic with SLIP overhead,
        // and keeps headroom if we later move the sender back towards 60 Hz.
        .with_baudrate(UART_BAUD)
        .with_data_bits(DataBits::_8)
        .with_parity(Parity::None)
        .with_stop_bits(StopBits::_1)
        .with_rx(
            RxConfig::default()
                .with_fifo_full_threshold(UART_RX_FIFO_FULL_THRESHOLD)
                .with_timeout(UART_RX_TIMEOUT_SYMS),
        );

    info!("UART1 cross-link: GPIO17=TX / GPIO18=RX");

    // 选择 UART 接收模式：默认启用 UHCI DMA 环形搬运，A/B 时可切换为 async no-DMA。
    let mut uart_async: Option<Uart<'static, Async>> = None;
    let mut uhci_rx_opt: Option<uhci::UhciRx<'static, Async>> = None;
    let mut uhci_tx_opt: Option<uhci::UhciTx<'static, Async>> = None;
    let mut uhci_dma_buf_opt: Option<DmaRxBuf> = None;

    if ENABLE_UART_UHCI_DMA {
        let uart_blocking = Uart::new(peripherals.UART1, uart_cfg)
            .expect("uart1 init")
            .with_tx(peripherals.GPIO17)
            .with_rx(peripherals.GPIO18);

        // 为 UART UHCI 配置独立 DMA 通道与缓冲；chunk_limit 不得超过 buf 长度且 <=4095。
        let (uhci_rx_buf, uhci_rx_desc, _uhci_tx_buf, _uhci_tx_desc) =
            esp_hal::dma_buffers!(UART_DMA_BUF_LEN);
        let dma_rx = DmaRxBuf::new(uhci_rx_desc, uhci_rx_buf).expect("uhci dma rx buf");

        let mut uhci =
            Uhci::new(uart_blocking, peripherals.UHCI0, peripherals.DMA_CH1).into_async();
        uhci.apply_rx_config(&UhciRxConfig::default().with_chunk_limit(UART_DMA_BUF_LEN as u16))
            .expect("uhci rx cfg");
        uhci.apply_tx_config(&UhciTxConfig::default())
            .expect("uhci tx cfg");
        uhci.set_uart_config(&uart_cfg).expect("uhci set cfg");

        let (uhci_rx, uhci_tx) = uhci.split();
        uhci_rx_opt = Some(uhci_rx);
        uhci_tx_opt = Some(uhci_tx);
        uhci_dma_buf_opt = Some(dma_rx);
    } else {
        let uart = Uart::new(peripherals.UART1, uart_cfg)
            .expect("uart1 init")
            .with_tx(peripherals.GPIO17)
            .with_rx(peripherals.GPIO18)
            .into_async();
        uart_async = Some(uart);
    }

    let uart1 = uart_async.map(|u| UART1_CELL.init(u));

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        info!("spawning ticker task");
        spawner.spawn(ticker()).expect("ticker spawn");
        info!("spawning diag task");
        spawner.spawn(diag_task()).expect("diag_task spawn");

        #[cfg(not(feature = "mock_setpoint"))]
        {
            info!("spawning encoder task");
            spawner
                .spawn(encoder_task(encoder_unit, encoder_counter, encoder_button))
                .expect("encoder_task spawn");
        }

        #[cfg(feature = "mock_setpoint")]
        {
            info!("spawning mock setpoint task");
            spawner
                .spawn(mock_setpoint_task())
                .expect("mock_setpoint_task spawn");
        }
        info!("spawning display task");
        spawner
            .spawn(display_task(resources, telemetry))
            .expect("display_task spawn");
        if ENABLE_UART_LINK_TASK {
            if ENABLE_UART_UHCI_DMA {
                let uhci_rx = uhci_rx_opt.take().expect("uhci rx missing");
                let dma_rx = uhci_dma_buf_opt.take().expect("uhci dma buf missing");
                info!("spawning uart link task (UHCI DMA)");
                spawner
                    .spawn(uart_link_task_dma(uhci_rx, dma_rx, telemetry))
                    .expect("uart_link_task_dma spawn");
            } else {
                let uart1 = uart1.expect("uart1 missing");
                info!("spawning uart link task (async no-DMA)");
                spawner
                    .spawn(uart_link_task(uart1, telemetry))
                    .expect("uart_link_task spawn");
            }
        } else {
            info!("UART link task disabled (ENABLE_UART_LINK_TASK=false)");
        }
        info!("spawning stats task");
        spawner.spawn(stats_task()).expect("stats_task spawn");
        if let Some(uhci_tx) = uhci_tx_opt.take() {
            info!("spawning setpoint tx task (UHCI TX, 20Hz fixed target)");
            spawner
                .spawn(setpoint_tx_task(uhci_tx))
                .expect("setpoint_tx_task spawn");
        } else {
            warn!("setpoint tx task not started (UHCI TX unavailable)");
        }
    })
}

// 周期性聚合统计，启动后每 5 秒打印一次（便于 DMA 验证阶段观察计数）
#[embassy_executor::task]
async fn stats_task() {
    let mut last_ms = timestamp_ms();
    loop {
        yield_now().await;
        let now = timestamp_ms();
        if now.saturating_sub(last_ms) >= 1_000 {
            last_ms = now;
            let ok = FAST_STATUS_OK_COUNT.load(Ordering::Relaxed);
            let de = PROTO_DECODE_ERRS.load(Ordering::Relaxed);
            let ut = UART_RX_ERR_TOTAL.load(Ordering::Relaxed);
            info!(
                "stats: fast_status_ok={}, decode_errs={}, uart_rx_err_total={}",
                ok, de, ut
            );
        }
    }
}

/// SetPoint 发送任务：10Hz，将编码器值映射为电流目标（步长 100mA）。
/// 仅在目标变化时发送，不做自注入或心跳，避免与实际操作混淆。
#[embassy_executor::task]
async fn setpoint_tx_task(mut uhci_tx: uhci::UhciTx<'static, Async>) {
    info!(
        "SetPoint TX task starting (encoder-driven, step={} mA, msg_id=0x{:02x}, period={} ms)",
        ENCODER_STEP_MA, MSG_SET_POINT, SETPOINT_TX_PERIOD_MS
    );

    let mut seq: u8 = 0;
    let mut raw = [0u8; 64];
    let mut slip = [0u8; 192];
    let mut last_sent: Option<i32> = None;
    let mut last_encoder_val = ENCODER_VALUE.load(Ordering::SeqCst);
    loop {
        let target_i_ma = ENCODER_VALUE.load(Ordering::SeqCst) * ENCODER_STEP_MA;
        let enc_now = ENCODER_VALUE.load(Ordering::SeqCst);
        let encoder_moved = enc_now != last_encoder_val;
        if encoder_moved {
            last_encoder_val = enc_now;
        }

        // 仅在值变化时发送
        let changed = last_sent.map(|v| v != target_i_ma).unwrap_or(true);

        if changed {
            let sp = SetPoint { target_i_ma };

            if let Ok(frame_len) = encode_set_point_frame(seq, &sp, &mut raw) {
                if let Ok(slip_len) = slip_encode(&raw[..frame_len], &mut slip) {
                    match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
                        Ok(written) if written == slip_len => {
                            let _ = uhci_tx.uart_tx.flush_async().await;
                            seq = seq.wrapping_add(1);
                            last_sent = Some(target_i_ma);
                            info!(
                                "setpoint sent: target_i_ma={} mA (seq={} reason=change)",
                                target_i_ma, seq
                            );
                        }
                        Ok(written) => {
                            warn!(
                                "SetPoint TX short write: written={} len={} (seq={})",
                                written, slip_len, seq
                            );
                        }
                        Err(err) => {
                            warn!(
                                "SetPoint TX write error (seq={} target={} mA): {:?}",
                                seq, target_i_ma, err
                            );
                        }
                    }
                } else {
                    warn!("SetPoint TX slip_encode error");
                }
            } else {
                warn!("SetPoint TX encode_set_point_frame error");
            }
        }

        // 10Hz: cooperative delay ~100ms
        for _ in 0..1000 {
            yield_now().await;
        }
    }
}

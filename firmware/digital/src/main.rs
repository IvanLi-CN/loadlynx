#![no_std]
#![no_main]

use core::convert::Infallible;
use defmt::*;
use embassy_executor::Executor;
use embassy_futures::yield_now;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::ErrorType as SpiErrorType;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
use embedded_hal_async::spi::{Operation, SpiBus, SpiDevice};
use esp_hal::time::Instant as HalInstant;
use esp_hal::uart::{Config as UartConfig, DataBits, Parity, RxConfig, StopBits, Uart};
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
use loadlynx_protocol::{FastStatus, SlipDecoder, decode_fast_status_frame};
use static_cell::StaticCell;
use {esp_backtrace as _, esp_println as _}; // panic handler + defmt logger over espflash

mod ui;
use ui::UiSnapshot;

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
type TelemetryMutex = Mutex<CriticalSectionRawMutex, TelemetryModel>;
static TELEMETRY: StaticCell<TelemetryMutex> = StaticCell::new();

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

struct DisplayResources {
    spi: Option<Spi<'static, Async>>,
    cs: Option<Output<'static>>,
    dc: Option<Output<'static>>,
    rst: Option<Output<'static>>,
    framebuffer: &'static mut [u8; FRAMEBUFFER_LEN],
}

struct TelemetryModel {
    snapshot: UiSnapshot,
    last_uptime_ms: Option<u32>,
}

impl TelemetryModel {
    fn new() -> Self {
        Self {
            snapshot: UiSnapshot::demo(),
            last_uptime_ms: None,
        }
    }

    fn update_from_status(&mut self, status: &FastStatus) {
        let main_voltage = status.v_remote_mv as f32 / 1000.0;
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

    fn snapshot(&self) -> UiSnapshot {
        self.snapshot.clone()
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
        ui::render_default(&mut frame);
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

    info!("Color bars rendered");
    loop {
        let snapshot = {
            let guard = telemetry.lock().await;
            guard.snapshot()
        };

        {
            let mut frame = RawFrameBuf::<Rgb565, _>::new(
                &mut ctx.framebuffer[..],
                DISPLAY_WIDTH,
                DISPLAY_HEIGHT,
            );
            ui::render(&mut frame, &snapshot);
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

        for _ in 0..2000 {
            yield_now().await;
        }
    }
}

#[embassy_executor::task]
async fn uart_link_task(
    uart: &'static mut Uart<'static, Async>,
    telemetry: &'static TelemetryMutex,
) {
    info!("UART link task starting");
    let mut decoder: SlipDecoder<512> = SlipDecoder::new();
    let mut chunk = [0u8; 64];

    loop {
        let count = match uart.read_async(&mut chunk).await {
            Ok(n) if n > 0 => n,
            Ok(_) => continue,
            Err(err) => {
                warn!("UART RX error ({:?}); resetting decoder", err);
                decoder.reset();
                continue;
            }
        };

        feed_decoder(&chunk[..count], &mut decoder, telemetry).await;
        // 让出执行，避免长时间占用导致其他外设任务饿死
        for _ in 0..64 {
            yield_now().await;
        }
    }
}

async fn feed_decoder(
    bytes: &[u8],
    decoder: &mut SlipDecoder<512>,
    telemetry: &'static TelemetryMutex,
) {
    for &byte in bytes {
        match decoder.push(byte) {
            Ok(Some(frame)) => match decode_fast_status_frame(&frame) {
                Ok((_header, status)) => {
                    apply_fast_status(telemetry, &status).await;
                }
                Err(err) => {
                    warn!(
                        "fast_status decode error ({}), len={}; resetting",
                        protocol_error_str(&err),
                        frame.len()
                    );
                    decoder.reset();
                }
            },
            Ok(None) => {}
            Err(err) => {
                warn!(
                    "slip decoder error ({}); resetting",
                    protocol_error_str(&err)
                );
                decoder.reset();
            }
        }
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

    let telemetry = TELEMETRY.init(Mutex::new(TelemetryModel::new()));

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
        .with_baudrate(57_600)
        .with_data_bits(DataBits::_8)
        .with_parity(Parity::None)
        .with_stop_bits(StopBits::_1)
        .with_rx(
            RxConfig::default()
                .with_fifo_full_threshold(32)
                .with_timeout(2),
        );

    info!("UART1 cross-link: GPIO17=TX / GPIO18=RX");

    let uart = Uart::new(peripherals.UART1, uart_cfg)
        .expect("uart1 init")
        .with_tx(peripherals.GPIO17)
        .with_rx(peripherals.GPIO18)
        .into_async();

    let uart1 = UART1_CELL.init(uart);

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(ticker()).ok();
        spawner.spawn(display_task(resources, telemetry)).ok();
        spawner.spawn(uart_link_task(uart1, telemetry)).ok();
    })
}

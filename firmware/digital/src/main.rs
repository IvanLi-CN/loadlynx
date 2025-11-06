#![no_std]
#![no_main]

use core::convert::Infallible;
use defmt::*;
use embassy_executor::Executor;
use embassy_futures::yield_now;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::ErrorType as SpiErrorType;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
use embedded_hal_async::spi::{Operation, SpiBus, SpiDevice};
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
use lcd_async::{
    Builder, interface::SpiInterface, models::ST7789, options::Orientation,
    raw_framebuf::RawFrameBuf,
};
use static_cell::StaticCell;
use {esp_backtrace as _, esp_println as _}; // panic handler + defmt logger over espflash

esp_bootloader_esp_idf::esp_app_desc!();

const DISPLAY_WIDTH: usize = 240;
const DISPLAY_HEIGHT: usize = 320;
const FRAMEBUFFER_LEN: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * 2;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();
static FRAMEBUFFER: StaticCell<[u8; FRAMEBUFFER_LEN]> = StaticCell::new();
static DISPLAY_RESOURCES: StaticCell<DisplayResources> = StaticCell::new();
static BACKLIGHT_TIMER: StaticCell<ledc_timer::Timer<'static, LowSpeed>> = StaticCell::new();
static BACKLIGHT_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> = StaticCell::new();

// 简单异步任务（未启用时间驱动，使用合作式让出）
#[embassy_executor::task]
async fn ticker() {
    loop {
        info!("LoadLynx digital tick");
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

fn render_color_bars(buffer: &mut RawFrameBuf<Rgb565, &mut [u8]>) {
    const COLOR_BAR_COLORS: [Rgb565; 8] = [
        Rgb565::new(31, 63, 31), // white
        Rgb565::new(31, 63, 0),  // yellow
        Rgb565::new(0, 63, 31),  // cyan
        Rgb565::new(0, 63, 0),   // green
        Rgb565::new(31, 0, 31),  // magenta
        Rgb565::new(31, 0, 0),   // red
        Rgb565::new(0, 0, 31),   // blue
        Rgb565::new(0, 0, 0),    // black
    ];

    let desired_width = DISPLAY_HEIGHT; // 320px when mounted horizontally
    let desired_height = DISPLAY_WIDTH; // 240px
    let bytes = buffer.as_mut_bytes();

    for x in 0..desired_width {
        let color_index = x * COLOR_BAR_COLORS.len() / desired_width;
        let color = COLOR_BAR_COLORS[color_index];
        let raw = color.into_storage();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;

        for y in 0..desired_height {
            let actual_x = y as usize;
            let actual_y = x;

            let byte_index = (actual_y * DISPLAY_WIDTH + actual_x) * 2;
            bytes[byte_index] = hi;
            bytes[byte_index + 1] = lo;
        }
    }
}

#[embassy_executor::task]
async fn display_task(ctx: &'static mut DisplayResources) {
    info!("Display task starting");

    let spi = ctx.spi.take().expect("SPI bus unavailable");
    let cs = ctx.cs.take().expect("CS pin unavailable");
    let dc = ctx.dc.take().expect("DC pin unavailable");
    let rst = ctx.rst.take().expect("RST pin unavailable");
    let spi_device = SimpleSpiDevice::new(spi, cs);
    let interface = SpiInterface::new(spi_device, dc);
    let mut delay = AsyncDelay::new();

    let mut display = Builder::new(ST7789, interface)
        .display_size(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16)
        .orientation(Orientation::new())
        .reset_pin(rst)
        .init(&mut delay)
        .await
        .expect("display init");

    {
        let mut frame =
            RawFrameBuf::<Rgb565, _>::new(&mut ctx.framebuffer[..], DISPLAY_WIDTH, DISPLAY_HEIGHT);
        render_color_bars(&mut frame);
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
        yield_now().await;
    }
}

#[main]
fn main() -> ! {
    let peripherals = hal::init(hal::Config::default());

    info!("LoadLynx digital alive");

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

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(ticker()).ok();
        spawner.spawn(display_task(resources)).ok();
    })
}

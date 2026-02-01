#![no_std]
#![no_main]

// Enable heap allocations (String, Vec, etc.) when the experimental net_http
// feature is used for Wi‑Fi + HTTP.
#[cfg(feature = "net_http")]
extern crate alloc;

use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, Ordering};
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::ErrorType as SpiErrorType;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
use embedded_hal_async::spi::{Operation, SpiBus, SpiDevice};
use embedded_io_async::Read as AsyncRead;
use esp_hal::gpio::OutputSignal as GpioOutputSignal;
use esp_hal::gpio::interconnect::OutputSignal as OutputSignalPin;
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::time::Instant as HalInstant;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::uhci::{self, RxConfig as UhciRxConfig, TxConfig as UhciTxConfig, Uhci};
use esp_hal::uart::{Config as UartConfig, DataBits, Parity, RxConfig, StopBits, Uart};
use esp_hal::{
    self as hal, Async,
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    gpio::{DriveMode, Level, NoPin, Output, OutputConfig},
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self as ledc_channel, ChannelIFace as _},
        timer::{self as ledc_timer, TimerIFace as _},
    },
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi, SpiDmaBus},
    },
    time::Rate,
};

use esp_hal::gpio::{Input, InputConfig, Pull};

#[cfg(not(feature = "mock_setpoint"))]
use esp_hal::pcnt::{self, Pcnt, channel};
// Async is already in scope via `use esp_hal::{ self as hal, Async, ... }`
// UART async API (`embedded-io`) provides awaitable reads; leveraged below
use lcd_async::{
    Builder, interface::SpiInterface, models::ST7789, options::Orientation,
    raw_framebuf::RawFrameBuf,
};
use loadlynx_protocol::{
    CRC_LEN, CalKind, CalMode, FAULT_MCU_OVER_TEMP, FAULT_OVERCURRENT, FAULT_OVERVOLTAGE,
    FAULT_SINK_OVER_TEMP, FLAG_ACK_REQ, FLAG_IS_ACK, FLAG_IS_NACK, FastStatus, FrameHeader,
    HEADER_LEN, LimitProfile, LoadMode, MSG_CAL_MODE, MSG_CAL_WRITE, MSG_FAST_STATUS, MSG_HELLO,
    MSG_LIMIT_PROFILE, MSG_PD_SINK_REQUEST, MSG_PD_STATUS, MSG_SET_MODE, MSG_SET_POINT,
    MSG_SOFT_RESET, PdSinkMode, PdSinkRequest, PdStatus, STATE_FLAG_UV_LATCHED, SetEnable, SetMode,
    SetPoint, SlipDecoder, SoftReset, SoftResetReason, decode_cal_mode_frame,
    decode_fast_status_frame, decode_frame, decode_hello_frame, decode_pd_status_frame,
    decode_soft_reset_frame, encode_cal_mode_frame, encode_cal_write_frame,
    encode_limit_profile_frame, encode_pd_sink_request_frame, encode_set_enable_frame,
    encode_set_mode_frame, encode_set_point_frame, encode_soft_reset_frame, slip_encode,
};
use loadlynx_screen_power::{ScreenPowerConfig, ScreenPowerModel, ScreenPowerState};
use static_cell::StaticCell;
use {esp_backtrace as _, esp_println as _}; // panic handler + defmt logger over espflash

pub(crate) const STATE_FLAG_REMOTE_ACTIVE: u32 = 1 << 0;
const STATE_FLAG_LINK_GOOD: u32 = 1 << 1;
const STATE_FLAG_ENABLED: u32 = 1 << 2;

const PD_T_PD_MS: u32 = 2_000;

const BACKLIGHT_DEFAULT_PCT: u8 = 50;

const SCREEN_DIM_AFTER_MS: u32 = 2 * 60 * 1000;
const SCREEN_OFF_AFTER_MS: u32 = 5 * 60 * 1000;
const SCREEN_DIM_MAX_PCT: u8 = 10;

// Plan #0021: touch spring (GPIO14 TouchPad14) + RGB status LED (GPIO38/39/40).
const TOUCH_SPRING_STARTUP_DELAY_MS: u32 = 300;
const TOUCH_SPRING_CAL_SAMPLES: u32 = 64;
const TOUCH_SPRING_POLL_MS: u32 = 10;
const TOUCH_SPRING_COOLDOWN_MS: u32 = 350;
const TOUCH_SPRING_BASELINE_EMA_DIV: i32 = 64; // larger = slower drift compensation

const RGB_STATUS_PWM_FREQUENCY_KHZ: u32 = 5; // avoid visible flicker
const RGB_STATUS_SOLID_BRIGHTNESS_PCT: u8 = 35;
const RGB_STATUS_BLINK_BRIGHTNESS_PCT: u8 = 35;
const RGB_STATUS_BLINK_TOGGLE_MS: u32 = 250; // 2 Hz: toggle every 250ms

const SCREEN_POWER_STATE_ACTIVE: u8 = 0;
const SCREEN_POWER_STATE_DIM: u8 = 1;
const SCREEN_POWER_STATE_OFF: u8 = 2;

static LAST_USER_ACTIVITY_MS: AtomicU32 = AtomicU32::new(0);
static SCREEN_POWER_STATE: AtomicU8 = AtomicU8::new(SCREEN_POWER_STATE_ACTIVE);

#[inline]
fn backlight_hw_duty_pct(logical_pct: u8) -> u8 {
    // BLK is active-low (PMOS high-side switch on BL_A, gate pulled up to 3V3 by default).
    // Therefore, LEDC duty_pct maps to "off time": logical brightness is inverted.
    let pct = logical_pct.min(100);
    100u8.saturating_sub(pct)
}

#[inline]
fn set_backlight_pct(
    backlight_channel: &'static ledc_channel::Channel<'static, LowSpeed>,
    logical_pct: u8,
    context: &'static str,
) {
    if backlight_channel
        .set_duty(backlight_hw_duty_pct(logical_pct))
        .is_err()
    {
        defmt::panic!("backlight duty set ({})", context);
    }
}

#[inline]
fn backlight_pwm_connect(backlight_pin: &'static OutputSignalPin<'static>) {
    // Connect LEDC low-speed channel 0 output to BLK.
    GpioOutputSignal::LEDC_LS_SIG0.connect_to(backlight_pin);
}

#[inline]
fn backlight_pwm_disconnect(backlight_pin: &'static OutputSignalPin<'static>) {
    GpioOutputSignal::LEDC_LS_SIG0.disconnect_from(backlight_pin);
    // Force gate high when disconnected to ensure the PMOS high-side switch stays off.
    backlight_pin.set_output_high(true);
}

#[inline]
fn rgb_hw_duty_pct(logical_pct: u8) -> u8 {
    // RGB channels are active-low (common anode on 3V3, GPIO sinks current).
    100u8.saturating_sub(logical_pct.min(100))
}

#[inline]
fn set_rgb_pct(
    r: &'static ledc_channel::Channel<'static, LowSpeed>,
    g: &'static ledc_channel::Channel<'static, LowSpeed>,
    b: &'static ledc_channel::Channel<'static, LowSpeed>,
    r_pct: u8,
    g_pct: u8,
    b_pct: u8,
) {
    r.set_duty(rgb_hw_duty_pct(r_pct)).expect("rgb r duty");
    g.set_duty(rgb_hw_duty_pct(g_pct)).expect("rgb g duty");
    b.set_duty(rgb_hw_duty_pct(b_pct)).expect("rgb b duty");
}

mod control;
use control::{ControlState, PresetsBlobError};

mod ui;
use ui::{AnalogState, UiSnapshot};

mod eeprom;
mod i2c0;
mod prompt_tone;
mod speaker;
mod touch;

// Optional Wi‑Fi + HTTP support; compiled only when `net_http` feature is set.
#[cfg(feature = "net_http")]
mod mdns;
#[cfg(feature = "net_http")]
mod net;

// Wi‑Fi compile-time configuration injected by firmware/digital/build.rs.
// Kept near the top so both main and the net module can rely on a single
// source of truth for SSID/PSK/static IP.
#[cfg(feature = "net_http")]
pub const WIFI_SSID: &str = env!("LOADLYNX_WIFI_SSID");
#[cfg(feature = "net_http")]
pub const WIFI_PSK: &str = env!("LOADLYNX_WIFI_PSK");
#[cfg(feature = "net_http")]
pub const WIFI_HOSTNAME: Option<&str> = option_env!("LOADLYNX_WIFI_HOSTNAME");
#[cfg(feature = "net_http")]
pub const WIFI_STATIC_IP: Option<&str> = option_env!("LOADLYNX_WIFI_STATIC_IP");
#[cfg(feature = "net_http")]
pub const WIFI_NETMASK: Option<&str> = option_env!("LOADLYNX_WIFI_NETMASK");
#[cfg(feature = "net_http")]
pub const WIFI_GATEWAY: Option<&str> = option_env!("LOADLYNX_WIFI_GATEWAY");
#[cfg(feature = "net_http")]
pub const WIFI_DNS: Option<&str> = option_env!("LOADLYNX_WIFI_DNS");

esp_bootloader_esp_idf::esp_app_desc!();

/// Digital firmware version string baked in at build time.
pub const FW_VERSION: &str = env!("LOADLYNX_FW_VERSION");

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
#[cfg_attr(feature = "mock_setpoint", allow(dead_code))]
const ENCODER_COUNTS_PER_STEP: i16 = 4; // quadrature: four edges per detent
#[cfg_attr(feature = "mock_setpoint", allow(dead_code))]
const ENCODER_POLL_YIELD_LOOPS: usize = 200; // cooperative delay between polls
#[cfg_attr(feature = "mock_setpoint", allow(dead_code))]
const ENCODER_DEBOUNCE_POLLS: u8 = 3; // simple stable-change debounce for button
#[cfg_attr(feature = "mock_setpoint", allow(dead_code))]
const ENCODER_FILTER_CYCLES: u16 = 800; // ≈10 µs @ 80 MHz APB, filters encoder bounce

// UART + 协议相关的关键参数，用于日志自描述与 A/B 对比
pub(crate) const UART_BAUD: u32 = 115_200;
const UART_RX_FIFO_FULL_THRESHOLD: u16 = 120;
const UART_RX_TIMEOUT_SYMS: u8 = 12;
const FAST_STATUS_SLIP_CAPACITY: usize = 1536; // 更大 SLIP 缓冲降低分段/截断
// UART DMA 环形缓冲长度（同时作为 UHCI chunk_limit），与 SLIP 容量对齐以减少分段。
const UART_DMA_BUF_LEN: usize = 1536;
// SetPoint 发送频率：降到 10Hz（100ms）以减轻模拟侧 UART 压力
const SETPOINT_TX_PERIOD_MS: u32 = 100; // used in encoder-driven mode
pub(crate) const ENCODER_STEP_MA: i32 = 100; // 每个编码器步进 100mA
pub(crate) const TARGET_I_MIN_MA: i32 = 0;
pub(crate) const TARGET_I_MAX_MA: i32 = 5_000;
// Software hard power limit (mW) used to clamp presets / API requests.
// NOTE: Hardware/thermal constraints may still require derating at runtime.
pub(crate) const HARD_MAX_P_MW: u32 = 200_000;
// 静态 LimitProfile v0：与当前硬保护阈值一致或略更保守。
pub(crate) const LIMIT_PROFILE_DEFAULT: LimitProfile = LimitProfile {
    max_i_ma: TARGET_I_MAX_MA,
    max_p_mw: HARD_MAX_P_MW,
    ovp_mv: 40_000,
    temp_trip_mc: 100_000,
    thermal_derate_pct: 100,
};
const ENABLE_UART_UHCI_DMA: bool = true;
// SetPoint 可靠传输：ACK 等待与退避重传（最新值优先）。
const SETPOINT_ACK_TIMEOUT_MS: u32 = 40;
const SETPOINT_RETRY_BACKOFF_MS: [u32; 3] = [40, 80, 160];
// SetMode 可靠传输：与 SetPoint 类似的 ACK 等待与退避重传（最新值优先）。
const SETMODE_ACK_TIMEOUT_MS: u32 = 40;
const SETMODE_RETRY_BACKOFF_MS: [u32; 3] = [40, 80, 160];
const SETMODE_TX_PERIOD_MS: u32 = 250;

// Fan PWM control (ESP32‑S3 本地，根据 G431 上报的 sink_core_temp + 功率驱动风扇占空比)。
// 数值集中在此处，便于后续调参。
const FAN_PWM_FREQUENCY_KHZ: u32 = 25; // 20–25 kHz 区间内，避开可闻频率
const FAN_DUTY_DEFAULT_PCT: u8 = 40; // 上电默认占空比，保证有一定风量
const FAN_DUTY_MIN_PCT: u8 = 15; // 温度进入控制区后的保底转速
const FAN_DUTY_MID_PCT: u8 = 70; // T_core=FAN_TEMP_HIGH_C 时的占空比
const FAN_DUTY_MAX_PCT: u8 = 100; // 高温段全速
const FAN_TEMP_STOP_C: f32 = 30.0; // 低功率时允许停转的温度阈值
const FAN_TEMP_LOW_C: f32 = 30.0; // 控制曲线起点（>= 此温度进入最小转速区）
const FAN_TEMP_HIGH_C: f32 = 55.0; // 线性拉升结束点
const FAN_LOG_HIGH_TEMP_C: f32 = 65.0; // 进入高温区时重点打印一次
const FAN_CONTROL_PERIOD_MS: u32 = 200; // 5 Hz 控制周期
const FAN_DUTY_UPDATE_THRESHOLD_PCT: u8 = 5; // 小于该差值则忽略，减小抖动
const FAN_LOG_DUTY_DELTA_LARGE_PCT: u8 = 20; // 占空比变化超过该阈值时可打印日志
const FAN_LOG_COOLDOWN_MS: u32 = 5_000; // fan 日志限频
const FAN_POWER_LOW_W: f32 = 5.0; // sink 功率低于该值时允许在低温下停转

#[repr(align(32))]
struct Align32<T>(T);

static FRAMEBUFFER: StaticCell<Align32<[u8; FRAMEBUFFER_LEN]>> = StaticCell::new();
#[cfg(not(feature = "net_http"))]
static PREVIOUS_FRAMEBUFFER: StaticCell<Align32<[u8; FRAMEBUFFER_LEN]>> = StaticCell::new();
static DISPLAY_RESOURCES: StaticCell<DisplayResources> = StaticCell::new();
static BACKLIGHT_TIMER: StaticCell<ledc_timer::Timer<'static, LowSpeed>> = StaticCell::new();
static BACKLIGHT_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> = StaticCell::new();
static BACKLIGHT_PIN: StaticCell<OutputSignalPin<'static>> = StaticCell::new();
static FAN_TIMER: StaticCell<ledc_timer::Timer<'static, LowSpeed>> = StaticCell::new();
static FAN_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> = StaticCell::new();
static BUZZER_TIMER: StaticCell<ledc_timer::Timer<'static, LowSpeed>> = StaticCell::new();
static BUZZER_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> = StaticCell::new();
static RGB_STATUS_TIMER: StaticCell<ledc_timer::Timer<'static, LowSpeed>> = StaticCell::new();
static RGB_STATUS_R_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> =
    StaticCell::new();
static RGB_STATUS_G_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> =
    StaticCell::new();
static RGB_STATUS_B_CHANNEL: StaticCell<ledc_channel::Channel<'static, LowSpeed>> =
    StaticCell::new();
static UART1_CELL: StaticCell<Uart<'static, Async>> = StaticCell::new();
static UART_DMA_DECODER: StaticCell<SlipDecoder<FAST_STATUS_SLIP_CAPACITY>> = StaticCell::new();
#[cfg(not(feature = "mock_setpoint"))]
static PCNT: StaticCell<Pcnt<'static>> = StaticCell::new();
pub type TelemetryMutex = Mutex<CriticalSectionRawMutex, TelemetryModel>;
static TELEMETRY: StaticCell<TelemetryMutex> = StaticCell::new();
pub(crate) static ANALOG_STATE: AtomicU8 = AtomicU8::new(AnalogState::Offline as u8);

pub type I2c0Bus = i2c0::I2c0Bus;
pub type EepromMutex = Mutex<CriticalSectionRawMutex, eeprom::SharedM24c64>;
static EEPROM: StaticCell<EepromMutex> = StaticCell::new();

use loadlynx_calibration_format::{self as calfmt, ActiveProfile, CurveKind};

#[derive(Clone, Debug)]
pub struct CalibrationState {
    pub profile: ActiveProfile,
    pub cal_mode: CalKind,
}

impl CalibrationState {
    pub fn new(profile: ActiveProfile) -> Self {
        Self {
            profile,
            cal_mode: CalKind::Off,
        }
    }
}

pub type CalibrationMutex = Mutex<CriticalSectionRawMutex, CalibrationState>;
static CALIBRATION: StaticCell<CalibrationMutex> = StaticCell::new();

pub type ControlMutex = Mutex<CriticalSectionRawMutex, ControlState>;
static CONTROL: StaticCell<ControlMutex> = StaticCell::new();
pub(crate) static CONTROL_REV: AtomicU32 = AtomicU32::new(0);
static PRESET_PREVIEW_ID: AtomicU8 = AtomicU8::new(0);

pub(crate) fn bump_control_rev() -> u32 {
    CONTROL_REV.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
}

// Soft-reset requests originating from the HTTP API are funneled through this
// small channel so they can be serialized onto the existing UART TX task.
#[cfg(feature = "net_http")]
static SOFT_RESET_REQUESTS: Channel<CriticalSectionRawMutex, SoftResetReason, 4> = Channel::new();

#[cfg(feature = "net_http")]
#[derive(Clone, Copy, Debug)]
pub enum CalUartCommand {
    SendAllCurves,
    SendCurve(CurveKind),
    SetMode(CalKind),
}

#[cfg(feature = "net_http")]
static CAL_UART_COMMANDS: Channel<CriticalSectionRawMutex, CalUartCommand, 8> = Channel::new();

#[cfg(feature = "net_http")]
pub(crate) fn enqueue_cal_uart(cmd: CalUartCommand) -> Result<(), &'static str> {
    CAL_UART_COMMANDS
        .try_send(cmd)
        .map_err(|_| "CAL_UART_QUEUE_FULL")
}

#[cfg(feature = "net_http")]
pub(crate) fn dequeue_cal_uart() -> Option<CalUartCommand> {
    CAL_UART_COMMANDS.try_receive().ok()
}

#[cfg(feature = "net_http")]
pub(crate) fn enqueue_soft_reset(reason: SoftResetReason) -> Result<(), &'static str> {
    SOFT_RESET_REQUESTS
        .try_send(reason)
        .map_err(|_| "SOFT_RESET_QUEUE_FULL")
}

#[cfg(not(feature = "net_http"))]
pub(crate) fn enqueue_soft_reset(_reason: SoftResetReason) -> Result<(), &'static str> {
    Err("net_http feature disabled")
}

#[cfg(feature = "net_http")]
pub(crate) fn dequeue_soft_reset() -> Option<SoftResetReason> {
    SOFT_RESET_REQUESTS.try_receive().ok()
}

#[cfg(not(feature = "net_http"))]
pub(crate) fn dequeue_soft_reset() -> Option<SoftResetReason> {
    None
}

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
static PROTO_FRAMING_DROPS: AtomicU32 = AtomicU32::new(0);
pub(crate) static FAST_STATUS_OK_COUNT: AtomicU32 = AtomicU32::new(0);
static LAST_UART_WARN_MS: AtomicU32 = AtomicU32::new(0);
static LAST_PROTO_WARN_MS: AtomicU32 = AtomicU32::new(0);
static DISPLAY_FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
static DISPLAY_TASK_RUNNING: AtomicBool = AtomicBool::new(false);
pub(crate) static ENCODER_VALUE: AtomicI32 = AtomicI32::new(0);
static TOUCH_SPRING_READ_TOTAL: AtomicU32 = AtomicU32::new(0);
static TOUCH_SPRING_TOUCHDOWN_TOTAL: AtomicU32 = AtomicU32::new(0);
static TOUCH_SPRING_SUPPRESS_COOLDOWN_TOTAL: AtomicU32 = AtomicU32::new(0);
static TOUCH_SPRING_ENABLE_BLOCK_TOTAL: AtomicU32 = AtomicU32::new(0);
static TOUCH_SPRING_LAST_RAW: AtomicU32 = AtomicU32::new(0);
static TOUCH_SPRING_LAST_BASELINE: AtomicU32 = AtomicU32::new(0);
static TOUCH_SPRING_LAST_DELTA_ABS: AtomicU32 = AtomicU32::new(0);
/// Digital-side CC load switch (default OFF on boot).
pub(crate) static LOAD_SWITCH_ENABLED: AtomicBool = AtomicBool::new(false);
static PD_LAST_APPLY_MS: AtomicU32 = AtomicU32::new(0);
static LAST_V_LOCAL_MV: AtomicI32 = AtomicI32::new(0);
static PD_STATUS_ATTACHED: AtomicBool = AtomicBool::new(false);
static PD_REQ_TX_TOTAL: AtomicU32 = AtomicU32::new(0);
static PD_REQ_ACK_TOTAL: AtomicU32 = AtomicU32::new(0);
static PD_REQ_TIMEOUT_TOTAL: AtomicU32 = AtomicU32::new(0);
static PD_REQ_LAST_ACK_SEQ: AtomicU8 = AtomicU8::new(0);
static PD_REQ_LAST_ACK_FLAGS: AtomicU8 = AtomicU8::new(0);
static PD_REQ_ACK_PENDING: AtomicBool = AtomicBool::new(false);
static PD_FORCE_SEND: AtomicBool = AtomicBool::new(false);
static PD_LAST_RESULT_CODE: AtomicU8 = AtomicU8::new(0);
static PD_LAST_RESULT_MS: AtomicU32 = AtomicU32::new(0);
static PD_UI_APPLY_MS: AtomicU32 = AtomicU32::new(0);
static SOFT_RESET_ACKED: AtomicBool = AtomicBool::new(false);
static CAL_MODE_ACK_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETPOINT_TX_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETPOINT_ACK_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETPOINT_RETX_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETPOINT_TIMEOUT_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETPOINT_LAST_ACK_SEQ: AtomicU8 = AtomicU8::new(0);
static SETPOINT_ACK_PENDING: AtomicBool = AtomicBool::new(false);

static SETMODE_TX_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETMODE_ACK_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETMODE_RETX_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETMODE_TIMEOUT_TOTAL: AtomicU32 = AtomicU32::new(0);
static SETMODE_LAST_ACK_SEQ: AtomicU8 = AtomicU8::new(0);
static SETMODE_ACK_PENDING: AtomicBool = AtomicBool::new(false);
pub(crate) static LAST_TARGET_VALUE_FROM_STATUS: AtomicI32 = AtomicI32::new(0);
pub(crate) static LAST_I_TOTAL_MA: AtomicI32 = AtomicI32::new(0);
pub(crate) static LAST_CALC_P_MW: AtomicU32 = AtomicU32::new(0);
pub(crate) static LAST_V_MAIN_MV: AtomicI32 = AtomicI32::new(0);
pub(crate) static LAST_FAULT_FLAGS: AtomicU32 = AtomicU32::new(0);
pub(crate) static UV_LATCHED: AtomicBool = AtomicBool::new(false);
static LAST_ENABLE_BLOCK_MS: AtomicU32 = AtomicU32::new(0);
static LAST_ENABLE_BLOCK_CODE: AtomicU8 = AtomicU8::new(0);
pub(crate) static LINK_UP: AtomicBool = AtomicBool::new(false);
pub(crate) static HELLO_SEEN: AtomicBool = AtomicBool::new(false);
pub(crate) static LAST_GOOD_FRAME_MS: AtomicU32 = AtomicU32::new(0);
static LAST_SETPOINT_GATE_WARN_MS: AtomicU32 = AtomicU32::new(0);
static LAST_FAULT_LOG_MS: AtomicU32 = AtomicU32::new(0);
/// Last analog firmware version identifier observed from HELLO (0 means unknown).
pub(crate) static ANALOG_FW_VERSION_RAW: AtomicU32 = AtomicU32::new(0);

#[inline]
pub fn now_ms32() -> u32 {
    timestamp_ms() as u32
}

pub fn timestamp_ms() -> u64 {
    HalInstant::now().duration_since_epoch().as_millis() as u64
}

defmt::timestamp!("{=u64:ms}", timestamp_ms());

#[inline]
fn screen_power_state_to_u8(state: ScreenPowerState) -> u8 {
    match state {
        ScreenPowerState::Active => SCREEN_POWER_STATE_ACTIVE,
        ScreenPowerState::Dim => SCREEN_POWER_STATE_DIM,
        ScreenPowerState::Off => SCREEN_POWER_STATE_OFF,
    }
}

#[inline]
fn screen_power_state_is_off() -> bool {
    SCREEN_POWER_STATE.load(Ordering::Relaxed) == SCREEN_POWER_STATE_OFF
}

#[inline]
fn note_user_activity() {
    LAST_USER_ACTIVITY_MS.store(now_ms32(), Ordering::Relaxed);
    prompt_tone::notify_local_activity();
}

#[inline]
fn note_user_activity_and_should_consume_off() -> bool {
    let was_off = screen_power_state_is_off();
    note_user_activity();
    was_off
}

fn fault_flags_abbrev(flags: u32) -> &'static str {
    // Public, user-facing abbreviations (frozen by docs/plan/0005:on-device-preset-ui/PLAN.md).
    if flags & FAULT_OVERVOLTAGE != 0 {
        "OVP"
    } else if flags & (FAULT_MCU_OVER_TEMP | FAULT_SINK_OVER_TEMP) != 0 {
        "OTP"
    } else if flags & FAULT_OVERCURRENT != 0 {
        "OCF"
    } else {
        "FLT"
    }
}

fn current_load_block_abbrev() -> Option<&'static str> {
    // Priority is frozen by docs/plan/0005:on-device-preset-ui/PLAN.md.
    let fault_flags = LAST_FAULT_FLAGS.load(Ordering::Relaxed);
    if fault_flags != 0 {
        Some(fault_flags_abbrev(fault_flags))
    } else if prompt_tone::is_link_alarm_latched() {
        // Critical link fault is represented by the latched alarm, NOT by transient
        // link-down windows (which are UI-only).
        Some("LNK")
    } else if UV_LATCHED.load(Ordering::Relaxed) {
        Some("UVLO")
    } else {
        None
    }
}

fn current_uvlo_inhibit(min_v_mv: i32) -> bool {
    // Match analog-side UVLO latch condition:
    // - Triggers when output_enabled=true and V_main <= min_v.
    //
    // We use this to *pre-check* and reject enabling so the analog side doesn't
    // latch UVLO when the system is already undervoltage before an enable attempt.
    if LAST_GOOD_FRAME_MS.load(Ordering::Relaxed) == 0 {
        return false;
    }

    if min_v_mv <= 0 {
        // UVLO disabled.
        return false;
    }
    let min_v_mv = min_v_mv.max(0);
    let v_main_mv = LAST_V_MAIN_MV.load(Ordering::Relaxed);
    v_main_mv <= min_v_mv
}

fn current_load_enable_block_abbrev(min_v_mv: i32) -> Option<&'static str> {
    // Priority is frozen by docs/plan/0005:on-device-preset-ui/PLAN.md.
    let fault_flags = LAST_FAULT_FLAGS.load(Ordering::Relaxed);
    if fault_flags != 0 {
        Some(fault_flags_abbrev(fault_flags))
    } else if !LINK_UP.load(Ordering::Relaxed) {
        if HELLO_SEEN.load(Ordering::Relaxed) {
            Some("LNK")
        } else {
            Some("OFF")
        }
    } else if current_uvlo_inhibit(min_v_mv) {
        Some("UVLO")
    } else {
        None
    }
}

const ENABLE_BLOCK_TTL_MS: u32 = 3_000;
const ENABLE_BLOCK_NONE: u8 = 0;
const ENABLE_BLOCK_UVLO: u8 = 1;

fn enable_block_code_to_abbrev(code: u8) -> Option<&'static str> {
    match code {
        ENABLE_BLOCK_UVLO => Some("UVLO"),
        _ => None,
    }
}

fn record_enable_block(reason: &'static str) {
    let code = match reason {
        "UVLO" => ENABLE_BLOCK_UVLO,
        _ => ENABLE_BLOCK_NONE,
    };
    if code == ENABLE_BLOCK_NONE {
        return;
    }
    LAST_ENABLE_BLOCK_CODE.store(code, Ordering::Relaxed);
    LAST_ENABLE_BLOCK_MS.store(now_ms32(), Ordering::Relaxed);
}

fn recent_enable_block_abbrev(now_ms: u32) -> Option<&'static str> {
    let code = LAST_ENABLE_BLOCK_CODE.load(Ordering::Relaxed);
    if code == ENABLE_BLOCK_NONE {
        return None;
    }
    let since_ms = LAST_ENABLE_BLOCK_MS.load(Ordering::Relaxed);
    if now_ms.wrapping_sub(since_ms) <= ENABLE_BLOCK_TTL_MS {
        enable_block_code_to_abbrev(code)
    } else {
        LAST_ENABLE_BLOCK_CODE.store(ENABLE_BLOCK_NONE, Ordering::Relaxed);
        None
    }
}

fn log_wifi_config() {
    // These values are injected at compile time by firmware/digital/build.rs.
    let ssid = env!("LOADLYNX_WIFI_SSID");
    let hostname = option_env!("LOADLYNX_WIFI_HOSTNAME");
    let static_ip = option_env!("LOADLYNX_WIFI_STATIC_IP");
    let netmask = option_env!("LOADLYNX_WIFI_NETMASK");
    let gateway = option_env!("LOADLYNX_WIFI_GATEWAY");
    let dns = option_env!("LOADLYNX_WIFI_DNS");
    let psk_present = option_env!("LOADLYNX_WIFI_PSK").is_some();

    info!(
        "Wi-Fi config: ssid=\"{}\", hostname={:?}, static_ip={:?}, netmask={:?}, gateway={:?}, dns={:?}, psk_present={}",
        ssid, hostname, static_ip, netmask, gateway, dns, psk_present
    );
}

/// Hook to release PAD‑JTAG so MTCK/MTDO/MTDI/MTMS (GPIO39–GPIO42) can be used
/// for reclaimed board IO (RGB PWM + FAN_PWM/FAN_TACH).
///
/// On ESP32‑S3, the recommended way in ESP‑IDF is to disable the PAD‑JTAG
/// mapping (e.g. via esp_apptrace APIs or EFUSE_DIS_PAD_JTAG). This firmware
/// currently runs directly on `esp-hal` without linking the full ESP‑IDF
/// runtime, so we rely on the GPIO/LEDC configuration below to re‑purpose
/// GPIO41 as a PWM output and keep GPIO42 reserved for future tach input.
///
/// If the project later adopts `esp-idf-sys`, a proper IDF call can be wired
/// in here so that all unsafe interaction stays confined to this function.
fn disable_pad_jtag_for_reclaimed_pins() {
    info!(
        "PAD-JTAG: preparing MTCK/MTDO/MTDI/MTMS (GPIO39–GPIO42) for reclaimed IO use; \
         relying on GPIO reconfiguration (no esp-idf runtime linked)"
    );
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

#[embassy_executor::task]
async fn diag_task() {
    info!("Display diag task alive");
    loop {
        for _ in 0..2000 {
            yield_now().await;
        }
    }
}

async fn cooperative_delay_ms(ms: u32) {
    let start = now_ms32();
    loop {
        let elapsed = now_ms32().wrapping_sub(start);
        if elapsed >= ms {
            break;
        }
        yield_now().await;
    }
}

#[cfg(feature = "mock_setpoint")]
const MOCK_STEP_MA: i32 = 100;
#[cfg(feature = "mock_setpoint")]
// Calibrated so the cooperative scheduler yields an effective ~80 ms cadence on hardware.
const MOCK_STEP_MS: u32 = 74;
#[cfg(feature = "mock_setpoint")]
const MOCK_PEAK_MA: i32 = 2000;
#[cfg(feature = "mock_setpoint")]
const MOCK_PEAK_HOLD_MS: u32 = 4_600;
#[cfg(feature = "mock_setpoint")]
const MOCK_PERIOD_MS: u32 = 9_200;

#[cfg(feature = "mock_setpoint")]
const MOCK_STEPS_TO_PEAK: i32 = MOCK_PEAK_MA / MOCK_STEP_MA;
#[cfg(feature = "mock_setpoint")]
const MOCK_RAMP_UP_MS: u32 = (MOCK_STEPS_TO_PEAK as u32) * MOCK_STEP_MS;
#[cfg(feature = "mock_setpoint")]
const MOCK_SCRIPT_LEN: usize =
    1 + MOCK_STEPS_TO_PEAK as usize + 1 + MOCK_STEPS_TO_PEAK as usize + 1;

#[cfg(feature = "mock_setpoint")]
const fn build_mock_script() -> [(u32, i32); MOCK_SCRIPT_LEN] {
    let mut script = [(0u32, 0i32); MOCK_SCRIPT_LEN];

    // t=0, ch1 = 0 mA
    let mut idx = 0usize;
    script[idx] = (0, 0);
    idx += 1;

    // Ramp up: 0 -> 2A @ +100 mA every 80 ms
    let mut step = 1i32;
    while step <= MOCK_STEPS_TO_PEAK {
        script[idx] = ((step as u32) * MOCK_STEP_MS, step * MOCK_STEP_MA);
        idx += 1;
        step += 1;
    }

    // Hold at peak for 5 s
    script[idx] = (MOCK_RAMP_UP_MS + MOCK_PEAK_HOLD_MS, MOCK_PEAK_MA);
    idx += 1;

    // Ramp down: 2A -> 0 @ -100 mA every 80 ms
    let mut down_step = 1i32;
    while down_step <= MOCK_STEPS_TO_PEAK {
        script[idx] = (
            MOCK_RAMP_UP_MS + MOCK_PEAK_HOLD_MS + (down_step as u32) * MOCK_STEP_MS,
            MOCK_PEAK_MA - down_step * MOCK_STEP_MA,
        );
        idx += 1;
        down_step += 1;
    }

    // Bottom hold to complete a 10 s period
    script[idx] = (MOCK_PERIOD_MS, 0);
    script
}

#[cfg(feature = "mock_setpoint")]
const MOCK_SETPOINT_SCRIPT: [(u32, i32); MOCK_SCRIPT_LEN] = build_mock_script();

#[cfg(feature = "mock_setpoint")]
const MOCK_SCRIPT_LOOP: bool = true;

#[cfg(feature = "mock_setpoint")]
#[embassy_executor::task]
async fn mock_setpoint_task() {
    info!(
        "mock setpoint task running (0->2A->0, step={} mA every {} ms, hold={} ms, period {} ms, entries={}, loop={})",
        MOCK_STEP_MA,
        MOCK_STEP_MS,
        MOCK_PEAK_HOLD_MS,
        MOCK_PERIOD_MS,
        MOCK_SETPOINT_SCRIPT.len(),
        MOCK_SCRIPT_LOOP
    );

    loop {
        let mut last_t = 0u32;
        for (idx, &(t_ms, target_ma)) in MOCK_SETPOINT_SCRIPT.iter().enumerate() {
            let delta = t_ms.saturating_sub(last_t);
            if delta > 0 {
                cooperative_delay_ms(delta).await;
            }
            last_t = t_ms;
            let steps = target_ma / ENCODER_STEP_MA;
            ENCODER_VALUE.store(steps, Ordering::SeqCst);
            if steps == 0 {
                LOAD_SWITCH_ENABLED.store(false, Ordering::SeqCst);
            }
            info!(
                "mock setpoint script: step={} t={} ms target={} mA (steps={})",
                idx, t_ms, target_ma, steps
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
    control: &'static ControlMutex,
    telemetry: &'static TelemetryMutex,
) {
    info!(
        "encoder task starting (GPIO1=ENC_A, GPIO2=ENC_B, GPIO0=ENC_SW active-low, counts_per_step={})",
        ENCODER_COUNTS_PER_STEP
    );

    let mut last_count = counter.get();
    let mut residual: i16 = 0;
    let mut last_button = button.is_low();
    let mut debounce: u8 = 0;
    let mut down_since_ms: Option<u32> = None;
    let mut long_action_fired: bool = false;
    const LONG_PRESS_MS: u32 = 800;

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
                if note_user_activity_and_should_consume_off() {
                    // Screen is waking from "off": consume this physical input so it can't
                    // change setpoints or toggle output while the UI is not visible.
                    residual = 0;
                    break;
                }
                prompt_tone::enqueue_ticks(1);
                let mut guard = control.lock().await;
                match guard.ui_view {
                    control::UiView::Main => {
                        let preset_id = guard.active_preset_id;
                        let preset_idx = preset_id.saturating_sub(1) as usize;
                        if preset_idx >= control::PRESET_COUNT {
                            continue;
                        }

                        let mut preset = guard.presets[preset_idx];
                        let mode = match preset.mode {
                            LoadMode::Cv => LoadMode::Cv,
                            LoadMode::Cp => LoadMode::Cp,
                            LoadMode::Cc | LoadMode::Reserved(_) => LoadMode::Cc,
                        };

                        let digit = coerce_adjust_digit_for_mode(mode, guard.adjust_digit);
                        if digit != guard.adjust_digit {
                            guard.adjust_digit = digit;
                        }
                        let step = digit.step_milli().saturating_mul(logical_step as i32);

                        let mut changed = false;
                        match mode {
                            LoadMode::Cc => {
                                let prev = preset.target_i_ma;
                                let max = preset.max_i_ma_total;
                                let next = (prev.saturating_add(step)).clamp(0, max);
                                if next != prev {
                                    preset.target_i_ma = next;
                                    changed = true;
                                }
                            }
                            LoadMode::Cv => {
                                let prev = preset.target_v_mv;
                                let next =
                                    (prev.saturating_add(step)).clamp(0, control::HARD_MAX_V_MV);
                                if next != prev {
                                    preset.target_v_mv = next;
                                    changed = true;
                                }
                            }
                            LoadMode::Cp => {
                                let prev = preset.target_p_mw as i64;
                                let max = preset.max_p_mw as i64;
                                let next = (prev + step as i64).clamp(0, max);
                                if next != prev {
                                    preset.target_p_mw = next as u32;
                                    changed = true;
                                }
                            }
                            LoadMode::Reserved(_) => {
                                let prev = preset.target_i_ma;
                                let max = preset.max_i_ma_total;
                                let next = (prev.saturating_add(step)).clamp(0, max);
                                if next != prev {
                                    preset.target_i_ma = next;
                                    changed = true;
                                }
                            }
                        }

                        if changed {
                            guard.presets[preset_idx] = preset.clamp();
                            guard.update_dirty_for_preset_id(preset_id);
                            bump_control_rev();
                        }
                    }
                    control::UiView::PresetPanelBlocked => {}
                    control::UiView::PresetPanel => {
                        let preset_id = guard.editing_preset_id;
                        let idx = preset_id.saturating_sub(1) as usize;
                        let Some(mut preset) = guard.presets.get(idx).copied() else {
                            continue;
                        };
                        let mode = match preset.mode {
                            LoadMode::Cv => LoadMode::Cv,
                            LoadMode::Cp => LoadMode::Cp,
                            LoadMode::Cc | LoadMode::Reserved(_) => LoadMode::Cc,
                        };

                        let field = coerce_panel_field_for_mode(mode, guard.panel_selected_field);
                        guard.panel_selected_field = field;
                        let digit =
                            coerce_panel_digit_for_field(mode, field, guard.panel_selected_digit);
                        guard.panel_selected_digit = digit;
                        let Some(step_unit) = panel_digit_step(mode, field, digit) else {
                            continue;
                        };
                        let step = step_unit.saturating_mul(logical_step as i32);

                        let mut changed = false;
                        match field {
                            ui::preset_panel::PresetPanelField::Mode => {}
                            ui::preset_panel::PresetPanelField::Target => match mode {
                                LoadMode::Cc => {
                                    let prev = preset.target_i_ma;
                                    let max = preset.max_i_ma_total;
                                    let next = (prev.saturating_add(step)).clamp(0, max);
                                    if next != prev {
                                        preset.target_i_ma = next;
                                        changed = true;
                                    }
                                }
                                LoadMode::Cv => {
                                    let prev = preset.target_v_mv;
                                    let next = (prev.saturating_add(step))
                                        .clamp(0, control::HARD_MAX_V_MV);
                                    if next != prev {
                                        preset.target_v_mv = next;
                                        changed = true;
                                    }
                                }
                                LoadMode::Cp => {
                                    let prev = preset.target_p_mw as i64;
                                    let max = preset.max_p_mw as i64;
                                    let next = (prev + step as i64).clamp(0, max);
                                    if next != prev {
                                        preset.target_p_mw = next as u32;
                                        changed = true;
                                    }
                                }
                                LoadMode::Reserved(_) => {
                                    let prev = preset.target_i_ma;
                                    let max = preset.max_i_ma_total;
                                    let next = (prev.saturating_add(step)).clamp(0, max);
                                    if next != prev {
                                        preset.target_i_ma = next;
                                        changed = true;
                                    }
                                }
                            },
                            ui::preset_panel::PresetPanelField::VLim => {
                                let prev = preset.min_v_mv;
                                let next =
                                    (prev.saturating_add(step)).clamp(0, control::HARD_MAX_V_MV);
                                if next != prev {
                                    preset.min_v_mv = next;
                                    changed = true;
                                }
                            }
                            ui::preset_panel::PresetPanelField::ILim => {
                                let prev = preset.max_i_ma_total;
                                let next = (prev.saturating_add(step))
                                    .clamp(0, control::HARD_MAX_I_MA_TOTAL);
                                if next != prev {
                                    preset.max_i_ma_total = next;
                                    changed = true;
                                }
                            }
                            ui::preset_panel::PresetPanelField::PLim => {
                                let prev = preset.max_p_mw as i64;
                                let next = (prev + step as i64).max(0) as u32;
                                if next != preset.max_p_mw {
                                    preset.max_p_mw = next;
                                    changed = true;
                                }
                            }
                        }

                        if changed {
                            preset = preset.clamp();
                            if idx < control::PRESET_COUNT {
                                guard.presets[idx] = preset;
                            }
                            guard.update_dirty_for_preset_id(preset_id);
                            bump_control_rev();
                        }
                    }
                    control::UiView::PdSettings => {
                        let focus = guard.pd_settings_focus;
                        if focus == control::PdSettingsFocus::None {
                            continue;
                        }

                        let draft = guard.pd_draft;
                        let digit = guard.pd_settings_digit;
                        drop(guard);

                        let pd_status = { telemetry.lock().await.last_pd_status.clone() };
                        let vm = build_pd_settings_vm(draft, focus, digit, pd_status.as_ref());

                        let step_dir = if logical_step >= 0 { 1i32 } else { -1i32 };
                        let step_mv = match digit {
                            control::AdjustDigit::Ones => 1000u32,
                            control::AdjustDigit::Tenths => 100u32,
                            control::AdjustDigit::Hundredths => 20u32,
                            _ => 100u32,
                        };
                        let step_ma = match digit {
                            control::AdjustDigit::Ones => 1000u32,
                            control::AdjustDigit::Tenths => 100u32,
                            control::AdjustDigit::Hundredths => 50u32,
                            _ => 100u32,
                        };

                        let mut guard = control.lock().await;
                        let mut changed = false;
                        match focus {
                            control::PdSettingsFocus::Vreq => {
                                if guard.pd_draft.mode != control::PdMode::Pps
                                    || vm.pps_object_pos == 0
                                {
                                    // PPS requires an explicit APDO selection.
                                } else {
                                    let pdo_pos = |pos: u8, idx: usize| -> u8 {
                                        if pos != 0 {
                                            pos
                                        } else {
                                            (idx + 1).min(u8::MAX as usize) as u8
                                        }
                                    };
                                    if let Some(apdo) = vm
                                        .pps_pdos
                                        .iter()
                                        .enumerate()
                                        .find(|(idx, p)| {
                                            pdo_pos(p.pos, *idx) == guard.pd_draft.pps_object_pos
                                        })
                                        .map(|(_idx, p)| *p)
                                    {
                                        if guard.pd_draft.target_mv == 0 {
                                            guard.pd_draft.target_mv = apdo.min_mv;
                                            changed = true;
                                        }
                                        let prev = guard.pd_draft.target_mv;
                                        let mut next = if step_dir > 0 {
                                            prev.saturating_add(step_mv)
                                        } else {
                                            prev.saturating_sub(step_mv)
                                        };
                                        next = next.clamp(apdo.min_mv, apdo.max_mv);
                                        if next != prev {
                                            guard.pd_draft.target_mv = next;
                                            changed = true;
                                        }
                                    }
                                }
                            }
                            control::PdSettingsFocus::Ireq => {
                                let prev = guard.pd_draft.i_req_ma;
                                let mut next = if step_dir > 0 {
                                    prev.saturating_add(step_ma)
                                } else {
                                    prev.saturating_sub(step_ma)
                                };
                                next = next.max(50);

                                let max_ma = match guard.pd_draft.mode {
                                    control::PdMode::Fixed => {
                                        let pdo_pos = |pos: u8, idx: usize| -> u8 {
                                            if pos != 0 {
                                                pos
                                            } else {
                                                (idx + 1).min(u8::MAX as usize) as u8
                                            }
                                        };
                                        if vm.fixed_object_pos == 0 {
                                            10_000
                                        } else {
                                            vm.fixed_pdos
                                                .iter()
                                                .enumerate()
                                                .find(|(idx, p)| {
                                                    pdo_pos(p.pos, *idx) == vm.fixed_object_pos
                                                })
                                                .map(|(_idx, p)| p.max_ma)
                                                .unwrap_or(10_000)
                                        }
                                    }
                                    control::PdMode::Pps => {
                                        if guard.pd_draft.pps_object_pos == 0 {
                                            10_000
                                        } else {
                                            let pdo_pos = |pos: u8, idx: usize| -> u8 {
                                                if pos != 0 {
                                                    pos
                                                } else {
                                                    (idx + 1).min(u8::MAX as usize) as u8
                                                }
                                            };
                                            vm.pps_pdos
                                                .iter()
                                                .enumerate()
                                                .find(|(idx, p)| {
                                                    pdo_pos(p.pos, *idx)
                                                        == guard.pd_draft.pps_object_pos
                                                })
                                                .map(|(_idx, p)| p.max_ma)
                                                .unwrap_or(10_000)
                                        }
                                    }
                                };
                                next = next.min(max_ma);
                                if next != prev {
                                    guard.pd_draft.i_req_ma = next;
                                    changed = true;
                                }
                            }
                            control::PdSettingsFocus::None => {}
                        }

                        if changed {
                            bump_control_rev();
                        }
                    }
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
                    if note_user_activity_and_should_consume_off() {
                        // Consume the wake input; ignore this press sequence.
                        down_since_ms = None;
                        long_action_fired = false;
                        continue;
                    }
                    // Start press window: short press toggles output, long press cycles preset.
                    down_since_ms = Some(now_ms32());
                    long_action_fired = false;
                } else {
                    // Release: if we didn't fire long action, treat as short press.
                    if down_since_ms.is_some() && !long_action_fired {
                        let mut guard = control.lock().await;
                        match guard.ui_view {
                            control::UiView::Main => {
                                let prev = guard.output_enabled;
                                if prev {
                                    guard.output_enabled = false;
                                    bump_control_rev();
                                    prompt_tone::enqueue_ui_ok();
                                    info!(
                                        "encoder short-press: LOAD ON -> OFF (preset_id={})",
                                        guard.active_preset_id
                                    );
                                } else if let Some(reason) =
                                    current_load_enable_block_abbrev(guard.active_preset().min_v_mv)
                                {
                                    prompt_tone::enqueue_ui_fail();
                                    record_enable_block(reason);
                                    info!(
                                        "encoder short-press: LOAD enable blocked (reason={})",
                                        reason
                                    );
                                } else {
                                    guard.output_enabled = true;
                                    bump_control_rev();
                                    prompt_tone::enqueue_ui_ok();
                                    info!(
                                        "encoder short-press: LOAD OFF -> ON (preset_id={})",
                                        guard.active_preset_id
                                    );
                                }
                            }
                            control::UiView::PresetPanel => {
                                let idx = guard.editing_preset_id.saturating_sub(1) as usize;
                                let mode = guard
                                    .presets
                                    .get(idx)
                                    .map(|p| match p.mode {
                                        LoadMode::Cv => LoadMode::Cv,
                                        LoadMode::Cp => LoadMode::Cp,
                                        LoadMode::Cc | LoadMode::Reserved(_) => LoadMode::Cc,
                                    })
                                    .unwrap_or(LoadMode::Cc);
                                let field =
                                    coerce_panel_field_for_mode(mode, guard.panel_selected_field);
                                guard.panel_selected_field = field;
                                guard.panel_selected_digit = cycle_panel_digit_right(
                                    mode,
                                    field,
                                    guard.panel_selected_digit,
                                );
                                bump_control_rev();
                                prompt_tone::enqueue_ui_ok();
                            }
                            control::UiView::PresetPanelBlocked => {}
                            control::UiView::PdSettings => {}
                        }
                    }
                    down_since_ms = None;
                    long_action_fired = false;
                }
            }
        } else {
            debounce = 0;
        }

        // Long press: cycle active preset once per press, force output OFF.
        if pressed {
            if let Some(start) = down_since_ms {
                let now = now_ms32();
                if !long_action_fired && now.wrapping_sub(start) >= LONG_PRESS_MS {
                    let mut guard = control.lock().await;
                    if guard.ui_view == control::UiView::Main {
                        let next = if guard.active_preset_id >= 5 {
                            1
                        } else {
                            guard.active_preset_id + 1
                        };
                        guard.activate_preset(next);
                        bump_control_rev();
                        prompt_tone::enqueue_ui_ok();
                        long_action_fired = true;
                        info!(
                            "encoder long-press: active_preset_id -> {} (output forced OFF)",
                            guard.active_preset_id
                        );
                    }
                }
            }
        }

        for _ in 0..ENCODER_POLL_YIELD_LOOPS {
            yield_now().await;
        }
    }
}

#[embassy_executor::task]
async fn touch_spring_task(
    _touch_pin: esp_hal::peripherals::GPIO14<'static>,
    control: &'static ControlMutex,
    rgb_r: &'static ledc_channel::Channel<'static, LowSpeed>,
    rgb_g: &'static ledc_channel::Channel<'static, LowSpeed>,
    rgb_b: &'static ledc_channel::Channel<'static, LowSpeed>,
) {
    info!(
        "touch spring task starting (GPIO14=TouchPad14, poll={}ms, cooldown={}ms, rgb_pwm={}kHz)",
        TOUCH_SPRING_POLL_MS, TOUCH_SPRING_COOLDOWN_MS, RGB_STATUS_PWM_FREQUENCY_KHZ
    );

    // Keep RGB dark until touch calibration completes.
    set_rgb_pct(rgb_r, rgb_g, rgb_b, 0, 0, 0);

    // NOTE: esp-hal 1.0.0 does not currently expose `esp_hal::touch` for ESP32-S3
    // (cfg(touch) is not set), so we configure + read the S3 touch controller via
    // PAC registers (RTC_CNTL + RTC_IO + SENS).
    fn init_touch_spring_gpio14() {
        // Configure RTC pad14 (GPIO14) for touch mode.
        let rtcio = esp_hal::peripherals::RTC_IO::regs();
        rtcio.touch_pad(14).modify(|_, w| {
            w.fun_ie().clear_bit(); // disable digital input path
            w.slp_oe().clear_bit();
            w.slp_ie().clear_bit();
            w.slp_sel().clear_bit();
            // Select RTC function and enable analog touch domain.
            unsafe { w.fun_sel().bits(0) };
            w.mux_sel().set_bit();
            w.xpd().set_bit();
            w.tie_opt().clear_bit();
            w.rue().clear_bit();
            w.rde().clear_bit();
            // Keep the default drive strength (reset value is 0b10).
            unsafe { w.drv().bits(0b10) };
            // In continuous/FSM mode, START/XPD are controlled by the touch FSM.
            // Still set them here to ensure the pad is powered/connected; the FSM will
            // override/toggle as needed when `touch_start_fsm_en=1`.
            w.start().set_bit();
            w.xpd().set_bit();
            w
        });

        // Touch sensor on ESP32-S3 is implemented via the RTC/SAR domain and
        // depends on the SARADC clock. Ensure it is not gated.
        let sens = esp_hal::peripherals::SENS::regs();
        sens.sar_peri_clk_gate_conf()
            .modify(|_, w| w.saradc_clk_en().set_bit());

        // Configure touch controller scan map to include pad14.
        let rtccntl = esp_hal::peripherals::LPWR::regs();
        // Match ESP-IDF defaults: idle pads connected to GND (not High-Z).
        rtccntl
            .touch_scan_ctrl()
            .modify(|_, w| w.touch_inactive_connection().set_bit());
        // Enable only TouchPad14 (GPIO14).
        rtccntl
            .touch_scan_ctrl()
            .modify(|_, w| unsafe { w.touch_scan_pad_map().bits(1 << 14) });
        // Match ESP-IDF defaults for non-ESP32 targets:
        // - sleep cycles (RTC_SLOW_CLK units): 0x000f
        // - measurement cycles (8MHz units): 500
        rtccntl.touch_ctrl1().modify(|_, w| unsafe {
            w.touch_sleep_cycles().bits(0x000f);
            w.touch_meas_num().bits(500)
        });
        // Enable touch FSM. For now we use SW-triggered measurements (start_force=1).
        rtccntl.touch_ctrl2().modify(|_, w| {
            // Match ESP-IDF defaults more closely (xpd wait cycles + self-bias).
            unsafe { w.touch_xpd_wait().bits(0xff) };
            w.touch_dbias().set_bit();
            // Voltage defaults: HIGH=2.7V, LOW=0.5V, ATTEN=0.5V.
            unsafe {
                w.touch_drefh().bits(3);
                w.touch_drefl().bits(0);
                w.touch_drange().bits(2);
            }

            w.touch_start_fsm_en().set_bit();
            w.touch_start_en().clear_bit();
            w.touch_start_force().set_bit();
            // In SW-triggered mode the IDF keeps the sleep-timer gate disabled.
            w.touch_slp_timer_en().clear_bit();
            w.touch_clk_fo().set_bit();
            w.touch_clkgate_en().set_bit()
        });

        // Touch slope (charge/discharge speed). Reset value is 0, and slope=0 can stall readings.
        // Match ESP-IDF default: TOUCH_PAD_SLOPE_DEFAULT = 7 (fast).
        rtccntl
            .touch_dac1()
            .modify(|_, w| unsafe { w.touch_pad14_dac().bits(7) });

        // Clear sleep benchmark state (ESP-IDF `touch_ll_sleep_reset_benchmark()`).
        rtccntl
            .touch_approach()
            .modify(|_, w| w.touch_slp_channel_clr().set_bit());
        rtccntl
            .touch_approach()
            .modify(|_, w| w.touch_slp_channel_clr().clear_bit());
        // Pulse timer-force-done, matching ESP-IDF's `touch_ll_timer_force_done()`.
        rtccntl
            .touch_ctrl2()
            .modify(|_, w| unsafe { w.touch_timer_force_done().bits(0x3) });
        rtccntl
            .touch_ctrl2()
            .modify(|_, w| unsafe { w.touch_timer_force_done().bits(0x0) });
        // Reset touch sensor FSM (ESP-IDF `touch_ll_reset()`): 0 -> 1 -> 0.
        rtccntl
            .touch_ctrl2()
            .modify(|_, w| w.touch_reset().clear_bit());
        rtccntl
            .touch_ctrl2()
            .modify(|_, w| w.touch_reset().set_bit());
        rtccntl
            .touch_ctrl2()
            .modify(|_, w| w.touch_reset().clear_bit());

        // Select raw touch data and clear stale status.
        sens.sar_touch_conf().modify(|_, w| unsafe {
            // Enable only TouchPad14 (bit 14), mirroring ESP-IDF's channel-mask convention.
            let outen = 1 << 14;
            w.sar_touch_outen().bits(outen);
            w.sar_touch_data_sel().bits(0);
            w.sar_touch_status_clr().set_bit()
        });
        // `sar_touch_status_clr` behaves like a clear pulse; ensure it is not
        // left asserted, otherwise touch status registers may stop updating.
        sens.sar_touch_conf()
            .modify(|_, w| w.sar_touch_status_clr().clear_bit());

        // Reset benchmark/raw for all channels so the controller starts producing fresh readings,
        // matching ESP-IDF's `touch_ll_reset_benchmark(TOUCH_PAD_MAX)`.
        sens.sar_touch_chn_st()
            .modify(|_, w| unsafe { w.sar_touch_channel_clr().bits(0x7fff) });
    }

    #[inline]
    fn touch14_read_raw() -> u32 {
        let sens = esp_hal::peripherals::SENS::regs();
        // Touch channel mask uses bit 14 for pad14, which corresponds to SAR_TOUCH_STATUS14.
        sens.sar_touch_status14().read().data().bits()
    }

    #[inline]
    fn touch14_trigger_sw_meas() {
        let rtccntl = esp_hal::peripherals::LPWR::regs();
        rtccntl
            .touch_ctrl2()
            .modify(|_, w| w.touch_start_en().clear_bit());
        rtccntl
            .touch_ctrl2()
            .modify(|_, w| w.touch_start_en().set_bit());
    }

    async fn touch14_read_measured_raw() -> u32 {
        touch14_trigger_sw_meas();

        // Wait for measurement done (ESP-IDF `touch_ll_is_measure_done()`), otherwise we may
        // repeatedly read stale status registers.
        let start = now_ms32();
        loop {
            let done = esp_hal::peripherals::SENS::regs()
                .sar_touch_chn_st()
                .read()
                .sar_touch_meas_done()
                .bit_is_set();
            if done {
                break;
            }
            if now_ms32().wrapping_sub(start) >= 20 {
                break;
            }
            cooperative_delay_ms(1).await;
        }

        touch14_read_raw()
    }

    init_touch_spring_gpio14();

    cooperative_delay_ms(TOUCH_SPRING_STARTUP_DELAY_MS).await;

    // Baseline calibration window: collect N samples for avg + noise span.
    let mut sum: u64 = 0;
    let mut min: u32 = u32::MAX;
    let mut max: u32 = 0;
    let mut samples: u32 = 0;
    while samples < TOUCH_SPRING_CAL_SAMPLES {
        let v = touch14_read_measured_raw().await;
        TOUCH_SPRING_READ_TOTAL.fetch_add(1, Ordering::Relaxed);
        sum = sum.saturating_add(v as u64);
        min = min.min(v);
        max = max.max(v);
        samples += 1;
        cooperative_delay_ms(TOUCH_SPRING_POLL_MS).await;
    }
    let baseline0 = (sum / samples.max(1) as u64) as u32;
    let noise_span = max.saturating_sub(min) as u32;
    // Heuristic: threshold based on observed noise + a small baseline fraction.
    //
    // IMPORTANT (HIL): Touch delta direction can be board/layout dependent, so
    // runtime detection is based on abs(v-baseline) rather than assuming "touch => smaller".
    let baseline_frac = (baseline0 / 1024).max(1);
    let noise_delta = noise_span.saturating_mul(8);
    let down_delta = noise_delta.max(baseline_frac).max(100);
    let up_delta = (down_delta / 2).max(50);

    let mut baseline: i32 = baseline0 as i32;
    let mut touched: bool = false;
    let mut last_action_ms: u32 = now_ms32().wrapping_sub(TOUCH_SPRING_COOLDOWN_MS);
    let mut last_touch_block_ms: Option<u32> = None;

    info!(
        "touch spring calibrated: baseline={} (min={}, max={}, span={}), delta_abs_down={}, delta_abs_up={}",
        baseline0, min, max, noise_span, down_delta, up_delta
    );

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct Rgb {
        r: u8,
        g: u8,
        b: u8,
    }
    let mut last_rgb = Rgb { r: 0, g: 0, b: 0 };
    let mut blink_on: bool = false;
    let mut last_blink_toggle_ms: u32 = now_ms32();
    let mut last_dbg_ms: u32 = now_ms32();

    loop {
        let now = now_ms32();

        let v = touch14_read_measured_raw().await;
        TOUCH_SPRING_READ_TOTAL.fetch_add(1, Ordering::Relaxed);

        let baseline_u32 = baseline.max(0) as u32;
        let delta_abs = v.abs_diff(baseline_u32);
        TOUCH_SPRING_LAST_RAW.store(v, Ordering::Relaxed);
        TOUCH_SPRING_LAST_BASELINE.store(baseline_u32, Ordering::Relaxed);
        TOUCH_SPRING_LAST_DELTA_ABS.store(delta_abs, Ordering::Relaxed);

        if !touched {
            // Drift compensation: only update baseline while clearly not touched.
            if delta_abs <= up_delta {
                baseline += ((v as i32) - baseline) / TOUCH_SPRING_BASELINE_EMA_DIV.max(1);
            }

            if delta_abs >= down_delta {
                // Touch-down edge candidate.
                if now.wrapping_sub(last_action_ms) < TOUCH_SPRING_COOLDOWN_MS {
                    TOUCH_SPRING_SUPPRESS_COOLDOWN_TOTAL.fetch_add(1, Ordering::Relaxed);
                } else {
                    touched = true;
                    last_action_ms = now;
                    TOUCH_SPRING_TOUCHDOWN_TOTAL.fetch_add(1, Ordering::Relaxed);

                    // Dedicated load switch input: treat as user activity (wake screen),
                    // but do NOT consume (this is intended to work without the UI).
                    note_user_activity();

                    let mut guard = control.lock().await;
                    let preset = guard.active_preset();
                    let setpoint_zero = match preset.mode {
                        LoadMode::Cp => preset.target_p_mw == 0,
                        LoadMode::Cv => preset.target_v_mv == 0,
                        LoadMode::Cc | LoadMode::Reserved(_) => preset.target_i_ma == 0,
                    };

                    if guard.output_enabled {
                        guard.output_enabled = false;
                        bump_control_rev();
                        prompt_tone::enqueue_ui_ok();
                        speaker::enqueue(speaker::SpeakerSound::UiOk);
                        info!(
                            "touch_spring: LOAD ON -> OFF (preset_id={}, mode={:?})",
                            guard.active_preset_id, preset.mode
                        );
                    } else if setpoint_zero {
                        TOUCH_SPRING_ENABLE_BLOCK_TOTAL.fetch_add(1, Ordering::Relaxed);
                        last_touch_block_ms = Some(now);
                        prompt_tone::enqueue_ui_fail();
                        speaker::enqueue(speaker::SpeakerSound::UiFail);
                        info!(
                            "touch_spring: LOAD enable blocked (reason=SETPOINT_ZERO, preset_id={}, mode={:?})",
                            guard.active_preset_id, preset.mode
                        );
                    } else if let Some(reason) = current_load_enable_block_abbrev(preset.min_v_mv) {
                        TOUCH_SPRING_ENABLE_BLOCK_TOTAL.fetch_add(1, Ordering::Relaxed);
                        last_touch_block_ms = Some(now);
                        prompt_tone::enqueue_ui_fail();
                        speaker::enqueue(speaker::SpeakerSound::UiFail);
                        record_enable_block(reason);
                        info!("touch_spring: LOAD enable blocked (reason={})", reason);
                    } else {
                        guard.output_enabled = true;
                        bump_control_rev();
                        prompt_tone::enqueue_ui_ok();
                        speaker::enqueue(speaker::SpeakerSound::UiOk);
                        info!(
                            "touch_spring: LOAD OFF -> ON (preset_id={}, mode={:?})",
                            guard.active_preset_id, preset.mode
                        );
                    }
                }
            }
        } else {
            // Release edge: require the signal to settle back close to the baseline.
            if delta_abs <= up_delta {
                touched = false;
            }
        }

        // Status LED mapping (frozen by Plan #0021):
        // abnormal (yellow blink) > load_enabled=ON (green) > load_enabled=OFF (red).
        let output_enabled = { control.lock().await.output_enabled };
        let abnormal = current_load_block_abbrev().is_some()
            || recent_enable_block_abbrev(now).is_some()
            || last_touch_block_ms
                .map(|t| now.wrapping_sub(t) <= ENABLE_BLOCK_TTL_MS)
                .unwrap_or(false);

        let desired = if abnormal {
            if now.wrapping_sub(last_blink_toggle_ms) >= RGB_STATUS_BLINK_TOGGLE_MS {
                blink_on = !blink_on;
                last_blink_toggle_ms = now;
            }
            if blink_on {
                Rgb {
                    r: RGB_STATUS_BLINK_BRIGHTNESS_PCT,
                    g: RGB_STATUS_BLINK_BRIGHTNESS_PCT,
                    b: 0,
                }
            } else {
                Rgb { r: 0, g: 0, b: 0 }
            }
        } else if output_enabled {
            Rgb {
                r: 0,
                g: RGB_STATUS_SOLID_BRIGHTNESS_PCT,
                b: 0,
            }
        } else {
            Rgb {
                r: RGB_STATUS_SOLID_BRIGHTNESS_PCT,
                g: 0,
                b: 0,
            }
        };

        if desired != last_rgb {
            set_rgb_pct(rgb_r, rgb_g, rgb_b, desired.r, desired.g, desired.b);
            last_rgb = desired;
        }

        if now.wrapping_sub(last_dbg_ms) >= 1000 {
            last_dbg_ms = now;
            let sens = esp_hal::peripherals::SENS::regs();
            let rtccntl = esp_hal::peripherals::LPWR::regs();
            let rtcio = esp_hal::peripherals::RTC_IO::regs();

            let outen = sens.sar_touch_conf().read().sar_touch_outen().bits();
            let unit_done = sens
                .sar_touch_conf()
                .read()
                .sar_touch_unit_end()
                .bit_is_set();
            let saradc_clk_en = sens
                .sar_peri_clk_gate_conf()
                .read()
                .saradc_clk_en()
                .bit_is_set();
            let ctrl2 = rtccntl.touch_ctrl2().read();
            let pad14 = rtcio.touch_pad(14).read();
            let scan_curr = sens
                .sar_touch_scan_status()
                .read()
                .sar_touch_scan_curr()
                .bits();
            let meas_done = sens
                .sar_touch_chn_st()
                .read()
                .sar_touch_meas_done()
                .bit_is_set();
            let pad_active = sens.sar_touch_chn_st().read().sar_touch_pad_active().bits();
            let pad_map = rtccntl.touch_scan_ctrl().read().touch_scan_pad_map().bits();

            let v13 = sens.sar_touch_status13().read().data().bits();
            let v14 = sens.sar_touch_status14().read().data().bits();

            info!(
                "touch_spring dbg: pad_map=0x{:04x}, outen=0x{:04x}, saradc_clk_en={}, ctrl2(start_fsm={}, start_en={}, start_force={}, slp_timer={}, clk_fo={}, clkgate={}, reset={}), pad14(mux_sel={}, start={}, xpd={}), scan_curr={}, unit_done={}, meas_done={}, pad_active=0x{:04x}, status13={}, status14={}",
                pad_map,
                outen,
                saradc_clk_en,
                ctrl2.touch_start_fsm_en().bit_is_set(),
                ctrl2.touch_start_en().bit_is_set(),
                ctrl2.touch_start_force().bit_is_set(),
                ctrl2.touch_slp_timer_en().bit_is_set(),
                ctrl2.touch_clk_fo().bit_is_set(),
                ctrl2.touch_clkgate_en().bit_is_set(),
                ctrl2.touch_reset().bit_is_set(),
                pad14.mux_sel().bit_is_set(),
                pad14.start().bit_is_set(),
                pad14.xpd().bit_is_set(),
                scan_curr,
                unit_done,
                meas_done,
                pad_active,
                v13,
                v14
            );
        }

        cooperative_delay_ms(TOUCH_SPRING_POLL_MS).await;
    }
}

#[embassy_executor::task]
async fn touch_ui_task(
    control: &'static ControlMutex,
    telemetry: &'static TelemetryMutex,
    eeprom: &'static EepromMutex,
) {
    info!("touch-ui task starting (preset entry + quick switch)");
    let mut last_seq: u32 = 0;
    let mut consume_touch_until_up: bool = false;
    #[derive(Copy, Clone)]
    enum ControlRowTouch {
        PresetSwitch {
            start_x: i32,
            base_id: u8,
            dragging: bool,
            preview_id: u8,
            boundary_fail_fired: bool,
            down_ms: u32,
            hold_preview_shown: bool,
        },
        TargetSelect {
            start_x: i32,
            unit: char,
            dragging: bool,
            last_digit: control::AdjustDigit,
            boundary_fail_fired: bool,
        },
        PresetPanelValue {
            start_x: i32,
            field: ui::preset_panel::PresetPanelField,
            dragging: bool,
            boundary_fail_fired: bool,
        },
        PdSettingsValue {
            start_x: i32,
            field: control::PdSettingsFocus,
            dragging: bool,
            boundary_fail_fired: bool,
        },
    }
    let mut quick_switch: Option<ControlRowTouch> = None;
    let mut last_tab_tap: Option<(u8, u32)> = None;
    const DRAG_START_THRESHOLD_PX: i32 = 10;
    const SWIPE_STEP_PX: i32 = 24;
    // Setpoint digit selection should feel like a deliberate left/right swipe.
    // Use a smaller threshold than preset swiping so it works reliably.
    const SETPOINT_SWIPE_STEP_PX: i32 = 14;
    const DOUBLE_TAP_WINDOW_MS: u32 = 350;
    const HOLD_PREVIEW_MS: u32 = 300;

    loop {
        if let Some(ControlRowTouch::PresetSwitch {
            start_x,
            base_id,
            dragging: false,
            preview_id,
            boundary_fail_fired,
            down_ms,
            hold_preview_shown: false,
        }) = quick_switch
        {
            if now_ms32().wrapping_sub(down_ms) >= HOLD_PREVIEW_MS {
                let view = { control.lock().await.ui_view };
                if view == control::UiView::Main {
                    PRESET_PREVIEW_ID.store(preview_id, Ordering::Relaxed);
                    quick_switch = Some(ControlRowTouch::PresetSwitch {
                        start_x,
                        base_id,
                        dragging: false,
                        preview_id,
                        boundary_fail_fired,
                        down_ms,
                        hold_preview_shown: true,
                    });
                }
            }
        }

        let seq = touch::touch_marker_seq();
        if seq == last_seq {
            yield_now().await;
            continue;
        }
        last_seq = seq;

        let Some(marker) = touch::load_touch_marker() else {
            yield_now().await;
            continue;
        };

        // Safety: when the screen is OFF, the first user input is consumed only to wake.
        // For touch gestures, this must include both DOWN/CONTACT and the matching UP.
        if consume_touch_until_up {
            note_user_activity();
            quick_switch = None;
            PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
            last_tab_tap = None;
            if marker.event == 1 {
                consume_touch_until_up = false;
            }
            yield_now().await;
            continue;
        }

        if note_user_activity_and_should_consume_off() {
            consume_touch_until_up = marker.event != 1;
            quick_switch = None;
            PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
            last_tab_tap = None;
            yield_now().await;
            continue;
        }

        let view = { control.lock().await.ui_view };
        match marker.event {
            // down
            0 => {
                if view == control::UiView::PdSettings {
                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                    last_tab_tap = None;
                    let (draft, focus, digit) = {
                        let guard = control.lock().await;
                        (
                            guard.pd_draft,
                            guard.pd_settings_focus,
                            guard.pd_settings_digit,
                        )
                    };
                    let pd_status = { telemetry.lock().await.last_pd_status.clone() };
                    let vm = build_pd_settings_vm(draft, focus, digit, pd_status.as_ref());
                    quick_switch =
                        match ui::pd_settings::hit_test_pd_settings(marker.x, marker.y, &vm) {
                            Some(ui::pd_settings::PdSettingsHit::VreqValue) => {
                                Some(ControlRowTouch::PdSettingsValue {
                                    start_x: marker.x,
                                    field: control::PdSettingsFocus::Vreq,
                                    dragging: false,
                                    boundary_fail_fired: false,
                                })
                            }
                            Some(ui::pd_settings::PdSettingsHit::IreqValue) => {
                                Some(ControlRowTouch::PdSettingsValue {
                                    start_x: marker.x,
                                    field: control::PdSettingsFocus::Ireq,
                                    dragging: false,
                                    boundary_fail_fired: false,
                                })
                            }
                            _ => None,
                        };
                    yield_now().await;
                    continue;
                }
                if view == control::UiView::PresetPanelBlocked {
                    quick_switch = None;
                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                } else {
                    match ui::hit_test_control_row(marker.x, marker.y) {
                        Some(ui::ControlRowHit::PresetEntry) => {
                            let now = now_ms32();
                            let base_id = { control.lock().await.active_preset_id };
                            quick_switch = Some(ControlRowTouch::PresetSwitch {
                                start_x: marker.x,
                                base_id,
                                dragging: false,
                                preview_id: base_id,
                                boundary_fail_fired: false,
                                down_ms: now,
                                hold_preview_shown: false,
                            });
                        }
                        Some(ui::ControlRowHit::TargetEntry) => {
                            let (unit, digit) = {
                                let guard = control.lock().await;
                                let preset = guard.active_preset();
                                let unit = match preset.mode {
                                    LoadMode::Cv => 'V',
                                    LoadMode::Cp => 'W',
                                    LoadMode::Cc | LoadMode::Reserved(_) => 'A',
                                };
                                (unit, guard.adjust_digit)
                            };
                            quick_switch = Some(ControlRowTouch::TargetSelect {
                                start_x: marker.x,
                                unit,
                                dragging: false,
                                last_digit: digit,
                                boundary_fail_fired: false,
                            });
                            PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                        }
                        None => {
                            if view == control::UiView::PresetPanel {
                                let vm = {
                                    let guard = control.lock().await;
                                    build_preset_panel_vm(&guard)
                                };
                                if let Some(ui::preset_panel::PresetPanelHit::FieldValue(field)) =
                                    ui::preset_panel::hit_test_preset_panel(marker.x, marker.y, &vm)
                                {
                                    quick_switch = Some(ControlRowTouch::PresetPanelValue {
                                        start_x: marker.x,
                                        field,
                                        dragging: false,
                                        boundary_fail_fired: false,
                                    });
                                } else {
                                    quick_switch = None;
                                }
                            } else {
                                quick_switch = None;
                            }
                            PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                        }
                    }
                }
            }
            // contact/move
            2 => {
                if view == control::UiView::PresetPanelBlocked {
                    quick_switch = None;
                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                    yield_now().await;
                    continue;
                }

                // The FT6336U can emit "contact" very quickly after "down", and our UI marker transport
                // only retains the latest sample. If touch_ui_task misses the initial down (event=0),
                // treat the first contact as an implicit arm so taps remain reliable.
                if quick_switch.is_none() {
                    match ui::hit_test_control_row(marker.x, marker.y) {
                        Some(ui::ControlRowHit::PresetEntry) => {
                            let now = now_ms32();
                            let base_id = { control.lock().await.active_preset_id };
                            quick_switch = Some(ControlRowTouch::PresetSwitch {
                                start_x: marker.x,
                                base_id,
                                dragging: false,
                                preview_id: base_id,
                                boundary_fail_fired: false,
                                down_ms: now,
                                hold_preview_shown: false,
                            });
                        }
                        Some(ui::ControlRowHit::TargetEntry) => {
                            let (unit, digit) = {
                                let guard = control.lock().await;
                                let preset = guard.active_preset();
                                let unit = match preset.mode {
                                    LoadMode::Cv => 'V',
                                    LoadMode::Cp => 'W',
                                    LoadMode::Cc | LoadMode::Reserved(_) => 'A',
                                };
                                (unit, guard.adjust_digit)
                            };
                            quick_switch = Some(ControlRowTouch::TargetSelect {
                                start_x: marker.x,
                                unit,
                                dragging: false,
                                last_digit: digit,
                                boundary_fail_fired: false,
                            });
                            PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                        }
                        None => {
                            if view == control::UiView::PresetPanel {
                                let vm = {
                                    let guard = control.lock().await;
                                    build_preset_panel_vm(&guard)
                                };
                                if let Some(ui::preset_panel::PresetPanelHit::FieldValue(field)) =
                                    ui::preset_panel::hit_test_preset_panel(marker.x, marker.y, &vm)
                                {
                                    quick_switch = Some(ControlRowTouch::PresetPanelValue {
                                        start_x: marker.x,
                                        field,
                                        dragging: false,
                                        boundary_fail_fired: false,
                                    });
                                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                }

                let Some(action) = quick_switch else {
                    yield_now().await;
                    continue;
                };

                match action {
                    ControlRowTouch::PresetSwitch {
                        start_x,
                        base_id,
                        dragging,
                        preview_id: last_preview_id,
                        boundary_fail_fired,
                        down_ms,
                        hold_preview_shown,
                    } => {
                        // Only support control-row drag gestures in the main Dashboard view.
                        if view != control::UiView::Main {
                            yield_now().await;
                            continue;
                        }
                        let dx = marker.x - start_x;
                        let now_dragging = dragging || dx.abs() >= DRAG_START_THRESHOLD_PX;
                        if !now_dragging {
                            yield_now().await;
                            continue;
                        }

                        let delta = dx / SWIPE_STEP_PX;
                        let raw_preview = base_id as i32 + delta;
                        let clamped_preview = raw_preview.clamp(1, control::PRESET_COUNT as i32);
                        let preview_id = clamped_preview as u8;
                        let attempted_oob = raw_preview != clamped_preview;

                        // If the user keeps dragging past the boundary, re-anchor the gesture so
                        // returning by one detent distance moves exactly one preset again.
                        // Otherwise, the extra overscroll distance would "stack up" and require a
                        // much larger movement to leave the boundary.
                        let mut next_start_x = start_x;
                        if attempted_oob {
                            let clamped_delta = clamped_preview - base_id as i32;
                            next_start_x = marker.x - clamped_delta * SWIPE_STEP_PX;
                        }

                        let mut next_boundary_fail_fired = boundary_fail_fired;
                        let mut need_state_update = false;

                        if attempted_oob && !next_boundary_fail_fired {
                            prompt_tone::enqueue_ui_fail();
                            next_boundary_fail_fired = true;
                            need_state_update = true;
                        }

                        if attempted_oob {
                            // Update the anchored start position even if fail has already fired.
                            need_state_update = true;
                        }

                        if preview_id != last_preview_id {
                            let steps = (preview_id as i32 - last_preview_id as i32).abs() as u32;
                            prompt_tone::enqueue_ticks(steps);
                            need_state_update = true;
                        }

                        if !dragging {
                            // Entered drag state but preset index may not have moved yet.
                            need_state_update = true;
                        }

                        if need_state_update {
                            PRESET_PREVIEW_ID.store(preview_id, Ordering::Relaxed);
                            quick_switch = Some(ControlRowTouch::PresetSwitch {
                                start_x: next_start_x,
                                base_id,
                                dragging: true,
                                preview_id,
                                boundary_fail_fired: next_boundary_fail_fired,
                                down_ms,
                                hold_preview_shown,
                            });
                        }
                    }
                    ControlRowTouch::TargetSelect {
                        start_x,
                        unit,
                        dragging,
                        last_digit,
                        boundary_fail_fired,
                    } => {
                        // Only support control-row drag gestures in the main Dashboard view.
                        if view != control::UiView::Main {
                            yield_now().await;
                            continue;
                        }
                        let dx = marker.x - start_x;
                        let now_dragging = dragging || dx.abs() >= SETPOINT_SWIPE_STEP_PX;
                        if !now_dragging {
                            yield_now().await;
                            continue;
                        }

                        let mut next_boundary_fail_fired = boundary_fail_fired;
                        let mut next_digit = last_digit;
                        let mut need_state_update = false;

                        // Setpoint digit selection: recognize a single left/right swipe per gesture.
                        // Once the swipe is consumed (dragging=true), ignore further motion until release.
                        if !dragging {
                            let dir = if dx > 0 { 1 } else { -1 };
                            let cur_rank = match last_digit {
                                control::AdjustDigit::Ones => 0,
                                control::AdjustDigit::Tenths => 1,
                                control::AdjustDigit::Hundredths => 2,
                                control::AdjustDigit::Thousandths => 3,
                            };
                            let raw_rank = cur_rank + dir;
                            let attempted_oob = raw_rank < 0 || raw_rank > 3;

                            if attempted_oob {
                                if !next_boundary_fail_fired {
                                    prompt_tone::enqueue_ui_fail();
                                    next_boundary_fail_fired = true;
                                }
                            } else {
                                let pick_digit = match raw_rank {
                                    0 => control::AdjustDigit::Ones,
                                    1 => control::AdjustDigit::Tenths,
                                    2 => control::AdjustDigit::Hundredths,
                                    _ => control::AdjustDigit::Thousandths,
                                };
                                if pick_digit != last_digit {
                                    prompt_tone::enqueue_ticks(1);
                                    let mut guard = control.lock().await;
                                    guard.adjust_digit = pick_digit;
                                    bump_control_rev();
                                    next_digit = pick_digit;
                                }
                            }

                            need_state_update = true;
                        }

                        if need_state_update {
                            quick_switch = Some(ControlRowTouch::TargetSelect {
                                start_x,
                                unit,
                                dragging: true,
                                last_digit: next_digit,
                                boundary_fail_fired: next_boundary_fail_fired,
                            });
                        }
                    }
                    ControlRowTouch::PdSettingsValue {
                        start_x,
                        field,
                        dragging,
                        boundary_fail_fired,
                    } => {
                        if view != control::UiView::PdSettings {
                            yield_now().await;
                            continue;
                        }

                        let dx = marker.x - start_x;
                        let now_dragging = dragging || dx.abs() >= SETPOINT_SWIPE_STEP_PX;
                        if !now_dragging {
                            yield_now().await;
                            continue;
                        }

                        let mut next_boundary_fail_fired = boundary_fail_fired;

                        if !dragging {
                            let dir = if dx > 0 { 1 } else { -1 };

                            let mut guard = control.lock().await;
                            let prev_focus = guard.pd_settings_focus;
                            let prev_digit = guard.pd_settings_digit;

                            if field == control::PdSettingsFocus::Vreq
                                && guard.pd_draft.mode != control::PdMode::Pps
                            {
                                if !next_boundary_fail_fired {
                                    prompt_tone::enqueue_ui_fail();
                                    next_boundary_fail_fired = true;
                                }
                            } else {
                                guard.pd_settings_focus = field;

                                let cur_rank = match guard.pd_settings_digit {
                                    control::AdjustDigit::Ones => 0,
                                    control::AdjustDigit::Tenths => 1,
                                    control::AdjustDigit::Hundredths => 2,
                                    _ => 1,
                                };
                                let raw_rank = cur_rank + dir;
                                let attempted_oob = raw_rank < 0 || raw_rank > 2;
                                if attempted_oob {
                                    if !next_boundary_fail_fired {
                                        prompt_tone::enqueue_ui_fail();
                                        next_boundary_fail_fired = true;
                                    }
                                } else {
                                    let next_digit = match raw_rank {
                                        0 => control::AdjustDigit::Ones,
                                        1 => control::AdjustDigit::Tenths,
                                        _ => control::AdjustDigit::Hundredths,
                                    };
                                    if next_digit != guard.pd_settings_digit {
                                        guard.pd_settings_digit = next_digit;
                                        prompt_tone::enqueue_ticks(1);
                                    }
                                }

                                if guard.pd_settings_focus != prev_focus
                                    || guard.pd_settings_digit != prev_digit
                                {
                                    bump_control_rev();
                                }
                            }
                        }

                        quick_switch = Some(ControlRowTouch::PdSettingsValue {
                            start_x,
                            field,
                            dragging: true,
                            boundary_fail_fired: next_boundary_fail_fired,
                        });
                    }
                    ControlRowTouch::PresetPanelValue {
                        start_x,
                        field,
                        dragging,
                        boundary_fail_fired,
                    } => {
                        if view != control::UiView::PresetPanel {
                            yield_now().await;
                            continue;
                        }

                        let dx = marker.x - start_x;
                        let now_dragging = dragging || dx.abs() >= SETPOINT_SWIPE_STEP_PX;
                        if !now_dragging {
                            yield_now().await;
                            continue;
                        }

                        let mut next_boundary_fail_fired = boundary_fail_fired;
                        let mut state_changed = false;

                        // Recognize a single left/right swipe per gesture.
                        // Once consumed (dragging=true), ignore further motion until release.
                        if !dragging {
                            let dir = if dx > 0 { 1 } else { -1 };

                            let mut guard = control.lock().await;
                            let prev_field = guard.panel_selected_field;
                            let prev_digit = guard.panel_selected_digit;

                            guard.panel_selected_field = field;
                            let idx = guard.editing_preset_id.saturating_sub(1) as usize;
                            let mode = guard
                                .presets
                                .get(idx)
                                .map(|p| match p.mode {
                                    LoadMode::Cv => LoadMode::Cv,
                                    LoadMode::Cp => LoadMode::Cp,
                                    _ => LoadMode::Cc,
                                })
                                .unwrap_or(LoadMode::Cc);
                            let cur_digit = coerce_panel_digit_for_field(
                                mode,
                                field,
                                guard.panel_selected_digit,
                            );
                            guard.panel_selected_digit = cur_digit;
                            if prev_field != field || prev_digit != cur_digit {
                                state_changed = true;
                            }

                            let (next_digit, attempted_oob) =
                                shift_panel_digit_once(mode, field, cur_digit, dir);
                            if attempted_oob {
                                if !next_boundary_fail_fired {
                                    prompt_tone::enqueue_ui_fail();
                                    next_boundary_fail_fired = true;
                                }
                            } else if next_digit != cur_digit {
                                prompt_tone::enqueue_ticks(1);
                                guard.panel_selected_digit = next_digit;
                                state_changed = true;
                            }

                            if state_changed {
                                bump_control_rev();
                            }
                        }

                        quick_switch = Some(ControlRowTouch::PresetPanelValue {
                            start_x,
                            field,
                            dragging: true,
                            boundary_fail_fired: next_boundary_fail_fired,
                        });
                    }
                }
            }
            // up
            1 => {
                if view == control::UiView::PresetPanelBlocked {
                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                    quick_switch = None;
                    last_tab_tap = None;

                    let vm = {
                        let guard = control.lock().await;
                        build_preset_panel_vm(&guard)
                    };
                    if let Some(ui::preset_panel::PresetPanelHit::Save) =
                        ui::preset_panel::hit_test_preset_panel(marker.x, marker.y, &vm)
                    {
                        let ok = save_editing_preset_to_eeprom(control, eeprom).await;
                        if ok {
                            prompt_tone::enqueue_ui_ok();
                        } else {
                            prompt_tone::enqueue_ui_fail();
                        }
                    }
                    yield_now().await;
                    continue;
                }

                let Some(action) = quick_switch.take() else {
                    // Fallback: if we missed both down and contact arming, infer a tap from the up
                    // location for the control row. This keeps the Preset pill responsive even under
                    // marker sample loss.
                    if matches!(view, control::UiView::Main | control::UiView::PresetPanel) {
                        if let Some(hit) = ui::hit_test_control_row(marker.x, marker.y) {
                            match hit {
                                ui::ControlRowHit::PresetEntry => match view {
                                    control::UiView::Main => {
                                        let mut guard = control.lock().await;
                                        guard.ui_view = control::UiView::PresetPanel;
                                        guard.editing_preset_id = guard.active_preset_id;
                                        guard.panel_selected_field =
                                            ui::preset_panel::PresetPanelField::Target;
                                        guard.panel_selected_digit =
                                            ui::preset_panel::PresetPanelDigit::Tenths;
                                        bump_control_rev();
                                        prompt_tone::enqueue_ui_ok();
                                        info!(
                                            "touch: preset entry tap (fallback) -> open preset panel (editing preset_id={})",
                                            guard.editing_preset_id
                                        );
                                    }
                                    control::UiView::PresetPanel => {
                                        let mut guard = control.lock().await;
                                        guard.close_panel_discard();
                                        guard.ui_view = control::UiView::Main;
                                        bump_control_rev();
                                        prompt_tone::enqueue_ui_ok();
                                        info!(
                                            "touch: preset entry tap (fallback) -> close preset panel (discard non-active)"
                                        );
                                    }
                                    control::UiView::PresetPanelBlocked => {}
                                    control::UiView::PdSettings => {}
                                },
                                ui::ControlRowHit::TargetEntry => {
                                    if view == control::UiView::Main {
                                        let mut guard = control.lock().await;
                                        let preset = guard.active_preset();
                                        let mode = match preset.mode {
                                            LoadMode::Cv => LoadMode::Cv,
                                            LoadMode::Cp => LoadMode::Cp,
                                            _ => LoadMode::Cc,
                                        };
                                        let unit = match mode {
                                            LoadMode::Cv => 'V',
                                            LoadMode::Cp => 'W',
                                            _ => 'A',
                                        };
                                        let pick =
                                            ui::pick_control_row_setpoint_digit(marker.x, unit);
                                        let digit = coerce_adjust_digit_for_mode(mode, pick.digit);
                                        if digit != guard.adjust_digit {
                                            guard.adjust_digit = digit;
                                            bump_control_rev();
                                            prompt_tone::enqueue_ui_ok();
                                            info!(
                                                "touch: setpoint entry tap (fallback) -> select adjust_digit ({:?})",
                                                guard.adjust_digit
                                            );
                                        }
                                    }
                                }
                            }

                            PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                            last_tab_tap = None;
                            yield_now().await;
                            continue;
                        }
                    }

                    if view == control::UiView::Main
                        && ui::hit_test_dashboard_pd_button(marker.x, marker.y)
                    {
                        let mut guard = control.lock().await;
                        guard.ui_view = control::UiView::PdSettings;
                        guard.pd_draft = guard.pd_saved;
                        guard.pd_settings_focus = control::PdSettingsFocus::DEFAULT;
                        guard.pd_settings_digit = control::AdjustDigit::Tenths;
                        bump_control_rev();
                        prompt_tone::enqueue_ui_ok();
                        PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                        last_tab_tap = None;
                        info!("touch: PD button tap -> open PD settings panel");
                        yield_now().await;
                        continue;
                    }

                    if view == control::UiView::PdSettings {
                        let (draft, focus, digit) = {
                            let guard = control.lock().await;
                            (
                                guard.pd_draft,
                                guard.pd_settings_focus,
                                guard.pd_settings_digit,
                            )
                        };
                        let pd_status = { telemetry.lock().await.last_pd_status.clone() };
                        let vm = build_pd_settings_vm(draft, focus, digit, pd_status.as_ref());

                        if let Some(hit) =
                            ui::pd_settings::hit_test_pd_settings(marker.x, marker.y, &vm)
                        {
                            match hit {
                                ui::pd_settings::PdSettingsHit::Back => {
                                    let mut guard = control.lock().await;
                                    guard.ui_view = control::UiView::Main;
                                    guard.pd_draft = guard.pd_saved;
                                    guard.pd_settings_focus = control::PdSettingsFocus::DEFAULT;
                                    guard.pd_settings_digit = control::AdjustDigit::Tenths;
                                    bump_control_rev();
                                    prompt_tone::enqueue_ui_ok();
                                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                    last_tab_tap = None;
                                    info!("touch: PD settings Back -> main");
                                    yield_now().await;
                                    continue;
                                }
                                ui::pd_settings::PdSettingsHit::Apply => {
                                    if !vm.apply_enabled {
                                        prompt_tone::enqueue_ui_fail();
                                        PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                        last_tab_tap = None;
                                        yield_now().await;
                                        continue;
                                    }

                                    // Apply = persist + update active PD policy.
                                    let apply_ms = now_ms32();
                                    PD_UI_APPLY_MS.store(apply_ms, Ordering::Relaxed);
                                    PD_FORCE_SEND.store(true, Ordering::Release);

                                    let (changed, cfg) = {
                                        let mut guard = control.lock().await;
                                        let changed = guard.pd_saved != guard.pd_draft;
                                        guard.pd_saved = guard.pd_draft;
                                        (changed, guard.pd_saved)
                                    };
                                    if changed {
                                        bump_control_rev();
                                        let ok = save_pd_config_to_eeprom(cfg, eeprom).await;
                                        if ok {
                                            prompt_tone::enqueue_ui_ok();
                                        } else {
                                            prompt_tone::enqueue_ui_fail();
                                            PD_LAST_RESULT_CODE.store(4, Ordering::Relaxed); // persist fail
                                            PD_LAST_RESULT_MS.store(apply_ms, Ordering::Relaxed);
                                        }
                                    } else {
                                        prompt_tone::enqueue_ui_ok();
                                    }
                                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                    last_tab_tap = None;
                                    yield_now().await;
                                    continue;
                                }
                                ui::pd_settings::PdSettingsHit::ModeFixed => {
                                    let mut guard = control.lock().await;
                                    if guard.pd_draft.mode != control::PdMode::Fixed {
                                        guard.pd_draft.mode = control::PdMode::Fixed;
                                        if guard.pd_settings_focus == control::PdSettingsFocus::Vreq
                                        {
                                            guard.pd_settings_focus =
                                                control::PdSettingsFocus::Ireq;
                                            guard.pd_settings_digit = control::AdjustDigit::Tenths;
                                        }
                                        // If we have capabilities, snap to a valid Fixed selection.
                                        if let Some((idx0, pdo0)) =
                                            vm.fixed_pdos.iter().enumerate().next()
                                        {
                                            let pdo_pos = |pos: u8, idx: usize| -> u8 {
                                                if pos != 0 {
                                                    pos
                                                } else {
                                                    (idx + 1).min(u8::MAX as usize) as u8
                                                }
                                            };

                                            let desired_pos =
                                                if guard.pd_draft.fixed_object_pos != 0 {
                                                    guard.pd_draft.fixed_object_pos
                                                } else {
                                                    vm.fixed_pdos
                                                        .iter()
                                                        .enumerate()
                                                        .find(|(_idx, p)| {
                                                            p.mv == guard.pd_draft.target_mv
                                                        })
                                                        .map(|(idx, p)| pdo_pos(p.pos, idx))
                                                        .unwrap_or(0)
                                                };

                                            let (pos, pdo) = vm
                                                .fixed_pdos
                                                .iter()
                                                .enumerate()
                                                .find(|(idx, p)| {
                                                    pdo_pos(p.pos, *idx) == desired_pos
                                                })
                                                .map(|(idx, p)| (pdo_pos(p.pos, idx), *p))
                                                .unwrap_or_else(|| {
                                                    (pdo_pos(pdo0.pos, idx0), *pdo0)
                                                });

                                            guard.pd_draft.fixed_object_pos = pos;
                                            guard.pd_draft.i_req_ma =
                                                guard.pd_draft.i_req_ma.min(pdo.max_ma).max(50);
                                        }
                                        bump_control_rev();
                                    }
                                    prompt_tone::enqueue_ui_ok();
                                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                    last_tab_tap = None;
                                    yield_now().await;
                                    continue;
                                }
                                ui::pd_settings::PdSettingsHit::ModePps => {
                                    let mut guard = control.lock().await;
                                    if guard.pd_draft.mode != control::PdMode::Pps {
                                        guard.pd_draft.mode = control::PdMode::Pps;
                                        // PPS requires an explicit APDO selection: do not auto-pick.
                                        if guard.pd_draft.pps_object_pos != 0 {
                                            let pdo_pos = |pos: u8, idx: usize| -> u8 {
                                                if pos != 0 {
                                                    pos
                                                } else {
                                                    (idx + 1).min(u8::MAX as usize) as u8
                                                }
                                            };
                                            if let Some(apdo) = vm
                                                .pps_pdos
                                                .iter()
                                                .enumerate()
                                                .find(|(idx, p)| {
                                                    pdo_pos(p.pos, *idx)
                                                        == guard.pd_draft.pps_object_pos
                                                })
                                                .map(|(_idx, p)| *p)
                                            {
                                                if guard.pd_draft.target_mv == 0 {
                                                    guard.pd_draft.target_mv = apdo.min_mv;
                                                }
                                                guard.pd_draft.target_mv = guard
                                                    .pd_draft
                                                    .target_mv
                                                    .clamp(apdo.min_mv, apdo.max_mv);
                                                guard.pd_draft.i_req_ma = guard
                                                    .pd_draft
                                                    .i_req_ma
                                                    .min(apdo.max_ma)
                                                    .max(50);
                                            }
                                        }
                                        bump_control_rev();
                                    }
                                    prompt_tone::enqueue_ui_ok();
                                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                    last_tab_tap = None;
                                    yield_now().await;
                                    continue;
                                }
                                ui::pd_settings::PdSettingsHit::VreqValue => {
                                    let mut guard = control.lock().await;
                                    if guard.pd_draft.mode != control::PdMode::Pps {
                                        prompt_tone::enqueue_ui_fail();
                                    } else {
                                        guard.pd_settings_focus = control::PdSettingsFocus::Vreq;
                                        guard.pd_settings_digit = ui::pd_settings::pick_value_digit(
                                            control::PdSettingsFocus::Vreq,
                                            marker.x,
                                            guard.pd_draft.mode,
                                        );
                                        bump_control_rev();
                                        prompt_tone::enqueue_ui_ok();
                                    }
                                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                    last_tab_tap = None;
                                    yield_now().await;
                                    continue;
                                }
                                ui::pd_settings::PdSettingsHit::IreqValue => {
                                    let mut guard = control.lock().await;
                                    guard.pd_settings_focus = control::PdSettingsFocus::Ireq;
                                    guard.pd_settings_digit = ui::pd_settings::pick_value_digit(
                                        control::PdSettingsFocus::Ireq,
                                        marker.x,
                                        guard.pd_draft.mode,
                                    );
                                    bump_control_rev();
                                    prompt_tone::enqueue_ui_ok();
                                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                    last_tab_tap = None;
                                    yield_now().await;
                                    continue;
                                }
                                ui::pd_settings::PdSettingsHit::ListRow(idx) => {
                                    let mut guard = control.lock().await;
                                    match guard.pd_draft.mode {
                                        control::PdMode::Fixed => {
                                            if let Some(pdo) = vm.fixed_pdos.get(idx).copied() {
                                                let pos = if pdo.pos != 0 {
                                                    pdo.pos
                                                } else {
                                                    (idx + 1).min(u8::MAX as usize) as u8
                                                };
                                                guard.pd_draft.fixed_object_pos = pos;
                                                guard.pd_draft.i_req_ma =
                                                    guard.pd_draft.i_req_ma.min(pdo.max_ma).max(50);
                                                bump_control_rev();
                                                prompt_tone::enqueue_ui_ok();
                                            } else {
                                                prompt_tone::enqueue_ui_fail();
                                            }
                                        }
                                        control::PdMode::Pps => {
                                            if let Some(apdo) = vm.pps_pdos.get(idx).copied() {
                                                let pos = if apdo.pos != 0 {
                                                    apdo.pos
                                                } else {
                                                    (idx + 1).min(u8::MAX as usize) as u8
                                                };
                                                guard.pd_draft.pps_object_pos = pos;
                                                if guard.pd_draft.target_mv == 0 {
                                                    guard.pd_draft.target_mv = apdo.min_mv;
                                                }
                                                guard.pd_draft.target_mv = guard
                                                    .pd_draft
                                                    .target_mv
                                                    .clamp(apdo.min_mv, apdo.max_mv);
                                                guard.pd_draft.i_req_ma = guard
                                                    .pd_draft
                                                    .i_req_ma
                                                    .min(apdo.max_ma)
                                                    .max(50);
                                                bump_control_rev();
                                                prompt_tone::enqueue_ui_ok();
                                            } else {
                                                prompt_tone::enqueue_ui_fail();
                                            }
                                        }
                                    }
                                    PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                                    last_tab_tap = None;
                                    yield_now().await;
                                    continue;
                                }
                            }
                        }
                    }

                    if view == control::UiView::Main
                        && ui::hit_test_dashboard_load_button(marker.x, marker.y)
                    {
                        let mut guard = control.lock().await;
                        if guard.output_enabled {
                            guard.output_enabled = false;
                            bump_control_rev();
                            prompt_tone::enqueue_ui_ok();
                        } else if let Some(reason) =
                            current_load_enable_block_abbrev(guard.active_preset().min_v_mv)
                        {
                            prompt_tone::enqueue_ui_fail();
                            record_enable_block(reason);
                            info!("touch: LOAD enable blocked (reason={})", reason);
                        } else {
                            guard.output_enabled = true;
                            bump_control_rev();
                            prompt_tone::enqueue_ui_ok();
                        }
                        PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                        last_tab_tap = None;
                        yield_now().await;
                        continue;
                    }

                    if view == control::UiView::PresetPanel {
                        let vm = {
                            let guard = control.lock().await;
                            build_preset_panel_vm(&guard)
                        };
                        if let Some(hit) =
                            ui::preset_panel::hit_test_preset_panel(marker.x, marker.y, &vm)
                        {
                            use ui::preset_panel::PresetPanelField as Field;
                            use ui::preset_panel::PresetPanelHit as Hit;

                            match hit {
                                Hit::Tab(preset_id) => {
                                    let now = now_ms32();
                                    let mut guard = control.lock().await;
                                    if preset_id != guard.editing_preset_id {
                                        guard.editing_preset_id = preset_id;
                                        let idx = preset_id.saturating_sub(1) as usize;
                                        if let Some(p) = guard.presets.get(idx).copied() {
                                            let mode = match p.mode {
                                                LoadMode::Cv => LoadMode::Cv,
                                                LoadMode::Cp => LoadMode::Cp,
                                                LoadMode::Cc | LoadMode::Reserved(_) => {
                                                    LoadMode::Cc
                                                }
                                            };
                                            guard.panel_selected_field =
                                                coerce_panel_field_for_mode(
                                                    mode,
                                                    guard.panel_selected_field,
                                                );
                                        }
                                        let mode = match guard
                                            .presets
                                            .get(idx)
                                            .map(|p| p.mode)
                                            .unwrap_or(LoadMode::Cc)
                                        {
                                            LoadMode::Cv => LoadMode::Cv,
                                            LoadMode::Cp => LoadMode::Cp,
                                            _ => LoadMode::Cc,
                                        };
                                        guard.panel_selected_digit = coerce_panel_digit_for_field(
                                            mode,
                                            guard.panel_selected_field,
                                            guard.panel_selected_digit,
                                        );
                                        bump_control_rev();
                                        prompt_tone::enqueue_ui_ok();
                                        last_tab_tap = None;
                                    } else if let Some((last_id, last_ms)) = last_tab_tap {
                                        if last_id == preset_id
                                            && now.wrapping_sub(last_ms) <= DOUBLE_TAP_WINDOW_MS
                                        {
                                            guard.activate_preset(preset_id);
                                            bump_control_rev();
                                            prompt_tone::enqueue_ui_ok();
                                            info!(
                                                "touch: tab double-tap activate preset {} (output forced OFF)",
                                                preset_id
                                            );
                                            last_tab_tap = None;
                                        } else {
                                            last_tab_tap = Some((preset_id, now));
                                        }
                                    } else {
                                        last_tab_tap = Some((preset_id, now));
                                    }
                                }
                                Hit::ModeCv | Hit::ModeCc | Hit::ModeCp => {
                                    let mode = match hit {
                                        Hit::ModeCv => LoadMode::Cv,
                                        Hit::ModeCp => LoadMode::Cp,
                                        _ => LoadMode::Cc,
                                    };
                                    let mut guard = control.lock().await;
                                    let preset_id = guard.editing_preset_id;
                                    let idx = preset_id.saturating_sub(1) as usize;
                                    let prev = guard
                                        .presets
                                        .get(idx)
                                        .map(|p| p.mode)
                                        .unwrap_or(LoadMode::Cc);
                                    guard.set_mode_for_editing_preset(mode);
                                    guard.panel_selected_field = Field::Mode;
                                    let next =
                                        guard.presets.get(idx).map(|p| p.mode).unwrap_or(prev);
                                    if next != prev {
                                        guard.panel_selected_field = coerce_panel_field_for_mode(
                                            mode,
                                            guard.panel_selected_field,
                                        );
                                        let mode = match next {
                                            LoadMode::Cv => LoadMode::Cv,
                                            LoadMode::Cp => LoadMode::Cp,
                                            _ => LoadMode::Cc,
                                        };
                                        guard.panel_selected_digit = coerce_panel_digit_for_field(
                                            mode,
                                            guard.panel_selected_field,
                                            guard.panel_selected_digit,
                                        );
                                        bump_control_rev();
                                    } else {
                                        bump_control_rev();
                                    }
                                    prompt_tone::enqueue_ui_ok();
                                    last_tab_tap = None;
                                }
                                Hit::FieldLabel(field) => {
                                    let mut guard = control.lock().await;
                                    let prev_field = guard.panel_selected_field;
                                    let prev_digit = guard.panel_selected_digit;
                                    guard.panel_selected_field = field;
                                    let idx = guard.editing_preset_id.saturating_sub(1) as usize;
                                    let mode = match guard
                                        .presets
                                        .get(idx)
                                        .map(|p| p.mode)
                                        .unwrap_or(LoadMode::Cc)
                                    {
                                        LoadMode::Cv => LoadMode::Cv,
                                        LoadMode::Cp => LoadMode::Cp,
                                        _ => LoadMode::Cc,
                                    };
                                    guard.panel_selected_digit = coerce_panel_digit_for_field(
                                        mode,
                                        guard.panel_selected_field,
                                        guard.panel_selected_digit,
                                    );
                                    let changed = guard.panel_selected_field != prev_field
                                        || guard.panel_selected_digit != prev_digit;
                                    if changed {
                                        bump_control_rev();
                                        prompt_tone::enqueue_ui_ok();
                                    }
                                    last_tab_tap = None;
                                }
                                Hit::FieldValue(field) => {
                                    let mut guard = control.lock().await;
                                    let prev_field = guard.panel_selected_field;
                                    let prev_digit = guard.panel_selected_digit;

                                    guard.panel_selected_field = field;
                                    let unit = match field {
                                        Field::Target => {
                                            let idx =
                                                guard.editing_preset_id.saturating_sub(1) as usize;
                                            let mode = guard
                                                .presets
                                                .get(idx)
                                                .map(|p| match p.mode {
                                                    LoadMode::Cv => LoadMode::Cv,
                                                    LoadMode::Cp => LoadMode::Cp,
                                                    _ => LoadMode::Cc,
                                                })
                                                .unwrap_or(LoadMode::Cc);
                                            match mode {
                                                LoadMode::Cv => 'V',
                                                LoadMode::Cp => 'W',
                                                _ => 'A',
                                            }
                                        }
                                        Field::VLim => 'V',
                                        Field::ILim => 'A',
                                        Field::PLim => 'W',
                                        Field::Mode => ' ',
                                    };
                                    let pick_digit =
                                        ui::preset_panel::pick_value_digit(field, marker.x, unit);
                                    let idx = guard.editing_preset_id.saturating_sub(1) as usize;
                                    let mode = guard
                                        .presets
                                        .get(idx)
                                        .map(|p| match p.mode {
                                            LoadMode::Cv => LoadMode::Cv,
                                            LoadMode::Cp => LoadMode::Cp,
                                            _ => LoadMode::Cc,
                                        })
                                        .unwrap_or(LoadMode::Cc);
                                    guard.panel_selected_digit =
                                        coerce_panel_digit_for_field(mode, field, pick_digit);
                                    let changed = guard.panel_selected_field != prev_field
                                        || guard.panel_selected_digit != prev_digit;
                                    if changed {
                                        bump_control_rev();
                                        prompt_tone::enqueue_ui_ok();
                                    }
                                    last_tab_tap = None;
                                }
                                Hit::LoadToggle => {
                                    let mut guard = control.lock().await;
                                    if guard.output_enabled {
                                        guard.output_enabled = false;
                                        bump_control_rev();
                                        prompt_tone::enqueue_ui_ok();
                                    } else if let Some(reason) = current_load_enable_block_abbrev(
                                        guard.active_preset().min_v_mv,
                                    ) {
                                        prompt_tone::enqueue_ui_fail();
                                        record_enable_block(reason);
                                        info!("touch: LOAD enable blocked (reason={})", reason);
                                    } else {
                                        guard.output_enabled = true;
                                        bump_control_rev();
                                        prompt_tone::enqueue_ui_ok();
                                    }
                                    last_tab_tap = None;
                                }
                                Hit::Save => {
                                    last_tab_tap = None;
                                    let dirty = {
                                        let guard = control.lock().await;
                                        let idx =
                                            guard.editing_preset_id.saturating_sub(1) as usize;
                                        guard.dirty.get(idx).copied().unwrap_or(false)
                                    };
                                    if !dirty {
                                        prompt_tone::enqueue_ui_fail();
                                    } else {
                                        let ok =
                                            save_editing_preset_to_eeprom(control, eeprom).await;
                                        if ok {
                                            prompt_tone::enqueue_ui_ok();
                                        } else {
                                            prompt_tone::enqueue_ui_fail();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    yield_now().await;
                    continue;
                };

                match action {
                    ControlRowTouch::PresetSwitch {
                        base_id,
                        dragging,
                        preview_id,
                        hold_preview_shown,
                        ..
                    } => {
                        if dragging {
                            if preview_id != base_id {
                                let mut guard = control.lock().await;
                                guard.activate_preset(preview_id);
                                bump_control_rev();
                                info!(
                                    "touch: quick switch preset {} -> {} (output forced OFF)",
                                    base_id, preview_id
                                );
                            }
                        } else if view == control::UiView::Main
                            && hold_preview_shown
                            && preview_id == base_id
                        {
                            info!(
                                "touch: preset entry hold-preview release -> noop (preset_id={})",
                                base_id
                            );
                        } else {
                            match view {
                                control::UiView::Main => {
                                    let mut guard = control.lock().await;
                                    guard.ui_view = control::UiView::PresetPanel;
                                    guard.editing_preset_id = guard.active_preset_id;
                                    guard.panel_selected_field =
                                        ui::preset_panel::PresetPanelField::Target;
                                    guard.panel_selected_digit =
                                        ui::preset_panel::PresetPanelDigit::Tenths;
                                    bump_control_rev();
                                    prompt_tone::enqueue_ui_ok();
                                    info!(
                                        "touch: preset entry tap -> open preset panel (editing preset_id={})",
                                        guard.editing_preset_id
                                    );
                                }
                                control::UiView::PresetPanel => {
                                    let mut guard = control.lock().await;
                                    guard.close_panel_discard();
                                    guard.ui_view = control::UiView::Main;
                                    bump_control_rev();
                                    prompt_tone::enqueue_ui_ok();
                                    info!(
                                        "touch: preset entry tap -> close preset panel (discard non-active)"
                                    );
                                }
                                control::UiView::PresetPanelBlocked => {}
                                control::UiView::PdSettings => {}
                            }
                        }
                    }
                    ControlRowTouch::TargetSelect {
                        unit,
                        dragging,
                        last_digit,
                        ..
                    } => {
                        if view == control::UiView::Main && !dragging {
                            let pick = ui::pick_control_row_setpoint_digit(marker.x, unit);
                            if pick.digit != last_digit {
                                let mut guard = control.lock().await;
                                guard.adjust_digit = pick.digit;
                                bump_control_rev();
                                prompt_tone::enqueue_ui_ok();
                                info!(
                                    "touch: setpoint entry tap -> select adjust_digit ({:?})",
                                    guard.adjust_digit
                                );
                            }
                        }
                    }
                    ControlRowTouch::PdSettingsValue {
                        field, dragging, ..
                    } => {
                        if view == control::UiView::PdSettings && !dragging {
                            let mut guard = control.lock().await;
                            guard.pd_settings_focus = field;
                            guard.pd_settings_digit = ui::pd_settings::pick_value_digit(
                                field,
                                marker.x,
                                guard.pd_draft.mode,
                            );
                            bump_control_rev();
                            prompt_tone::enqueue_ui_ok();
                        }
                    }
                    ControlRowTouch::PresetPanelValue {
                        field, dragging, ..
                    } => {
                        if view == control::UiView::PresetPanel && !dragging {
                            use ui::preset_panel::PresetPanelField as Field;

                            let mut guard = control.lock().await;
                            let prev_field = guard.panel_selected_field;
                            let prev_digit = guard.panel_selected_digit;

                            guard.panel_selected_field = field;
                            let unit = match field {
                                Field::Target => {
                                    let idx = guard.editing_preset_id.saturating_sub(1) as usize;
                                    let mode = guard
                                        .presets
                                        .get(idx)
                                        .map(|p| match p.mode {
                                            LoadMode::Cv => LoadMode::Cv,
                                            LoadMode::Cp => LoadMode::Cp,
                                            _ => LoadMode::Cc,
                                        })
                                        .unwrap_or(LoadMode::Cc);
                                    match mode {
                                        LoadMode::Cv => 'V',
                                        LoadMode::Cp => 'W',
                                        _ => 'A',
                                    }
                                }
                                Field::VLim => 'V',
                                Field::ILim => 'A',
                                Field::PLim => 'W',
                                Field::Mode => ' ',
                            };
                            let pick_digit =
                                ui::preset_panel::pick_value_digit(field, marker.x, unit);
                            let idx = guard.editing_preset_id.saturating_sub(1) as usize;
                            let mode = guard
                                .presets
                                .get(idx)
                                .map(|p| match p.mode {
                                    LoadMode::Cv => LoadMode::Cv,
                                    LoadMode::Cp => LoadMode::Cp,
                                    _ => LoadMode::Cc,
                                })
                                .unwrap_or(LoadMode::Cc);
                            guard.panel_selected_digit =
                                coerce_panel_digit_for_field(mode, field, pick_digit);
                            let changed = guard.panel_selected_field != prev_field
                                || guard.panel_selected_digit != prev_digit;
                            if changed {
                                bump_control_rev();
                                prompt_tone::enqueue_ui_ok();
                            }
                        }
                    }
                }

                PRESET_PREVIEW_ID.store(0, Ordering::Relaxed);
                last_tab_tap = None;
            }
            _ => {}
        }

        yield_now().await;
    }
}

fn preset_panel_visible(view: control::UiView) -> bool {
    matches!(
        view,
        control::UiView::PresetPanel | control::UiView::PresetPanelBlocked
    )
}

fn coerce_panel_field_for_mode(
    _mode: LoadMode,
    field: ui::preset_panel::PresetPanelField,
) -> ui::preset_panel::PresetPanelField {
    field
}

fn coerce_adjust_digit_for_mode(
    mode: LoadMode,
    digit: control::AdjustDigit,
) -> control::AdjustDigit {
    match mode {
        LoadMode::Cp => match digit {
            control::AdjustDigit::Hundredths | control::AdjustDigit::Thousandths => {
                control::AdjustDigit::Tenths
            }
            _ => digit,
        },
        _ => digit,
    }
}

fn coerce_panel_digit_for_field(
    mode: LoadMode,
    field: ui::preset_panel::PresetPanelField,
    digit: ui::preset_panel::PresetPanelDigit,
) -> ui::preset_panel::PresetPanelDigit {
    use ui::preset_panel::PresetPanelDigit as D;
    use ui::preset_panel::PresetPanelField as F;

    let power_field = field == F::PLim || (mode == LoadMode::Cp && field == F::Target);
    match field {
        F::Mode => digit,
        _ if power_field => match digit {
            D::Tens | D::Ones | D::Tenths => digit,
            // For PLim we allow hundredths; for CP target we coerce to tenths (0.1 W).
            D::Hundredths if field == F::PLim => D::Hundredths,
            D::Hundredths | D::Thousandths => D::Tenths,
        },
        _ => match digit {
            D::Ones | D::Tenths | D::Hundredths | D::Thousandths => digit,
            D::Tens => D::Ones,
        },
    }
}

fn cycle_panel_digit_right(
    mode: LoadMode,
    field: ui::preset_panel::PresetPanelField,
    digit: ui::preset_panel::PresetPanelDigit,
) -> ui::preset_panel::PresetPanelDigit {
    use ui::preset_panel::PresetPanelDigit as D;
    use ui::preset_panel::PresetPanelField as F;

    let digit = coerce_panel_digit_for_field(mode, field, digit);
    let power_field = field == F::PLim || (mode == LoadMode::Cp && field == F::Target);
    match field {
        F::Mode => digit,
        _ if power_field => {
            if field == F::PLim {
                match digit {
                    D::Tens => D::Ones,
                    D::Ones => D::Tenths,
                    D::Tenths => D::Hundredths,
                    D::Hundredths => D::Tens,
                    D::Thousandths => D::Tens,
                }
            } else {
                // CP target: allow only Tens/Ones/Tenths (0.1 W).
                match digit {
                    D::Tens => D::Ones,
                    D::Ones => D::Tenths,
                    _ => D::Tens,
                }
            }
        }
        _ => match digit {
            D::Ones => D::Tenths,
            D::Tenths => D::Hundredths,
            D::Hundredths => D::Thousandths,
            D::Thousandths => D::Ones,
            D::Tens => D::Ones,
        },
    }
}

fn shift_panel_digit_once(
    mode: LoadMode,
    field: ui::preset_panel::PresetPanelField,
    digit: ui::preset_panel::PresetPanelDigit,
    dir: i32,
) -> (ui::preset_panel::PresetPanelDigit, bool) {
    use ui::preset_panel::PresetPanelDigit as D;
    use ui::preset_panel::PresetPanelField as F;

    let dir = dir.signum();
    if dir == 0 {
        return (digit, false);
    }

    let digit = coerce_panel_digit_for_field(mode, field, digit);
    let power_field = field == F::PLim || (mode == LoadMode::Cp && field == F::Target);
    let (cur_rank, max_rank) = if power_field {
        if field == F::PLim {
            (
                match digit {
                    D::Tens => 0,
                    D::Ones => 1,
                    D::Tenths => 2,
                    D::Hundredths => 3,
                    _ => 0,
                },
                3,
            )
        } else {
            (
                match digit {
                    D::Tens => 0,
                    D::Ones => 1,
                    D::Tenths => 2,
                    _ => 1,
                },
                2,
            )
        }
    } else {
        (
            match digit {
                D::Ones => 0,
                D::Tenths => 1,
                D::Hundredths => 2,
                D::Thousandths => 3,
                _ => 0,
            },
            3,
        )
    };

    let raw_rank = cur_rank + dir;
    if raw_rank < 0 || raw_rank > max_rank {
        return (digit, true);
    }

    let next = if power_field {
        if field == F::PLim {
            match raw_rank {
                0 => D::Tens,
                1 => D::Ones,
                2 => D::Tenths,
                _ => D::Hundredths,
            }
        } else {
            match raw_rank {
                0 => D::Tens,
                1 => D::Ones,
                _ => D::Tenths,
            }
        }
    } else {
        match raw_rank {
            0 => D::Ones,
            1 => D::Tenths,
            2 => D::Hundredths,
            _ => D::Thousandths,
        }
    };

    (next, false)
}

fn panel_digit_step(
    mode: LoadMode,
    field: ui::preset_panel::PresetPanelField,
    digit: ui::preset_panel::PresetPanelDigit,
) -> Option<i32> {
    use ui::preset_panel::PresetPanelDigit as D;
    use ui::preset_panel::PresetPanelField as F;

    let power_field = field == F::PLim || (mode == LoadMode::Cp && field == F::Target);
    match field {
        F::Mode => None,
        _ if power_field => match digit {
            D::Tens => Some(10_000),
            D::Ones => Some(1_000),
            D::Tenths => Some(100),
            D::Hundredths => Some(10),
            _ => None,
        },
        _ => match digit {
            D::Ones => Some(1_000),
            D::Tenths => Some(100),
            D::Hundredths => Some(10),
            D::Thousandths => Some(1),
            _ => None,
        },
    }
}

fn build_preset_panel_vm(state: &ControlState) -> ui::preset_panel::PresetPanelVm {
    use ui::preset_panel::{format_av_3dp, format_power_2dp};

    let idx = state.editing_preset_id.saturating_sub(1) as usize;
    let editing = state
        .presets
        .get(idx)
        .copied()
        .unwrap_or_else(|| state.active_preset());
    let editing_mode = match editing.mode {
        LoadMode::Cv => LoadMode::Cv,
        LoadMode::Cp => LoadMode::Cp,
        LoadMode::Cc | LoadMode::Reserved(_) => LoadMode::Cc,
    };

    let selected_field = coerce_panel_field_for_mode(editing_mode, state.panel_selected_field);
    let selected_digit =
        coerce_panel_digit_for_field(editing_mode, selected_field, state.panel_selected_digit);

    let (target_milli, target_unit) = match editing_mode {
        LoadMode::Cv => (editing.target_v_mv, 'V'),
        LoadMode::Cp => (editing.target_p_mw as i32, 'W'),
        _ => (editing.target_i_ma, 'A'),
    };

    ui::preset_panel::PresetPanelVm {
        active_preset_id: state.active_preset_id,
        editing_preset_id: state.editing_preset_id,
        editing_mode,
        load_enabled: state.output_enabled,
        load_toggle_disabled: false,
        blocked_save: state.ui_view == control::UiView::PresetPanelBlocked,
        dirty: state.dirty.get(idx).copied().unwrap_or(false),
        selected_field,
        selected_digit,
        target_text: if editing_mode == LoadMode::Cp {
            format_power_2dp(target_milli)
        } else {
            format_av_3dp(target_milli, target_unit)
        },
        v_lim_text: format_av_3dp(editing.min_v_mv, 'V'),
        i_lim_text: format_av_3dp(editing.max_i_ma_total, 'A'),
        p_lim_text: format_power_2dp(editing.max_p_mw as i32),
    }
}

fn build_pd_settings_vm(
    draft: control::PdConfig,
    focus: control::PdSettingsFocus,
    digit: control::AdjustDigit,
    status: Option<&PdStatus>,
) -> ui::pd_settings::PdSettingsVm {
    fn pdo_pos(pos: u8, idx: usize) -> u8 {
        if pos != 0 {
            pos
        } else {
            (idx + 1).min(u8::MAX as usize) as u8
        }
    }

    let mut attached = false;
    let mut contract_mv = 0u32;
    let mut contract_ma = 0u32;
    let mut fixed_pdos = loadlynx_protocol::FixedPdoList::new();
    let mut pps_pdos = loadlynx_protocol::PpsPdoList::new();

    if let Some(s) = status {
        attached = s.attached;
        contract_mv = s.contract_mv;
        contract_ma = s.contract_ma;
        fixed_pdos = s.fixed_pdos.clone();
        pps_pdos = s.pps_pdos.clone();
    }

    // Fixed selection: if not explicitly selected, fall back to target_mv matching for legacy blobs.
    let fixed_object_pos = if draft.fixed_object_pos != 0 {
        draft.fixed_object_pos
    } else {
        fixed_pdos
            .iter()
            .enumerate()
            .find(|(_idx, pdo)| pdo.mv == draft.target_mv)
            .map(|(idx, pdo)| pdo_pos(pdo.pos, idx))
            .unwrap_or(0)
    };

    let pps_object_pos = draft.pps_object_pos;

    let fixed_selected = if fixed_object_pos != 0 {
        fixed_pdos
            .iter()
            .enumerate()
            .find(|(idx, pdo)| pdo_pos(pdo.pos, *idx) == fixed_object_pos)
            .map(|(_idx, pdo)| *pdo)
    } else {
        None
    };

    let pps_selected = if pps_object_pos != 0 {
        pps_pdos
            .iter()
            .enumerate()
            .find(|(idx, pdo)| pdo_pos(pdo.pos, *idx) == pps_object_pos)
            .map(|(_idx, pdo)| *pdo)
    } else {
        None
    };

    let mut selection_missing = false;
    let apply_enabled = if !attached {
        false
    } else {
        match draft.mode {
            control::PdMode::Fixed => match fixed_selected {
                Some(pdo) => draft.i_req_ma >= 50 && draft.i_req_ma <= pdo.max_ma,
                None => {
                    if fixed_object_pos != 0 {
                        selection_missing = true;
                    }
                    false
                }
            },
            control::PdMode::Pps => {
                if pps_object_pos == 0 {
                    false
                } else {
                    match pps_selected {
                        Some(pdo) => {
                            draft.target_mv >= pdo.min_mv
                                && draft.target_mv <= pdo.max_mv
                                && draft.i_req_ma >= 50
                                && draft.i_req_ma <= pdo.max_ma
                        }
                        None => {
                            selection_missing = true;
                            false
                        }
                    }
                }
            }
        }
    };

    let message = if selection_missing {
        ui::pd_settings::PdSettingsMessage::Unavailable
    } else {
        // Only surface apply state transitions when the user (or HTTP client) explicitly
        // triggered an Apply. Background auto-apply (attach/link recovery) should not spam
        // the PD settings UI.
        let now = now_ms32();
        let ui_apply_ms = PD_UI_APPLY_MS.load(Ordering::Relaxed);
        if ui_apply_ms == 0 {
            ui::pd_settings::PdSettingsMessage::None
        } else {
            let last_result_ms = PD_LAST_RESULT_MS.load(Ordering::Relaxed);
            let last_result = PD_LAST_RESULT_CODE.load(Ordering::Relaxed);
            let pending = PD_REQ_ACK_PENDING.load(Ordering::Relaxed);

            if (pending || now.wrapping_sub(ui_apply_ms) < 800) && last_result_ms < ui_apply_ms {
                ui::pd_settings::PdSettingsMessage::Applying
            } else if last_result != 0
                && last_result_ms >= ui_apply_ms
                && now.wrapping_sub(last_result_ms) < 1500
            {
                match last_result {
                    1 => ui::pd_settings::PdSettingsMessage::ApplyOk,
                    _ => ui::pd_settings::PdSettingsMessage::ApplyFailed,
                }
            } else {
                ui::pd_settings::PdSettingsMessage::None
            }
        }
    };

    ui::pd_settings::PdSettingsVm {
        attached,
        mode: draft.mode,
        focus,
        focused_digit: digit,
        fixed_pdos,
        pps_pdos,
        contract_mv,
        contract_ma,
        fixed_object_pos,
        pps_object_pos,
        pps_target_mv: draft.target_mv,
        i_req_ma: draft.i_req_ma,
        apply_enabled,
        message,
    }
}

async fn save_editing_preset_to_eeprom(control: &ControlMutex, eeprom: &EepromMutex) -> bool {
    let (preset_id, blob) = {
        let guard = control.lock().await;
        let preset_id = guard.editing_preset_id;
        let idx = preset_id.saturating_sub(1) as usize;
        if idx >= control::PRESET_COUNT {
            return false;
        }
        let mut to_write = guard.saved;
        to_write[idx] = guard.presets[idx];
        (preset_id, control::encode_presets_blob(&to_write))
    };

    let res = {
        let mut guard = eeprom.lock().await;
        guard.write_presets_blob(&blob).await
    };

    let mut guard = control.lock().await;
    match res {
        Ok(()) => {
            guard.commit_saved_for_preset_id(preset_id);
            if guard.ui_view == control::UiView::PresetPanelBlocked {
                guard.ui_view = control::UiView::PresetPanel;
            }
            info!("touch: SAVE ok (preset_id={})", preset_id);
            true
        }
        Err(err) => {
            guard.ui_view = control::UiView::PresetPanelBlocked;
            warn!(
                "touch: SAVE failed (preset_id={}, err={:?})",
                preset_id, err
            );
            false
        }
    }
}

async fn save_pd_config_to_eeprom(cfg: control::PdConfig, eeprom: &EepromMutex) -> bool {
    let blob = control::encode_pd_blob(&cfg);
    let res = {
        let mut guard = eeprom.lock().await;
        guard.write_pd_blob(&blob).await
    };
    match res {
        Ok(()) => {
            info!(
                "touch: PD SAVE ok (mode={:?}, target_mv={}, i_req_ma={})",
                cfg.mode, cfg.target_mv, cfg.i_req_ma
            );
            true
        }
        Err(err) => {
            warn!(
                "touch: PD SAVE failed (mode={:?}, target_mv={}, i_req_ma={}, err={:?})",
                cfg.mode, cfg.target_mv, cfg.i_req_ma, err
            );
            false
        }
    }
}

async fn apply_pd_status(telemetry: &'static TelemetryMutex, status: PdStatus) {
    let prev = PD_STATUS_ATTACHED.swap(status.attached, Ordering::Relaxed);
    if status.attached && !prev {
        // PD auto-apply: trigger a re-send of the persisted policy on attach rising edge.
        PD_FORCE_SEND.store(true, Ordering::Release);
    }
    let mut guard = telemetry.lock().await;
    let changed = match guard.last_pd_status.as_ref() {
        None => true,
        Some(prev) => {
            prev.attached != status.attached
                || prev.contract_mv != status.contract_mv
                || prev.contract_ma != status.contract_ma
        }
    };
    guard.last_pd_status = Some(status);
    if changed {
        let s = guard.last_pd_status.as_ref().unwrap();
        info!(
            "PD_STATUS update: attached={} contract={}mV {}mA fixed_pdos={} pps_pdos={}",
            s.attached,
            s.contract_mv,
            s.contract_ma,
            s.fixed_pdos.len(),
            s.pps_pdos.len()
        );
    }
}

struct DisplayResources {
    spi: Option<SpiDmaBus<'static, Async>>,
    cs: Option<Output<'static>>,
    dc: Option<Output<'static>>,
    rst: Option<Output<'static>>,
    framebuffer: &'static mut [u8; FRAMEBUFFER_LEN],
    #[cfg(not(feature = "net_http"))]
    previous_framebuffer: &'static mut [u8; FRAMEBUFFER_LEN],
}

pub struct TelemetryModel {
    /// Cached snapshot used by the local UI renderer.
    pub snapshot: UiSnapshot,
    last_uptime_ms: Option<u32>,
    last_rendered: Option<UiSnapshot>,
    /// Last raw FastStatus frame observed from the analog side. This is used
    /// by the optional HTTP API to expose a structured status view.
    pub last_status: Option<FastStatus>,
    pub last_pd_status: Option<PdStatus>,
    last_touch_marker_seq: u32,
}

impl TelemetryModel {
    fn new() -> Self {
        Self {
            snapshot: UiSnapshot::demo(),
            last_uptime_ms: None,
            last_rendered: None,
            last_status: None,
            last_pd_status: None,
            last_touch_marker_seq: 0,
        }
    }

    fn update_from_status(&mut self, status: &FastStatus) {
        // Keep a copy of the last raw FastStatus for external status views.
        self.last_status = Some(*status);

        let remote_active = (status.state_flags & STATE_FLAG_REMOTE_ACTIVE) != 0;

        let remote_voltage = status.v_remote_mv as f32 / 1000.0;
        let local_voltage = status.v_local_mv as f32 / 1000.0;
        let main_voltage = if remote_active {
            remote_voltage
        } else {
            local_voltage
        };
        let i_local = status.i_local_ma as f32 / 1000.0;
        let i_remote = status.i_remote_ma as f32 / 1000.0;
        let i_total = i_local + i_remote;
        let power_w = status.calc_p_mw as f32 / 1000.0;

        self.snapshot.main_voltage = main_voltage;
        self.snapshot.remote_voltage = remote_voltage;
        self.snapshot.local_voltage = local_voltage;
        self.snapshot.remote_active = remote_active;
        // 左侧主电流显示：两通道合计电流；CURRENT 标签右侧镜像条形图反映 CH1/CH2 单通道电流。
        self.snapshot.main_current = i_total;
        self.snapshot.ch1_current = i_local;
        self.snapshot.ch2_current = i_remote;
        self.snapshot.main_power = power_w;
        self.snapshot.sink_core_temp = status.sink_core_temp_mc as f32 / 1000.0;
        self.snapshot.sink_exhaust_temp = status.sink_exhaust_temp_mc as f32 / 1000.0;
        self.snapshot.mcu_temp = status.mcu_temp_mc as f32 / 1000.0;
        self.snapshot.fault_flags = status.fault_flags;
        let analog_state = AnalogState::from_u8(ANALOG_STATE.load(Ordering::Relaxed));
        self.snapshot.analog_state = analog_state;

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

    fn set_wifi_ui_status(&mut self, status: ui::WifiUiStatus) {
        self.snapshot.wifi_status = status;
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

        let touch_seq = touch::touch_marker_seq();
        if touch_seq != self.last_touch_marker_seq {
            mask.touch_marker = true;
            self.last_touch_marker_seq = touch_seq;
        }

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
                mask.channel_currents = true;
            }

            if prev.active_preset_id != current.active_preset_id
                || prev.active_mode != current.active_mode
                || prev.control_target_text != current.control_target_text
                || prev.adjust_digit != current.adjust_digit
            {
                mask.control_row = true;
            }

            if prev.output_enabled != current.output_enabled
                || prev.uv_latched != current.uv_latched
                || prev.fault_flags != current.fault_flags
                || prev.link_up != current.link_up
                || prev.link_alarm_latched != current.link_alarm_latched
                || prev.hello_seen != current.hello_seen
                || prev.trip_alarm_abbrev != current.trip_alarm_abbrev
                || prev.blocked_enable_abbrev != current.blocked_enable_abbrev
                || prev.pd_state != current.pd_state
                || prev.pd_display_mode != current.pd_display_mode
                || prev.pd_target_mv != current.pd_target_mv
                || prev.pd_target_available != current.pd_target_available
            {
                mask.load_row = true;
            }

            if prev.status_lines != current.status_lines {
                mask.telemetry_lines = true;
            }
            let ctl_alert = current.fault_flags != 0
                || current.link_alarm_latched
                || current.trip_alarm_abbrev.is_some()
                || current.blocked_enable_abbrev.is_some()
                || current.uv_latched
                || !current.link_up;
            if ctl_alert && prev.blink_on != current.blink_on {
                mask.telemetry_lines = true;
            }
            if prev.wifi_status != current.wifi_status {
                mask.wifi_status = true;
            }
        } else {
            // First-frame render: everything is considered dirty so that the
            // initial layout is fully drawn.
            mask.main_metrics = true;
            mask.voltage_pair = true;
            mask.channel_currents = true;
            mask.control_row = true;
            mask.load_row = true;
            mask.telemetry_lines = true;
            mask.wifi_status = true;
            mask.touch_marker = true;
        }

        // 记录当前快照用于下一次 diff；只在这里 clone 一次，避免在栈上持有多份大对象。
        self.last_rendered = Some(self.snapshot.clone());
        (self.snapshot.clone(), mask)
    }
}

fn compute_fan_duty_pct(temp_c: f32, main_power_w: f32) -> u8 {
    // 简单分段线性曲线：
    //   低功率 & T_core <= FAN_TEMP_STOP_C → 0%（可停转）
    //   其他情况下：
    //     T_core <= FAN_TEMP_LOW_C  → FAN_DUTY_MIN_PCT
    //   FAN_TEMP_LOW_C..FAN_TEMP_HIGH_C 线性插值到 FAN_DUTY_MID_PCT
    //   T_core >= FAN_TEMP_HIGH_C → FAN_DUTY_MAX_PCT
    if !temp_c.is_finite() {
        return FAN_DUTY_DEFAULT_PCT;
    }

    let low_power = main_power_w < FAN_POWER_LOW_W;
    if low_power && temp_c <= FAN_TEMP_STOP_C {
        return 0;
    }

    if temp_c <= FAN_TEMP_LOW_C {
        FAN_DUTY_MIN_PCT
    } else if temp_c > FAN_TEMP_HIGH_C {
        FAN_DUTY_MAX_PCT
    } else {
        let span = FAN_TEMP_HIGH_C - FAN_TEMP_LOW_C;
        let frac = ((temp_c - FAN_TEMP_LOW_C) / span).clamp(0.0, 1.0);
        let duty_min = FAN_DUTY_MIN_PCT as f32;
        let duty_mid = FAN_DUTY_MID_PCT as f32;
        let duty = duty_min + frac * (duty_mid - duty_min);
        let duty = duty.clamp(FAN_DUTY_MIN_PCT as f32, FAN_DUTY_MAX_PCT as f32);
        duty as u8
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

#[embassy_executor::task]
async fn fan_task(
    telemetry: &'static TelemetryMutex,
    fan_channel: &'static ledc_channel::Channel<'static, LowSpeed>,
) {
    info!(
        "fan task starting (period_ms={}, temp_low={}C, temp_high={}C, duty_min={}%, duty_mid={}%, duty_max={}%)",
        FAN_CONTROL_PERIOD_MS,
        FAN_TEMP_LOW_C,
        FAN_TEMP_HIGH_C,
        FAN_DUTY_MIN_PCT,
        FAN_DUTY_MID_PCT,
        FAN_DUTY_MAX_PCT,
    );

    // 上电时设置一个安全默认占空比，避免完全静音导致热惯性过大。
    fan_channel
        .set_duty(FAN_DUTY_DEFAULT_PCT)
        .expect("fan duty init");

    let mut last_duty_pct: u8 = FAN_DUTY_DEFAULT_PCT;
    let mut last_log_duty_pct: u8 = FAN_DUTY_DEFAULT_PCT;
    let mut last_log_ms: u32 = now_ms32();

    loop {
        let (core_temp_c, exhaust_temp_c, main_power_w) = {
            let guard = telemetry.lock().await;
            let core = guard.snapshot.sink_core_temp;
            let exhaust = guard.snapshot.sink_exhaust_temp;
            let power = guard.snapshot.main_power;
            (core, exhaust, power)
        };

        let target_duty_pct = compute_fan_duty_pct(core_temp_c, main_power_w);
        let diff = if target_duty_pct > last_duty_pct {
            target_duty_pct - last_duty_pct
        } else {
            last_duty_pct - target_duty_pct
        };

        if diff >= FAN_DUTY_UPDATE_THRESHOLD_PCT {
            fan_channel
                .set_duty(target_duty_pct)
                .expect("fan duty update");

            let now = now_ms32();
            let log_diff = if target_duty_pct > last_log_duty_pct {
                target_duty_pct - last_log_duty_pct
            } else {
                last_log_duty_pct - target_duty_pct
            };
            let high_temp = core_temp_c >= FAN_LOG_HIGH_TEMP_C;
            if high_temp
                || (log_diff >= FAN_LOG_DUTY_DELTA_LARGE_PCT
                    && now.wrapping_sub(last_log_ms) >= FAN_LOG_COOLDOWN_MS)
            {
                info!(
                    "fan duty update: T_core={}C T_exhaust={}C duty={}%",
                    core_temp_c, exhaust_temp_c, target_duty_pct
                );
                last_log_duty_pct = target_duty_pct;
                last_log_ms = now;
            }

            last_duty_pct = target_duty_pct;
        }

        cooperative_delay_ms(FAN_CONTROL_PERIOD_MS).await;
    }
}

/// Bridge Wi‑Fi connection state into the UI model so the display task can
/// render a compact status indicator without performing async net locking.
#[cfg(feature = "net_http")]
#[embassy_executor::task]
async fn wifi_ui_task(state: &'static net::WifiStateMutex, telemetry: &'static TelemetryMutex) {
    use ui::WifiUiStatus;

    loop {
        let ui_status = {
            let guard = state.lock().await;
            match guard.state {
                net::WifiConnectionState::Connected => WifiUiStatus::Ok,
                net::WifiConnectionState::Connecting => WifiUiStatus::Connecting,
                net::WifiConnectionState::Idle => {
                    if guard.last_error.is_some() {
                        WifiUiStatus::Error
                    } else {
                        WifiUiStatus::Disabled
                    }
                }
                net::WifiConnectionState::Error => WifiUiStatus::Error,
            }
        };

        {
            let mut guard = telemetry.lock().await;
            guard.set_wifi_ui_status(ui_status);
        }

        // UI 刷新频率远低于 Wi‑Fi 事件频率，这里约 4Hz 轮询即可。
        cooperative_delay_ms(250).await;
    }
}

async fn apply_fast_status(telemetry: &'static TelemetryMutex, status: &FastStatus) {
    let link_up = LINK_UP.load(Ordering::Relaxed);
    let fault_flags = status.fault_flags;
    LAST_FAULT_FLAGS.store(fault_flags, Ordering::Relaxed);
    prompt_tone::set_fault_flags(fault_flags);
    let remote_active = (status.state_flags & STATE_FLAG_REMOTE_ACTIVE) != 0;
    let v_main_mv = if remote_active {
        status.v_local_mv.max(status.v_remote_mv)
    } else {
        status.v_local_mv
    };
    LAST_V_MAIN_MV.store(v_main_mv, Ordering::Relaxed);
    LAST_V_LOCAL_MV.store(status.v_local_mv, Ordering::Relaxed);
    LAST_I_TOTAL_MA.store(
        status.i_local_ma.saturating_add(status.i_remote_ma),
        Ordering::Relaxed,
    );
    LAST_CALC_P_MW.store(status.calc_p_mw, Ordering::Relaxed);
    let uv_latched = (status.state_flags & STATE_FLAG_UV_LATCHED) != 0;
    let prev_uv_latched = UV_LATCHED.swap(uv_latched, Ordering::Relaxed);
    prompt_tone::set_uv_latched(uv_latched);
    if uv_latched && !prev_uv_latched {
        prompt_tone::latch_trip_alarm(prompt_tone::TripReason::Uvlo);
    }
    let enabled = status.enable;
    let link_flag = (status.state_flags & STATE_FLAG_LINK_GOOD) != 0;

    // Offline 主要由 LINK_UP 推导；仅在 LINK_UP 与模拟侧 LINK_GOOD 均为 false 时视为离线，
    // 避免模拟侧未完全实现 LINK_GOOD 时 UI 误报 OFFLINE。
    let state = if !link_up && !link_flag {
        AnalogState::Offline
    } else if fault_flags != 0 {
        AnalogState::Faulted
    } else if enabled {
        AnalogState::Ready
    } else {
        AnalogState::CalMissing
    };
    ANALOG_STATE.store(state as u8, Ordering::Relaxed);

    let mut guard = telemetry.lock().await;
    guard.update_from_status(status);
    LAST_TARGET_VALUE_FROM_STATUS.store(status.target_value, Ordering::Relaxed);

    if fault_flags != 0 {
        let now = now_ms32();
        let last = LAST_FAULT_LOG_MS.load(Ordering::Relaxed);
        if now.wrapping_sub(last) >= 1_000 {
            LAST_FAULT_LOG_MS.store(now, Ordering::Relaxed);
            warn!("analog fault flags set: 0x{:08x}", fault_flags);
        }
    }
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
async fn display_task(
    ctx: &'static mut DisplayResources,
    telemetry: &'static TelemetryMutex,
    control: &'static ControlMutex,
    backlight_channel: &'static ledc_channel::Channel<'static, LowSpeed>,
    backlight_pin: &'static OutputSignalPin<'static>,
) {
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
        #[cfg(not(feature = "net_http"))]
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
    let mut last_panel_visible: bool = false;
    let mut last_preview_active: bool = false;
    let mut last_preview_mode: LoadMode = LoadMode::Cc;
    let mut last_ui_view: control::UiView = control::UiView::Main;
    let screen_cfg =
        ScreenPowerConfig::new(SCREEN_DIM_AFTER_MS, SCREEN_OFF_AFTER_MS, SCREEN_DIM_MAX_PCT);
    let mut screen_power = ScreenPowerModel::new(BACKLIGHT_DEFAULT_PCT);
    let mut last_backlight_pct: u8 = BACKLIGHT_DEFAULT_PCT;
    let mut backlight_pwm_attached: bool = true;
    let mut display_sleeping: bool = false;
    let mut screen_force_full_render: bool = false;
    loop {
        let now = timestamp_ms() as u32;

        // Screen power management (Plan #0015): only auto-dim/off when LOAD is OFF,
        // and consume the first input in OFF so it cannot trigger UI actions.
        let last_user_activity_ms = LAST_USER_ACTIVITY_MS.load(Ordering::Relaxed);
        let load_enabled = control.lock().await.output_enabled;
        let tick = screen_power.tick(screen_cfg, now, last_user_activity_ms, load_enabled);
        if let Some(tr) = tick.transition {
            let from_label = match tr.from {
                ScreenPowerState::Active => "active",
                ScreenPowerState::Dim => "dim",
                ScreenPowerState::Off => "off",
            };
            let to_label = match tr.to {
                ScreenPowerState::Active => "active",
                ScreenPowerState::Dim => "dim",
                ScreenPowerState::Off => "off",
            };
            info!(
                "screen_power: {}->{} (idle_ms={}, load_enabled={}, backlight={}%)",
                from_label, to_label, tick.idle_ms, load_enabled, tick.target_backlight_pct
            );

            match (tr.from, tr.to) {
                (ScreenPowerState::Active, ScreenPowerState::Dim) => {
                    set_backlight_pct(backlight_channel, tick.target_backlight_pct, "dim");
                    if !backlight_pwm_attached {
                        backlight_pwm_connect(backlight_pin);
                        backlight_pwm_attached = true;
                    }
                    last_backlight_pct = tick.target_backlight_pct;
                }
                (ScreenPowerState::Dim, ScreenPowerState::Active) => {
                    set_backlight_pct(backlight_channel, tick.target_backlight_pct, "restore");
                    if !backlight_pwm_attached {
                        backlight_pwm_connect(backlight_pin);
                        backlight_pwm_attached = true;
                    }
                    last_backlight_pct = tick.target_backlight_pct;
                }
                (ScreenPowerState::Dim, ScreenPowerState::Off)
                | (ScreenPowerState::Active, ScreenPowerState::Off) => {
                    set_backlight_pct(backlight_channel, 0, "off");
                    last_backlight_pct = 0;
                    backlight_pwm_disconnect(backlight_pin);
                    backlight_pwm_attached = false;

                    display.sleep(&mut delay).await.expect("display sleep");
                    display_sleeping = true;
                    screen_force_full_render = true;
                    last_push_ms = now;
                }
                (ScreenPowerState::Off, ScreenPowerState::Active) => {
                    if display_sleeping {
                        display.wake(&mut delay).await.expect("display wake");
                        display_sleeping = false;
                    }
                    unsafe {
                        // Some controllers require DISPON after SLP OUT.
                        use lcd_async::dcs::{EnterNormalMode, InterfaceExt as _, SetDisplayOn};
                        let dcs = display.dcs();
                        dcs.write_command(EnterNormalMode)
                            .await
                            .expect("display normal mode");
                        dcs.write_command(SetDisplayOn).await.expect("display on");
                    }
                    delay.delay_ns(120_000_000).await;

                    set_backlight_pct(backlight_channel, tick.target_backlight_pct, "wake");
                    if !backlight_pwm_attached {
                        backlight_pwm_connect(backlight_pin);
                        backlight_pwm_attached = true;
                    }
                    last_backlight_pct = tick.target_backlight_pct;
                    screen_force_full_render = true;
                }
                _ => {}
            }

            SCREEN_POWER_STATE.store(screen_power_state_to_u8(tick.state), Ordering::Relaxed);
        } else if tick.target_backlight_pct != last_backlight_pct
            && tick.state != ScreenPowerState::Off
        {
            // Keep the backlight aligned with the policy even if we re-enter a state
            // (e.g. after a firmware brightness change in future work).
            set_backlight_pct(backlight_channel, tick.target_backlight_pct, "sync");
            if !backlight_pwm_attached {
                backlight_pwm_connect(backlight_pin);
                backlight_pwm_attached = true;
            }
            last_backlight_pct = tick.target_backlight_pct;
        }

        if tick.state == ScreenPowerState::Off {
            cooperative_delay_ms(20).await;
            continue;
        }
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

            let preview_id = PRESET_PREVIEW_ID.load(Ordering::Relaxed);
            let (
                overlay_preset_id,
                output_enabled,
                overlay_mode,
                active_target_milli,
                active_target_unit,
                adjust_digit,
                ui_view,
                pd_saved,
                pd_draft,
                pd_focus,
                pd_digit,
                preview_panel,
                panel_visible,
                panel_vm,
            ) = {
                let guard = control.lock().await;
                let preview_active = (1..=(control::PRESET_COUNT as u8)).contains(&preview_id);
                let overlay_preset_id = if preview_active {
                    preview_id
                } else {
                    guard.active_preset_id
                };
                let overlay_idx = overlay_preset_id.saturating_sub(1) as usize;
                let overlay_preset = if overlay_idx < control::PRESET_COUNT {
                    guard.presets[overlay_idx]
                } else {
                    guard.active_preset()
                };
                let overlay_mode = match overlay_preset.mode {
                    LoadMode::Cv => LoadMode::Cv,
                    LoadMode::Cp => LoadMode::Cp,
                    LoadMode::Cc | LoadMode::Reserved(_) => LoadMode::Cc,
                };
                let active_preset = guard.active_preset();
                let active_mode = match active_preset.mode {
                    LoadMode::Cv => LoadMode::Cv,
                    LoadMode::Cp => LoadMode::Cp,
                    LoadMode::Cc | LoadMode::Reserved(_) => LoadMode::Cc,
                };
                let (active_target_milli, active_target_unit) = match active_mode {
                    LoadMode::Cv => (active_preset.target_v_mv, 'V'),
                    LoadMode::Cp => (active_preset.target_p_mw as i32, 'W'),
                    LoadMode::Cc | LoadMode::Reserved(_) => (active_preset.target_i_ma, 'A'),
                };
                let preview_panel = if preview_active {
                    use ui::preset_panel::{format_av_3dp, format_power_2dp};
                    let target_text = match overlay_mode {
                        LoadMode::Cv => format_av_3dp(overlay_preset.target_v_mv, 'V'),
                        LoadMode::Cp => format_power_2dp(overlay_preset.target_p_mw as i32),
                        _ => format_av_3dp(overlay_preset.target_i_ma, 'A'),
                    };
                    Some((
                        target_text,
                        format_av_3dp(overlay_preset.min_v_mv, 'V'),
                        format_av_3dp(overlay_preset.max_i_ma_total, 'A'),
                        format_power_2dp(overlay_preset.max_p_mw as i32),
                    ))
                } else {
                    None
                };
                (
                    overlay_preset_id,
                    guard.output_enabled,
                    overlay_mode,
                    active_target_milli,
                    active_target_unit,
                    coerce_adjust_digit_for_mode(active_mode, guard.adjust_digit),
                    guard.ui_view,
                    guard.pd_saved,
                    guard.pd_draft,
                    guard.pd_settings_focus,
                    guard.pd_settings_digit,
                    preview_panel,
                    preset_panel_visible(guard.ui_view),
                    if preset_panel_visible(guard.ui_view) {
                        Some(build_preset_panel_vm(&guard))
                    } else {
                        None
                    },
                )
            };

            let preview_active = preview_panel.is_some();
            let mut force_full_render = screen_force_full_render
                || panel_visible
                || (panel_visible != last_panel_visible)
                || frame_idx == 1;
            if ui_view != last_ui_view {
                force_full_render = true;
            }
            if preview_active != last_preview_active {
                force_full_render = true;
            }
            if preview_active && last_preview_active && overlay_mode != last_preview_mode {
                force_full_render = true;
            }
            last_panel_visible = panel_visible;
            last_preview_active = preview_active;
            if preview_active {
                last_preview_mode = overlay_mode;
            }
            last_ui_view = ui_view;
            if force_full_render && screen_force_full_render {
                screen_force_full_render = false;
            }

            let (snapshot, mask, pd_status_for_panel) = {
                let mut guard = telemetry.lock().await;
                guard.snapshot.blink_on = ((now / 500) & 1) == 0;
                let uv_latched_raw = guard
                    .last_status
                    .map(|s| (s.state_flags & STATE_FLAG_UV_LATCHED) != 0)
                    .unwrap_or(false);
                let uv_latched = uv_latched_raw;
                let link_up = LINK_UP.load(Ordering::Relaxed);
                let link_alarm_latched = prompt_tone::is_link_alarm_latched();
                let hello_seen = HELLO_SEEN.load(Ordering::Relaxed);
                let trip_alarm_abbrev =
                    prompt_tone::trip_alarm_reason().map(|reason| reason.abbrev());
                let blocked_enable_abbrev = if output_enabled {
                    None
                } else {
                    recent_enable_block_abbrev(now)
                };
                let (pd_attached, pd_display_mode, pd_target_mv, pd_target_available) =
                    if let Some(s) = guard.last_pd_status.as_ref() {
                        fn pdo_pos(pos: u8, idx: usize) -> u8 {
                            if pos != 0 {
                                pos
                            } else {
                                (idx + 1).min(u8::MAX as usize) as u8
                            }
                        }

                        let pd_attached = s.attached;
                        let pd_display_mode = if !pd_attached {
                            ui::PdButtonDisplayMode::Detach
                        } else {
                            match pd_saved.mode {
                                control::PdMode::Fixed => ui::PdButtonDisplayMode::Fixed,
                                control::PdMode::Pps => ui::PdButtonDisplayMode::Pps,
                            }
                        };
                        let pd_target_mv = if pd_attached && s.contract_mv != 0 {
                            Some(s.contract_mv)
                        } else {
                            None
                        };
                        let pd_target_available = if !pd_attached {
                            false
                        } else {
                            match pd_saved.mode {
                                control::PdMode::Fixed => {
                                    let fixed_object_pos = if pd_saved.fixed_object_pos != 0 {
                                        pd_saved.fixed_object_pos
                                    } else {
                                        s.fixed_pdos
                                            .iter()
                                            .enumerate()
                                            .find(|(_idx, pdo)| pdo.mv == pd_saved.target_mv)
                                            .map(|(idx, pdo)| pdo_pos(pdo.pos, idx))
                                            .unwrap_or(0)
                                    };

                                    let fixed_selected = if fixed_object_pos != 0 {
                                        s.fixed_pdos
                                            .iter()
                                            .enumerate()
                                            .find(|(idx, pdo)| {
                                                pdo_pos(pdo.pos, *idx) == fixed_object_pos
                                            })
                                            .map(|(_idx, pdo)| *pdo)
                                    } else {
                                        None
                                    };

                                    match fixed_selected {
                                        Some(pdo) => {
                                            pd_saved.i_req_ma >= 50
                                                && pd_saved.i_req_ma <= pdo.max_ma
                                        }
                                        None => false,
                                    }
                                }
                                control::PdMode::Pps => {
                                    if pd_saved.pps_object_pos == 0 {
                                        false
                                    } else {
                                        let pps_selected = s
                                            .pps_pdos
                                            .iter()
                                            .enumerate()
                                            .find(|(idx, pdo)| {
                                                pdo_pos(pdo.pos, *idx) == pd_saved.pps_object_pos
                                            })
                                            .map(|(_idx, pdo)| *pdo);
                                        match pps_selected {
                                            Some(pdo) => {
                                                pd_saved.target_mv >= pdo.min_mv
                                                    && pd_saved.target_mv <= pdo.max_mv
                                                    && pd_saved.i_req_ma >= 50
                                                    && pd_saved.i_req_ma <= pdo.max_ma
                                            }
                                            None => false,
                                        }
                                    }
                                }
                            }
                        };
                        (
                            pd_attached,
                            pd_display_mode,
                            pd_target_mv,
                            pd_target_available,
                        )
                    } else {
                        (
                            false,
                            ui::PdButtonDisplayMode::Detach,
                            None,
                            true, // No caps; default to "available" to avoid false gray.
                        )
                    };

                guard.snapshot.pd_display_mode = pd_display_mode;
                guard.snapshot.pd_target_mv = pd_target_mv;
                guard.snapshot.pd_target_available = pd_target_available;

                let attached = pd_attached;
                let contract_mv = pd_target_mv.unwrap_or(0);
                let last_apply_ms = PD_LAST_APPLY_MS.load(Ordering::Relaxed);
                let in_window = last_apply_ms != 0 && now.wrapping_sub(last_apply_ms) <= PD_T_PD_MS;
                let prev_pd_state = guard.snapshot.pd_state;
                guard.snapshot.pd_state = if !link_up || !attached {
                    ui::PdButtonState::Standby
                } else if contract_mv != 0 {
                    ui::PdButtonState::Active
                } else if in_window || last_apply_ms == 0 {
                    ui::PdButtonState::Negotiating
                } else {
                    ui::PdButtonState::Error
                };
                if prev_pd_state != guard.snapshot.pd_state {
                    info!(
                        "pd_button: state {:?}->{:?} attached={} contract={}mV last_apply_ms={} in_window={}",
                        prev_pd_state,
                        guard.snapshot.pd_state,
                        attached,
                        contract_mv,
                        last_apply_ms,
                        in_window
                    );
                }
                guard.snapshot.set_control_overlay(
                    overlay_preset_id,
                    output_enabled,
                    overlay_mode,
                    uv_latched,
                    link_up,
                    link_alarm_latched,
                    hello_seen,
                    trip_alarm_abbrev,
                    blocked_enable_abbrev,
                );
                guard.snapshot.preset_preview_active = preview_active;
                if let Some((target, v_lim, i_lim, p_lim)) = preview_panel {
                    guard.snapshot.preset_preview_target_text = target;
                    guard.snapshot.preset_preview_v_lim_text = v_lim;
                    guard.snapshot.preset_preview_i_lim_text = i_lim;
                    guard.snapshot.preset_preview_p_lim_text = p_lim;
                } else {
                    guard.snapshot.preset_preview_target_text.clear();
                    guard.snapshot.preset_preview_v_lim_text.clear();
                    guard.snapshot.preset_preview_i_lim_text.clear();
                    guard.snapshot.preset_preview_p_lim_text.clear();
                }
                guard.snapshot.set_control_row(
                    active_target_milli,
                    active_target_unit,
                    adjust_digit,
                );
                let pd_status = guard.last_pd_status.clone();
                let (snapshot, mask) = guard.diff_for_render();
                (snapshot, mask, pd_status)
            };

            if ui_view != control::UiView::PdSettings && mask.is_empty() && !force_full_render {
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
                    match ui_view {
                        control::UiView::PdSettings => {
                            let vm = build_pd_settings_vm(
                                pd_draft,
                                pd_focus,
                                pd_digit,
                                pd_status_for_panel.as_ref(),
                            );
                            ui::pd_settings::render_pd_settings(&mut frame, &vm);
                        }
                        _ => {
                            if force_full_render || mask.touch_marker {
                                // 首帧：完整绘制静态布局 + 动态内容。
                                ui::render(&mut frame, &snapshot);
                            } else {
                                // 后续帧：仅按掩码重绘受影响区域。
                                ui::render_partial(&mut frame, &snapshot, &mask);
                            }
                            if let Some(vm) = panel_vm.as_ref() {
                                ui::preset_panel::render_preset_panel(&mut frame, vm);
                            }
                        }
                    }
                    // 在左上角叠加 FPS 信息，使用上一统计窗口得到的整数 FPS。
                    ui::render_fps_overlay(&mut frame, last_fps);
                    ui::render_touch_marker(&mut frame, touch::load_touch_marker());
                }

                if frame_idx <= FRAME_SAMPLE_FRAMES {
                    log_framebuffer_span("rendered-frame", &ctx.framebuffer[..]);
                    log_framebuffer_samples("rendered-frame", &ctx.framebuffer[..]);
                }
            }

            let mut dirty_rows = 0usize;
            let mut dirty_spans = 0usize;

            if ENABLE_DISPLAY_SPI_UPDATES {
                #[cfg(not(feature = "net_http"))]
                {
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
                }

                #[cfg(feature = "net_http")]
                {
                    // 在启用 Wi‑Fi/HTTP 的构建中，为了节省 DRAM，仅保留单帧缓冲，
                    // 这里退化为“整帧推送”，但必须分块（避免单次大事务卡住 SPI/DMA）。
                    let bytes_per_row = DISPLAY_WIDTH * 2;
                    let mut y = 0usize;
                    let mut spans = 0usize;
                    while y < DISPLAY_HEIGHT {
                        let rows = core::cmp::min(DISPLAY_CHUNK_ROWS, DISPLAY_HEIGHT - y);
                        let start = y * bytes_per_row;
                        let end = start + rows * bytes_per_row;
                        display
                            .show_raw_data(
                                0,
                                y as u16,
                                DISPLAY_WIDTH as u16,
                                rows as u16,
                                &ctx.framebuffer[start..end],
                            )
                            .await
                            .expect("frame push (full chunked)");
                        spans += 1;

                        for _ in 0..DISPLAY_CHUNK_YIELD_LOOPS {
                            yield_now().await;
                        }
                        y += rows;
                    }
                    dirty_rows = DISPLAY_HEIGHT;
                    dirty_spans = spans;
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
                // When chunk_limit < dma buffer len, received bytes may wrap across descriptors.
                // Always consume via the provided iterator to preserve ordering.
                for chunk in buf_back.received_data() {
                    feed_decoder(chunk, decoder, telemetry).await;
                }
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
            Ok(Some(frame)) => {
                // Ignore obvious noise: SLIP frame shorter than header+CRC cannot be valid.
                if frame.len() < HEADER_LEN + CRC_LEN {
                    decoder.reset();
                    continue;
                }

                // Fast-path framing sanity: drop frames whose declared length does not
                // match the actual SLIP payload to avoid surfacing spurious
                // `payload length mismatch` decode errors when bytes are truncated in
                // transit.
                let declared_payload_len = u16::from_le_bytes([frame[4], frame[5]]) as usize;
                let expected_total = HEADER_LEN + declared_payload_len + CRC_LEN;
                if expected_total != frame.len() {
                    let drops = PROTO_FRAMING_DROPS.fetch_add(1, Ordering::Relaxed) + 1;
                    rate_limited_framing_warn(frame.len(), declared_payload_len, drops);
                    decoder.reset();
                    continue;
                }

                match decode_frame(&frame) {
                    Ok((header, _payload)) => match header.msg {
                        MSG_HELLO => match decode_hello_frame(&frame) {
                            Ok((_hdr, hello)) => {
                                record_link_activity();
                                // Cache the last fw_version so higher-level views (e.g. HTTP
                                // identity endpoint) can expose a compact analog firmware
                                // identifier without having to inspect UART traffic.
                                ANALOG_FW_VERSION_RAW.store(hello.fw_version, Ordering::Relaxed);

                                let first = !HELLO_SEEN.swap(true, Ordering::Relaxed);
                                if first {
                                    LINK_UP.store(true, Ordering::Relaxed);
                                    prompt_tone::set_link_up(true);
                                }
                                if first {
                                    info!(
                                        "HELLO received from analog (link up): proto_ver={} fw_ver=0x{:08x}",
                                        hello.protocol_version, hello.fw_version
                                    );
                                } else {
                                    info!(
                                        "HELLO received again from analog: proto_ver={} fw_ver=0x{:08x}",
                                        hello.protocol_version, hello.fw_version
                                    );
                                }
                            }
                            Err(err) => {
                                PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                                rate_limited_proto_warn(
                                    protocol_error_str(&err),
                                    Some(frame.as_slice()),
                                );
                                decoder.reset();
                            }
                        },
                        MSG_FAST_STATUS => match decode_fast_status_frame(&frame) {
                            Ok((_hdr, status)) => {
                                record_link_activity();
                                apply_fast_status(telemetry, &status).await;
                                let total =
                                    FAST_STATUS_OK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                                if total % 32 == 0 {
                                    let display_running =
                                        DISPLAY_TASK_RUNNING.load(Ordering::Relaxed);
                                    info!(
                                        "fast_status ok (count={}, display_running={}, i_local_ma={} mA, target_value={} mA)",
                                        total,
                                        display_running,
                                        status.i_local_ma,
                                        status.target_value
                                    );
                                }
                            }
                            Err(err) => {
                                PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                                rate_limited_proto_warn(
                                    protocol_error_str(&err),
                                    Some(frame.as_slice()),
                                );
                                decoder.reset();
                            }
                        },
                        MSG_SET_POINT => {
                            if header.flags & FLAG_IS_ACK != 0 {
                                record_link_activity();
                                handle_setpoint_ack(&header);
                            } else {
                                // For now we do not expect SetPoint requests on the digital side.
                                rate_limited_proto_warn("unexpected setpoint frame", None);
                            }
                        }
                        MSG_SET_MODE => {
                            if header.flags & FLAG_IS_ACK != 0 {
                                record_link_activity();
                                handle_setmode_ack(&header);
                            } else {
                                // For now we do not expect SetMode requests on the digital side.
                                rate_limited_proto_warn("unexpected setmode frame", None);
                            }
                        }
                        MSG_SOFT_RESET => match decode_soft_reset_frame(&frame) {
                            Ok((hdr, reset)) => {
                                record_link_activity();
                                handle_soft_reset_frame(&hdr, &reset);
                            }
                            Err(err) => {
                                PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                                rate_limited_proto_warn(
                                    protocol_error_str(&err),
                                    Some(frame.as_slice()),
                                );
                                decoder.reset();
                            }
                        },
                        MSG_PD_STATUS => match decode_pd_status_frame(&frame) {
                            Ok((_hdr, status)) => {
                                record_link_activity();
                                apply_pd_status(telemetry, status).await;
                            }
                            Err(err) => {
                                PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                                rate_limited_proto_warn(
                                    protocol_error_str(&err),
                                    Some(frame.as_slice()),
                                );
                                decoder.reset();
                            }
                        },
                        MSG_PD_SINK_REQUEST => {
                            if header.flags & (FLAG_IS_ACK | FLAG_IS_NACK) != 0 {
                                record_link_activity();
                                handle_pd_sink_request_ack(&header);
                            } else {
                                rate_limited_proto_warn(
                                    "unexpected PD_SINK_REQUEST frame (not ack/nack)",
                                    Some(frame.as_slice()),
                                );
                            }
                        }
                        MSG_CAL_MODE => match decode_cal_mode_frame(&frame) {
                            Ok((hdr, mode)) => {
                                record_link_activity();
                                if hdr.flags & FLAG_IS_ACK != 0 {
                                    let total =
                                        CAL_MODE_ACK_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
                                    info!(
                                        "cal_mode ACK received: seq={} kind={:?} (ack_total={})",
                                        hdr.seq, mode.kind, total
                                    );
                                } else {
                                    warn!("cal_mode request received from analog side; ignoring");
                                }
                            }
                            Err(err) => {
                                PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                                rate_limited_proto_warn(
                                    protocol_error_str(&err),
                                    Some(frame.as_slice()),
                                );
                                decoder.reset();
                            }
                        },
                        _ => {
                            rate_limited_proto_warn("unsupported msg", Some(frame.as_slice()));
                        }
                    },
                    Err(err) => {
                        PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                        rate_limited_proto_warn(protocol_error_str(&err), Some(frame.as_slice()));
                        decoder.reset();
                    }
                }
            }
            Ok(None) => {}
            Err(err) => {
                PROTO_DECODE_ERRS.fetch_add(1, Ordering::Relaxed);
                rate_limited_proto_warn(protocol_error_str(&err), None);
                decoder.reset();
            }
        }
    }
}

fn record_uart_error() {
    UART_RX_ERR_TOTAL.fetch_add(1, Ordering::Relaxed);
}

fn record_link_activity() {
    let now = now_ms32();
    LAST_GOOD_FRAME_MS.store(now, Ordering::Relaxed);
}

fn handle_setpoint_ack(header: &FrameHeader) {
    SETPOINT_LAST_ACK_SEQ.store(header.seq, Ordering::Relaxed);
    SETPOINT_ACK_PENDING.store(false, Ordering::Release);
    let total = SETPOINT_ACK_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
    info!(
        "setpoint ack received: seq={} flags=0x{:02x} len={} (ack_total={})",
        header.seq, header.flags, header.len, total
    );
}

fn handle_setmode_ack(header: &FrameHeader) {
    SETMODE_LAST_ACK_SEQ.store(header.seq, Ordering::Relaxed);
    SETMODE_ACK_PENDING.store(false, Ordering::Release);
    let total = SETMODE_ACK_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
    info!(
        "setmode ack received: seq={} flags=0x{:02x} len={} (ack_total={})",
        header.seq, header.flags, header.len, total
    );
}

fn handle_pd_sink_request_ack(header: &FrameHeader) {
    PD_REQ_LAST_ACK_SEQ.store(header.seq, Ordering::Relaxed);
    PD_REQ_LAST_ACK_FLAGS.store(header.flags, Ordering::Relaxed);
    PD_REQ_ACK_PENDING.store(false, Ordering::Release);
    let total = PD_REQ_ACK_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
    if header.flags & FLAG_IS_NACK != 0 {
        warn!(
            "pd_sink_request NACK received: seq={} flags=0x{:02x} len={} (ack_total={})",
            header.seq, header.flags, header.len, total
        );
    } else {
        info!(
            "pd_sink_request ACK received: seq={} flags=0x{:02x} len={} (ack_total={})",
            header.seq, header.flags, header.len, total
        );
    }
}

fn handle_soft_reset_frame(header: &FrameHeader, reset: &SoftReset) {
    if header.flags & FLAG_IS_ACK != 0 {
        SOFT_RESET_ACKED.store(true, Ordering::Relaxed);
        info!(
            "soft_reset ACK received: seq={} reason={:?} ts_ms={}",
            header.seq, reset.reason, reset.timestamp_ms
        );
    } else {
        warn!("soft_reset request received from analog side; ignoring");
    }
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

fn rate_limited_proto_warn(kind: &str, frame: Option<&[u8]>) {
    let now = now_ms32();
    let last = LAST_PROTO_WARN_MS.load(Ordering::Relaxed);
    if now.wrapping_sub(last) >= 2000 {
        LAST_PROTO_WARN_MS.store(now, Ordering::Relaxed);
        let cnt = PROTO_DECODE_ERRS.load(Ordering::Relaxed);
        let len = frame.map(|f| f.len()).unwrap_or(0);
        let mut head_buf = [0u8; 8];
        let head = frame.map(|f| {
            let head_len = f.len().min(head_buf.len());
            head_buf[..head_len].copy_from_slice(&f[..head_len]);
            &head_buf[..head_len]
        });
        warn!(
            "protocol decode error ({}), frame_len={}, head={:02x} [total={}]; resetting",
            kind,
            len,
            head.unwrap_or(&[]),
            cnt
        );
    }
}

fn rate_limited_framing_warn(frame_len: usize, declared_payload_len: usize, drops: u32) {
    let now = now_ms32();
    let last = LAST_PROTO_WARN_MS.load(Ordering::Relaxed);
    if now.wrapping_sub(last) >= 2000 {
        LAST_PROTO_WARN_MS.store(now, Ordering::Relaxed);
        warn!(
            "protocol framing drop (payload length mismatch): frame_len={} declared_payload_len={} total_drop={}",
            frame_len, declared_payload_len, drops
        );
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = hal::init(hal::Config::default());

    #[cfg(feature = "net_http")]
    {
        // Reserve reclaimed bootloader RAM as heap for Wi‑Fi + HTTP stack
        // allocations, avoiding additional pressure on the main DRAM region.
        esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 64 * 1024);
    }

    // Initialize the preemptive scheduler used by esp-radio + embassy-net
    // before any Wi‑Fi/HTTP tasks are spawned.
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("LoadLynx digital firmware version: {}", FW_VERSION);
    log_wifi_config();
    info!("LoadLynx digital alive; initializing local peripherals");
    // Lightweight probe to help verify that application logs are reaching the
    // same serial monitor path as the ROM/bootloader output.
    esp_println::println!("digital-log-probe: main() started, peripherals initialized");

    // 禁用 PAD‑JTAG，将 MTCK/MTDO/MTDI/MTMS (GPIO39–GPIO42) 释放为普通 GPIO，
    // 以便下文配置 RGB PWM 与 FAN_PWM/FAN_TACH。
    disable_pad_jtag_for_reclaimed_pins();

    // GPIO33 → FPC → 5V_EN, which drives the TPS82130SILR buck (docs/power/netlists/analog-board-netlist.enet).
    let alg_en_pin = peripherals.GPIO33;
    let mut alg_en = Output::new(alg_en_pin, Level::Low, OutputConfig::default());

    // External I2C EEPROM (M24C64-FMC6TG) on GPIO8=SDA / GPIO9=SCL, 7-bit addr 0x50.
    info!(
        "initializing I2C0 EEPROM (GPIO8=SDA, GPIO9=SCL, addr=0x{:02x})",
        calfmt::EEPROM_I2C_ADDR_7BIT
    );
    let i2c0 = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .expect("i2c0 init")
    .with_sda(peripherals.GPIO8)
    .with_scl(peripherals.GPIO9)
    .into_async();
    let i2c0_bus = i2c0::init(i2c0);
    let eeprom = EEPROM.init(Mutex::new(eeprom::SharedM24c64::new(i2c0_bus)));

    // Load calibration profile from EEPROM; if invalid, fall back to firmware defaults.
    let initial_profile = {
        let mut guard = eeprom.lock().await;
        match guard.read_profile_blob().await {
            Ok(blob) => match calfmt::deserialize_profile(&blob, calfmt::DIGITAL_HW_REV) {
                Ok(profile) => {
                    info!(
                        "EEPROM calibration profile loaded (fmt_version={}, hw_rev={})",
                        profile.fmt_version, profile.hw_rev
                    );
                    profile
                }
                Err(err) => {
                    let err_kind = match err {
                        calfmt::ProfileLoadError::InvalidLength => "invalid_length",
                        calfmt::ProfileLoadError::UnsupportedFmtVersion(_) => "fmt_version",
                        calfmt::ProfileLoadError::HwRevMismatch { .. } => "hw_rev",
                        calfmt::ProfileLoadError::InvalidCounts => "counts",
                        calfmt::ProfileLoadError::CrcMismatch { .. } => "crc32",
                    };
                    warn!(
                        "EEPROM calibration profile invalid; using factory-default (err={})",
                        err_kind
                    );
                    ActiveProfile::factory_default(calfmt::DIGITAL_HW_REV)
                }
            },
            Err(err) => {
                warn!("EEPROM read failed; using factory-default (err={:?})", err);
                ActiveProfile::factory_default(calfmt::DIGITAL_HW_REV)
            }
        }
    };

    // Load presets from EEPROM (non-overlapping with the calibration blob).
    // On invalid blob (version/CRC), fall back to firmware defaults.
    let initial_presets = {
        let mut guard = eeprom.lock().await;
        match guard.read_presets_blob().await {
            Ok(blob) => match control::decode_presets_blob(&blob) {
                Ok(presets) => {
                    info!("EEPROM presets loaded (count={})", presets.len());
                    presets
                }
                Err(err) => {
                    let kind = match err {
                        PresetsBlobError::InvalidMagic => "magic",
                        PresetsBlobError::UnsupportedVersion(_) => "version",
                        PresetsBlobError::InvalidCount(_) => "count",
                        PresetsBlobError::CrcMismatch { .. } => "crc32",
                        PresetsBlobError::InvalidLayout => "layout",
                        PresetsBlobError::InvalidPresetId(_) => "preset_id",
                        PresetsBlobError::InvalidMode(_) => "mode",
                    };
                    warn!("EEPROM presets invalid; using defaults (err={})", kind);
                    control::default_presets()
                }
            },
            Err(err) => {
                warn!("EEPROM presets read failed; using defaults (err={:?})", err);
                control::default_presets()
            }
        }
    };

    // Load PD config from EEPROM (non-overlapping with presets).
    // On invalid blob (version/CRC), fall back to firmware defaults (5V Fixed).
    let initial_pd = {
        let mut guard = eeprom.lock().await;
        match guard.read_pd_blob().await {
            Ok(blob) => match control::decode_pd_blob(&blob) {
                Ok(cfg) => {
                    info!(
                        "EEPROM PD config loaded (mode={:?}, target_mv={}, i_req_ma={})",
                        cfg.mode, cfg.target_mv, cfg.i_req_ma
                    );
                    cfg
                }
                Err(err) => {
                    let kind = match err {
                        control::PdBlobError::InvalidMagic => "magic",
                        control::PdBlobError::UnsupportedVersion(_) => "version",
                        control::PdBlobError::InvalidMode(_) => "mode",
                        control::PdBlobError::InvalidTarget(_) => "target_mv",
                        control::PdBlobError::CrcMismatch { .. } => "crc32",
                        control::PdBlobError::InvalidLayout => "layout",
                    };
                    warn!("EEPROM PD config invalid; using defaults (err={})", kind);
                    control::PdConfig::default()
                }
            },
            Err(err) => {
                warn!(
                    "EEPROM PD config read failed; using defaults (err={:?})",
                    err
                );
                control::PdConfig::default()
            }
        }
    };

    // SPI2 provides the high-speed channel for the TFT.
    let spi_peripheral = peripherals.SPI2;
    let sck = peripherals.GPIO12;
    let mosi = peripherals.GPIO11;
    let cs_pin = peripherals.GPIO13;
    let dc_pin = peripherals.GPIO10;
    let rst_pin = peripherals.GPIO6;
    let backlight_pin = peripherals.GPIO15;
    let fan_pwm_pin = peripherals.GPIO41; // MTDI / FAN_PWM（PAD‑JTAG 已在启动早期释放）
    let buzzer_pin = peripherals.GPIO21; // BUZZER (prompt tone manager)
    let amp_sd_mode_pin = peripherals.GPIO34; // MAX98357A SD_MODE (AMP_EN)
    let i2s_bclk_pin = peripherals.GPIO35; // I2S_BCLK
    let i2s_lrclk_pin = peripherals.GPIO36; // I2S_LRCLK
    let i2s_din_pin = peripherals.GPIO37; // I2S_DIN (ESP -> AMP)
    // NOTE: GPIO42 (MTMS) is wired to FAN_TACH and intentionally left unused here;
    // a future task will configure it for tachometer feedback.
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

    let backlight_pin = BACKLIGHT_PIN.init(backlight_pin.into());
    backlight_pin
        .apply_output_config(&OutputConfig::default().with_drive_mode(DriveMode::PushPull));
    backlight_pin.set_output_enable(true);
    backlight_pin.set_output_high(true);

    let mut backlight_timer = ledc.timer::<LowSpeed>(ledc_timer::Number::Timer0);
    backlight_timer
        .configure(ledc_timer::config::Config {
            duty: ledc_timer::config::Duty::Duty10Bit,
            clock_source: ledc_timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(20),
        })
        .expect("backlight timer");
    let backlight_timer = BACKLIGHT_TIMER.init(backlight_timer);

    let mut backlight_channel = ledc.channel::<LowSpeed>(ledc_channel::Number::Channel0, NoPin);
    backlight_channel
        .configure(ledc_channel::config::Config {
            timer: &*backlight_timer,
            // 调试阶段提升背光亮度，避免“有画面但看起来近似黑屏”。
            duty_pct: backlight_hw_duty_pct(BACKLIGHT_DEFAULT_PCT),
            drive_mode: DriveMode::PushPull,
        })
        .expect("backlight channel");
    let backlight_channel = BACKLIGHT_CHANNEL.init(backlight_channel);
    backlight_pwm_connect(backlight_pin);
    set_backlight_pct(backlight_channel, BACKLIGHT_DEFAULT_PCT, "init");

    // FAN_PWM: 低速 LEDC 通道，恒定 20–25 kHz 频率，由 fan_task 周期性更新占空比。
    let mut fan_timer = ledc.timer::<LowSpeed>(ledc_timer::Number::Timer1);
    fan_timer
        .configure(ledc_timer::config::Config {
            duty: ledc_timer::config::Duty::Duty10Bit,
            clock_source: ledc_timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(FAN_PWM_FREQUENCY_KHZ),
        })
        .expect("fan timer");
    let fan_timer = FAN_TIMER.init(fan_timer);

    let mut fan_channel = ledc.channel::<LowSpeed>(ledc_channel::Number::Channel1, fan_pwm_pin);
    fan_channel
        .configure(ledc_channel::config::Config {
            timer: &*fan_timer,
            duty_pct: FAN_DUTY_DEFAULT_PCT,
            drive_mode: DriveMode::PushPull,
        })
        .expect("fan channel");
    let fan_channel = FAN_CHANNEL.init(fan_channel);
    fan_channel
        .set_duty(FAN_DUTY_DEFAULT_PCT)
        .expect("fan duty default");

    // BUZZER: low-speed LEDC Timer2/Channel2, used by prompt_tone_task.
    let mut buzzer_timer = ledc.timer::<LowSpeed>(ledc_timer::Number::Timer2);
    buzzer_timer
        .configure(ledc_timer::config::Config {
            duty: ledc_timer::config::Duty::Duty10Bit,
            clock_source: ledc_timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(prompt_tone::BUZZER_FREQ_HZ),
        })
        .expect("buzzer timer");
    let buzzer_timer = BUZZER_TIMER.init(buzzer_timer);

    let mut buzzer_channel = ledc.channel::<LowSpeed>(ledc_channel::Number::Channel2, buzzer_pin);
    buzzer_channel
        .configure(ledc_channel::config::Config {
            timer: &*buzzer_timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })
        .expect("buzzer channel");
    let buzzer_channel = BUZZER_CHANNEL.init(buzzer_channel);
    buzzer_channel.set_duty(0).expect("buzzer duty init");

    // RGB status LED (Plan #0021): low-speed LEDC Timer3 + Channel3/4/5.
    // Pin map (digital board netlist): R=GPIO38, G=GPIO39(MTCK), B=GPIO40(MTDO).
    let mut rgb_timer = ledc.timer::<LowSpeed>(ledc_timer::Number::Timer3);
    rgb_timer
        .configure(ledc_timer::config::Config {
            duty: ledc_timer::config::Duty::Duty10Bit,
            clock_source: ledc_timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(RGB_STATUS_PWM_FREQUENCY_KHZ),
        })
        .expect("rgb timer");
    let rgb_timer = RGB_STATUS_TIMER.init(rgb_timer);

    let mut rgb_r_channel =
        ledc.channel::<LowSpeed>(ledc_channel::Number::Channel3, peripherals.GPIO38);
    rgb_r_channel
        .configure(ledc_channel::config::Config {
            timer: &*rgb_timer,
            duty_pct: 100, // active-low: 100% HIGH == OFF
            drive_mode: DriveMode::PushPull,
        })
        .expect("rgb r channel");
    let rgb_r_channel = RGB_STATUS_R_CHANNEL.init(rgb_r_channel);
    rgb_r_channel.set_duty(100).expect("rgb r duty init");

    let mut rgb_g_channel =
        ledc.channel::<LowSpeed>(ledc_channel::Number::Channel4, peripherals.GPIO39);
    rgb_g_channel
        .configure(ledc_channel::config::Config {
            timer: &*rgb_timer,
            duty_pct: 100, // active-low: 100% HIGH == OFF
            drive_mode: DriveMode::PushPull,
        })
        .expect("rgb g channel");
    let rgb_g_channel = RGB_STATUS_G_CHANNEL.init(rgb_g_channel);
    rgb_g_channel.set_duty(100).expect("rgb g duty init");

    let mut rgb_b_channel =
        ledc.channel::<LowSpeed>(ledc_channel::Number::Channel5, peripherals.GPIO40);
    rgb_b_channel
        .configure(ledc_channel::config::Config {
            timer: &*rgb_timer,
            duty_pct: 100, // active-low: 100% HIGH == OFF
            drive_mode: DriveMode::PushPull,
        })
        .expect("rgb b channel");
    let rgb_b_channel = RGB_STATUS_B_CHANNEL.init(rgb_b_channel);
    rgb_b_channel.set_duty(100).expect("rgb b duty init");

    let framebuffer = &mut FRAMEBUFFER.init_with(|| Align32([0; FRAMEBUFFER_LEN])).0;
    #[cfg(not(feature = "net_http"))]
    let prev_framebuffer = &mut PREVIOUS_FRAMEBUFFER
        .init_with(|| Align32([0; FRAMEBUFFER_LEN]))
        .0;

    let resources = DISPLAY_RESOURCES.init(DisplayResources {
        spi: Some(spi),
        cs: Some(cs),
        dc: Some(dc),
        rst: Some(rst),
        framebuffer,
        #[cfg(not(feature = "net_http"))]
        previous_framebuffer: prev_framebuffer,
    });

    let telemetry = TELEMETRY.init(Mutex::new(TelemetryModel::new()));
    let calibration = CALIBRATION.init(Mutex::new(CalibrationState::new(initial_profile)));
    let control = CONTROL.init(Mutex::new(ControlState::new(initial_presets, initial_pd)));
    CONTROL_REV.store(1, Ordering::Relaxed);
    LAST_USER_ACTIVITY_MS.store(now_ms32(), Ordering::Relaxed);
    SCREEN_POWER_STATE.store(SCREEN_POWER_STATE_ACTIVE, Ordering::Relaxed);

    let touch_input_cfg = InputConfig::default().with_pull(Pull::Up);
    let ctp_int = Input::new(peripherals.GPIO7, touch_input_cfg);
    let ctp_rst = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());

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
        let (uhci_rx_buf, uhci_rx_desc, _uhci_tx_buf, _uhci_tx_desc) =
            esp_hal::dma_buffers!(UART_DMA_BUF_LEN);
        let dma_rx = DmaRxBuf::new(uhci_rx_desc, uhci_rx_buf).expect("uhci dma rx buf");

        let uart_blocking = Uart::new(peripherals.UART1, uart_cfg)
            .expect("uart1 init")
            .with_tx(peripherals.GPIO17)
            .with_rx(peripherals.GPIO18);

        let mut uhci =
            Uhci::new(uart_blocking, peripherals.UHCI0, peripherals.DMA_CH1).into_async();
        // NOTE: chunk_limit must not exceed the DMA buffer length.
        // Keep it equal to the DMA buffer so UHCI/DMA does not lock up.
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

    info!("spawning ticker task");
    spawner.spawn(ticker()).expect("ticker spawn");
    info!("spawning diag task");
    spawner.spawn(diag_task()).expect("diag_task spawn");

    #[cfg(not(feature = "mock_setpoint"))]
    {
        info!("spawning encoder task");
        spawner
            .spawn(encoder_task(
                encoder_unit,
                encoder_counter,
                encoder_button,
                control,
                telemetry,
            ))
            .expect("encoder_task spawn");
    }

    #[cfg(feature = "mock_setpoint")]
    {
        info!("spawning mock setpoint task");
        spawner
            .spawn(mock_setpoint_task())
            .expect("mock_setpoint_task spawn");
    }
    info!("spawning touch spring task");
    spawner
        .spawn(touch_spring_task(
            peripherals.GPIO14,
            control,
            rgb_r_channel,
            rgb_g_channel,
            rgb_b_channel,
        ))
        .expect("touch_spring_task spawn");
    info!("spawning display task");
    spawner
        .spawn(display_task(
            resources,
            telemetry,
            control,
            backlight_channel,
            backlight_pin,
        ))
        .expect("display_task spawn");
    info!("spawning touch task");
    spawner
        .spawn(touch::touch_task(ctp_int, ctp_rst))
        .expect("touch_task spawn");
    info!("spawning touch-ui task");
    spawner
        .spawn(touch_ui_task(control, telemetry, eeprom))
        .expect("touch_ui_task spawn");
    info!("spawning prompt tone task");
    spawner
        .spawn(prompt_tone::prompt_tone_task(buzzer_channel))
        .expect("prompt_tone_task spawn");
    info!("spawning speaker task");
    spawner
        .spawn(speaker::speaker_task(
            peripherals.I2S0,
            peripherals.DMA_CH2,
            amp_sd_mode_pin,
            i2s_bclk_pin,
            i2s_lrclk_pin,
            i2s_din_pin,
        ))
        .expect("speaker_task spawn");
    info!("spawning fan task");
    spawner
        .spawn(fan_task(telemetry, fan_channel))
        .expect("fan_task spawn");
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
    info!("spawning load guard task");
    spawner
        .spawn(load_guard_task(control))
        .expect("load_guard_task spawn");
    if let Some(uhci_tx) = uhci_tx_opt.take() {
        info!("spawning SetMode tx task (UHCI TX, active control)");
        spawner
            .spawn(setmode_tx_task(uhci_tx, calibration, control, telemetry))
            .expect("setmode_tx_task spawn");
    } else {
        warn!("SetMode tx task not started (UHCI TX unavailable)");
    }

    // Wi‑Fi + HTTP server: runs as a separate Embassy task tree. Failures are
    // logged and retried internally; UART/UI functionality must not depend
    // on Wi‑Fi availability.
    #[cfg(feature = "net_http")]
    {
        let wifi_state = net::init_wifi_state();
        info!("spawning Wi-Fi + HTTP net tasks");
        net::spawn_wifi_and_http(
            &spawner,
            peripherals.WIFI,
            wifi_state,
            telemetry,
            calibration,
            eeprom,
            control,
        );
        info!("spawning Wi-Fi UI bridge task");
        spawner
            .spawn(wifi_ui_task(wifi_state, telemetry))
            .expect("wifi_ui_task spawn");
    }

    // Keep the async main task alive; all real work runs in spawned tasks.
    loop {
        yield_now().await;
    }
}

// 周期性聚合统计，启动后每 5 秒打印一次（便于 DMA 验证阶段观察计数）
#[embassy_executor::task]
async fn stats_task() {
    let mut last_stats_ms = timestamp_ms();
    let mut prev_link_up = LINK_UP.load(Ordering::Relaxed);
    let mut link_alarm_fired_for_down: bool = false;

    const LINK_DOWN_DETECT_MS: u32 = 300;
    // "Persistent link fault" threshold: only latch the audible alarm when
    // link-down has been continuous for this long.
    const LINK_ALARM_LATCH_MS: u32 = 3_000;
    loop {
        cooperative_delay_ms(100).await;

        // Link health: derive LINK_UP from the age of the last successfully
        // processed frame. A gap >LINK_DOWN_DETECT_MS is treated as a *transient*
        // link down (UI-only).
        let now_ms32 = now_ms32();
        let last_good = LAST_GOOD_FRAME_MS.load(Ordering::Relaxed);
        let age_ms = now_ms32.wrapping_sub(last_good);
        let link_now = last_good != 0 && age_ms <= LINK_DOWN_DETECT_MS;
        if link_now != prev_link_up {
            prev_link_up = link_now;
            LINK_UP.store(link_now, Ordering::Relaxed);
            prompt_tone::set_link_up(link_now);
            if link_now {
                info!("link up (last_good_frame_age={} ms)", age_ms);
            } else {
                warn!("link down (no frames for {} ms)", age_ms);
                ANALOG_STATE.store(AnalogState::Offline as u8, Ordering::Relaxed);
            }
        }

        // Audible link alarm policy:
        // - do NOT latch before we have ever received HELLO (boot handshake may take time)
        // - only latch when link-down persists for a while ("continuous fault")
        let link_down = HELLO_SEEN.load(Ordering::Relaxed) && !link_now;
        if link_down {
            if age_ms >= LINK_ALARM_LATCH_MS && !link_alarm_fired_for_down {
                link_alarm_fired_for_down = true;
                warn!(
                    "link fault persisted (no frames for {} ms); latching Critical LNK alarm",
                    age_ms
                );
                prompt_tone::latch_link_alarm();
            }
        } else {
            link_alarm_fired_for_down = false;
        }

        let now = timestamp_ms();
        if now.saturating_sub(last_stats_ms) >= 1_000 {
            last_stats_ms = now;
            let ok = FAST_STATUS_OK_COUNT.load(Ordering::Relaxed);
            let de = PROTO_DECODE_ERRS.load(Ordering::Relaxed);
            let df = PROTO_FRAMING_DROPS.load(Ordering::Relaxed);
            let ut = UART_RX_ERR_TOTAL.load(Ordering::Relaxed);
            let sp_tx = SETPOINT_TX_TOTAL.load(Ordering::Relaxed);
            let sp_ack = SETPOINT_ACK_TOTAL.load(Ordering::Relaxed);
            let sp_retx = SETPOINT_RETX_TOTAL.load(Ordering::Relaxed);
            let sp_timeout = SETPOINT_TIMEOUT_TOTAL.load(Ordering::Relaxed);
            let touch_int = touch::TOUCH_INT_COUNT.load(Ordering::Relaxed);
            let touch_i2c = touch::TOUCH_I2C_READ_COUNT.load(Ordering::Relaxed);
            let touch_parse_fail = touch::TOUCH_PARSE_FAIL_COUNT.load(Ordering::Relaxed);
            let ts_reads = TOUCH_SPRING_READ_TOTAL.load(Ordering::Relaxed);
            let ts_down = TOUCH_SPRING_TOUCHDOWN_TOTAL.load(Ordering::Relaxed);
            let ts_suppress = TOUCH_SPRING_SUPPRESS_COOLDOWN_TOTAL.load(Ordering::Relaxed);
            let ts_block = TOUCH_SPRING_ENABLE_BLOCK_TOTAL.load(Ordering::Relaxed);
            let ts_raw = TOUCH_SPRING_LAST_RAW.load(Ordering::Relaxed);
            let ts_base = TOUCH_SPRING_LAST_BASELINE.load(Ordering::Relaxed);
            let ts_da = TOUCH_SPRING_LAST_DELTA_ABS.load(Ordering::Relaxed);
            let spk_play = speaker::SPEAKER_PLAY_TOTAL.load(Ordering::Relaxed);
            let spk_drop = speaker::SPEAKER_ENQUEUE_DROPS.load(Ordering::Relaxed);
            info!(
                "stats: fast_status_ok={}, decode_errs={}, framing_drops={}, uart_rx_err_total={}, setpoint_tx={}, ack={}, retx={}, timeout={}, touch_int={}, touch_i2c_reads={}, touch_parse_fail={}, touch_spring_reads={}, touch_spring_down={}, touch_spring_suppress={}, touch_spring_block={}, touch_spring_raw={}, touch_spring_baseline={}, touch_spring_delta_abs={}, speaker_play={}, speaker_drop={}",
                ok,
                de,
                df,
                ut,
                sp_tx,
                sp_ack,
                sp_retx,
                sp_timeout,
                touch_int,
                touch_i2c,
                touch_parse_fail,
                ts_reads,
                ts_down,
                ts_suppress,
                ts_block,
                ts_raw,
                ts_base,
                ts_da,
                spk_play,
                spk_drop,
            );
        }
    }
}

#[embassy_executor::task]
async fn load_guard_task(control: &'static ControlMutex) {
    info!(
        "load guard task starting (edge-triggered LOAD OFF on UVLO/* + Critical LNK + OCP/OPP trip)"
    );
    let mut last_reason: Option<&'static str> = None;
    let mut ocp_over_streak: u8 = 0;
    let mut opp_over_streak: u8 = 0;
    loop {
        let reason = current_load_block_abbrev();
        let reason_appeared = reason.is_some() && reason != last_reason;
        last_reason = reason;

        {
            let mut guard = control.lock().await;
            if guard.output_enabled {
                if reason_appeared {
                    guard.output_enabled = false;
                    bump_control_rev();
                    if let Some(r) = reason {
                        info!("LOAD forced OFF (reason={})", r);
                    } else {
                        info!("LOAD forced OFF (reason=?)");
                    }
                    ocp_over_streak = 0;
                    opp_over_streak = 0;
                } else {
                    let preset = guard.active_preset();
                    let i_total_ma = LAST_I_TOTAL_MA.load(Ordering::Relaxed).max(0);
                    let p_mw = LAST_CALC_P_MW.load(Ordering::Relaxed);

                    // Debounce: require a sustained overshoot before tripping.
                    const TRIP_STREAK_TICKS: u8 = 3; // 3 * 50ms = 150ms

                    let ocp_ma = preset.max_i_ma_total;
                    let ocp_trip_ma = if ocp_ma <= 0 {
                        i32::MAX
                    } else {
                        // +max(5%, 50mA)
                        let margin = (ocp_ma / 20).max(50);
                        ocp_ma.saturating_add(margin)
                    };

                    let opp_mw = preset.max_p_mw;
                    let opp_trip_mw = if opp_mw == 0 {
                        u32::MAX
                    } else {
                        // +max(5%, 0.5W)
                        let margin = (opp_mw / 20).max(500);
                        opp_mw.saturating_add(margin)
                    };

                    if i_total_ma > ocp_trip_ma {
                        ocp_over_streak = ocp_over_streak.saturating_add(1);
                    } else {
                        ocp_over_streak = 0;
                    }

                    if p_mw > opp_trip_mw {
                        opp_over_streak = opp_over_streak.saturating_add(1);
                    } else {
                        opp_over_streak = 0;
                    }

                    if ocp_over_streak >= TRIP_STREAK_TICKS {
                        ocp_over_streak = 0;
                        opp_over_streak = 0;
                        guard.output_enabled = false;
                        bump_control_rev();
                        prompt_tone::latch_trip_alarm(prompt_tone::TripReason::Ocp);
                        info!(
                            "LOAD forced OFF (trip=OCP, i_total_ma={}mA > {}mA, ocp={}mA)",
                            i_total_ma, ocp_trip_ma, ocp_ma
                        );
                    } else if opp_over_streak >= TRIP_STREAK_TICKS {
                        ocp_over_streak = 0;
                        opp_over_streak = 0;
                        guard.output_enabled = false;
                        bump_control_rev();
                        prompt_tone::latch_trip_alarm(prompt_tone::TripReason::Opp);
                        info!(
                            "LOAD forced OFF (trip=OPP, p_mw={}mW > {}mW, opp={}mW)",
                            p_mw, opp_trip_mw, opp_mw
                        );
                    }
                }
            } else {
                ocp_over_streak = 0;
                opp_over_streak = 0;
            }
        }
        cooperative_delay_ms(50).await;
    }
}

/// SetPoint 发送任务：20 Hz（或按需要），带 ACK 等待与退避重传，最新值优先。
#[embassy_executor::task]
async fn setpoint_tx_task(
    mut uhci_tx: uhci::UhciTx<'static, Async>,
    calibration: &'static CalibrationMutex,
) {
    info!(
        "SetPoint TX task starting (ack_timeout={} ms, backoff={:?})",
        SETPOINT_ACK_TIMEOUT_MS, SETPOINT_RETRY_BACKOFF_MS
    );

    #[derive(Clone, Copy)]
    struct Pending {
        seq: u8,
        target_i_ma: i32,
        attempts: u8, // includes initial send
        ack_total_at_send: u32,
        deadline_ms: u32,
    }

    let mut raw = [0u8; 64];
    let mut slip = [0u8; 192];

    // Soft-reset handshake (fixed seq=0); proceed even if ACK arrives late.
    let soft_reset_seq: u8 = 0;
    let soft_reset_acked =
        send_soft_reset_handshake(&mut uhci_tx, soft_reset_seq, &mut raw, &mut slip).await;
    if !soft_reset_acked {
        warn!("soft_reset ack missing within retry window; continuing after quiet gap");
    }

    // 更长的静默让模拟侧 UART 启动稳定，避免一上电被突发刷屏。
    cooperative_delay_ms(300).await;

    let mut seq: u8 = 1;

    // Cold boot: send the full 4-curve calibration set so the analog side can
    // reach CAL_READY (empty curves are rejected on G431).
    let profile = { calibration.lock().await.profile.clone() };
    send_all_calibration_curves(
        &mut uhci_tx,
        &mut seq,
        &profile,
        &mut raw,
        &mut slip,
        "boot",
    )
    .await;

    // 启动链路后发送一次 SetEnable(true)，用于拉起模拟侧输出 gating。
    let enable_cmd = SetEnable { enable: true };
    match encode_set_enable_frame(seq, &enable_cmd, &mut raw) {
        Ok(frame_len) => match slip_encode(&raw[..frame_len], &mut slip) {
            Ok(slip_len) => match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
                Ok(written) if written == slip_len => {
                    let _ = uhci_tx.uart_tx.flush_async().await;
                    info!(
                        "SetEnable(true) frame sent seq={} len={} slip_len={}",
                        seq, frame_len, slip_len
                    );
                }
                Ok(written) => {
                    warn!(
                        "SetEnable(true) short write {} < {} (seq={})",
                        written, slip_len, seq
                    );
                }
                Err(err) => {
                    warn!(
                        "SetEnable(true) uart write error for seq={}: {:?}",
                        seq, err
                    );
                }
            },
            Err(err) => {
                warn!("SetEnable(true) slip_encode error: {:?}", err);
            }
        },
        Err(err) => {
            warn!("SetEnable(true) encode_set_enable_frame error: {:?}", err);
        }
    }
    seq = seq.wrapping_add(1);

    // 在握手完成后发送一次静态 LimitProfile v0，供模拟板建立软件软限。
    match encode_limit_profile_frame(seq, &LIMIT_PROFILE_DEFAULT, &mut raw) {
        Ok(frame_len) => match slip_encode(&raw[..frame_len], &mut slip) {
            Ok(slip_len) => match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
                Ok(written) if written == slip_len => {
                    let _ = uhci_tx.uart_tx.flush_async().await;
                    info!(
                        "LimitProfile v0 sent (msg=0x{:02x}): max_i={}mA max_p={}mW ovp={}mV temp_trip={}mC derate={}%, seq={} len={} slip_len={}",
                        MSG_LIMIT_PROFILE,
                        LIMIT_PROFILE_DEFAULT.max_i_ma,
                        LIMIT_PROFILE_DEFAULT.max_p_mw,
                        LIMIT_PROFILE_DEFAULT.ovp_mv,
                        LIMIT_PROFILE_DEFAULT.temp_trip_mc,
                        LIMIT_PROFILE_DEFAULT.thermal_derate_pct,
                        seq,
                        frame_len,
                        slip_len
                    );
                }
                Ok(written) => {
                    warn!(
                        "LimitProfile v0 short write {} < {} (seq={})",
                        written, slip_len, seq
                    );
                }
                Err(err) => {
                    warn!(
                        "LimitProfile v0 uart write error for seq={}: {:?}",
                        seq, err
                    );
                }
            },
            Err(err) => {
                warn!("LimitProfile v0 slip_encode error: {:?}", err);
            }
        },
        Err(err) => {
            warn!(
                "LimitProfile v0 encode_limit_profile_frame error: {:?}",
                err
            );
        }
    }
    seq = seq.wrapping_add(1);

    let mut pending: Option<Pending> = None;
    let mut last_sent_target: Option<i32> = None;
    let mut last_sent_ms: u32 = now_ms32();
    let mut mismatch_streak: u8 = 0;
    // Track how long we've been stuck in AnalogState::CalMissing so we can
    // log and opportunistically retry the CalWrite/SetEnable handshake.
    let mut calmissing_since_ms: Option<u32> = None;
    let mut last_calmissing_warn_ms: u32 = 0;
    let mut last_calmissing_handshake_ms: u32 = 0;
    let mut prev_link_up: bool = LINK_UP.load(Ordering::Relaxed);

    loop {
        yield_now().await;

        // Drain at most one pending soft-reset request per loop iteration.
        if let Some(reason) = crate::dequeue_soft_reset() {
            let soft_seq = seq;
            seq = seq.wrapping_add(1);
            send_soft_reset_one_shot(&mut uhci_tx, soft_seq, &mut raw, &mut slip, reason).await;
        }

        // Handle low-frequency calibration UART commands from the HTTP API.
        #[cfg(feature = "net_http")]
        if let Some(cmd) = crate::dequeue_cal_uart() {
            match cmd {
                CalUartCommand::SendAllCurves => {
                    let profile = { calibration.lock().await.profile.clone() };
                    send_all_calibration_curves(
                        &mut uhci_tx,
                        &mut seq,
                        &profile,
                        &mut raw,
                        &mut slip,
                        "http-cal",
                    )
                    .await;
                }
                CalUartCommand::SendCurve(kind) => {
                    let profile = { calibration.lock().await.profile.clone() };
                    send_calibration_curve(
                        &mut uhci_tx,
                        &mut seq,
                        &profile,
                        kind,
                        &mut raw,
                        &mut slip,
                        "http-cal",
                    )
                    .await;
                }
                CalUartCommand::SetMode(kind) => {
                    let seq_now = seq;
                    seq = seq.wrapping_add(1);
                    let _ = send_cal_mode_frame(
                        &mut uhci_tx,
                        seq_now,
                        kind,
                        &mut raw,
                        &mut slip,
                        "http-cal",
                    )
                    .await;
                }
            }
        }

        // On link recovery, re-send the full calibration set.
        let link_up_now = LINK_UP.load(Ordering::Relaxed);
        if link_up_now && !prev_link_up {
            prev_link_up = true;
            let profile = { calibration.lock().await.profile.clone() };
            send_all_calibration_curves(
                &mut uhci_tx,
                &mut seq,
                &profile,
                &mut raw,
                &mut slip,
                "link-recover",
            )
            .await;
        } else if !link_up_now && prev_link_up {
            prev_link_up = false;
        }

        let now = now_ms32();
        let setpoint_ma = clamp_target_ma(ENCODER_VALUE.load(Ordering::SeqCst) * ENCODER_STEP_MA);
        if setpoint_ma == 0 {
            LOAD_SWITCH_ENABLED.store(false, Ordering::SeqCst);
        }
        let desired_target = if LOAD_SWITCH_ENABLED.load(Ordering::SeqCst) {
            setpoint_ma
        } else {
            0
        };
        let observed_target = LAST_TARGET_VALUE_FROM_STATUS.load(Ordering::Relaxed);

        if observed_target == desired_target {
            mismatch_streak = 0;
        } else {
            mismatch_streak = mismatch_streak.saturating_add(1);
        }

        // Check for ACK arrival on the current pending seq.
        let ack_hit = if let Some(p) = pending.as_ref() {
            let ack_total = SETPOINT_ACK_TOTAL.load(Ordering::Relaxed);
            let ack_seq = SETPOINT_LAST_ACK_SEQ.load(Ordering::Relaxed);
            ack_total != p.ack_total_at_send && ack_seq == p.seq
        } else {
            false
        };
        if ack_hit {
            if let Some(p) = pending.take() {
                SETPOINT_ACK_PENDING.store(false, Ordering::Release);
                last_sent_target = Some(p.target_i_ma);
                last_sent_ms = now;
            }
            continue;
        }

        // Pre-empt with latest value if target changed while waiting.
        let mut should_send_new = false;
        let mut send_reason = "periodic";
        if let Some(p) = pending.as_ref() {
            if p.target_i_ma != desired_target {
                should_send_new = true;
                send_reason = "latest-value-preempt";
                pending = None; // drop old pending; latest值优先
            }
        } else if now.saturating_sub(last_sent_ms) >= SETPOINT_TX_PERIOD_MS {
            should_send_new = true;
            if last_sent_target
                .map(|t| t != desired_target)
                .unwrap_or(true)
            {
                send_reason = "target-change";
            } else {
                send_reason = "periodic";
            }
        } else if mismatch_streak >= 3 {
            should_send_new = true;
            send_reason = "telemetry-mismatch";
        }

        if should_send_new {
            // Gate 新 SetPoint：仅在链路就绪时才允许发送；HELLO 仅作附加信息。
            if !LINK_UP.load(Ordering::Relaxed) {
                let now_ms = now;
                let last_gate = LAST_SETPOINT_GATE_WARN_MS.load(Ordering::Relaxed);
                if now_ms.wrapping_sub(last_gate) >= 1_000 {
                    LAST_SETPOINT_GATE_WARN_MS.store(now_ms, Ordering::Relaxed);
                    warn!(
                        "SetPoint TX gated (link_up=false, hello_seen={}, target={} mA)",
                        HELLO_SEEN.load(Ordering::Relaxed),
                        desired_target
                    );
                }
                // 保留现有 pending/ACK 状态，仅抑制新指令。
                continue;
            }

            let analog_state = AnalogState::from_u8(ANALOG_STATE.load(Ordering::Relaxed));
            match analog_state {
                AnalogState::Faulted => {
                    // Leaving CalMissing; reset stuck timer.
                    calmissing_since_ms = None;

                    let now_ms = now;
                    let last_gate = LAST_SETPOINT_GATE_WARN_MS.load(Ordering::Relaxed);
                    if now_ms.wrapping_sub(last_gate) >= 1_000 {
                        LAST_SETPOINT_GATE_WARN_MS.store(now_ms, Ordering::Relaxed);
                        warn!("SetPoint TX gated: analog fault (state=FAULTED)");
                    }
                    continue;
                }
                AnalogState::CalMissing => {
                    let now_ms = now;
                    let last_gate = LAST_SETPOINT_GATE_WARN_MS.load(Ordering::Relaxed);
                    if now_ms.wrapping_sub(last_gate) >= 1_000 {
                        LAST_SETPOINT_GATE_WARN_MS.store(now_ms, Ordering::Relaxed);
                        warn!("SetPoint TX gated: analog not ready (calibration missing?)");
                    }

                    // Record when we first entered CalMissing.
                    let since = calmissing_since_ms.get_or_insert(now_ms);
                    let stuck_ms = now_ms.wrapping_sub(*since);

                    // After a short grace period, emit a rate-limited diagnostic and
                    // retry the SoftReset + CalWrite + SetEnable handshake to recover
                    // from a potentially dropped calibration write.
                    if stuck_ms >= 2_000 {
                        if now_ms.wrapping_sub(last_calmissing_warn_ms) >= 5_000 {
                            last_calmissing_warn_ms = now_ms;
                            warn!(
                                "analog stuck in CalMissing (link_up=true, fault_flags=0, enable=false, stuck_ms={})",
                                stuck_ms
                            );
                        }

                        if now_ms.wrapping_sub(last_calmissing_handshake_ms) >= 5_000 {
                            last_calmissing_handshake_ms = now_ms;
                            warn!(
                                "retrying SoftReset + CalWrite + SetEnable handshake due to CalMissing"
                            );

                            // SoftReset re-handshake: use a fresh seq to keep framing sane.
                            let soft_reset_seq = seq;
                            seq = seq.wrapping_add(1);
                            let soft_reset_acked = send_soft_reset_handshake(
                                &mut uhci_tx,
                                soft_reset_seq,
                                &mut raw,
                                &mut slip,
                            )
                            .await;
                            if !soft_reset_acked {
                                warn!(
                                    "soft_reset re-handshake: ack missing; continuing with CalWrite+SetEnable"
                                );
                            }

                            // Re-send the full calibration set (multi-chunk CalWrite) to unlock
                            // CAL_READY on the analog side.
                            let profile = { calibration.lock().await.profile.clone() };
                            send_all_calibration_curves(
                                &mut uhci_tx,
                                &mut seq,
                                &profile,
                                &mut raw,
                                &mut slip,
                                "calmissing-recover",
                            )
                            .await;

                            // Re-send SetEnable(true) to re-arm ENABLE_REQUESTED on analog.
                            let enable_seq = seq;
                            let enable_cmd = SetEnable { enable: true };
                            match encode_set_enable_frame(enable_seq, &enable_cmd, &mut raw) {
                                Ok(frame_len) => match slip_encode(&raw[..frame_len], &mut slip) {
                                    Ok(slip_len) => {
                                        match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
                                            Ok(written) if written == slip_len => {
                                                let _ = uhci_tx.uart_tx.flush_async().await;
                                                info!(
                                                    "SetEnable(true) frame re-sent seq={} len={} slip_len={}",
                                                    enable_seq, frame_len, slip_len
                                                );
                                            }
                                            Ok(written) => {
                                                warn!(
                                                    "SetEnable(true) re-send short write {} < {} (seq={})",
                                                    written, slip_len, enable_seq
                                                );
                                            }
                                            Err(err) => {
                                                warn!(
                                                    "SetEnable(true) re-send uart write error for seq={}: {:?}",
                                                    enable_seq, err
                                                );
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        warn!(
                                            "SetEnable(true) re-send slip_encode error: {:?}",
                                            err
                                        );
                                    }
                                },
                                Err(err) => {
                                    warn!(
                                        "SetEnable(true) re-send encode_set_enable_frame error: {:?}",
                                        err
                                    );
                                }
                            }
                            seq = seq.wrapping_add(1);
                        }
                    }

                    continue;
                }
                AnalogState::Offline | AnalogState::Ready => {
                    // Leaving CalMissing; reset stuck timer.
                    calmissing_since_ms = None;
                }
            }

            let send_seq = seq;
            seq = seq.wrapping_add(1);
            let ack_baseline = SETPOINT_ACK_TOTAL.load(Ordering::Relaxed);
            if send_setpoint_frame(
                &mut uhci_tx,
                send_seq,
                desired_target,
                &mut raw,
                &mut slip,
                send_reason,
            )
            .await
            {
                SETPOINT_TX_TOTAL.fetch_add(1, Ordering::Relaxed);
                let deadline = now.saturating_add(SETPOINT_ACK_TIMEOUT_MS);
                last_sent_target = Some(desired_target);
                last_sent_ms = now;
                pending = Some(Pending {
                    seq: send_seq,
                    target_i_ma: desired_target,
                    attempts: 1,
                    ack_total_at_send: ack_baseline,
                    deadline_ms: deadline,
                });
            } else {
                SETPOINT_ACK_PENDING.store(false, Ordering::Release);
            }
        } else if let Some(mut p) = pending.take() {
            // Timeout + retry path
            if now >= p.deadline_ms {
                if (p.attempts as usize) <= SETPOINT_RETRY_BACKOFF_MS.len() {
                    let backoff_ms = SETPOINT_RETRY_BACKOFF_MS[(p.attempts - 1) as usize];
                    let ack_baseline = SETPOINT_ACK_TOTAL.load(Ordering::Relaxed);
                    let send_seq = p.seq;
                    if send_setpoint_frame(
                        &mut uhci_tx,
                        send_seq,
                        p.target_i_ma,
                        &mut raw,
                        &mut slip,
                        "retx",
                    )
                    .await
                    {
                        SETPOINT_RETX_TOTAL.fetch_add(1, Ordering::Relaxed);
                        p.attempts = p.attempts.saturating_add(1);
                        p.ack_total_at_send = ack_baseline;
                        p.deadline_ms = now.saturating_add(backoff_ms);
                        last_sent_ms = now;
                        pending = Some(p);
                    } else {
                        SETPOINT_ACK_PENDING.store(false, Ordering::Release);
                        pending = None;
                    }
                } else {
                    SETPOINT_TIMEOUT_TOTAL.fetch_add(1, Ordering::Relaxed);
                    warn!(
                        "setpoint ack timeout after {} attempts (seq={}, target={} mA)",
                        p.attempts, p.seq, p.target_i_ma
                    );
                    SETPOINT_ACK_PENDING.store(false, Ordering::Release);
                    pending = None;
                }
            } else {
                pending = Some(p);
            }
        }

        cooperative_delay_ms(10).await;
    }
}

fn clamp_target_ma(v: i32) -> i32 {
    v.clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA)
}

async fn send_setpoint_frame(
    uhci_tx: &mut uhci::UhciTx<'static, Async>,
    seq: u8,
    target_i_ma: i32,
    raw: &mut [u8; 64],
    slip: &mut [u8; 192],
    ctx: &str,
) -> bool {
    let setpoint = SetPoint { target_i_ma };

    let frame_len = match encode_set_point_frame(seq, &setpoint, raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("{}: encode_set_point_frame error: {:?}", ctx, err);
            return false;
        }
    };

    let slip_len = match slip_encode(&raw[..frame_len], slip) {
        Ok(len) => len,
        Err(err) => {
            warn!("{}: slip_encode error: {:?}", ctx, err);
            return false;
        }
    };

    match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
        Ok(written) if written == slip_len => {
            let _ = uhci_tx.uart_tx.flush_async().await;
            SETPOINT_ACK_PENDING.store(true, Ordering::Release);
            info!(
                "{}: setpoint frame sent seq={} target={} mA len={} slip_len={}",
                ctx, seq, target_i_ma, frame_len, slip_len
            );
            true
        }
        Ok(written) => {
            warn!(
                "{}: short write {} < {} (seq={}, target={} mA)",
                ctx, written, slip_len, seq, target_i_ma
            );
            false
        }
        Err(err) => {
            warn!(
                "{}: uart write error for setpoint seq={}: {:?}",
                ctx, seq, err
            );
            false
        }
    }
}

/// SetMode 发送任务：原子 Active Control（CC/CV + 目标 + 限值），带 ACK 等待与退避重传，最新值优先。
#[embassy_executor::task]
async fn setmode_tx_task(
    mut uhci_tx: uhci::UhciTx<'static, Async>,
    calibration: &'static CalibrationMutex,
    control: &'static ControlMutex,
    telemetry: &'static TelemetryMutex,
) {
    info!(
        "SetMode TX task starting (ack_timeout={} ms, backoff={:?}, period={} ms)",
        SETMODE_ACK_TIMEOUT_MS, SETMODE_RETRY_BACKOFF_MS, SETMODE_TX_PERIOD_MS
    );

    #[derive(Clone, Copy)]
    struct Pending {
        seq: u8,
        cmd: SetMode,
        attempts: u8, // includes initial send
        ack_total_at_send: u32,
        deadline_ms: u32,
    }

    let mut raw = [0u8; 64];
    let mut slip = [0u8; 192];

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct PdPolicyKey {
        mode: control::PdMode,
        fixed_object_pos: u8,
        pps_object_pos: u8,
        target_mv: u32,
        i_req_ma: u32,
    }

    #[derive(Clone, Copy)]
    struct PdPending {
        seq: u8,
        key: PdPolicyKey,
        ack_total_at_send: u32,
        deadline_ms: u32,
        user_initiated: bool,
    }

    let mut pd_pending: Option<PdPending> = None;
    let mut pd_last_sent: Option<PdPolicyKey> = None;
    let mut pd_force_send: bool = false;
    let mut last_pd_req_skip_warn_ms: u32 = 0;

    // Soft-reset handshake (fixed seq=0); proceed even if ACK arrives late.
    let soft_reset_seq: u8 = 0;
    let soft_reset_acked =
        send_soft_reset_handshake(&mut uhci_tx, soft_reset_seq, &mut raw, &mut slip).await;
    if !soft_reset_acked {
        warn!("soft_reset ack missing within retry window; continuing after quiet gap");
    }

    // 更长的静默让模拟侧 UART 启动稳定，避免一上电被突发刷屏。
    cooperative_delay_ms(300).await;

    let mut seq: u8 = 1;

    // Cold boot: send the full 4-curve calibration set so the analog side can
    // reach CAL_READY (empty curves are rejected on G431).
    let profile = { calibration.lock().await.profile.clone() };
    send_all_calibration_curves(
        &mut uhci_tx,
        &mut seq,
        &profile,
        &mut raw,
        &mut slip,
        "boot",
    )
    .await;

    // 启动链路后发送一次 SetEnable(true)，用于拉起模拟侧输出 gating。
    let enable_cmd = SetEnable { enable: true };
    match encode_set_enable_frame(seq, &enable_cmd, &mut raw) {
        Ok(frame_len) => match slip_encode(&raw[..frame_len], &mut slip) {
            Ok(slip_len) => match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
                Ok(written) if written == slip_len => {
                    let _ = uhci_tx.uart_tx.flush_async().await;
                    info!(
                        "SetEnable(true) frame sent seq={} len={} slip_len={}",
                        seq, frame_len, slip_len
                    );
                }
                Ok(written) => {
                    warn!(
                        "SetEnable(true) short write {} < {} (seq={})",
                        written, slip_len, seq
                    );
                }
                Err(err) => {
                    warn!(
                        "SetEnable(true) uart write error for seq={}: {:?}",
                        seq, err
                    );
                }
            },
            Err(err) => {
                warn!("SetEnable(true) slip_encode error: {:?}", err);
            }
        },
        Err(err) => {
            warn!("SetEnable(true) encode_set_enable_frame error: {:?}", err);
        }
    }
    seq = seq.wrapping_add(1);

    // 在握手完成后发送一次静态 LimitProfile v0，供模拟板建立软件软限。
    match encode_limit_profile_frame(seq, &LIMIT_PROFILE_DEFAULT, &mut raw) {
        Ok(frame_len) => match slip_encode(&raw[..frame_len], &mut slip) {
            Ok(slip_len) => match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
                Ok(written) if written == slip_len => {
                    let _ = uhci_tx.uart_tx.flush_async().await;
                    info!(
                        "LimitProfile v0 sent (msg=0x{:02x}): max_i={}mA max_p={}mW ovp={}mV temp_trip={}mC derate={}%, seq={} len={} slip_len={}",
                        MSG_LIMIT_PROFILE,
                        LIMIT_PROFILE_DEFAULT.max_i_ma,
                        LIMIT_PROFILE_DEFAULT.max_p_mw,
                        LIMIT_PROFILE_DEFAULT.ovp_mv,
                        LIMIT_PROFILE_DEFAULT.temp_trip_mc,
                        LIMIT_PROFILE_DEFAULT.thermal_derate_pct,
                        seq,
                        frame_len,
                        slip_len
                    );
                }
                Ok(written) => {
                    warn!(
                        "LimitProfile v0 short write {} < {} (seq={})",
                        written, slip_len, seq
                    );
                }
                Err(err) => {
                    warn!(
                        "LimitProfile v0 uart write error for seq={}: {:?}",
                        seq, err
                    );
                }
            },
            Err(err) => {
                warn!("LimitProfile v0 slip_encode error: {:?}", err);
            }
        },
        Err(err) => {
            warn!(
                "LimitProfile v0 encode_limit_profile_frame error: {:?}",
                err
            );
        }
    }
    seq = seq.wrapping_add(1);

    let mut pending: Option<Pending> = None;
    let mut last_sent_cmd: Option<SetMode> = None;
    let mut last_sent_ms: u32 = now_ms32();
    let mut last_sent_rev: u32 = 0;
    let mut prev_link_up: bool = LINK_UP.load(Ordering::Relaxed);
    let mut force_send: bool = true; // boot
    // Track how long we've been stuck in AnalogState::CalMissing so we can
    // retry the SoftReset + CalWrite + SetEnable handshake after analog resets.
    let mut calmissing_since_ms: Option<u32> = None;
    let mut last_calmissing_warn_ms: u32 = 0;
    let mut last_calmissing_handshake_ms: u32 = 0;

    loop {
        yield_now().await;

        // Drain at most one pending soft-reset request per loop iteration.
        if let Some(reason) = crate::dequeue_soft_reset() {
            let soft_seq = seq;
            seq = seq.wrapping_add(1);
            send_soft_reset_one_shot(&mut uhci_tx, soft_seq, &mut raw, &mut slip, reason).await;
        }

        // Handle low-frequency calibration UART commands from the HTTP API.
        #[cfg(feature = "net_http")]
        if let Some(cmd) = crate::dequeue_cal_uart() {
            match cmd {
                CalUartCommand::SendAllCurves => {
                    let profile = { calibration.lock().await.profile.clone() };
                    send_all_calibration_curves(
                        &mut uhci_tx,
                        &mut seq,
                        &profile,
                        &mut raw,
                        &mut slip,
                        "http-cal",
                    )
                    .await;
                }
                CalUartCommand::SendCurve(kind) => {
                    let profile = { calibration.lock().await.profile.clone() };
                    send_calibration_curve(
                        &mut uhci_tx,
                        &mut seq,
                        &profile,
                        kind,
                        &mut raw,
                        &mut slip,
                        "http-cal",
                    )
                    .await;
                }
                CalUartCommand::SetMode(kind) => {
                    let seq_now = seq;
                    seq = seq.wrapping_add(1);
                    let _ = send_cal_mode_frame(
                        &mut uhci_tx,
                        seq_now,
                        kind,
                        &mut raw,
                        &mut slip,
                        "http-cal",
                    )
                    .await;
                }
            }
        }

        // On link recovery, re-send the full calibration set and force a SetMode snapshot.
        let link_up_now = LINK_UP.load(Ordering::Relaxed);
        if link_up_now && !prev_link_up {
            prev_link_up = true;
            let profile = { calibration.lock().await.profile.clone() };
            send_all_calibration_curves(
                &mut uhci_tx,
                &mut seq,
                &profile,
                &mut raw,
                &mut slip,
                "link-recover",
            )
            .await;
            force_send = true;
        } else if !link_up_now && prev_link_up {
            prev_link_up = false;
        }

        let now = now_ms32();

        // If the analog side resets while the digital stays up, we can end up in CalMissing
        // until we resend the calibration curves. Do a conservative retry loop (only when
        // idle) to avoid deadlocking output enable behind CAL_READY.
        let analog_state = AnalogState::from_u8(ANALOG_STATE.load(Ordering::Relaxed));
        if LINK_UP.load(Ordering::Relaxed)
            && analog_state == AnalogState::CalMissing
            && pending.is_none()
            && pd_pending.is_none()
        {
            let since = match calmissing_since_ms {
                Some(v) => v,
                None => {
                    calmissing_since_ms = Some(now);
                    now
                }
            };
            let stuck_ms = now.wrapping_sub(since);
            if stuck_ms >= 2_000 {
                if now.wrapping_sub(last_calmissing_warn_ms) >= 5_000 {
                    last_calmissing_warn_ms = now;
                    warn!(
                        "analog stuck in CalMissing; retrying calibration handshake (stuck_ms={})",
                        stuck_ms
                    );
                }

                if now.wrapping_sub(last_calmissing_handshake_ms) >= 5_000 {
                    last_calmissing_handshake_ms = now;

                    // SoftReset re-handshake: use a fresh seq to keep framing sane.
                    let soft_reset_seq = seq;
                    seq = seq.wrapping_add(1);
                    let soft_reset_acked = send_soft_reset_handshake(
                        &mut uhci_tx,
                        soft_reset_seq,
                        &mut raw,
                        &mut slip,
                    )
                    .await;
                    if !soft_reset_acked {
                        warn!(
                            "soft_reset re-handshake: ack missing; continuing with CalWrite+SetEnable"
                        );
                    }

                    // Re-send the full calibration set (multi-chunk CalWrite) to unlock
                    // CAL_READY on the analog side.
                    let profile = { calibration.lock().await.profile.clone() };
                    send_all_calibration_curves(
                        &mut uhci_tx,
                        &mut seq,
                        &profile,
                        &mut raw,
                        &mut slip,
                        "calmissing-recover",
                    )
                    .await;

                    // Re-send SetEnable(true) to re-arm ENABLE_REQUESTED on analog.
                    let enable_seq = seq;
                    let enable_cmd = SetEnable { enable: true };
                    match encode_set_enable_frame(enable_seq, &enable_cmd, &mut raw) {
                        Ok(frame_len) => match slip_encode(&raw[..frame_len], &mut slip) {
                            Ok(slip_len) => {
                                match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
                                    Ok(written) if written == slip_len => {
                                        let _ = uhci_tx.uart_tx.flush_async().await;
                                        info!(
                                            "SetEnable(true) frame re-sent seq={} len={} slip_len={}",
                                            enable_seq, frame_len, slip_len
                                        );
                                    }
                                    Ok(written) => {
                                        warn!(
                                            "SetEnable(true) re-send short write {} < {} (seq={})",
                                            written, slip_len, enable_seq
                                        );
                                    }
                                    Err(err) => {
                                        warn!(
                                            "SetEnable(true) re-send uart write error for seq={}: {:?}",
                                            enable_seq, err
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                warn!("SetEnable(true) re-send slip_encode error: {:?}", err);
                            }
                        },
                        Err(err) => {
                            warn!(
                                "SetEnable(true) re-send encode_set_enable_frame error: {:?}",
                                err
                            );
                        }
                    }
                    seq = seq.wrapping_add(1);

                    // Force a SetMode snapshot after recovering.
                    force_send = true;
                }
            }
        } else {
            calmissing_since_ms = None;
        }

        let (rev_now, desired_cmd, mut pd_cfg) = {
            let guard = control.lock().await;
            let p = guard.active_preset();
            let cmd = SetMode {
                preset_id: guard.active_preset_id,
                output_enabled: guard.output_enabled,
                mode: match p.mode {
                    LoadMode::Cc => LoadMode::Cc,
                    LoadMode::Cv => LoadMode::Cv,
                    LoadMode::Cp => LoadMode::Cp,
                    LoadMode::Reserved(_) => LoadMode::Cc,
                },
                target_p_mw: if p.mode == LoadMode::Cp {
                    Some(p.target_p_mw)
                } else {
                    None
                },
                target_i_ma: p.target_i_ma,
                target_v_mv: p.target_v_mv,
                min_v_mv: p.min_v_mv,
                max_i_ma_total: p.max_i_ma_total,
                max_p_mw: p.max_p_mw,
            };
            (
                CONTROL_REV.load(Ordering::Relaxed),
                sanitize_setmode(cmd),
                guard.pd_saved,
            )
        };
        if pd_cfg.target_mv < 3_000 || pd_cfg.target_mv > 21_000 {
            pd_cfg.target_mv = 5_000;
        }
        pd_cfg.i_req_ma = pd_cfg.i_req_ma.clamp(50, 10_000);
        let pd_key = PdPolicyKey {
            mode: pd_cfg.mode,
            fixed_object_pos: pd_cfg.fixed_object_pos,
            pps_object_pos: pd_cfg.pps_object_pos,
            target_mv: pd_cfg.target_mv,
            i_req_ma: pd_cfg.i_req_ma,
        };

        if PD_FORCE_SEND.swap(false, Ordering::AcqRel) {
            pd_force_send = true;
        }

        // PD ACK / timeout / preemption handling.
        if let Some(p) = pd_pending.as_ref() {
            let ack_total = PD_REQ_ACK_TOTAL.load(Ordering::Relaxed);
            let ack_seq = PD_REQ_LAST_ACK_SEQ.load(Ordering::Relaxed);
            let ack_hit = ack_total != p.ack_total_at_send && ack_seq == p.seq;
            if ack_hit {
                if p.user_initiated {
                    let flags = PD_REQ_LAST_ACK_FLAGS.load(Ordering::Relaxed);
                    let code = if (flags & FLAG_IS_NACK) != 0 { 2 } else { 1 };
                    PD_LAST_RESULT_CODE.store(code, Ordering::Relaxed);
                    PD_LAST_RESULT_MS.store(now, Ordering::Relaxed);
                }
                pd_pending = None;
                PD_REQ_ACK_PENDING.store(false, Ordering::Release);
            } else if now >= p.deadline_ms {
                PD_REQ_TIMEOUT_TOTAL.fetch_add(1, Ordering::Relaxed);
                warn!(
                    "pd_sink_request ack timeout (seq={}, mode={:?}, target_mv={}, i_req_ma={})",
                    p.seq, p.key.mode, p.key.target_mv, p.key.i_req_ma
                );
                if p.user_initiated {
                    PD_LAST_RESULT_CODE.store(3, Ordering::Relaxed); // timeout
                    PD_LAST_RESULT_MS.store(now, Ordering::Relaxed);
                }
                pd_pending = None;
                PD_REQ_ACK_PENDING.store(false, Ordering::Release);
            } else if p.key != pd_key {
                pd_pending = None;
                PD_REQ_ACK_PENDING.store(false, Ordering::Release);
            }
        }

        // Send PD policy when forced (attach/link edge) or when the target changes.
        if LINK_UP.load(Ordering::Relaxed)
            && pd_pending.is_none()
            && (pd_force_send || pd_last_sent != Some(pd_key))
        {
            // UART link can briefly flap while the analog side is busy; keep the ACK window
            // comfortably above the link-down threshold so UI/API doesn't report false failures.
            const PD_ACK_TIMEOUT_MS: u32 = 1200;
            let seq_now = seq;
            seq = seq.wrapping_add(1);
            let ack_baseline = PD_REQ_ACK_TOTAL.load(Ordering::Relaxed);
            let user_initiated = {
                let ui_apply_ms = PD_UI_APPLY_MS.load(Ordering::Relaxed);
                ui_apply_ms != 0 && now.wrapping_sub(ui_apply_ms) < 2_000
            };
            let pd_status = {
                let guard = telemetry.lock().await;
                guard.last_pd_status.clone()
            };
            if let Some(req) = build_pd_sink_request(&pd_cfg, pd_status.as_ref()) {
                if send_pd_sink_request_frame(&mut uhci_tx, seq_now, &req, &mut raw, &mut slip)
                    .await
                {
                    PD_LAST_APPLY_MS.store(now, Ordering::Relaxed);
                    pd_last_sent = Some(pd_key);
                    pd_force_send = false;
                    pd_pending = Some(PdPending {
                        seq: seq_now,
                        key: pd_key,
                        ack_total_at_send: ack_baseline,
                        deadline_ms: now.saturating_add(PD_ACK_TIMEOUT_MS),
                        user_initiated,
                    });
                }
            } else {
                let delta = now.wrapping_sub(last_pd_req_skip_warn_ms);
                if delta >= 2000 {
                    last_pd_req_skip_warn_ms = now;
                    warn!(
                        "pd_sink_request skipped: missing PDO/APDO (mode={:?}, target_mv={}, have_status={})",
                        pd_cfg.mode,
                        pd_cfg.target_mv,
                        pd_status.is_some()
                    );
                }
            }
        }

        // ACK arrival check for the current pending seq.
        let ack_hit = if let Some(p) = pending.as_ref() {
            let ack_total = SETMODE_ACK_TOTAL.load(Ordering::Relaxed);
            let ack_seq = SETMODE_LAST_ACK_SEQ.load(Ordering::Relaxed);
            ack_total != p.ack_total_at_send && ack_seq == p.seq
        } else {
            false
        };
        if ack_hit {
            if let Some(p) = pending.take() {
                SETMODE_ACK_PENDING.store(false, Ordering::Release);
                last_sent_cmd = Some(p.cmd);
                last_sent_ms = now;
                last_sent_rev = rev_now;
            }
            continue;
        }

        // Pre-empt with latest value if changed while waiting.
        let mut should_send_new = false;
        let mut send_reason = "periodic";
        if let Some(p) = pending.as_ref() {
            if p.cmd != desired_cmd {
                should_send_new = true;
                send_reason = "latest-value-preempt";
                pending = None;
            }
        } else if force_send {
            should_send_new = true;
            send_reason = "force";
        } else if rev_now != last_sent_rev {
            if last_sent_cmd.map(|c| c != desired_cmd).unwrap_or(true) {
                should_send_new = true;
                send_reason = "rev-change";
            } else {
                last_sent_rev = rev_now;
            }
        } else if now.saturating_sub(last_sent_ms) >= SETMODE_TX_PERIOD_MS {
            if last_sent_cmd.map(|c| c != desired_cmd).unwrap_or(true) {
                should_send_new = true;
                send_reason = "state-change";
            } else {
                // Keepalive (rare).
                should_send_new = true;
                send_reason = "periodic";
            }
        }

        if should_send_new {
            // Avoid attempting to drive output ON when we have no healthy link.
            if desired_cmd.output_enabled && !LINK_UP.load(Ordering::Relaxed) {
                let now_ms = now;
                let last_gate = LAST_SETPOINT_GATE_WARN_MS.load(Ordering::Relaxed);
                if now_ms.wrapping_sub(last_gate) >= 1_000 {
                    LAST_SETPOINT_GATE_WARN_MS.store(now_ms, Ordering::Relaxed);
                    warn!(
                        "SetMode TX gated (link_up=false, hello_seen={}, preset_id={}, output_enabled=true)",
                        HELLO_SEEN.load(Ordering::Relaxed),
                        desired_cmd.preset_id
                    );
                }
                force_send = false;
                continue;
            }

            let analog_state = AnalogState::from_u8(ANALOG_STATE.load(Ordering::Relaxed));
            if desired_cmd.output_enabled {
                match analog_state {
                    AnalogState::Faulted => {
                        warn!("SetMode TX gated: analog fault (state=FAULTED)");
                        force_send = false;
                        continue;
                    }
                    // CalMissing means "not producing enable" (FastStatus.enable==false),
                    // not "missing calibration". We must still allow output_enabled=true
                    // to be sent, otherwise HTTP/UI output enable becomes deadlocked.
                    //
                    // The analog firmware remains the source of truth for safety gating.
                    AnalogState::CalMissing => {}
                    AnalogState::Offline => {
                        warn!("SetMode TX gated: analog offline");
                        force_send = false;
                        continue;
                    }
                    AnalogState::Ready => {}
                }
            }

            let send_seq = seq;
            seq = seq.wrapping_add(1);
            let ack_baseline = SETMODE_ACK_TOTAL.load(Ordering::Relaxed);
            if send_setmode_frame(
                &mut uhci_tx,
                send_seq,
                &desired_cmd,
                &mut raw,
                &mut slip,
                send_reason,
            )
            .await
            {
                SETMODE_TX_TOTAL.fetch_add(1, Ordering::Relaxed);
                let deadline = now.saturating_add(SETMODE_ACK_TIMEOUT_MS);
                last_sent_cmd = Some(desired_cmd);
                last_sent_ms = now;
                last_sent_rev = rev_now;
                pending = Some(Pending {
                    seq: send_seq,
                    cmd: desired_cmd,
                    attempts: 1,
                    ack_total_at_send: ack_baseline,
                    deadline_ms: deadline,
                });
                force_send = false;
            } else {
                SETMODE_ACK_PENDING.store(false, Ordering::Release);
                force_send = false;
            }
        } else if let Some(mut p) = pending.take() {
            // Timeout + retry path
            if now >= p.deadline_ms {
                if (p.attempts as usize) <= SETMODE_RETRY_BACKOFF_MS.len() {
                    let backoff_ms = SETMODE_RETRY_BACKOFF_MS[(p.attempts - 1) as usize];
                    let ack_baseline = SETMODE_ACK_TOTAL.load(Ordering::Relaxed);
                    let send_seq = p.seq;
                    if send_setmode_frame(
                        &mut uhci_tx,
                        send_seq,
                        &p.cmd,
                        &mut raw,
                        &mut slip,
                        "retx",
                    )
                    .await
                    {
                        SETMODE_RETX_TOTAL.fetch_add(1, Ordering::Relaxed);
                        p.attempts = p.attempts.saturating_add(1);
                        p.ack_total_at_send = ack_baseline;
                        p.deadline_ms = now.saturating_add(backoff_ms);
                        last_sent_ms = now;
                        pending = Some(p);
                    } else {
                        SETMODE_ACK_PENDING.store(false, Ordering::Release);
                        pending = None;
                    }
                } else {
                    SETMODE_TIMEOUT_TOTAL.fetch_add(1, Ordering::Relaxed);
                    warn!(
                        "setmode ack timeout after {} attempts (seq={}, preset_id={}, output_enabled={})",
                        p.attempts, p.seq, p.cmd.preset_id, p.cmd.output_enabled
                    );
                    SETMODE_ACK_PENDING.store(false, Ordering::Release);
                    pending = None;
                }
            } else {
                pending = Some(p);
            }
        }

        cooperative_delay_ms(10).await;
    }
}

fn sanitize_setmode(mut cmd: SetMode) -> SetMode {
    if cmd.preset_id == 0 || cmd.preset_id > 5 {
        cmd.preset_id = 1;
    }
    cmd.target_i_ma = cmd.target_i_ma.max(0);
    cmd.target_v_mv = cmd.target_v_mv.max(0);
    cmd.min_v_mv = cmd.min_v_mv.max(0);
    cmd.max_i_ma_total = cmd.max_i_ma_total.max(0).min(control::HARD_MAX_I_MA_TOTAL);
    let hard_max_p = LIMIT_PROFILE_DEFAULT.max_p_mw;
    cmd.max_p_mw = cmd.max_p_mw.min(hard_max_p);
    if let Some(v) = cmd.target_p_mw {
        cmd.target_p_mw = Some(v.min(hard_max_p));
    }
    if cmd.mode == LoadMode::Cv && cmd.target_v_mv < cmd.min_v_mv {
        cmd.target_v_mv = cmd.min_v_mv;
    }
    if cmd.mode == LoadMode::Cp {
        if let Some(v) = cmd.target_p_mw {
            if v > cmd.max_p_mw {
                cmd.target_p_mw = Some(cmd.max_p_mw);
            }
        } else {
            cmd.target_p_mw = Some(0);
        }
    } else {
        cmd.target_p_mw = None;
    }
    if cmd.target_i_ma > cmd.max_i_ma_total {
        cmd.target_i_ma = cmd.max_i_ma_total;
    }
    cmd
}

async fn send_setmode_frame(
    uhci_tx: &mut uhci::UhciTx<'static, Async>,
    seq: u8,
    cmd: &SetMode,
    raw: &mut [u8; 64],
    slip: &mut [u8; 192],
    ctx: &str,
) -> bool {
    let frame_len = match encode_set_mode_frame(seq, cmd, raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("{}: encode_set_mode_frame error: {:?}", ctx, err);
            return false;
        }
    };

    let slip_len = match slip_encode(&raw[..frame_len], slip) {
        Ok(len) => len,
        Err(err) => {
            warn!("{}: slip_encode error: {:?}", ctx, err);
            return false;
        }
    };

    match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
        Ok(written) if written == slip_len => {
            let _ = uhci_tx.uart_tx.flush_async().await;
            SETMODE_ACK_PENDING.store(true, Ordering::Release);
            info!(
                "{}: setmode frame sent seq={} preset_id={} mode={:?} out={} len={} slip_len={}",
                ctx, seq, cmd.preset_id, cmd.mode, cmd.output_enabled, frame_len, slip_len
            );
            true
        }
        Ok(written) => {
            warn!(
                "{}: short write {} < {} (seq={}, preset_id={})",
                ctx, written, slip_len, seq, cmd.preset_id
            );
            false
        }
        Err(err) => {
            warn!(
                "{}: uart write error for setmode seq={}: {:?}",
                ctx, seq, err
            );
            false
        }
    }
}

fn build_pd_sink_request(
    cfg: &control::PdConfig,
    status: Option<&PdStatus>,
) -> Option<PdSinkRequest> {
    fn pdo_pos(pos: u8, idx: usize) -> u8 {
        if pos != 0 {
            pos
        } else {
            (idx + 1).min(u8::MAX as usize) as u8
        }
    }

    let status = status?;
    let i_req_ma = cfg.i_req_ma.clamp(50, 10_000);

    let mode = match cfg.mode {
        control::PdMode::Fixed => PdSinkMode::Fixed,
        control::PdMode::Pps => PdSinkMode::Pps,
    };

    match cfg.mode {
        control::PdMode::Fixed => {
            let fixed_object_pos = if cfg.fixed_object_pos != 0 {
                cfg.fixed_object_pos
            } else {
                // Legacy fallback: derive selection from target_mv.
                status
                    .fixed_pdos
                    .iter()
                    .enumerate()
                    .find(|(_idx, pdo)| pdo.mv == cfg.target_mv)
                    .map(|(idx, pdo)| pdo_pos(pdo.pos, idx))?
            };

            let pdo = status
                .fixed_pdos
                .iter()
                .enumerate()
                .find(|(idx, p)| pdo_pos(p.pos, *idx) == fixed_object_pos)
                .map(|(_idx, p)| *p)?;

            if i_req_ma > pdo.max_ma {
                return None;
            }

            Some(PdSinkRequest {
                mode,
                target_mv: pdo.mv,
                object_pos: fixed_object_pos,
                i_req_ma,
            })
        }
        control::PdMode::Pps => {
            let pps_object_pos = cfg.pps_object_pos;
            if pps_object_pos == 0 || cfg.target_mv == 0 {
                return None;
            }

            let apdo = status
                .pps_pdos
                .iter()
                .enumerate()
                .find(|(idx, p)| pdo_pos(p.pos, *idx) == pps_object_pos)
                .map(|(_idx, p)| *p)?;

            if cfg.target_mv < apdo.min_mv || cfg.target_mv > apdo.max_mv || i_req_ma > apdo.max_ma
            {
                return None;
            }

            Some(PdSinkRequest {
                mode,
                target_mv: cfg.target_mv,
                object_pos: pps_object_pos,
                i_req_ma,
            })
        }
    }
}

async fn send_pd_sink_request_frame(
    uhci_tx: &mut uhci::UhciTx<'static, Async>,
    seq: u8,
    req: &PdSinkRequest,
    raw: &mut [u8; 64],
    slip: &mut [u8; 192],
) -> bool {
    let frame_len = match encode_pd_sink_request_frame(seq, req, raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("pd_sink_request: encode error: {:?}", err);
            return false;
        }
    };

    let slip_len = match slip_encode(&raw[..frame_len], slip) {
        Ok(len) => len,
        Err(err) => {
            warn!("pd_sink_request: slip_encode error: {:?}", err);
            return false;
        }
    };

    match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
        Ok(written) if written == slip_len => {
            let _ = uhci_tx.uart_tx.flush_async().await;
            PD_REQ_TX_TOTAL.fetch_add(1, Ordering::Relaxed);
            PD_REQ_ACK_PENDING.store(true, Ordering::Release);
            let mode_raw = u8::from(req.mode);
            info!(
                "pd_sink_request sent (msg=0x{:02x}, flags=0x{:02x}): seq={} mode={} object_pos={} target_mv={} i_req_ma={} len={} slip_len={}",
                MSG_PD_SINK_REQUEST,
                FLAG_ACK_REQ,
                seq,
                mode_raw,
                req.object_pos,
                req.target_mv,
                req.i_req_ma,
                frame_len,
                slip_len
            );
            true
        }
        Ok(written) => {
            warn!(
                "pd_sink_request short write {} < {} (seq={})",
                written, slip_len, seq
            );
            false
        }
        Err(err) => {
            warn!(
                "pd_sink_request uart write error for seq={}: {:?}",
                seq, err
            );
            false
        }
    }
}

async fn send_cal_mode_frame(
    uhci_tx: &mut uhci::UhciTx<'static, Async>,
    seq: u8,
    kind: CalKind,
    raw: &mut [u8; 64],
    slip: &mut [u8; 192],
    ctx: &str,
) -> bool {
    let mode = CalMode { kind };
    let frame_len = match encode_cal_mode_frame(seq, &mode, raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("{}: encode_cal_mode_frame error: {:?}", ctx, err);
            return false;
        }
    };

    let slip_len = match slip_encode(&raw[..frame_len], slip) {
        Ok(len) => len,
        Err(err) => {
            warn!("{}: CalMode slip_encode error: {:?}", ctx, err);
            return false;
        }
    };

    match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
        Ok(written) if written == slip_len => {
            let _ = uhci_tx.uart_tx.flush_async().await;
            info!(
                "{}: CalMode(kind={:?}, flags=0x{:02x}) sent seq={} len={} slip_len={}",
                ctx, kind, FLAG_ACK_REQ, seq, frame_len, slip_len
            );
            true
        }
        Ok(written) => {
            warn!(
                "{}: CalMode short write {} < {} (seq={})",
                ctx, written, slip_len, seq
            );
            false
        }
        Err(err) => {
            warn!(
                "{}: CalMode uart write error for seq={}: {:?}",
                ctx, seq, err
            );
            false
        }
    }
}

async fn send_calibration_curve(
    uhci_tx: &mut uhci::UhciTx<'static, Async>,
    seq: &mut u8,
    profile: &ActiveProfile,
    kind: CurveKind,
    raw: &mut [u8; 64],
    slip: &mut [u8; 192],
    ctx: &str,
) {
    fn kind_name(kind: CurveKind) -> &'static str {
        match kind {
            CurveKind::VLocal => "v_local",
            CurveKind::VRemote => "v_remote",
            CurveKind::CurrentCh1 => "current_ch1",
            CurveKind::CurrentCh2 => "current_ch2",
        }
    }

    let points = profile.points_for(kind);
    let chunks = calfmt::encode_calwrite_chunks(profile.fmt_version, profile.hw_rev, kind, points);

    info!(
        "{}: sending CalWrite curve kind={} points={} chunks={}",
        ctx,
        kind_name(kind),
        points.len(),
        chunks.len()
    );

    for chunk in chunks.iter() {
        let seq_now = *seq;
        *seq = seq_now.wrapping_add(1);

        match encode_cal_write_frame(seq_now, chunk, raw) {
            Ok(frame_len) => match slip_encode(&raw[..frame_len], slip) {
                Ok(slip_len) => match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
                    Ok(written) if written == slip_len => {
                        let _ = uhci_tx.uart_tx.flush_async().await;
                        info!(
                            "{}: CalWrite(kind={}, chunk_index={}, msg=0x{:02x}) sent seq={} len={} slip_len={}",
                            ctx,
                            kind_name(kind),
                            chunk.index,
                            MSG_CAL_WRITE,
                            seq_now,
                            frame_len,
                            slip_len
                        );
                    }
                    Ok(written) => {
                        warn!(
                            "{}: CalWrite(kind={}, chunk_index={}) short write {} < {} (seq={})",
                            ctx,
                            kind_name(kind),
                            chunk.index,
                            written,
                            slip_len,
                            seq_now
                        );
                    }
                    Err(err) => {
                        warn!(
                            "{}: CalWrite(kind={}, chunk_index={}) uart write error for seq={}: {:?}",
                            ctx,
                            kind_name(kind),
                            chunk.index,
                            seq_now,
                            err
                        );
                    }
                },
                Err(err) => {
                    warn!(
                        "{}: CalWrite(kind={}) slip_encode error: {:?}",
                        ctx,
                        kind_name(kind),
                        err
                    );
                }
            },
            Err(err) => {
                warn!("{}: encode_cal_write_frame error: {:?}", ctx, err);
            }
        }

        // Small yield gap to avoid UART bursty backpressure on cold boot.
        cooperative_delay_ms(10).await;
    }
}

async fn send_all_calibration_curves(
    uhci_tx: &mut uhci::UhciTx<'static, Async>,
    seq: &mut u8,
    profile: &ActiveProfile,
    raw: &mut [u8; 64],
    slip: &mut [u8; 192],
    ctx: &str,
) {
    // Recommended order: current_ch1 → current_ch2 → v_local → v_remote
    send_calibration_curve(uhci_tx, seq, profile, CurveKind::CurrentCh1, raw, slip, ctx).await;
    send_calibration_curve(uhci_tx, seq, profile, CurveKind::CurrentCh2, raw, slip, ctx).await;
    send_calibration_curve(uhci_tx, seq, profile, CurveKind::VLocal, raw, slip, ctx).await;
    send_calibration_curve(uhci_tx, seq, profile, CurveKind::VRemote, raw, slip, ctx).await;
}

async fn send_soft_reset_one_shot(
    uhci_tx: &mut uhci::UhciTx<'static, Async>,
    seq: u8,
    raw: &mut [u8; 64],
    slip: &mut [u8; 192],
    reason: SoftResetReason,
) {
    let reset = SoftReset {
        reason,
        timestamp_ms: now_ms32(),
    };

    let frame_len = match encode_soft_reset_frame(seq, &reset, false, raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("soft_reset(one-shot) encode error: {:?}", err);
            return;
        }
    };
    let slip_len = match slip_encode(&raw[..frame_len], slip) {
        Ok(len) => len,
        Err(err) => {
            warn!("soft_reset(one-shot) slip_encode error: {:?}", err);
            return;
        }
    };

    match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
        Ok(written) if written == slip_len => {
            let _ = uhci_tx.uart_tx.flush_async().await;
            info!(
                "soft_reset(one-shot) frame sent seq={} reason={:?} len={} slip_len={}",
                seq, reset.reason, frame_len, slip_len
            );
        }
        Ok(written) => {
            warn!(
                "soft_reset(one-shot) short write: written={} len={} (seq={})",
                written, slip_len, seq
            );
        }
        Err(err) => {
            warn!("soft_reset(one-shot) write error: {:?}", err);
        }
    }
}

async fn send_soft_reset_handshake(
    uhci_tx: &mut uhci::UhciTx<'static, Async>,
    seq: u8,
    raw: &mut [u8; 64],
    slip: &mut [u8; 192],
) -> bool {
    if SOFT_RESET_ACKED.load(Ordering::Relaxed) {
        return true;
    }

    let reset = SoftReset {
        reason: SoftResetReason::FirmwareUpdate,
        timestamp_ms: now_ms32(),
    };

    for attempt in 0..3 {
        if SOFT_RESET_ACKED.load(Ordering::Relaxed) {
            break;
        }

        let frame_len = match encode_soft_reset_frame(seq, &reset, false, raw) {
            Ok(len) => len,
            Err(err) => {
                warn!("soft_reset encode error: {:?}", err);
                break;
            }
        };
        let slip_len = match slip_encode(&raw[..frame_len], slip) {
            Ok(len) => len,
            Err(err) => {
                warn!("soft_reset slip encode error: {:?}", err);
                break;
            }
        };

        match uhci_tx.uart_tx.write_async(&slip[..slip_len]).await {
            Ok(written) if written == slip_len => {
                let _ = uhci_tx.uart_tx.flush_async().await;
                info!(
                    "soft_reset req sent (attempt={}, seq={}, reason={:?}, ts_ms={})",
                    attempt + 1,
                    seq,
                    reset.reason,
                    reset.timestamp_ms
                );
            }
            Ok(written) => {
                warn!(
                    "soft_reset short write: written={} len={} (seq={})",
                    written, slip_len, seq
                );
            }
            Err(err) => {
                warn!("soft_reset write error: {:?}", err);
            }
        }

        if SOFT_RESET_ACKED.load(Ordering::Relaxed) {
            break;
        }
        cooperative_delay_ms(150).await;
    }

    if SOFT_RESET_ACKED.load(Ordering::Relaxed) {
        info!("soft_reset ack received; continuing link init");
        true
    } else {
        warn!("soft_reset ack not received after retries; proceed with caution");
        false
    }
}

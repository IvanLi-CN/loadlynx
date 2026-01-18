#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, Ordering};
use embassy_executor::Spawner;
use embassy_stm32 as stm32;
use embassy_stm32::adc::{
    Adc, AdcChannel, SampleTime, Temperature as AdcTemperature, VREF_CALIB_MV,
};
use embassy_stm32::bind_interrupts;
use embassy_stm32::dac::{Dac, Mode as DacMode, Value as DacValue};
use embassy_stm32::gpio::{Flex, Level, Output, Speed};
use embassy_stm32::mode::Async as UartAsync;
use embassy_stm32::usart::{
    Config as UartConfig, DataBits as UartDataBits, Parity as UartParity, RingBufferedUartRx,
    StopBits as UartStopBits, Uart, UartRx, UartTx,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer};
use libm::logf;
use loadlynx_protocol::{
    CRC_LEN, CalKind, Error as ProtocolError, FAST_STATUS_MODE_CC, FAST_STATUS_MODE_CP,
    FAST_STATUS_MODE_CV, FAULT_MCU_OVER_TEMP, FAULT_OVERCURRENT, FAULT_OVERVOLTAGE,
    FAULT_SINK_OVER_TEMP, FLAG_IS_ACK, FastStatus, FrameHeader, HEADER_LEN, Hello, LoadMode,
    MSG_CAL_MODE, MSG_SET_MODE, MSG_SET_POINT, STATE_FLAG_CURRENT_LIMITED, STATE_FLAG_ENABLED,
    STATE_FLAG_LINK_GOOD, STATE_FLAG_POWER_LIMITED, STATE_FLAG_REMOTE_ACTIVE,
    STATE_FLAG_UV_LATCHED, SlipDecoder, SoftReset, SoftResetReason, decode_cal_mode_frame,
    decode_cal_write_frame, decode_frame, decode_limit_profile_frame, decode_pd_sink_request_frame,
    decode_set_enable_frame, decode_set_mode_frame, decode_set_point_frame,
    decode_soft_reset_frame, encode_ack_only_frame, encode_fast_status_frame, encode_hello_frame,
    encode_soft_reset_frame, slip_encode,
};
use static_cell::StaticCell;

mod calibration;
mod pd;
use calibration::{
    CalCurve, CalibrationState, CurveKind, inverse_piecewise, mv_to_raw_100uv, piecewise_linear,
    raw_100uv_to_dac_code_calibrated, raw_100uv_to_dac_code_vref,
};

// STM32G431 VREFBUF 基址/寄存器地址（同 pd-sink-stm32g431cbu6-rs 工程）
const VREFBUF_BASE: u32 = 0x4001_0030;
const VREFBUF_CSR_ADDR: *mut u32 = VREFBUF_BASE as *mut u32;

bind_interrupts!(struct Irqs {
    USART3 => stm32::usart::InterruptHandler<stm32::peripherals::USART3>;
    UCPD1 => stm32::ucpd::InterruptHandler<stm32::peripherals::UCPD1>;
});

// 模拟板 FAST_STATUS 发送周期：20 Hz → 50 ms
const FAST_STATUS_PERIOD_US: u64 = 1_000_000 / 20; // 50_000 us
// 控制环（DAC 更新）运行周期：提高闭环更新频率以提升瞬态响应能力；
// FastStatus 仍保持 20 Hz，不影响数字板协议/带宽。
const CONTROL_PERIOD_US: u64 = 100; // 10 kHz
const CONTROL_TICKS_PER_STATUS: u32 = (FAST_STATUS_PERIOD_US / CONTROL_PERIOD_US) as u32;
const CONTROL_TICKS_PER_SEC: u32 = (1_000_000 / CONTROL_PERIOD_US) as u32;
// 若超过该时间（ms）未收到任何来自数字板的有效控制帧，则认为链路当前异常。
const LINK_DEAD_TIMEOUT_MS: u32 = 300;
// 调试开关：如需只验证数字板→模拟板的 SetPoint 路径，可暂时关闭 FAST_STATUS TX。
const ENABLE_FAST_STATUS_TX: bool = true;
// Compact firmware identifier exported via HELLO; currently a simple placeholder
// that can be refined to encode semver/git describe in future revisions.
const HELLO_FW_VERSION: u32 = 0;

// Calibration-only smoothing window:
// FastStatus is emitted at 20 Hz (50 ms). A 6-frame window is ~300 ms.
const CAL_SMOOTH_WINDOW_FRAMES: usize = 6;
// Calibration-only oversampling (within a single FastStatus cycle) to suppress
// 1–10 kHz ripple by time-averaging multiple ADC conversions.
const CAL_OVERSAMPLE_SAMPLES: u32 = 256;

// ADC 公共参数（G431 12bit ADC）。
// 电压换算遵循 STM32G4 官方推荐流程：
//   1. 读取工厂校准常数 VREFINT_CAL（VREF_INT 在 VREF_CALIB_MV 电压下的 ADC 码）。
//   2. 读取当前 VrefInt ADC 码 vrefint_raw。
//   3. 计算当前 VREF+（VDDA）：vref_mv = VREF_CALIB_MV * VREFINT_CAL / vrefint_raw。
//   4. 其它通道电压：Vch = code * vref_mv / ADC_FULL_SCALE。
//
// 其中 VREF_CALIB_MV 由 embassy-stm32 提供（通常为 3300 mV，对应 ST 文档的标称电压）。
const ADC_FULL_SCALE: u32 = 4095;

// STM32G431 内置 VREFINT_CAL 工厂标定值存放地址（参见 ST RM/DS：TS_CAL/VREFINT_CAL 表）。
// 该地址存放的是在 VREF+ = VREF_CALIB_MV 条件下测得的 VrefInt ADC 原始码（12bit）。
const VREFINT_CAL_ADDR: *const u16 = 0x1FFF_75AA as *const u16;

// 远端 sense 软判定阈值：电压范围与 ADC 饱和裕度。
const REMOTE_V_MIN_MV: i32 = 500;
const REMOTE_V_MAX_MV: i32 = 55_000;
const ADC_SAT_MARGIN: u16 = 32;

// 近端 / 远端电压测量的缩放关系来自网表中 OPA2365 差分放大器：
//
// 远端（V_RMT_P / V_RMT_N）：
//   - 正端分压：R16=124k, R19=10k → V+ = V_RMT_P * 10 / (124 + 10)
//   - 反相端网络：R15=124k (到 V_RMT_N), R14=10k (到 V_RMT_SP)
//   - 令 V+ = V-，写节点方程可得：
//       V_RMT_SP = (10 / 124) * (V_RMT_P - V_RMT_N)
//     即 MCU 侧 V_RMT_SP 正比于负载差分电压，比例 10/124。
//
// 近端（V_NR_P / V_NR_N）使用完全对称的网络（R23/R24/R21/R22），得到同样关系：
//       V_NR_SP = (10 / 124) * (V_NR_P - V_NR_N)
//
// 因此反推负载端差分电压：
//   V_load = (124 / 10) * V_SP ≈ 12.4 * V_SP
const SENSE_GAIN_NUM: u32 = 124;
const SENSE_GAIN_DEN: u32 = 10;

// 默认恒流目标（mA）：启动时先保持 0 mA，等待数字板下发 SetPoint 再开始带载。
const DEFAULT_TARGET_I_LOCAL_MA: i32 = 0;
// 可接受的目标电流范围（mA），用于防止异常指令导致过流。
const TARGET_I_MIN_MA: i32 = 0;
const TARGET_I_MAX_MA: i32 = 10_000;
const TARGET_I_CH_MAX_MA: i32 = 5_000;

// Basic protection thresholds (units: mA, mV, m°C).
const OC_LIMIT_CH_MA: i32 = 5_500; // 过流阈值（略高于 TARGET_I_CH_MAX_MA）
const OC_LIMIT_TOTAL_MA: i32 = 11_000; // 略高于 10A 总目标（双通道同时略超时保护）
const OV_LIMIT_MV: i32 = 55_000; // 过压阈值（与文档 55V 对齐）
const MCU_TEMP_LIMIT_MC: i32 = 110_000; // 110 °C
const SINK_TEMP_LIMIT_MC: i32 = 100_000; // 100 °C

// 通道调度阈值：总目标电流 < 2 A 时仅驱动通道 1；≥ 2 A 时两通道近似均分。
const I_SHARE_THRESHOLD_MA: i32 = 2_000;

// CV loop tuning (legacy constants were historically tuned at FAST_STATUS_PERIOD_US cadence).
//
// Control model: integrate on conductance `G` so current demand scales with voltage:
//   I = G * V
// This behaves better on current-limited sources (avoids "snap to 0 current"
// when V dips, which would otherwise let the source jump back up).
//
// Units:
// - `G` stored as uA per mV (uA/mV)
// - `I` computed as: (G * V[mV]) / 1000 -> mA
const CV_ERR_DEADBAND_MV: i32 = 50;
const CV_G_ERR_DIV_MV: i32 = 500; // 500 mV error -> 1 uA/mV step (before clamping)
const CV_G_STEP_UP_MAX_UA_PER_MV: i32 = 5;
const CV_G_STEP_DN_MAX_UA_PER_MV: i32 = 10;
const CV_G_MAX_UA_PER_MV: i32 = 2_000;
// Control loop tick density relative to a 1ms baseline (used to keep time constants stable when
// CONTROL_PERIOD_US changes).
const CONTROL_TICKS_PER_MS: u32 = (1_000 / CONTROL_PERIOD_US) as u32;
const CONTROL_RATE_SCALE: i32 = if CONTROL_PERIOD_US >= 1_000 {
    1
} else {
    CONTROL_TICKS_PER_MS as i32
};

// CV voltage measurement smoothing for the control law (not used for faults).
// y += (x - y) / N ; larger N => more smoothing / more phase lag.
const CV_V_FILT_DIV: i32 = 8 * CONTROL_RATE_SCALE;
// CP voltage measurement smoothing for I ≈ P/V (not used for faults).
const CP_V_FILT_DIV: i32 = 3 * CONTROL_RATE_SCALE;
// If V_main changes sharply (e.g. PD contract step), snap the CP voltage filter to the new value
// to avoid an artificial current lag in CP mode.
const CP_V_STEP_RESET_MV: i32 = 200;
// CP steady-state trim: integrate a small current bias from power error to compensate calibration/model error.
// Kept conservative to avoid oscillation; feed-forward I≈P/V remains the primary path.
const CP_I_BIAS_ERR_DIV: i32 = 4 * CONTROL_RATE_SCALE;
const CP_I_BIAS_STEP_MAX_MA: i32 = 200;
const CP_I_BIAS_RESET_STEP_MW: u32 = 1_000;
// CP P-term gain: feedforward already provides I≈P/V; keep the P correction
// relatively gentle to avoid "double target" overshoot (notably on large down-steps).
const CP_I_P_ERR_DIV: i32 = 4;
// At low power (<=10W range), tolerance is tight; use a stronger P-term to reduce
// "sitting below target" after large down-steps without increasing high-power overshoot.
const CP_I_P_ERR_DIV_LOW_POWER: i32 = 2;
const CP_I_P_STEP_MAX_MA: i32 = 3_000;
const CP_I_SLEW_MAX_UP_MA_PER_MS: i32 = 2_000;
const CP_I_SLEW_MAX_DN_MA_PER_MS: i32 = 1_500;
const CP_I_SLEW_MAX_UP_MA_PER_TICK: i32 = if CONTROL_TICKS_PER_MS <= 1 {
    CP_I_SLEW_MAX_UP_MA_PER_MS
} else {
    {
        let per = CP_I_SLEW_MAX_UP_MA_PER_MS / (CONTROL_TICKS_PER_MS as i32);
        if per <= 0 { 1 } else { per }
    }
};
const CP_I_SLEW_MAX_DN_MA_PER_TICK: i32 = if CONTROL_TICKS_PER_MS <= 1 {
    CP_I_SLEW_MAX_DN_MA_PER_MS
} else {
    {
        let per = CP_I_SLEW_MAX_DN_MA_PER_MS / (CONTROL_TICKS_PER_MS as i32);
        if per <= 0 { 1 } else { per }
    }
};
const CP_STEP_BOOST_DETECT_MW: u32 = 10_000;
// Small positive bump during the post-downstep settling window to avoid
// sitting just below the tight 10W tolerance band due to quantization/noise.
const CP_DOWNSTEP_I_BOOST_MA: i32 = 4;

const CP_PTERM_NEG_FREEZE_MS: u32 = 20;
const CP_PTERM_POS_FREEZE_MS: u32 = 5;

const fn control_ticks_from_ms(ms: u32) -> u32 {
    let us = (ms as u64) * 1_000;
    let ticks = us / CONTROL_PERIOD_US;
    if ticks == 0 { 1 } else { ticks as u32 }
}

const CP_PTERM_NEG_FREEZE_TICKS: u32 = control_ticks_from_ms(CP_PTERM_NEG_FREEZE_MS);
const CP_PTERM_POS_FREEZE_TICKS: u32 = control_ticks_from_ms(CP_PTERM_POS_FREEZE_MS);

// CP performance capture (for on-device quick checks; external measurement remains the source of truth).
const CP_PERF_PERIOD_MS: u16 = 1;
const CP_PERF_SAMPLES: usize = 512;
const CP_PERF_WINDOW_CONSECUTIVE: usize = 3;
const CP_PERF_SMOOTH_WINDOW_SAMPLES: usize = 5;
const CP_FS_L_MW: u32 = 10_000;
const CP_FS_H_MW: u32 = 100_000;

// Best-effort TX sequencing for messages originating on the analog side (HELLO / FAST_STATUS).
// Acks reply with the request's seq and do not use this counter.
static TX_SEQ: AtomicU8 = AtomicU8::new(0);

// Dedicated fast-status TX queue to keep the control loop free of async waits.
static FAST_STATUS_TX_CH: Channel<CriticalSectionRawMutex, FastStatus, 4> = Channel::new();

fn update_zero_mv_iir(zero_mv: &mut u32, sample_mv: u32, div: u32) {
    let div = div.max(1);
    let sample_mv = sample_mv.min(1_000);
    let z = *zero_mv as i32;
    let s = sample_mv as i32;
    let dz = s - z;
    *zero_mv = (z + dz / (div as i32)).clamp(0, 1_000) as u32;
}

#[derive(Clone, Copy)]
struct CpPerfSample {
    dt_ms: u16,
    calc_p_mw: u32,
    v_main_mv: i32,
    i_total_ma: i32,
    target_i_total_ma: i32,
    flags: u8,
}

// CP performance capture shared state.
//
// Notes:
// - Sampling is done in a dedicated 1ms task to improve time resolution.
// - The sampled signals are "latest values" published by the control loop, so samples may repeat
//   if capture and control ticks align; this is still useful to quantify ms-level time-to-tolerance.
static CP_PERF_ARM_SEQ: AtomicU32 = AtomicU32::new(0);
static CP_PERF_ARM_MS: AtomicU32 = AtomicU32::new(0);
static CP_PERF_ARM_TARGET_P_MW: AtomicU32 = AtomicU32::new(0);
static CP_PERF_ARM_P0_MW: AtomicU32 = AtomicU32::new(0);
static CP_PERF_ACTIVE: AtomicBool = AtomicBool::new(false);
static CP_PERF_DONE: AtomicBool = AtomicBool::new(false);
static CP_PERF_START_MS: AtomicU32 = AtomicU32::new(0);
static CP_PERF_TARGET_P_MW: AtomicU32 = AtomicU32::new(0);
static CP_PERF_P0_MW: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LEN: AtomicU32 = AtomicU32::new(0);

static CP_PERF_LATEST_SEQ: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_P_MW: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_V_MAIN_MV: AtomicI32 = AtomicI32::new(0);
static CP_PERF_LATEST_V_LOCAL_MV: AtomicI32 = AtomicI32::new(0);
static CP_PERF_LATEST_I_TOTAL_MA: AtomicI32 = AtomicI32::new(0);
static CP_PERF_LATEST_TARGET_I_MA: AtomicI32 = AtomicI32::new(0);
static CP_PERF_LATEST_DAC1_CODE: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_DAC2_CODE: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_CUR1_SNS_MV: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_CUR2_SNS_MV: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_CUR1_SNS_MV_EFF: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_CUR2_SNS_MV_EFF: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_CUR1_ZERO_MV: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_CUR2_ZERO_MV: AtomicU32 = AtomicU32::new(0);
static CP_PERF_LATEST_EFFECTIVE_ENABLE: AtomicU8 = AtomicU8::new(0);
static CP_PERF_LATEST_FLAGS: AtomicU8 = AtomicU8::new(0);

static mut CP_PERF_BUF: [CpPerfSample; CP_PERF_SAMPLES] = [CpPerfSample {
    dt_ms: 0,
    calc_p_mw: 0,
    v_main_mv: 0,
    i_total_ma: 0,
    target_i_total_ma: 0,
    flags: 0,
}; CP_PERF_SAMPLES];

fn cp_tol_mw(target_p_mw: u32) -> u32 {
    let fs_mw = if target_p_mw <= CP_FS_L_MW {
        CP_FS_L_MW
    } else {
        CP_FS_H_MW
    };

    // tol(T) = 0.005*T + 0.005*FS  => (5/1000)*(T+FS)
    // Round up to be conservative.
    let numer = target_p_mw.saturating_add(fs_mw).saturating_mul(5);
    (numer.saturating_add(999)) / 1_000
}

fn cp_abs_diff_u32(a: u32, b: u32) -> u32 {
    if a >= b { a - b } else { b - a }
}

fn cp_find_first_consecutive_within_tol(
    samples: &[CpPerfSample],
    target_p_mw: u32,
    consecutive: usize,
) -> Option<u16> {
    if consecutive == 0 || samples.is_empty() {
        return None;
    }
    let tol_mw = cp_tol_mw(target_p_mw);

    let mut run = 0usize;
    for s in samples {
        let ok = cp_abs_diff_u32(s.calc_p_mw, target_p_mw) <= tol_mw;
        if ok {
            run += 1;
            if run >= consecutive {
                return Some(s.dt_ms);
            }
        } else {
            run = 0;
        }
    }
    None
}

fn cp_find_first_consecutive_within_tol_smoothed(
    samples: &[CpPerfSample],
    target_p_mw: u32,
    consecutive: usize,
    smooth_window: usize,
) -> Option<u16> {
    if consecutive == 0 || samples.is_empty() {
        return None;
    }
    let smooth_window = smooth_window
        .max(1)
        .min(CP_PERF_SMOOTH_WINDOW_SAMPLES.max(1));
    let tol_mw = cp_tol_mw(target_p_mw);

    let mut run = 0usize;
    let mut sum: u64 = 0;
    let mut buf: [u32; CP_PERF_SMOOTH_WINDOW_SAMPLES] = [0; CP_PERF_SMOOTH_WINDOW_SAMPLES];
    let mut filled: usize = 0;
    let mut idx: usize = 0;

    for s in samples {
        // Maintain a small moving average window over calc_p_mw to reduce ADC noise impact
        // on the (tight) low-range programming-accuracy tolerance.
        if filled < smooth_window {
            buf[filled] = s.calc_p_mw;
            sum = sum.saturating_add(s.calc_p_mw as u64);
            filled += 1;
        } else {
            let old = buf[idx];
            sum = sum.saturating_sub(old as u64);
            buf[idx] = s.calc_p_mw;
            sum = sum.saturating_add(s.calc_p_mw as u64);
            idx += 1;
            if idx >= smooth_window {
                idx = 0;
            }
        }
        let denom = (filled.max(1)) as u64;
        let avg = (sum / denom) as u32;

        let ok = cp_abs_diff_u32(avg, target_p_mw) <= tol_mw;
        if ok {
            run += 1;
            if run >= consecutive {
                return Some(s.dt_ms);
            }
        } else {
            run = 0;
        }
    }
    None
}

fn cp_find_t10_t90_ms(
    samples: &[CpPerfSample],
    p0_mw: u32,
    target_p_mw: u32,
) -> Option<(u16, u16)> {
    if samples.is_empty() || p0_mw == target_p_mw {
        return None;
    }
    let rising = target_p_mw > p0_mw;
    let delta = cp_abs_diff_u32(target_p_mw, p0_mw);
    let p10 = if rising {
        p0_mw.saturating_add((delta as u64 * 1 / 10) as u32)
    } else {
        p0_mw.saturating_sub((delta as u64 * 1 / 10) as u32)
    };
    let p90 = if rising {
        p0_mw.saturating_add((delta as u64 * 9 / 10) as u32)
    } else {
        p0_mw.saturating_sub((delta as u64 * 9 / 10) as u32)
    };

    let mut t10: Option<u16> = None;
    let mut t90: Option<u16> = None;
    for s in samples {
        let p = s.calc_p_mw;
        if t10.is_none() {
            let crossed = if rising { p >= p10 } else { p <= p10 };
            if crossed {
                t10 = Some(s.dt_ms);
            }
        }
        if t10.is_some() && t90.is_none() {
            let crossed = if rising { p >= p90 } else { p <= p90 };
            if crossed {
                t90 = Some(s.dt_ms);
                break;
            }
        }
    }

    match (t10, t90) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
    }
}

#[embassy_executor::task]
async fn cp_perf_sampler_task() {
    info!(
        "cp_perf sampler task starting (period={}ms samples={})",
        CP_PERF_PERIOD_MS, CP_PERF_SAMPLES
    );

    let mut last_arm_seq = CP_PERF_ARM_SEQ.load(Ordering::Relaxed);

    loop {
        Timer::after_millis(CP_PERF_PERIOD_MS as u64).await;

        let arm_seq = CP_PERF_ARM_SEQ.load(Ordering::Acquire);
        if arm_seq != last_arm_seq {
            last_arm_seq = arm_seq;

            let target_p_mw = CP_PERF_ARM_TARGET_P_MW.load(Ordering::Relaxed);
            if target_p_mw > 0 {
                let start_ms = CP_PERF_ARM_MS.load(Ordering::Relaxed);
                let p0_mw = CP_PERF_ARM_P0_MW.load(Ordering::Relaxed);
                CP_PERF_START_MS.store(start_ms, Ordering::Relaxed);
                CP_PERF_TARGET_P_MW.store(target_p_mw, Ordering::Relaxed);
                CP_PERF_P0_MW.store(p0_mw, Ordering::Relaxed);
                CP_PERF_LEN.store(0, Ordering::Relaxed);
                CP_PERF_DONE.store(false, Ordering::Relaxed);
                CP_PERF_ACTIVE.store(true, Ordering::Relaxed);
            } else {
                CP_PERF_ACTIVE.store(false, Ordering::Relaxed);
                CP_PERF_DONE.store(false, Ordering::Relaxed);
                CP_PERF_LEN.store(0, Ordering::Relaxed);
            }
        }

        if !CP_PERF_ACTIVE.load(Ordering::Relaxed) {
            continue;
        }

        let idx = CP_PERF_LEN.load(Ordering::Relaxed) as usize;
        if idx < CP_PERF_SAMPLES {
            // Use index-derived time to avoid ±1ms jitter from task scheduling/start alignment.
            let dt_ms = (idx as u32).saturating_mul(CP_PERF_PERIOD_MS as u32);

            // Read a consistent "latest" snapshot published by the control loop.
            let mut latest_p_mw: u32 = 0;
            let mut latest_v_main_mv: i32 = 0;
            let mut latest_i_total_ma: i32 = 0;
            let mut latest_target_i_ma: i32 = 0;
            let mut latest_flags: u8 = 0;
            for _ in 0..4 {
                let seq0 = CP_PERF_LATEST_SEQ.load(Ordering::Acquire);
                if (seq0 & 1) != 0 {
                    continue;
                }
                let p = CP_PERF_LATEST_P_MW.load(Ordering::Relaxed);
                let v_main = CP_PERF_LATEST_V_MAIN_MV.load(Ordering::Relaxed);
                let i_total = CP_PERF_LATEST_I_TOTAL_MA.load(Ordering::Relaxed);
                let tgt_i = CP_PERF_LATEST_TARGET_I_MA.load(Ordering::Relaxed);
                let flags = CP_PERF_LATEST_FLAGS.load(Ordering::Relaxed);
                let seq1 = CP_PERF_LATEST_SEQ.load(Ordering::Acquire);
                if seq0 == seq1 {
                    latest_p_mw = p;
                    latest_v_main_mv = v_main;
                    latest_i_total_ma = i_total;
                    latest_target_i_ma = tgt_i;
                    latest_flags = flags;
                    break;
                }
            }

            let sample = CpPerfSample {
                dt_ms: (dt_ms.min(u16::MAX as u32)) as u16,
                calc_p_mw: latest_p_mw,
                v_main_mv: latest_v_main_mv,
                i_total_ma: latest_i_total_ma,
                target_i_total_ma: latest_target_i_ma,
                flags: latest_flags,
            };
            unsafe {
                CP_PERF_BUF[idx] = sample;
            }
            CP_PERF_LEN.store((idx + 1) as u32, Ordering::Release);
        }

        let done = (idx + 1) >= CP_PERF_SAMPLES;
        if done {
            CP_PERF_ACTIVE.store(false, Ordering::Relaxed);
            CP_PERF_DONE.store(true, Ordering::Release);
        }
    }
}

#[embassy_executor::task]
async fn fast_status_tx_task(
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
) {
    info!("fast_status TX task starting");

    let mut raw_frame = [0u8; 192];
    let mut slip_frame = [0u8; 384];

    loop {
        let status = FAST_STATUS_TX_CH.receive().await;
        let seq = TX_SEQ.fetch_add(1, Ordering::Relaxed);

        let frame_len = match encode_fast_status_frame(seq, &status, &mut raw_frame) {
            Ok(len) => len,
            Err(err) => {
                warn!("fast_status encode error: {:?}", err);
                continue;
            }
        };
        let slip_len = match slip_encode(&raw_frame[..frame_len], &mut slip_frame) {
            Ok(len) => len,
            Err(err) => {
                warn!("fast_status slip encode error: {:?}", err);
                continue;
            }
        };

        let mut tx = uart_tx.lock().await;
        if let Err(_err) = tx.write(&slip_frame[..slip_len]).await {
            warn!("uart tx error; dropping fast_status");
        }
    }
}

// Fixed-point representation for conductance G (uA/mV) to avoid quantization-induced dithering:
// store as Q8: G_fp = G * 256.
const CV_G_FP_SHIFT: i32 = 8;
const CV_G_FP_SCALE: i32 = 1 << CV_G_FP_SHIFT;
const CV_G_MAX_FP: i32 = CV_G_MAX_UA_PER_MV * CV_G_FP_SCALE;
// Per-control-tick step clamp derived from the legacy per-FAST_STATUS tick clamp.
const CV_G_STEP_UP_MAX_FP: i32 = ((CV_G_STEP_UP_MAX_UA_PER_MV * CV_G_FP_SCALE) as i64
    * (CONTROL_PERIOD_US as i64)
    / (FAST_STATUS_PERIOD_US as i64)) as i32;
const CV_G_STEP_DN_MAX_FP: i32 = ((CV_G_STEP_DN_MAX_UA_PER_MV * CV_G_FP_SCALE) as i64
    * (CONTROL_PERIOD_US as i64)
    / (FAST_STATUS_PERIOD_US as i64)) as i32;

// 由数字板通过 SetPoint 消息更新的电流设定（mA，视为“两通道合计目标电流”）。
//
// - 初始值为 DEFAULT_TARGET_I_LOCAL_MA（1.0 A）。
// - uart_setpoint_rx_task 解析 SetPoint 帧并写入该原子量；
// - 采样/遥测主循环在每次迭代中读取该值，并按 I_SHARE_THRESHOLD_MA 决定单/双通道：
//   - I_total < 2 A：仅驱动通道 1（CH1），CH2 目标为 0；
//   - I_total ≥ 2 A：CH1/CH2 近似均分（奇数 mA 由 CH1 多承担 1 mA）。
static TARGET_I_LOCAL_MA: AtomicI32 = AtomicI32::new(DEFAULT_TARGET_I_LOCAL_MA);
// 启动时默认不使能输出：由数字板在握手完成后显式下发 SetEnable。
static ENABLE_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Copy, Clone)]
struct WindowAvg<const N: usize> {
    buf: [i32; N],
    sum: i64,
    idx: usize,
    initialized: bool,
}

impl<const N: usize> WindowAvg<N> {
    const fn new() -> Self {
        Self {
            buf: [0; N],
            sum: 0,
            idx: 0,
            initialized: false,
        }
    }

    fn clear(&mut self) {
        self.sum = 0;
        self.idx = 0;
        self.initialized = false;
    }

    fn reset_to(&mut self, x: i32) {
        self.buf.fill(x);
        self.sum = (x as i64) * (N as i64);
        self.idx = 0;
        self.initialized = true;
    }

    fn update(&mut self, x: i32) -> i32 {
        if !self.initialized {
            self.reset_to(x);
            return x;
        }

        let old = self.buf[self.idx];
        self.buf[self.idx] = x;
        self.idx += 1;
        if self.idx >= N {
            self.idx = 0;
        }
        self.sum += (x - old) as i64;
        (self.sum / (N as i64)) as i32
    }
}

struct CalSmoother<const N: usize> {
    last_kind_u8: u8,
    v_nr_100uv: WindowAvg<N>,
    v_rmt_100uv: WindowAvg<N>,
    cur1_100uv: WindowAvg<N>,
    cur2_100uv: WindowAvg<N>,
}

impl<const N: usize> CalSmoother<N> {
    const fn new() -> Self {
        Self {
            last_kind_u8: 0, // CalKind::Off == 0 in the protocol.
            v_nr_100uv: WindowAvg::new(),
            v_rmt_100uv: WindowAvg::new(),
            cur1_100uv: WindowAvg::new(),
            cur2_100uv: WindowAvg::new(),
        }
    }

    fn clear(&mut self) {
        self.v_nr_100uv.clear();
        self.v_rmt_100uv.clear();
        self.cur1_100uv.clear();
        self.cur2_100uv.clear();
    }

    fn update(
        &mut self,
        kind: CalKind,
        raw_v_nr_100uv: i16,
        raw_v_rmt_100uv: i16,
        raw_cur1_100uv: i16,
        raw_cur2_100uv: i16,
    ) -> (i16, i16, i16, i16) {
        let kind_u8 = u8::from(kind);

        // Only smooth while in calibration mode. Leaving calibration clears state so the
        // next entry starts from the current sample without stale history.
        if kind == CalKind::Off {
            self.last_kind_u8 = kind_u8;
            self.clear();
            return (
                raw_v_nr_100uv,
                raw_v_rmt_100uv,
                raw_cur1_100uv,
                raw_cur2_100uv,
            );
        }

        if self.last_kind_u8 != kind_u8 {
            self.last_kind_u8 = kind_u8;
            self.v_nr_100uv.reset_to(raw_v_nr_100uv as i32);
            self.v_rmt_100uv.reset_to(raw_v_rmt_100uv as i32);
            self.cur1_100uv.reset_to(raw_cur1_100uv as i32);
            self.cur2_100uv.reset_to(raw_cur2_100uv as i32);
            return (
                raw_v_nr_100uv,
                raw_v_rmt_100uv,
                raw_cur1_100uv,
                raw_cur2_100uv,
            );
        }

        let v_nr = self.v_nr_100uv.update(raw_v_nr_100uv as i32);
        let v_rmt = self.v_rmt_100uv.update(raw_v_rmt_100uv as i32);
        let cur1 = self.cur1_100uv.update(raw_cur1_100uv as i32);
        let cur2 = self.cur2_100uv.update(raw_cur2_100uv as i32);

        (
            v_nr.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            v_rmt.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            cur1.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            cur2.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        )
    }
}

fn raw_100uv_to_mv(raw_100uv: i16) -> u32 {
    let raw = raw_100uv as i32;
    if raw <= 0 { 0 } else { (raw as u32) / 10 }
}

// When determining whether a current calibration curve has already compensated
// the current-sense chain's 0A offset, accept a small residual around 0A.
const CURRENT_ZERO_CURVE_OK_MA: i32 = 20;

macro_rules! adc_avg_from_first {
    ($adc:expr, $ch:expr, $first:expr, $n:expr) => {{
        let n_u32: u32 = $n;
        if n_u32 <= 1 {
            $first
        } else {
            let mut acc: u32 = $first as u32;
            let mut i: u32 = 1;
            while i < n_u32 {
                acc = acc.saturating_add($adc.blocking_read($ch) as u32);
                i += 1;
            }
            (acc / n_u32) as u16
        }
    }};
}
// 用户校准是否已加载完成。仅当四条曲线均接收并验证合法后置为 true。
static CAL_READY: AtomicBool = AtomicBool::new(false);
// 当前校准模式选择（CalMode.kind），默认 off。
static CAL_MODE_KIND: AtomicU8 = AtomicU8::new(0);
// 校准曲线与多块接收状态。
static CAL_STATE: Mutex<CriticalSectionRawMutex, CalibrationState> =
    Mutex::new(CalibrationState::new());
// Active calibration curve snapshot published from the UART RX task.
// The control loop reads this without awaiting/locking (copy only when it changes).
static CAL_CURVES_SEQ: AtomicU32 = AtomicU32::new(0);
static mut CAL_CURVES_ACTIVE: [CalCurve; 4] = [CalCurve::empty(); 4];

fn cal_curves_publish(curves: [CalCurve; 4]) {
    CAL_CURVES_SEQ.fetch_add(1, Ordering::Release);
    unsafe {
        CAL_CURVES_ACTIVE = curves;
    }
    CAL_CURVES_SEQ.fetch_add(1, Ordering::Release);
}

fn cal_curves_read_consistent() -> ([CalCurve; 4], u32) {
    loop {
        let seq1 = CAL_CURVES_SEQ.load(Ordering::Acquire);
        if (seq1 & 1) != 0 {
            continue;
        }
        let snap = unsafe { CAL_CURVES_ACTIVE };
        let seq2 = CAL_CURVES_SEQ.load(Ordering::Acquire);
        if seq1 == seq2 {
            return (snap, seq2);
        }
    }
}
// 最近一次成功接收到来自数字板的协议控制帧（SetMode/SetPoint/SoftReset/SetEnable/...）的时间戳（ms）。
// LED1 闪烁逻辑基于该时间差实现“当前是否通信异常”的粗略指示。
static LAST_RX_GOOD_MS: AtomicU32 = AtomicU32::new(0);
// 是否曾经见过至少一帧来自数字板的有效控制消息（SetPoint / SoftReset / SetEnable）。
// 仅用于后续扩展统计，不再单独驱动 LED 指示。
static LINK_EVER_GOOD: AtomicBool = AtomicBool::new(false);
static SOFT_RESET_PENDING: AtomicBool = AtomicBool::new(false);
static LAST_SOFT_RESET_REASON: AtomicU8 = AtomicU8::new(0);
static LAST_SETPOINT_SEQ_VALID: AtomicBool = AtomicBool::new(false);
static LAST_SETPOINT_SEQ: AtomicU8 = AtomicU8::new(0);
static LAST_SETMODE_SEQ_VALID: AtomicBool = AtomicBool::new(false);
static LAST_SETMODE_SEQ: AtomicU8 = AtomicU8::new(0);
static QUIET_UNTIL_MS: AtomicU32 = AtomicU32::new(0);
static ACTIVE_MODE_SEEN: AtomicBool = AtomicBool::new(false);
static LAST_SETPOINT_IGNORED_LOG_MS: AtomicU32 = AtomicU32::new(0);

// Latched protection faults reported via FastStatus and used to gate output.
static FAULT_FLAGS: AtomicU32 = AtomicU32::new(0);

static UART_TX_SHARED: StaticCell<Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>> =
    StaticCell::new();

#[derive(Copy, Clone)]
struct ActiveControl {
    preset_id: u8,
    output_enabled: bool,
    mode: LoadMode,
    target_i_ma: i32,
    target_v_mv: i32,
    target_p_mw: u32,
    min_v_mv: i32,
    max_i_ma_total: i32,
    max_p_mw: u32,
    uv_latched: bool,
}

impl ActiveControl {
    const fn new() -> Self {
        Self {
            preset_id: 0,
            output_enabled: false,
            mode: LoadMode::Cc,
            target_i_ma: 0,
            target_v_mv: 0,
            target_p_mw: 0,
            min_v_mv: 0,
            max_i_ma_total: TARGET_I_MAX_MA,
            max_p_mw: 0,
            uv_latched: false,
        }
    }
}

// Active SetMode snapshot shared between UART RX task and the control loop.
//
// The control loop runs at a high rate; avoid `.await`/async mutexes on this path.
// Use a simple seqlock (even=stable, odd=writer-in-progress) to read a consistent snapshot.
static ACTIVE_CTRL_SEQ: AtomicU32 = AtomicU32::new(0);
static ACTIVE_CTRL_PRESET_ID: AtomicU8 = AtomicU8::new(0);
static ACTIVE_CTRL_OUTPUT_ENABLED: AtomicBool = AtomicBool::new(false);
static ACTIVE_CTRL_MODE_U8: AtomicU8 = AtomicU8::new(loadlynx_protocol::LOAD_MODE_CC);
static ACTIVE_CTRL_TARGET_I_MA: AtomicI32 = AtomicI32::new(0);
static ACTIVE_CTRL_TARGET_V_MV: AtomicI32 = AtomicI32::new(0);
static ACTIVE_CTRL_TARGET_P_MW: AtomicU32 = AtomicU32::new(0);
static ACTIVE_CTRL_MIN_V_MV: AtomicI32 = AtomicI32::new(0);
static ACTIVE_CTRL_MAX_I_MA_TOTAL: AtomicI32 = AtomicI32::new(TARGET_I_MAX_MA);
static ACTIVE_CTRL_MAX_P_MW: AtomicU32 = AtomicU32::new(0);
static ACTIVE_CTRL_UV_LATCHED: AtomicBool = AtomicBool::new(false);

fn active_control_reset() {
    ACTIVE_CTRL_SEQ.fetch_add(1, Ordering::Release);
    ACTIVE_CTRL_PRESET_ID.store(0, Ordering::Relaxed);
    ACTIVE_CTRL_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    ACTIVE_CTRL_MODE_U8.store(u8::from(LoadMode::Cc), Ordering::Relaxed);
    ACTIVE_CTRL_TARGET_I_MA.store(0, Ordering::Relaxed);
    ACTIVE_CTRL_TARGET_V_MV.store(0, Ordering::Relaxed);
    ACTIVE_CTRL_TARGET_P_MW.store(0, Ordering::Relaxed);
    ACTIVE_CTRL_MIN_V_MV.store(0, Ordering::Relaxed);
    ACTIVE_CTRL_MAX_I_MA_TOTAL.store(TARGET_I_MAX_MA, Ordering::Relaxed);
    ACTIVE_CTRL_MAX_P_MW.store(0, Ordering::Relaxed);
    ACTIVE_CTRL_UV_LATCHED.store(false, Ordering::Relaxed);
    ACTIVE_CTRL_SEQ.fetch_add(1, Ordering::Release);
}

fn active_control_set_uv_latched(v: bool) {
    ACTIVE_CTRL_SEQ.fetch_add(1, Ordering::Release);
    ACTIVE_CTRL_UV_LATCHED.store(v, Ordering::Relaxed);
    ACTIVE_CTRL_SEQ.fetch_add(1, Ordering::Release);
}

fn active_control_snapshot() -> ActiveControl {
    // Try a few times to get a consistent snapshot; fall back to best-effort.
    for _ in 0..3 {
        let seq1 = ACTIVE_CTRL_SEQ.load(Ordering::Acquire);
        if (seq1 & 1) != 0 {
            continue;
        }
        let snap = ActiveControl {
            preset_id: ACTIVE_CTRL_PRESET_ID.load(Ordering::Relaxed),
            output_enabled: ACTIVE_CTRL_OUTPUT_ENABLED.load(Ordering::Relaxed),
            mode: LoadMode::from(ACTIVE_CTRL_MODE_U8.load(Ordering::Relaxed)),
            target_i_ma: ACTIVE_CTRL_TARGET_I_MA.load(Ordering::Relaxed),
            target_v_mv: ACTIVE_CTRL_TARGET_V_MV.load(Ordering::Relaxed),
            target_p_mw: ACTIVE_CTRL_TARGET_P_MW.load(Ordering::Relaxed),
            min_v_mv: ACTIVE_CTRL_MIN_V_MV.load(Ordering::Relaxed),
            max_i_ma_total: ACTIVE_CTRL_MAX_I_MA_TOTAL.load(Ordering::Relaxed),
            max_p_mw: ACTIVE_CTRL_MAX_P_MW.load(Ordering::Relaxed),
            uv_latched: ACTIVE_CTRL_UV_LATCHED.load(Ordering::Relaxed),
        };
        let seq2 = ACTIVE_CTRL_SEQ.load(Ordering::Acquire);
        if seq1 == seq2 {
            return snap;
        }
    }

    ActiveControl {
        preset_id: ACTIVE_CTRL_PRESET_ID.load(Ordering::Relaxed),
        output_enabled: ACTIVE_CTRL_OUTPUT_ENABLED.load(Ordering::Relaxed),
        mode: LoadMode::from(ACTIVE_CTRL_MODE_U8.load(Ordering::Relaxed)),
        target_i_ma: ACTIVE_CTRL_TARGET_I_MA.load(Ordering::Relaxed),
        target_v_mv: ACTIVE_CTRL_TARGET_V_MV.load(Ordering::Relaxed),
        target_p_mw: ACTIVE_CTRL_TARGET_P_MW.load(Ordering::Relaxed),
        min_v_mv: ACTIVE_CTRL_MIN_V_MV.load(Ordering::Relaxed),
        max_i_ma_total: ACTIVE_CTRL_MAX_I_MA_TOTAL.load(Ordering::Relaxed),
        max_p_mw: ACTIVE_CTRL_MAX_P_MW.load(Ordering::Relaxed),
        uv_latched: ACTIVE_CTRL_UV_LATCHED.load(Ordering::Relaxed),
    }
}

#[derive(Copy, Clone)]
struct LimitProfileLocal {
    max_i_ma: i32,
    max_p_mw: u32,
    ovp_mv: i32,
    temp_trip_mc: i32,
    thermal_derate_pct: u8,
}

// Latest software-configured limits from the digital side. Defaults align with
// existing hard limits so that behavior remains unchanged until a profile is
// received.
static LIMIT_PROFILE: Mutex<CriticalSectionRawMutex, LimitProfileLocal> =
    Mutex::new(LimitProfileLocal {
        max_i_ma: TARGET_I_MAX_MA,
        max_p_mw: 100_000,
        ovp_mv: OV_LIMIT_MV,
        temp_trip_mc: SINK_TEMP_LIMIT_MC,
        thermal_derate_pct: 100,
    });

fn timestamp_ms() -> u64 {
    Instant::now().as_millis()
}

defmt::timestamp!("{=u64:ms}", timestamp_ms());

fn ntc_mv_to_mc(node_mv: u32) -> i32 {
    // NTC channels TS1/TS2: 10 kΩ @25 °C, B=3950 K, 5.11 kΩ pull-up to 3.3 V.
    // See docs/thermal/ntc-temperature-sensing.md for details.
    const VSUP_MV: f32 = 3300.0;
    const RPULL_OHM: f32 = 5_110.0;
    const R0_OHM: f32 = 10_000.0;
    const B: f32 = 3950.0;
    const T0_K: f32 = 273.15 + 25.0;

    let v_mv = node_mv as f32;
    if v_mv <= 0.0 || v_mv >= VSUP_MV {
        return 0;
    }

    let v_ratio = v_mv / VSUP_MV;
    let r_ntc = RPULL_OHM * v_ratio / (1.0 - v_ratio);

    let ln_ratio = logf(r_ntc / R0_OHM);
    let inv_t = 1.0 / T0_K + (1.0 / B) * ln_ratio;
    let t_k = 1.0 / inv_t;
    let t_c = t_k - 273.15;
    let t_mc = (t_c * 1000.0) as i32;

    t_mc.clamp(0, 150_000)
}

fn g4_internal_mcu_temp_to_mc(adc_code: u16) -> i32 {
    // STM32G4 internal temperature sensor calibration points (see RM0440 + DS13122):
    // - TS_CAL1: 30 °C  factory calibration, address 0x1FFF_75A8 (16-bit)
    // - TS_CAL2: 110 °C factory calibration, address 0x1FFF_75CA (16-bit)
    const TS_CAL1_ADDR: *const u16 = 0x1FFF_75A8 as *const u16;
    const TS_CAL2_ADDR: *const u16 = 0x1FFF_75CA as *const u16;
    const TS_CAL1_TEMP_C: i32 = 30;
    const TS_CAL2_TEMP_C: i32 = 110;

    let ts_cal1 = unsafe { core::ptr::read(TS_CAL1_ADDR) as i32 };
    let ts_cal2 = unsafe { core::ptr::read(TS_CAL2_ADDR) as i32 };
    let adc = adc_code as i32;

    if ts_cal2 <= ts_cal1 {
        return 0;
    }

    let temp_c = (TS_CAL2_TEMP_C - TS_CAL1_TEMP_C).saturating_mul(adc - ts_cal1)
        / (ts_cal2 - ts_cal1)
        + TS_CAL1_TEMP_C;

    (temp_c * 1_000).clamp(0, 150_000)
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    // Clock config:
    // Align with the reference PD sink bring-up (pd-sink-stm32g431cbu6-rs):
    // - SYSCLK = 170MHz (PLL1_R)
    // - CLK48 = HSI48 (for UCPD)
    let mut config = stm32::Config::default();
    {
        use embassy_stm32::rcc::mux;
        use embassy_stm32::rcc::*;

        config.rcc.hsi48 = Some(Hsi48Config {
            sync_from_usb: true,
        });
        config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
        config.rcc.mux.adc12sel = mux::Adcsel::SYS;

        config.rcc.pll = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL85,
            divp: None,
            divq: None,
            divr: Some(PllRDiv::DIV2),
        });
        config.rcc.sys = Sysclk::PLL1_R;

        config.enable_ucpd1_dead_battery = true;
    }
    let p = stm32::init(config);

    info!("LoadLynx analog alive; init VREFBUF/ADC/DAC/UART (CC 0.5A, real telemetry)");

    // Ensure the DBCC pins don't load/distort the CC lines.
    // On this board they may be tied to the USB-C CC nets for dead-battery behavior.
    {
        let mut dbcc1 = Flex::new(p.PA9);
        let mut dbcc2 = Flex::new(p.PA10);
        dbcc1.set_as_analog();
        dbcc2.set_as_analog();
        let _ = (dbcc1, dbcc2);
    }

    // VREFBUF：仿照 pd-sink-stm32g431cbu6-rs，直接写 CSR=0x0000_0021
    // ENVR=1（bit0），HIZ=0（bit1，VREF+ 连接缓冲输出），VRS=0b10（bits5:4，约 2.9V 档位）。
    unsafe {
        core::ptr::write_volatile(VREFBUF_CSR_ADDR, 0x0000_0021u32);
        let csr = core::ptr::read_volatile(VREFBUF_CSR_ADDR);
        info!("VREFBUF CSR after write: 0x{:08x}", csr);
    }

    // TPS22810 负载开关：PB13=LOAD_EN_CTL，PB14=LOAD_EN_TS。
    // 逻辑：LOAD_EN = LOAD_EN_CTL AND LOAD_EN_TS。
    //
    // 启动时默认保持关断（尤其是 USB-PD 协商阶段），由数字板显式下发 SetEnable 后再打开。
    let mut load_en_ctl = Output::new(p.PB13, Level::Low, Speed::Low);
    let mut load_en_ts = Output::new(p.PB14, Level::Low, Speed::Low);
    load_en_ctl.set_low();
    load_en_ts.set_low();

    // 板载状态 LED1：PB3 对应 LEDK1（阴极），低电平点亮、默认熄灭。
    let mut led1 = Output::new(p.PB3, Level::High, Speed::Low);
    led1.set_high();

    // 上电自检：闪烁 LED1 若干次，方便确认硬件连线正常。
    // Keep this short to avoid delaying UART bring-up (the digital side may
    // already be transmitting during reset).
    for _ in 0..1 {
        led1.set_low();
        Timer::after_millis(50).await;
        led1.set_high();
        Timer::after_millis(50).await;
    }

    // UART3：与数字板交互的链路，115200 8N1。
    let mut uart_cfg = UartConfig::default();
    // Match digital side exactly: 115200 baud, 8 data bits, no parity, 1 stop bit.
    uart_cfg.baudrate = 115_200;
    uart_cfg.data_bits = UartDataBits::DataBits8;
    uart_cfg.parity = UartParity::ParityNone;
    uart_cfg.stop_bits = UartStopBits::STOP1;

    let uart = Uart::new(
        p.USART3, p.PC11, p.PC10, Irqs, p.DMA1_CH1, p.DMA1_CH2, uart_cfg,
    )
    .unwrap();

    // 拆分为 TX/RX 两个半通道：
    // - FAST_STATUS 通过独立 TX 任务发送（避免控制环 await）
    // - 另起任务在 RX 上监听来自数字板的 SetMode/SetPoint 控制帧。
    let (uart_tx, uart_rx): (UartTx<'static, UartAsync>, UartRx<'static, UartAsync>) = uart.split();

    let uart_tx_shared = UART_TX_SHARED.init(Mutex::new(uart_tx));

    if let Err(e) = _spawner.spawn(fast_status_tx_task(uart_tx_shared)) {
        warn!("failed to spawn fast_status_tx_task: {:?}", e);
    }

    // 将 RX 端转换为环形缓冲 UART，以避免在任务之间存在调度间隙时丢字节。
    // 115200 baud ≈ 11.5 kB/s; 4 KiB buffer provides ~350ms of headroom for
    // bursty traffic (e.g. calibration curve writes) without triggering overruns.
    static UART_RX_DMA_BUF: StaticCell<[u8; 4096]> = StaticCell::new();
    let uart_rx_ring: RingBufferedUartRx<'static> =
        uart_rx.into_ring_buffered(UART_RX_DMA_BUF.init([0; 4096]));

    // 启动独立任务接收 SetPoint 控制消息。
    if let Err(e) = _spawner.spawn(uart_setpoint_rx_task(uart_rx_ring, uart_tx_shared)) {
        warn!("failed to spawn uart_setpoint_rx_task: {:?}", e);
    }

    // CP performance sampler (1ms task).
    if let Err(e) = _spawner.spawn(cp_perf_sampler_task()) {
        warn!("failed to spawn cp_perf_sampler_task: {:?}", e);
    }

    // UCPD1: USB-PD sink core (runs independently from the control loop).
    // The PD task owns UCPD and handles attach/detach/orientation.
    if let Err(e) = _spawner.spawn(pd::pd_task(
        p.UCPD1,
        p.PB6,
        p.PB4,
        p.DMA2_CH4,
        p.DMA2_CH5,
        uart_tx_shared,
    )) {
        warn!("failed to spawn pd_task: {:?}", e);
    }

    // ADC1/ADC2：控制环需要较高更新速率；外部通道使用较短采样时间，
    // 内部温度传感器/参考电压在读取时临时切换到长采样时间。
    let mut adc1 = Adc::new(p.ADC1);
    let mut adc2 = Adc::new(p.ADC2);
    // Default to a shorter sampling time for external channels to keep the control loop budget.
    adc1.set_sample_time(SampleTime::CYCLES24_5);
    adc2.set_sample_time(SampleTime::CYCLES24_5);
    info!("ADC1/ADC2 init complete");

    // 片内温度传感器（连接到 ADC1_IN16），用于 MCU die 温度遥测。
    let mut mcu_temp_ch: AdcTemperature = adc1.enable_temperature();

    // 使能内部基准电压通道（VrefInt），按官方编程手册流程计算当前 VREF+（VDDA）。
    let mut vrefint_ch = adc1.enable_vrefint();
    let vrefint_cal = unsafe { core::ptr::read(VREFINT_CAL_ADDR) as u32 };
    // 内部参考电压通道在 G4 上推荐使用较长采样时间（≥ 20µs 级别），使用 640.5 周期。
    adc1.set_sample_time(SampleTime::CYCLES640_5);

    // 多次采样 VrefInt，获得更稳定的 raw 值用于 VDDA 计算。
    let mut vrefint_acc: u32 = 0;
    let samples: u32 = 16;
    for _ in 0..samples {
        let raw = adc1.blocking_read(&mut vrefint_ch) as u32;
        vrefint_acc = vrefint_acc.saturating_add(raw);
    }
    let vrefint_raw = vrefint_acc / samples;

    // 使用 ST 推荐公式：VDDA = VREF_CALIB_MV * VREFINT_CAL / VREFINT_RAW。
    // 注意：VREF_CALIB_MV 对应数据手册中“工厂校准时的 VREF+”（通常为 3.0V 或 3.3V）。
    let vref_mv = if vrefint_raw > 0 {
        VREF_CALIB_MV.saturating_mul(vrefint_cal) / vrefint_raw
    } else {
        VREF_CALIB_MV
    };
    // Restore short sampling time for external channels.
    adc1.set_sample_time(SampleTime::CYCLES24_5);
    info!(
        "ADC Vref calibration: cal_code={} raw_code_avg={} vref_mv={}mV",
        vrefint_cal, vrefint_raw, vref_mv
    );

    // 通道映射（硬件 v4.2，参考 loadlynx.ioc 与 analog-board-netlist.enet）：
    // - PA0: CUR1_SNS   → CH1 电流采样（OPA2365 输出，经 Rsense=50 mΩ + G=10，单端）
    // - PA1: CUR2_SNS   → CH2 电流采样（同上）
    // - PA2: V_RMT_SNS  → 远端电压差分放大后输出（单端）
    // - PA3: V_NR_SNS   → 近端电压差分放大后输出（单端）
    // - PB12: _5V_SNS   → 模拟板 5V 轨电压分压
    // - PB0/PB1: TS1/TS2→ 温度传感器（打包入 FastStatus）
    let mut cur1_sns = p.PA0.degrade_adc();
    let mut cur2_sns = p.PA1.degrade_adc();
    let mut v_rmt_sns = p.PA2.degrade_adc();
    let mut v_nr_sns = p.PA3.degrade_adc();

    let mut sns_5v = p.PB12.degrade_adc();
    let mut ts1 = p.PB0.degrade_adc();
    let mut ts2 = p.PB1.degrade_adc();

    // DAC1：PA4/PA5 → CH1/CH2。上电默认按总目标电流应用通道调度：
    //   - I_total < 2 A：仅 CH1 有输出，CH2=0；
    //   - I_total ≥ 2 A：CH1/CH2 近似均分。
    let mut dac = {
        let mut dac = Dac::new_blocking(p.DAC1, p.PA4, p.PA5);

        // Disable the output buffer (unbuffered mode) for both channels.
        dac.ch1().set_mode(DacMode::NormalExternalUnbuffered);
        dac.ch1().enable();
        dac.ch2().set_mode(DacMode::NormalExternalUnbuffered);
        dac.ch2().enable();

        info!("DAC mode: external unbuffered (buffer disabled)");
        dac
    };

    // Best-effort current-sense zero offset capture.
    //
    // Some boards exhibit a fixed offset on CUR*_SNS (e.g. ~200–300mV at 0A),
    // which is catastrophic for CP loop correctness if the active curve is
    // "zero-anchored" (expects raw≈0 at 0A). We capture the baseline at boot
    // with output disabled and DAC set to 0, then subtract it only for
    // zero-anchored current curves during normal operation.
    dac.ch1().set(DacValue::Bit12Right(0));
    dac.ch2().set(DacValue::Bit12Right(0));
    let (mut cur1_zero_mv, mut cur2_zero_mv) = {
        let adc_to_mv = |code: u16| -> u32 { (code as u32) * vref_mv / ADC_FULL_SCALE };
        let samples: u32 = 64;
        let mut acc1: u32 = 0;
        let mut acc2: u32 = 0;
        for _ in 0..samples {
            acc1 = acc1.saturating_add(adc2.blocking_read(&mut cur1_sns) as u32);
            acc2 = acc2.saturating_add(adc2.blocking_read(&mut cur2_sns) as u32);
        }
        let code1 = (acc1 / samples) as u16;
        let code2 = (acc2 / samples) as u16;
        let mv1 = adc_to_mv(code1);
        let mv2 = adc_to_mv(code2);
        info!(
            "current zero offset: cur1={}mV(code={}) cur2={}mV(code={})",
            mv1, code1, mv2, code2
        );
        (mv1, mv2)
    };
    let init_total_i_ma = DEFAULT_TARGET_I_LOCAL_MA;
    let (init_ch1_ma, init_ch2_ma) = if init_total_i_ma < I_SHARE_THRESHOLD_MA {
        (init_total_i_ma, 0)
    } else {
        let half = init_total_i_ma / 2;
        let rem = init_total_i_ma - 2 * half;
        (half + rem, half)
    };
    let init_raw_ch1_100uv = init_ch1_ma.saturating_mul(5).clamp(0, i16::MAX as i32) as i16;
    let init_raw_ch2_100uv = init_ch2_ma.saturating_mul(5).clamp(0, i16::MAX as i32) as i16;
    let init_dac_code_ch1 = raw_100uv_to_dac_code_vref(init_raw_ch1_100uv, vref_mv);
    let init_dac_code_ch2 = raw_100uv_to_dac_code_vref(init_raw_ch2_100uv, vref_mv);
    dac.ch1().set(DacValue::Bit12Right(init_dac_code_ch1));
    dac.ch2().set(DacValue::Bit12Right(init_dac_code_ch2));

    info!(
        "CC setpoint: default total target {} mA (CH1={} mA, CH2={} mA, DAC1={}, DAC2={})",
        init_total_i_ma, init_ch1_ma, init_ch2_ma, init_dac_code_ch1, init_dac_code_ch2
    );

    let mut raw_frame = [0u8; 192];
    let mut slip_frame = [0u8; 384];

    // 上电后发送一次 HELLO，携带最小协议/固件信息，供数字侧建立链路状态。
    let hello = Hello {
        protocol_version: loadlynx_protocol::PROTOCOL_VERSION,
        fw_version: HELLO_FW_VERSION,
    };
    let hello_seq = TX_SEQ.fetch_add(1, Ordering::Relaxed);
    match encode_hello_frame(hello_seq, &hello, &mut raw_frame) {
        Ok(frame_len) => match slip_encode(&raw_frame[..frame_len], &mut slip_frame) {
            Ok(slip_len) => {
                let mut tx = uart_tx_shared.lock().await;
                match tx.write(&slip_frame[..slip_len]).await {
                    Ok(_) => {
                        info!(
                            "HELLO sent: seq={} proto_ver={} fw_ver=0x{:08x}",
                            hello_seq, hello.protocol_version, hello.fw_version
                        );
                    }
                    Err(err) => {
                        warn!("HELLO write error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                warn!("HELLO slip encode error: {:?}", err);
            }
        },
        Err(err) => {
            warn!("HELLO encode error: {:?}", err);
        }
    }

    let mut last_link_fault = false;

    // 远端 sense 判定状态（3 帧进入 / 2 帧退出）。
    let mut remote_active: bool = false;
    let mut remote_good_streak: u8 = 0;
    let mut remote_bad_streak: u8 = 0;

    // Calibration-only UI/RAW smoothing (see CAL_SMOOTH_WINDOW_FRAMES).
    let mut cal_smoother: CalSmoother<CAL_SMOOTH_WINDOW_FRAMES> = CalSmoother::new();
    // Cache calibration curves; refresh without awaiting in the fast control loop.
    let mut curves = {
        let state = CAL_STATE.lock().await;
        state.snapshot()
    };
    cal_curves_publish(curves);
    let mut curves_seq_seen = CAL_CURVES_SEQ.load(Ordering::Acquire);

    // CV loop internal state:
    // - conductance G (uA/mV) stored as fixed-point Q8 (x256)
    // - filtered V_main used by the CV control law (not used for protection)
    let mut cv_g_uapermv_fp: i32 = 0;
    let mut cv_v_main_filt_mv: i32 = 0;
    let mut cv_v_filt_init: bool = false;

    // CP loop internal state: filtered V_main used for I ≈ P/V.
    let mut cp_v_main_filt_mv: i32 = 0;
    let mut cp_v_filt_init: bool = false;
    let mut cp_i_bias_ma: i32 = 0;
    let mut cp_last_target_p_mw: u32 = 0;
    let mut cp_i_cmd_slewed_ma: i32 = 0;
    // After large CP down-steps, temporarily suppress negative P-term corrections.
    // Feed-forward I≈P/V should handle the falling edge; allowing negative P-term
    // immediately can pull the command below feed-forward and cause undershoot
    // + slow recovery around the tight 10W tolerance band.
    let mut cp_pterm_neg_freeze_ticks: u32 = 0;
    // After large CP up-steps, temporarily suppress positive P-term corrections.
    // Feed-forward already provides the primary step. Allowing a large positive
    // P-term immediately tends to overshoot power and delays settling.
    let mut cp_pterm_pos_freeze_ticks: u32 = 0;

    // Current-sense zero tracking state (see below).
    let mut cur_zero_cmd_ticks: u16 = 0;
    let mut cur_zero_disabled_ticks: u16 = 0;

    // FastStatus cadence divider: control loop runs faster than status TX.
    let mut status_div: u32 = 0;
    // Throttle verbose telemetry logs to reduce RTT load during time-sensitive operations (e.g. USB-PD).
    // CONTROL_TICKS_PER_STATUS yields 20 Hz; we log every 20 status ticks => ~1 Hz.
    let mut telemetry_log_div: u8 = 0;

    // Slow ADC channels (updated at FAST_STATUS cadence).
    let mut sns_5v_code: u16 = adc1.blocking_read(&mut sns_5v);
    let mut ts1_code: u16 = adc1.blocking_read(&mut ts1);
    let mut ts2_code: u16 = adc1.blocking_read(&mut ts2);
    let mut mcu_temp_code: u16 = adc1.blocking_read(&mut mcu_temp_ch);

    // Control-loop timing stats (best-effort, for on-device regression checks).
    let mut loop_last_us: u64 = Instant::now().as_micros();
    let mut loop_dt_min_us: u32 = u32::MAX;
    let mut loop_dt_max_us: u32 = 0;
    let mut loop_dt_sum_us: u64 = 0;
    let mut loop_dt_n: u32 = 0;

    let control_period = Duration::from_micros(CONTROL_PERIOD_US);
    let mut next_tick = Instant::now() + control_period;

    loop {
        let now_ms = timestamp_ms() as u32;

        // Track actual loop period (includes Timer::after_millis sleep + work).
        let now_us = Instant::now().as_micros();
        let dt_us = now_us.saturating_sub(loop_last_us);
        loop_last_us = now_us;
        if loop_dt_n > 0 {
            let dt = (dt_us.min(u32::MAX as u64)) as u32;
            loop_dt_min_us = loop_dt_min_us.min(dt);
            loop_dt_max_us = loop_dt_max_us.max(dt);
            loop_dt_sum_us = loop_dt_sum_us.saturating_add(dt as u64);
        }
        loop_dt_n = loop_dt_n.saturating_add(1);
        if loop_dt_n >= CONTROL_TICKS_PER_SEC {
            let denom = (loop_dt_n.saturating_sub(1)).max(1) as u64;
            let avg = (loop_dt_sum_us / denom) as u32;
            info!(
                "control_loop dt_us: min={} max={} avg={} (target={}us)",
                loop_dt_min_us,
                loop_dt_max_us,
                avg,
                (CONTROL_PERIOD_US as u32),
            );
            loop_dt_min_us = u32::MAX;
            loop_dt_max_us = 0;
            loop_dt_sum_us = 0;
            loop_dt_n = 0;
        }

        let is_status_tick = status_div == 0;
        let is_ms_tick = (status_div % CONTROL_TICKS_PER_MS.max(1)) == 0;
        if is_status_tick {
            let seq = CAL_CURVES_SEQ.load(Ordering::Acquire);
            if seq != curves_seq_seen && (seq & 1) == 0 {
                let (snap, snap_seq) = cal_curves_read_consistent();
                curves = snap;
                curves_seq_seen = snap_seq;
            }
        }
        let is_telemetry_log_tick = if is_status_tick {
            let tick = telemetry_log_div == 0;
            telemetry_log_div = (telemetry_log_div + 1) % 20;
            tick
        } else {
            false
        };
        if is_telemetry_log_tick {
            info!("main loop top");
        }
        if SOFT_RESET_PENDING.swap(false, Ordering::SeqCst) {
            apply_soft_reset_safing(&mut dac, &mut load_en_ctl, &mut load_en_ts).await;

            // 在软复位 safing 完成后重新发送 HELLO，提示数字侧重新握手。
            let hello = Hello {
                protocol_version: loadlynx_protocol::PROTOCOL_VERSION,
                fw_version: HELLO_FW_VERSION,
            };
            let hello_seq = TX_SEQ.fetch_add(1, Ordering::Relaxed);
            match encode_hello_frame(hello_seq, &hello, &mut raw_frame) {
                Ok(frame_len) => match slip_encode(&raw_frame[..frame_len], &mut slip_frame) {
                    Ok(slip_len) => {
                        let mut tx = uart_tx_shared.lock().await;
                        match tx.write(&slip_frame[..slip_len]).await {
                            Ok(_) => {
                                info!(
                                    "HELLO re-sent after soft_reset: seq={} proto_ver={} fw_ver=0x{:08x}",
                                    hello_seq, hello.protocol_version, hello.fw_version
                                );
                            }
                            Err(err) => {
                                warn!("HELLO(after soft_reset) write error: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        warn!("HELLO(after soft_reset) slip encode error: {:?}", err);
                    }
                },
                Err(err) => {
                    warn!("HELLO(after soft_reset) encode error: {:?}", err);
                }
            }
        }

        // 通信健康监控：基于“最近一次收到有效控制帧”的时间戳判断当前是否通信异常。
        //
        // - 在上电后的一小段宽限期（LINK_DEAD_TIMEOUT_MS）内，允许链路尚未建立；
        // - 若自上次收到 SetPoint / SoftReset / SetEnable 起超过 LINK_DEAD_TIMEOUT_MS
        //   未再看到任何控制帧，则认为当前处于“通信异常”状态，让 LED1 闪烁；
        // - 一旦重新收到有效控制帧，则视作恢复正常，LED1 熄灭。
        let last_rx = LAST_RX_GOOD_MS.load(Ordering::Relaxed);
        let link_fault = if last_rx == 0 {
            now_ms > LINK_DEAD_TIMEOUT_MS
        } else {
            now_ms.wrapping_sub(last_rx) > LINK_DEAD_TIMEOUT_MS
        };

        if link_fault != last_link_fault {
            if link_fault {
                warn!(
                    "link fault: no control frames from digital for >{} ms (last_rx_ms={})",
                    LINK_DEAD_TIMEOUT_MS, last_rx
                );
            } else {
                info!(
                    "link recovered: control frame seen recently (last_rx_ms={})",
                    last_rx
                );
            }
            last_link_fault = link_fault;
        }

        if link_fault {
            // 以约 2 Hz 频率闪烁：每 250 ms 翻转一次。
            #[allow(clippy::manual_is_multiple_of)]
            if (now_ms / 250) % 2 == 0 {
                led1.set_low();
            } else {
                led1.set_high();
            }
        } else {
            // 链路看起来正常时保持灭灯。
            led1.set_high();
        }

        // --- ADC sampling (blocking) ---
        // Fast channels (every control tick): V sense + I sense (used by control + fast protection).
        // Slow channels (FAST_STATUS cadence): 5V + NTC + MCU temp (slow dynamics, still safety-gated).
        let cal_kind = CalKind::from(CAL_MODE_KIND.load(Ordering::Relaxed));

        let v_rmt_sns_code = adc1.blocking_read(&mut v_rmt_sns);
        let v_nr_sns_code = adc1.blocking_read(&mut v_nr_sns);

        let cur1_sns_code = adc2.blocking_read(&mut cur1_sns);
        let cur2_sns_code = adc2.blocking_read(&mut cur2_sns);

        // Calibration-only oversampling to reduce 1–10 kHz ripple influence on capture/display.
        // Keep instantaneous samples above for fault detection and edge conditions.
        let (v_rmt_sns_code_cal, v_nr_sns_code_cal, cur1_sns_code_cal, cur2_sns_code_cal) =
            if is_status_tick {
                match cal_kind {
                    CalKind::Voltage => (
                        adc_avg_from_first!(
                            adc1,
                            &mut v_rmt_sns,
                            v_rmt_sns_code,
                            CAL_OVERSAMPLE_SAMPLES
                        ),
                        adc_avg_from_first!(
                            adc1,
                            &mut v_nr_sns,
                            v_nr_sns_code,
                            CAL_OVERSAMPLE_SAMPLES
                        ),
                        cur1_sns_code,
                        cur2_sns_code,
                    ),
                    CalKind::CurrentCh1 => (
                        v_rmt_sns_code,
                        adc_avg_from_first!(
                            adc1,
                            &mut v_nr_sns,
                            v_nr_sns_code,
                            CAL_OVERSAMPLE_SAMPLES
                        ),
                        adc_avg_from_first!(
                            adc2,
                            &mut cur1_sns,
                            cur1_sns_code,
                            CAL_OVERSAMPLE_SAMPLES
                        ),
                        cur2_sns_code,
                    ),
                    CalKind::CurrentCh2 => (
                        v_rmt_sns_code,
                        adc_avg_from_first!(
                            adc1,
                            &mut v_nr_sns,
                            v_nr_sns_code,
                            CAL_OVERSAMPLE_SAMPLES
                        ),
                        cur1_sns_code,
                        adc_avg_from_first!(
                            adc2,
                            &mut cur2_sns,
                            cur2_sns_code,
                            CAL_OVERSAMPLE_SAMPLES
                        ),
                    ),
                    CalKind::Off => (v_rmt_sns_code, v_nr_sns_code, cur1_sns_code, cur2_sns_code),
                }
            } else {
                (v_rmt_sns_code, v_nr_sns_code, cur1_sns_code, cur2_sns_code)
            };

        if is_status_tick {
            sns_5v_code = adc1.blocking_read(&mut sns_5v);
            ts1_code = adc1.blocking_read(&mut ts1);
            ts2_code = adc1.blocking_read(&mut ts2);
            // Internal temperature sensor needs a longer sampling time on G4.
            adc1.set_sample_time(SampleTime::CYCLES640_5);
            mcu_temp_code = adc1.blocking_read(&mut mcu_temp_ch);
            adc1.set_sample_time(SampleTime::CYCLES24_5);
        }

        // 节点电压：使用基于 VrefInt 的当前 VREF+（vref_mv）进行换算，单位 mV。
        let adc_to_mv = |code: u16| -> u32 { (code as u32) * vref_mv / ADC_FULL_SCALE };

        let v_rmt_sns_mv = adc_to_mv(v_rmt_sns_code);
        let v_nr_sns_mv = adc_to_mv(v_nr_sns_code);

        let cur1_sns_mv = adc_to_mv(cur1_sns_code);
        let cur2_sns_mv = adc_to_mv(cur2_sns_code);

        // Optional current-sense baseline correction (normal operation only).
        //
        // We can subtract a measured 0A baseline (cur*_zero_mv) from the ADC sense voltage
        // to improve low-current accuracy. However, some calibration curves already embed
        // an offset (i.e. they map the baseline raw value to ~0mA). In that case, applying
        // a baseline subtraction again would double-compensate and distort readings.
        //
        // Heuristic: if the active current curve maps the *measured* baseline raw value
        // to ~0mA, consider it already compensated; otherwise apply baseline subtraction.
        let cur1_points = curves[CurveKind::CurrentCh1.index()].as_slice();
        let cur2_points = curves[CurveKind::CurrentCh2.index()].as_slice();
        let cur1_zero_raw_100uv = mv_to_raw_100uv(cur1_zero_mv);
        let cur2_zero_raw_100uv = mv_to_raw_100uv(cur2_zero_mv);
        let cur1_curve_zero_ok = if cur1_points.is_empty() {
            false
        } else {
            match piecewise_linear(cur1_points, cur1_zero_raw_100uv) {
                Ok(i_ma) => i_ma.abs() <= CURRENT_ZERO_CURVE_OK_MA,
                Err(_) => false,
            }
        };
        let cur2_curve_zero_ok = if cur2_points.is_empty() {
            false
        } else {
            match piecewise_linear(cur2_points, cur2_zero_raw_100uv) {
                Ok(i_ma) => i_ma.abs() <= CURRENT_ZERO_CURVE_OK_MA,
                Err(_) => false,
            }
        };
        let apply_cur1_zero =
            cal_kind == CalKind::Off && (cur1_sns_mv <= cur1_zero_mv || !cur1_curve_zero_ok);
        let apply_cur2_zero =
            cal_kind == CalKind::Off && (cur2_sns_mv <= cur2_zero_mv || !cur2_curve_zero_ok);
        let cur1_sns_mv_eff = if apply_cur1_zero {
            cur1_sns_mv.saturating_sub(cur1_zero_mv)
        } else {
            cur1_sns_mv
        };
        let cur2_sns_mv_eff = if apply_cur2_zero {
            cur2_sns_mv.saturating_sub(cur2_zero_mv)
        } else {
            cur2_sns_mv
        };

        let v_rmt_sns_mv_cal = adc_to_mv(v_rmt_sns_code_cal);
        let v_nr_sns_mv_cal = adc_to_mv(v_nr_sns_code_cal);
        let cur1_sns_mv_cal = adc_to_mv(cur1_sns_code_cal);
        let cur2_sns_mv_cal = adc_to_mv(cur2_sns_code_cal);

        let v_5v_sns_mv = adc_to_mv(sns_5v_code);
        let ts1_mv = adc_to_mv(ts1_code);
        let ts2_mv = adc_to_mv(ts2_code);

        if is_telemetry_log_tick {
            info!(
                "raw_adc: vrefint={} v_rmt_sns={} v_nr_sns={} cur1_sns={} cur2_sns={} sns_5v={} ts1={} ts2={}",
                vrefint_raw,
                v_rmt_sns_code,
                v_nr_sns_code,
                cur1_sns_code,
                cur2_sns_code,
                sns_5v_code,
                ts1_code,
                ts2_code
            );
        }

        // --- Raw (ADC pin voltage) in 100 µV units ---
        let raw_v_nr_100uv = mv_to_raw_100uv(v_nr_sns_mv);
        let raw_v_rmt_100uv = mv_to_raw_100uv(v_rmt_sns_mv);

        let raw_cur1_eff_100uv = mv_to_raw_100uv(cur1_sns_mv_eff);
        let raw_cur2_eff_100uv = mv_to_raw_100uv(cur2_sns_mv_eff);

        let raw_v_nr_100uv_cal = mv_to_raw_100uv(v_nr_sns_mv_cal);
        let raw_v_rmt_100uv_cal = mv_to_raw_100uv(v_rmt_sns_mv_cal);
        let raw_cur1_100uv_cal = mv_to_raw_100uv(cur1_sns_mv_cal);
        let raw_cur2_100uv_cal = mv_to_raw_100uv(cur2_sns_mv_cal);

        // --- Ideal physical (fallback) ---
        // Voltage ideal scaling:
        //   V_SNS = (10/124) * V_load  →  V_load = (124/10) * V_SNS
        let v_local_mv_uncal = (v_nr_sns_mv * SENSE_GAIN_NUM / SENSE_GAIN_DEN) as i32;
        let v_remote_mv_uncal = (v_rmt_sns_mv * SENSE_GAIN_NUM / SENSE_GAIN_DEN) as i32;
        // Current ideal scaling:
        //   I[mA] ≈ 2 * V_CUR[mV]
        let i_ch1_ma_uncal = (2 * cur1_sns_mv_eff) as i32;
        let i_ch2_ma_uncal = (2 * cur2_sns_mv_eff) as i32;

        // --- Active-calibrated physical values ---
        let v_local_mv = if curves[CurveKind::VLocal.index()].is_empty() {
            v_local_mv_uncal
        } else {
            piecewise_linear(curves[CurveKind::VLocal.index()].as_slice(), raw_v_nr_100uv)
                .unwrap_or(v_local_mv_uncal)
        };
        let v_remote_mv = if curves[CurveKind::VRemote.index()].is_empty() {
            v_remote_mv_uncal
        } else {
            piecewise_linear(
                curves[CurveKind::VRemote.index()].as_slice(),
                raw_v_rmt_100uv,
            )
            .unwrap_or(v_remote_mv_uncal)
        };

        // 远端电压软判定：仅在电压处于合理范围且 ADC 原码未接近饱和时认为“看起来正常”。
        let remote_abs_mv = if v_remote_mv < 0 {
            -v_remote_mv
        } else {
            v_remote_mv
        };
        let remote_in_range = (REMOTE_V_MIN_MV..=REMOTE_V_MAX_MV).contains(&remote_abs_mv);

        let not_saturated = v_rmt_sns_code > ADC_SAT_MARGIN
            && v_rmt_sns_code < (ADC_FULL_SCALE as u16 - ADC_SAT_MARGIN);

        let remote_ok = remote_in_range && not_saturated;

        if is_status_tick {
            if remote_ok {
                remote_good_streak = remote_good_streak.saturating_add(1);
                remote_bad_streak = 0;
                if !remote_active && remote_good_streak >= 3 {
                    remote_active = true;
                    info!(
                        "remote sense became ACTIVE (v_remote_mv={}mV, code={})",
                        v_remote_mv, v_rmt_sns_code
                    );
                }
            } else {
                remote_bad_streak = remote_bad_streak.saturating_add(1);
                remote_good_streak = 0;
                if remote_active && remote_bad_streak >= 2 {
                    remote_active = false;
                    info!(
                        "remote sense became INACTIVE (v_remote_mv={}mV, code={})",
                        v_remote_mv, v_rmt_sns_code
                    );
                }
            }
        }

        // 模拟板 5V 轨电压：R25=75k (5V→5V_SNS)，R26=10k (5V_SNS→GND)
        //   V_5V_SNS = 5V * 10 / (75+10) = 5V * 10/85
        //   V_5V     = V_5V_SNS * (75+10)/10 = V_5V_SNS * 8.5
        let v_5v_mv = (v_5v_sns_mv * 85 / 10) as i32;

        // 电流检测链路（v4.1/v4.2 均满足相同换算关系）：
        //   - v4.1：Rsense=25 mΩ，INA193 G=20 → V_CUR ≈ 0.5 * I [V/A]
        //   - v4.2：Rsense=50 mΩ，OPA2365 G=10 → V_CUR ≈ 0.5 * I [V/A]
        //   I[mA] ≈ 2 * V_CUR[mV] 适用于两版硬件。
        //
        // 约定：
        //   - i_ch1_ma 使用 CUR1_SNS，对应功率通道 1；
        //   - i_ch2_ma 使用 CUR2_SNS，对应功率通道 2；
        //   - i_total_ma = i_ch1_ma + i_ch2_ma，用于功率估算与闭环误差计算。
        let i_ch1_ma = if curves[CurveKind::CurrentCh1.index()].is_empty() {
            i_ch1_ma_uncal
        } else {
            piecewise_linear(
                curves[CurveKind::CurrentCh1.index()].as_slice(),
                raw_cur1_eff_100uv,
            )
            .unwrap_or(i_ch1_ma_uncal)
        };
        let i_ch2_ma = if curves[CurveKind::CurrentCh2.index()].is_empty() {
            i_ch2_ma_uncal
        } else {
            piecewise_linear(
                curves[CurveKind::CurrentCh2.index()].as_slice(),
                raw_cur2_eff_100uv,
            )
            .unwrap_or(i_ch2_ma_uncal)
        };
        let i_total_ma = i_ch1_ma.saturating_add(i_ch2_ma);

        // V_main selection:
        // - Remote may participate only when `remote_active` is true.
        // - Otherwise, fall back to local only.
        let v_main_mv = if remote_active {
            v_local_mv.max(v_remote_mv)
        } else {
            v_local_mv
        };

        // Power estimate used for UI/diagnostics and CP control feedback.
        //
        // Use V_main (remote-aware) to match the CP control law (I≈P/V_main) and
        // avoid chasing a systematic error when V_remote != V_local.
        let calc_p_mw = ((i_total_ma as i64 * (v_main_mv.max(0) as i64)) / 1_000)
            .clamp(0, u32::MAX as i64) as u32;

        // 实物板确认：TS2 (R40) 靠近 MOSFET / 散热片热点；TS1 (R39) 更靠近出风口/侧壁。
        // 约定：
        // - sink_core_temp_mc 始终表示“靠 MOS 的 NTC”（CORE，TS2/R40）
        // - sink_exhaust_temp_mc 表示“靠外壳/出风口一侧的 NTC”（SINK/EXHAUST，TS1/R39）
        let sink_core_temp_mc: i32 = ntc_mv_to_mc(ts2_mv);
        let sink_exhaust_temp_mc: i32 = ntc_mv_to_mc(ts1_mv);
        let mcu_temp_mc: i32 = g4_internal_mcu_temp_to_mc(mcu_temp_code);

        // --- Fault detection ---
        let mut new_faults: u32 = 0;

        if i_ch1_ma > OC_LIMIT_CH_MA || i_ch2_ma > OC_LIMIT_CH_MA || i_total_ma > OC_LIMIT_TOTAL_MA
        {
            new_faults |= FAULT_OVERCURRENT;
        }
        if v_local_mv > OV_LIMIT_MV {
            new_faults |= FAULT_OVERVOLTAGE;
        }
        if mcu_temp_mc > MCU_TEMP_LIMIT_MC {
            new_faults |= FAULT_MCU_OVER_TEMP;
        }
        if sink_core_temp_mc > SINK_TEMP_LIMIT_MC {
            new_faults |= FAULT_SINK_OVER_TEMP;
        }

        if new_faults != 0 {
            let prev = FAULT_FLAGS.fetch_or(new_faults, Ordering::Relaxed);
            let combined = prev | new_faults;
            if combined != prev {
                warn!(
                    "protection fault latched: new=0x{:08x} combined=0x{:08x}",
                    new_faults, combined
                );
            }
        }

        let fault_flags = FAULT_FLAGS.load(Ordering::Relaxed);
        let has_fault = fault_flags != 0;

        let cal_ready = CAL_READY.load(Ordering::Relaxed);

        // Active control selection:
        // - Before first valid SetMode: legacy SetEnable + SetPoint path (CC only).
        // - After SetMode: ignore legacy SetPoint updates; use the atomic SetMode snapshot.
        let active_mode_seen = ACTIVE_MODE_SEEN.load(Ordering::Relaxed);
        let ctrl_snapshot = if active_mode_seen {
            active_control_snapshot()
        } else {
            ActiveControl::new()
        };

        // Undervoltage latch (non-fault):
        // - Trigger when output_enabled=true and V_main <= min_v.
        // - Clears ONLY on output enable rising edge (handled in SetMode RX path).
        let mut uv_latched = active_mode_seen && ctrl_snapshot.uv_latched;
        if active_mode_seen
            && ctrl_snapshot.output_enabled
            && ctrl_snapshot.min_v_mv > 0
            && v_main_mv <= ctrl_snapshot.min_v_mv
            && !uv_latched
        {
            uv_latched = true;
            active_control_set_uv_latched(true);
            warn!(
                "uv_latched set: preset_id={} v_main={}mV <= min_v={}mV",
                ctrl_snapshot.preset_id, v_main_mv, ctrl_snapshot.min_v_mv
            );
        }

        // Output gating + safety rule: effective output MUST be 0 when any of these apply:
        // - output_enabled == false
        // - CAL_READY == false
        // - any FAULT_FLAGS != 0
        // - uv_latched == true
        let enable_requested = ENABLE_REQUESTED.load(Ordering::Relaxed);
        let effective_output_enable = if active_mode_seen {
            ctrl_snapshot.output_enabled && cal_ready && !has_fault && !uv_latched
        } else {
            enable_requested && cal_ready && !has_fault
        };

        // Physically gate the TPS22810 load switch based on the effective enable state.
        if effective_output_enable {
            load_en_ctl.set_high();
            load_en_ts.set_high();
        } else {
            load_en_ctl.set_low();
            load_en_ts.set_low();
        }

        let status_mode: u8 = if active_mode_seen {
            match ctrl_snapshot.mode {
                LoadMode::Cv => FAST_STATUS_MODE_CV,
                LoadMode::Cp => FAST_STATUS_MODE_CP,
                _ => FAST_STATUS_MODE_CC,
            }
        } else {
            FAST_STATUS_MODE_CC
        };

        let mut current_limited = false;
        let mut power_limited = false;

        // Desired total current target (mA), prior to channel split.
        let desired_i_total_ma: i32 = if active_mode_seen {
            // True limiting (v1): enforce preset current + power limits.
            let current_limit_ma = ctrl_snapshot
                .max_i_ma_total
                .clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA);
            let power_limit_ma: i32 = if v_main_mv <= 0 {
                0
            } else {
                let i_by_power_ma =
                    (ctrl_snapshot.max_p_mw as i64).saturating_mul(1_000) / (v_main_mv as i64);
                i_by_power_ma.clamp(TARGET_I_MIN_MA as i64, TARGET_I_MAX_MA as i64) as i32
            };

            let mut cp_large_step: bool = false;
            let mut desired_i_total_ma: i32 = match ctrl_snapshot.mode {
                LoadMode::Cv => {
                    // CV outer loop: integrate conductance (G) based on smoothed voltage error.
                    // This runs at CONTROL_PERIOD_US, while the legacy tuning constants are
                    // defined at FAST_STATUS_PERIOD_US cadence; scale the update accordingly.
                    if !effective_output_enable {
                        cv_g_uapermv_fp = 0;
                        cv_v_filt_init = false;
                        cv_v_main_filt_mv = 0;
                        cp_v_filt_init = false;
                        cp_v_main_filt_mv = 0;
                        cp_i_bias_ma = 0;
                        cp_last_target_p_mw = 0;
                    } else {
                        if !cv_v_filt_init {
                            cv_v_main_filt_mv = v_main_mv;
                            cv_v_filt_init = true;
                        } else {
                            cv_v_main_filt_mv += (v_main_mv - cv_v_main_filt_mv) / CV_V_FILT_DIV;
                        }

                        let err_mv = cv_v_main_filt_mv - ctrl_snapshot.target_v_mv;
                        if err_mv.abs() > CV_ERR_DEADBAND_MV {
                            // Voltage error (mV) -> conductance step (uA/mV), fixed-point Q8.
                            let denom = (CV_G_ERR_DIV_MV as i64) * (FAST_STATUS_PERIOD_US as i64);
                            let mut step_fp = (err_mv as i64)
                                .saturating_mul(CV_G_FP_SCALE as i64)
                                .saturating_mul(CONTROL_PERIOD_US as i64)
                                / denom;
                            step_fp = step_fp
                                .clamp(-(CV_G_STEP_DN_MAX_FP as i64), CV_G_STEP_UP_MAX_FP as i64);
                            cv_g_uapermv_fp =
                                (cv_g_uapermv_fp + step_fp as i32).clamp(0, CV_G_MAX_FP);
                        }
                    }

                    let v_cv_mv = if cv_v_filt_init {
                        cv_v_main_filt_mv
                    } else {
                        v_main_mv
                    };
                    if v_cv_mv <= 0 {
                        0
                    } else {
                        let i_ma = (cv_g_uapermv_fp as i64).saturating_mul(v_cv_mv as i64)
                            / ((1_000 * CV_G_FP_SCALE) as i64);
                        i_ma.clamp(TARGET_I_MIN_MA as i64, TARGET_I_MAX_MA as i64) as i32
                    }
                }
                LoadMode::Cp => {
                    // CP control: I_target ≈ P_target / V_meas.
                    // Use a filtered V_main to reduce noise-induced current dithering.
                    cv_g_uapermv_fp = 0;
                    cv_v_filt_init = false;
                    cv_v_main_filt_mv = 0;

                    if !effective_output_enable {
                        cp_v_filt_init = false;
                        cp_v_main_filt_mv = 0;
                        cp_i_bias_ma = 0;
                        cp_last_target_p_mw = 0;
                        cp_pterm_neg_freeze_ticks = 0;
                        cp_pterm_pos_freeze_ticks = 0;
                        0
                    } else {
                        let prev_target_p_mw = cp_last_target_p_mw;
                        let new_target_p_mw = ctrl_snapshot.target_p_mw;
                        let delta_p_mw = if prev_target_p_mw == 0 {
                            new_target_p_mw
                        } else if new_target_p_mw >= prev_target_p_mw {
                            new_target_p_mw - prev_target_p_mw
                        } else {
                            prev_target_p_mw - new_target_p_mw
                        };

                        // CP performance capture arm (best-effort internal self-test):
                        // - arm on large CP target steps
                        // - use the control tick timestamp + current power as p0 to avoid RX/control alignment jitter
                        if prev_target_p_mw != 0
                            && new_target_p_mw > 0
                            && new_target_p_mw != prev_target_p_mw
                            && delta_p_mw >= CP_STEP_BOOST_DETECT_MW
                        {
                            let arm_ms = timestamp_ms() as u32;
                            CP_PERF_ARM_MS.store(arm_ms, Ordering::Relaxed);
                            CP_PERF_ARM_TARGET_P_MW.store(new_target_p_mw, Ordering::Relaxed);
                            CP_PERF_ARM_P0_MW.store(calc_p_mw, Ordering::Relaxed);
                            CP_PERF_ARM_SEQ.fetch_add(1, Ordering::Release);
                            info!(
                                "cp_perf armed (control tick): target={}mW (prev={}mW) p0={}mW at_ms={}",
                                new_target_p_mw, prev_target_p_mw, calc_p_mw, arm_ms
                            );
                        }

                        // Large setpoint steps are handled without the slew limiter (see below),
                        // to keep the CP loop responsive while still damping small dithers.
                        cp_large_step = delta_p_mw >= CP_STEP_BOOST_DETECT_MW;
                        let cp_downstep =
                            prev_target_p_mw != 0 && new_target_p_mw < prev_target_p_mw;
                        if cp_large_step && cp_downstep {
                            // Freeze for ~20ms (scaled to control tick rate).
                            cp_pterm_neg_freeze_ticks = CP_PTERM_NEG_FREEZE_TICKS;
                            cp_pterm_pos_freeze_ticks = 0;
                        } else if prev_target_p_mw != 0 && new_target_p_mw > prev_target_p_mw {
                            // Clear the freeze on large up-steps.
                            cp_pterm_neg_freeze_ticks = 0;
                            // Freeze the positive P-term briefly on large up-steps to reduce overshoot.
                            if cp_large_step {
                                cp_pterm_pos_freeze_ticks = CP_PTERM_POS_FREEZE_TICKS;
                            }
                        }

                        // The bias term is intended for steady-state trim; clear it on large setpoint
                        // steps to avoid slow unwinding on falling edges.
                        if prev_target_p_mw != 0 {
                            if delta_p_mw >= CP_I_BIAS_RESET_STEP_MW {
                                cp_i_bias_ma = 0;
                            }
                        }
                        cp_last_target_p_mw = new_target_p_mw;

                        if !cp_v_filt_init {
                            cp_v_main_filt_mv = v_main_mv;
                            cp_v_filt_init = true;
                            cp_i_bias_ma = 0;
                        } else {
                            let dv = v_main_mv - cp_v_main_filt_mv;
                            if dv.abs() >= CP_V_STEP_RESET_MV {
                                cp_v_main_filt_mv = v_main_mv;
                            } else {
                                cp_v_main_filt_mv += dv / CP_V_FILT_DIV;
                            }
                        }

                        let v_cp_mv = if cp_v_filt_init {
                            cp_v_main_filt_mv
                        } else {
                            v_main_mv
                        };
                        if v_cp_mv <= 0 {
                            0
                        } else {
                            // Power error (used for P-term + bias + transient helpers).
                            let p_err_mw =
                                (ctrl_snapshot.target_p_mw as i64).saturating_sub(calc_p_mw as i64);

                            // Feed-forward current: I≈P/V (rounded to nearest mA to avoid
                            // a systematic floor bias at low power where tolerance is tight).
                            let mut i_ff_ma = (ctrl_snapshot.target_p_mw as i64)
                                .saturating_mul(1_000)
                                .saturating_add((v_cp_mv as i64) / 2)
                                / (v_cp_mv as i64);
                            if cp_pterm_neg_freeze_ticks > 0 && p_err_mw > 0 {
                                i_ff_ma = i_ff_ma.saturating_add(CP_DOWNSTEP_I_BOOST_MA as i64);
                            }

                            // Power-error derived current correction (P + I trim):
                            // i_err ~= (P_target - P_meas) / V.
                            let i_err_ma = (p_err_mw.saturating_mul(1_000) / (v_cp_mv as i64))
                                .clamp(-(TARGET_I_MAX_MA as i64), TARGET_I_MAX_MA as i64);
                            let i_err_clamped =
                                (i_err_ma as i32).clamp(-CP_I_P_STEP_MAX_MA, CP_I_P_STEP_MAX_MA);
                            // Feed-forward I≈P/V is the primary path. After large down-steps, keep
                            // the P-term from going negative for a short window to avoid pulling the
                            // command below feed-forward while the plant is still settling.
                            let i_p_ma = if cp_pterm_neg_freeze_ticks > 0 && i_err_clamped < 0 {
                                0
                            } else if cp_pterm_pos_freeze_ticks > 0 && i_err_clamped > 0 {
                                0
                            } else {
                                let p_div = if ctrl_snapshot.target_p_mw <= CP_FS_L_MW {
                                    CP_I_P_ERR_DIV_LOW_POWER
                                } else {
                                    CP_I_P_ERR_DIV
                                };
                                i_err_clamped / p_div.max(1)
                            };

                            // Conservative integral trim: only integrate when we are not saturating
                            // against limits (anti-windup).
                            let i_pre = (i_ff_ma as i32)
                                .saturating_add(i_p_ma)
                                .saturating_add(cp_i_bias_ma)
                                .clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA);
                            let pred_current_limited =
                                effective_output_enable && i_pre > current_limit_ma;
                            let pred_power_limited =
                                effective_output_enable && i_pre > power_limit_ma;
                            let pred_low_saturated =
                                effective_output_enable && i_pre <= TARGET_I_MIN_MA;
                            let err_ma = i_err_ma as i32;
                            let sat_increasing =
                                (pred_current_limited || pred_power_limited) && err_ma > 0;
                            let sat_decreasing = pred_low_saturated && err_ma < 0;
                            if !(sat_increasing || sat_decreasing) {
                                let step = (i_err_ma as i32)
                                    .clamp(-CP_I_BIAS_STEP_MAX_MA, CP_I_BIAS_STEP_MAX_MA);
                                // During the post-downstep settling window, avoid integrating
                                // negative bias (which would pull the command below feed-forward
                                // while the plant is still falling).
                                if !(cp_pterm_neg_freeze_ticks > 0 && step < 0)
                                    && !(cp_pterm_pos_freeze_ticks > 0 && step > 0)
                                {
                                    cp_i_bias_ma = cp_i_bias_ma
                                        .saturating_add(step / CP_I_BIAS_ERR_DIV)
                                        .clamp(-TARGET_I_MAX_MA, TARGET_I_MAX_MA);
                                }
                            }

                            let i_ma = (i_ff_ma as i32)
                                .saturating_add(i_p_ma)
                                .saturating_add(cp_i_bias_ma);
                            if effective_output_enable && cp_pterm_neg_freeze_ticks > 0 {
                                cp_pterm_neg_freeze_ticks =
                                    cp_pterm_neg_freeze_ticks.saturating_sub(1);
                            }
                            if effective_output_enable && cp_pterm_pos_freeze_ticks > 0 {
                                cp_pterm_pos_freeze_ticks =
                                    cp_pterm_pos_freeze_ticks.saturating_sub(1);
                            }
                            if is_telemetry_log_tick {
                                info!(
                                    "cp_ctl: p_tgt={}mW p_meas={}mW v_main={}mV v_cp={}mV i_ff={}mA i_err={}mA i_p={}mA bias={}mA i_pre={}mA lim_i={}mA lim_p_i={}mA",
                                    ctrl_snapshot.target_p_mw,
                                    calc_p_mw,
                                    v_main_mv,
                                    v_cp_mv,
                                    i_ff_ma,
                                    i_err_ma,
                                    i_p_ma,
                                    cp_i_bias_ma,
                                    i_pre,
                                    current_limit_ma,
                                    power_limit_ma,
                                );
                            }
                            (i_ma as i64).clamp(TARGET_I_MIN_MA as i64, TARGET_I_MAX_MA as i64)
                                as i32
                        }
                    }
                }
                _ => {
                    cv_g_uapermv_fp = 0;
                    cv_v_filt_init = false;
                    cv_v_main_filt_mv = 0;
                    cp_v_filt_init = false;
                    cp_v_main_filt_mv = 0;
                    cp_i_bias_ma = 0;
                    cp_last_target_p_mw = 0;
                    ctrl_snapshot.target_i_ma
                }
            };

            let desired_clamped = desired_i_total_ma.clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA);
            current_limited = effective_output_enable && desired_clamped > current_limit_ma;
            power_limited = effective_output_enable && desired_clamped > power_limit_ma;

            desired_i_total_ma = desired_clamped
                .min(current_limit_ma)
                .min(power_limit_ma)
                .clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA);

            if ctrl_snapshot.mode == LoadMode::Cp && effective_output_enable {
                if cp_large_step {
                    // For large setpoint steps, bypass the slew limiter so the transient
                    // response is dominated by the analog plant, not by firmware ramping.
                    cp_i_cmd_slewed_ma = desired_i_total_ma;
                } else {
                    let delta = desired_i_total_ma.saturating_sub(cp_i_cmd_slewed_ma);
                    let step = if delta >= 0 {
                        delta.min(CP_I_SLEW_MAX_UP_MA_PER_TICK)
                    } else {
                        delta.max(-CP_I_SLEW_MAX_DN_MA_PER_TICK)
                    };
                    cp_i_cmd_slewed_ma = cp_i_cmd_slewed_ma.saturating_add(step);
                }
                desired_i_total_ma = cp_i_cmd_slewed_ma;
            } else {
                cp_i_cmd_slewed_ma = 0;
            }

            // Anti-windup: only apply when we're saturating on a limit.
            if ctrl_snapshot.mode == LoadMode::Cv && (current_limited || power_limited) {
                let v_cv_mv = if cv_v_filt_init {
                    cv_v_main_filt_mv
                } else {
                    v_main_mv
                };
                if v_cv_mv <= 0 || desired_i_total_ma <= 0 {
                    cv_g_uapermv_fp = 0;
                } else {
                    // desired_i_total_ma is mA; convert back to uA/mV (Q8).
                    let g_fp = (desired_i_total_ma as i64)
                        .saturating_mul(1_000)
                        .saturating_mul(CV_G_FP_SCALE as i64)
                        / (v_cv_mv as i64);
                    cv_g_uapermv_fp = (g_fp as i32).clamp(0, CV_G_MAX_FP);
                }
            }
            desired_i_total_ma
        } else {
            // Legacy CC-only path: SetEnable + SetPoint with LimitProfile clamp.
            let mut desired_i_total_ma = if effective_output_enable {
                TARGET_I_LOCAL_MA.load(Ordering::Relaxed)
            } else {
                0
            }
            .clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA);

            // Apply legacy LimitProfile current derate; keep v0 power limit as log-only.
            {
                let limits = LIMIT_PROFILE.lock().await;
                let derate_pct = limits.thermal_derate_pct.min(100);
                let mut derated_max_i = limits.max_i_ma.saturating_mul(derate_pct as i32) / 100;
                derated_max_i = derated_max_i.clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA);

                if desired_i_total_ma > derated_max_i {
                    desired_i_total_ma = derated_max_i;
                }

                if calc_p_mw > limits.max_p_mw {
                    warn!(
                        "soft power limit exceeded: calc_p={}mW max_p={}mW (no action in v0)",
                        calc_p_mw, limits.max_p_mw
                    );
                }
            }
            desired_i_total_ma
        };

        // Effective total current target after all gating/limits.
        let target_i_total_ma = if effective_output_enable {
            desired_i_total_ma
        } else {
            0
        };

        // CP performance capture:
        // - main loop publishes "latest" values for the 1ms sampler task
        // - printing/analysis happens here once capture finishes (to keep heavy work off the 1ms task)
        let flags = (if current_limited { 1 } else { 0 }) | (if power_limited { 2 } else { 0 });
        // NOTE: we publish the latest signals after the DAC update below, so that the
        // captured samples include the actual command + sense values for that tick.

        // 按总目标电流拆分两路通道：
        //
        // - I_total < I_SHARE_THRESHOLD_MA：仅 CH1 承担全部电流，CH2=0；
        // - I_total ≥ I_SHARE_THRESHOLD_MA：CH1/CH2 近似均分（奇数 mA 由 CH1 多承担 1 mA）。
        let (mut target_ch1_ma, mut target_ch2_ma) = if !effective_output_enable {
            (0, 0)
        } else {
            match cal_kind {
                CalKind::CurrentCh1 => (target_i_total_ma, 0),
                CalKind::CurrentCh2 => (0, target_i_total_ma),
                _ => {
                    if target_i_total_ma < I_SHARE_THRESHOLD_MA {
                        (target_i_total_ma, 0)
                    } else {
                        let half = target_i_total_ma / 2;
                        let rem = target_i_total_ma - 2 * half;
                        (half + rem, half)
                    }
                }
            }
        };
        // Enforce per-channel clamp after split.
        target_ch1_ma = target_ch1_ma.clamp(0, TARGET_I_CH_MAX_MA);
        target_ch2_ma = target_ch2_ma.clamp(0, TARGET_I_CH_MAX_MA);

        // Inverse mapping: physical target → raw 100 µV target.
        let ideal_raw_ch1_des_100uv =
            target_ch1_ma.saturating_mul(5).clamp(0, i16::MAX as i32) as i16;
        let ideal_raw_ch2_des_100uv =
            target_ch2_ma.saturating_mul(5).clamp(0, i16::MAX as i32) as i16;

        let raw_ch1_des_100uv = if cal_kind == CalKind::CurrentCh2 {
            0
        } else if curves[CurveKind::CurrentCh1.index()].is_empty() {
            ideal_raw_ch1_des_100uv
        } else {
            inverse_piecewise(
                curves[CurveKind::CurrentCh1.index()].as_slice(),
                target_ch1_ma,
            )
            .unwrap_or(ideal_raw_ch1_des_100uv)
        };
        let raw_ch2_des_100uv = if cal_kind == CalKind::CurrentCh1 {
            0
        } else if curves[CurveKind::CurrentCh2.index()].is_empty() {
            ideal_raw_ch2_des_100uv
        } else {
            inverse_piecewise(
                curves[CurveKind::CurrentCh2.index()].as_slice(),
                target_ch2_ma,
            )
            .unwrap_or(ideal_raw_ch2_des_100uv)
        };

        // Raw 100 µV target → DAC code.
        //
        // Prefer user calibration (raw_100uv ↔ raw_dac_code captured during current calibration).
        // Fallback to reference-voltage scaling when DAC samples are not present (e.g. factory defaults).
        let dac_code_ch1 = if curves[CurveKind::CurrentCh1.index()].is_empty() {
            raw_100uv_to_dac_code_vref(raw_ch1_des_100uv, vref_mv)
        } else {
            raw_100uv_to_dac_code_calibrated(
                curves[CurveKind::CurrentCh1.index()].as_slice(),
                raw_ch1_des_100uv,
            )
            .unwrap_or_else(|_| raw_100uv_to_dac_code_vref(raw_ch1_des_100uv, vref_mv))
        };
        let dac_code_ch2 = if curves[CurveKind::CurrentCh2.index()].is_empty() {
            raw_100uv_to_dac_code_vref(raw_ch2_des_100uv, vref_mv)
        } else {
            raw_100uv_to_dac_code_calibrated(
                curves[CurveKind::CurrentCh2.index()].as_slice(),
                raw_ch2_des_100uv,
            )
            .unwrap_or_else(|_| raw_100uv_to_dac_code_vref(raw_ch2_des_100uv, vref_mv))
        };
        dac.ch1().set(DacValue::Bit12Right(dac_code_ch1));
        dac.ch2().set(DacValue::Bit12Right(dac_code_ch2));

        // Current-sense zero tracking while output is disabled (safe baseline refresh).
        //
        // When the load switch is open, the actual sink current should be ~0, so any
        // non-zero CUR*_SNS voltage is a measurement offset. Track it slowly so that
        // post-step recovery/drift does not poison the next CP session.
        if cal_kind == CalKind::Off && !effective_output_enable {
            if is_ms_tick {
                cur_zero_disabled_ticks = cur_zero_disabled_ticks.saturating_add(1);
                // Require a short settle time (~20ms) before adapting.
                if cur_zero_disabled_ticks >= 20 {
                    update_zero_mv_iir(&mut cur1_zero_mv, cur1_sns_mv, 4);
                    update_zero_mv_iir(&mut cur2_zero_mv, cur2_sns_mv, 4);
                }
            }
            // Do not concurrently run the "near-zero command" learner.
            cur_zero_cmd_ticks = 0;
        } else {
            cur_zero_disabled_ticks = 0;
        }

        // Current-sense zero tracking (guarded).
        //
        // Under some conditions (notably after large current steps), CUR*_SNS may
        // exhibit an offset that makes the measured current appear non-zero even
        // when the DAC command is ~0. This can break CP convergence at low power
        // (10W range) because the controller believes it is still over target.
        //
        // We adapt the zero baseline only when we're *confident* the command is
        // "near zero" and the sense readings have been stable for a few ms.
        // This avoids accidentally learning a decaying real current as "offset".
        if cal_kind == CalKind::Off
            && effective_output_enable
            && target_i_total_ma <= 20
            && dac_code_ch1 <= 2
            && dac_code_ch2 <= 2
        {
            if is_ms_tick {
                cur_zero_cmd_ticks = cur_zero_cmd_ticks.saturating_add(1);

                // Require ~10ms of "near-zero command", and only start learning once
                // the measured sense voltages are already in the "small" region to
                // reduce the risk of learning a decaying real current.
                if cur_zero_cmd_ticks >= 10 && cur1_sns_mv <= 400 && cur2_sns_mv <= 400 {
                    update_zero_mv_iir(&mut cur1_zero_mv, cur1_sns_mv, 4);
                    update_zero_mv_iir(&mut cur2_zero_mv, cur2_sns_mv, 4);
                }
            }
        } else {
            cur_zero_cmd_ticks = 0;
        }

        // CP performance capture:
        // - publish "latest" values for the 1ms sampler task (atomic, no awaits)
        // - print/analysis happens here once capture finishes
        CP_PERF_LATEST_SEQ.fetch_add(1, Ordering::Release);
        CP_PERF_LATEST_P_MW.store(calc_p_mw, Ordering::Relaxed);
        CP_PERF_LATEST_V_MAIN_MV.store(v_main_mv, Ordering::Relaxed);
        CP_PERF_LATEST_V_LOCAL_MV.store(v_local_mv, Ordering::Relaxed);
        CP_PERF_LATEST_I_TOTAL_MA.store(i_total_ma, Ordering::Relaxed);
        CP_PERF_LATEST_TARGET_I_MA.store(target_i_total_ma, Ordering::Relaxed);
        CP_PERF_LATEST_DAC1_CODE.store(dac_code_ch1 as u32, Ordering::Relaxed);
        CP_PERF_LATEST_DAC2_CODE.store(dac_code_ch2 as u32, Ordering::Relaxed);
        CP_PERF_LATEST_CUR1_SNS_MV.store(cur1_sns_mv, Ordering::Relaxed);
        CP_PERF_LATEST_CUR2_SNS_MV.store(cur2_sns_mv, Ordering::Relaxed);
        CP_PERF_LATEST_CUR1_SNS_MV_EFF.store(cur1_sns_mv_eff, Ordering::Relaxed);
        CP_PERF_LATEST_CUR2_SNS_MV_EFF.store(cur2_sns_mv_eff, Ordering::Relaxed);
        CP_PERF_LATEST_CUR1_ZERO_MV.store(cur1_zero_mv, Ordering::Relaxed);
        CP_PERF_LATEST_CUR2_ZERO_MV.store(cur2_zero_mv, Ordering::Relaxed);
        CP_PERF_LATEST_EFFECTIVE_ENABLE.store(
            if effective_output_enable { 1 } else { 0 },
            Ordering::Relaxed,
        );
        CP_PERF_LATEST_FLAGS.store(flags, Ordering::Relaxed);
        CP_PERF_LATEST_SEQ.fetch_add(1, Ordering::Release);

        if CP_PERF_DONE.swap(false, Ordering::AcqRel) {
            let len = (CP_PERF_LEN.load(Ordering::Acquire) as usize).min(CP_PERF_SAMPLES);
            let samples: &[CpPerfSample] = unsafe { &CP_PERF_BUF[..len] };

            let target_p_mw = CP_PERF_TARGET_P_MW.load(Ordering::Relaxed);
            let p0_mw = CP_PERF_P0_MW.load(Ordering::Relaxed);
            let tol_mw = cp_tol_mw(target_p_mw);
            let t_enter_1 = cp_find_first_consecutive_within_tol(samples, target_p_mw, 1);
            let t_enter_3 = cp_find_first_consecutive_within_tol(
                samples,
                target_p_mw,
                CP_PERF_WINDOW_CONSECUTIVE,
            );
            let t_enter_1_sm = cp_find_first_consecutive_within_tol_smoothed(
                samples,
                target_p_mw,
                1,
                CP_PERF_SMOOTH_WINDOW_SAMPLES,
            );
            let t_enter_3_sm = cp_find_first_consecutive_within_tol_smoothed(
                samples,
                target_p_mw,
                CP_PERF_WINDOW_CONSECUTIVE,
                CP_PERF_SMOOTH_WINDOW_SAMPLES,
            );
            let t10t90 = cp_find_t10_t90_ms(samples, p0_mw, target_p_mw);

            let mut any_current_limited = false;
            let mut any_power_limited = false;
            let mut min_p_mw: u32 = u32::MAX;
            let mut max_p_mw: u32 = 0;
            let mut min_v_main_mv: i32 = i32::MAX;
            let mut max_v_main_mv: i32 = i32::MIN;
            let mut min_i_total_ma: i32 = i32::MAX;
            let mut max_i_total_ma: i32 = i32::MIN;
            let mut min_tgt_i_ma: i32 = i32::MAX;
            let mut max_tgt_i_ma: i32 = i32::MIN;
            for s in samples {
                any_current_limited |= (s.flags & 1) != 0;
                any_power_limited |= (s.flags & 2) != 0;
                min_p_mw = min_p_mw.min(s.calc_p_mw);
                max_p_mw = max_p_mw.max(s.calc_p_mw);
                min_v_main_mv = min_v_main_mv.min(s.v_main_mv);
                max_v_main_mv = max_v_main_mv.max(s.v_main_mv);
                min_i_total_ma = min_i_total_ma.min(s.i_total_ma);
                max_i_total_ma = max_i_total_ma.max(s.i_total_ma);
                min_tgt_i_ma = min_tgt_i_ma.min(s.target_i_total_ma);
                max_tgt_i_ma = max_tgt_i_ma.max(s.target_i_total_ma);
            }
            let last = samples.last().unwrap_or(&samples[0]);

            info!(
                "cp_perf: target={}mW tol={}mW p0={}mW samples={} window={} tick={}ms",
                target_p_mw, tol_mw, p0_mw, len, CP_PERF_WINDOW_CONSECUTIVE, CP_PERF_PERIOD_MS,
            );
            info!(
                "cp_perf: range p=[{}..{}]mW v_main=[{}..{}]mV i_total=[{}..{}]mA tgt_i=[{}..{}]mA last_p={}mW last_i={}mA last_tgt_i={}mA",
                min_p_mw,
                max_p_mw,
                min_v_main_mv,
                max_v_main_mv,
                min_i_total_ma,
                max_i_total_ma,
                min_tgt_i_ma,
                max_tgt_i_ma,
                last.calc_p_mw,
                last.i_total_ma,
                last.target_i_total_ma
            );
            let last_en = CP_PERF_LATEST_EFFECTIVE_ENABLE.load(Ordering::Relaxed);
            let last_v_local = CP_PERF_LATEST_V_LOCAL_MV.load(Ordering::Relaxed);
            let last_i_total = CP_PERF_LATEST_I_TOTAL_MA.load(Ordering::Relaxed);
            let last_dac1 = CP_PERF_LATEST_DAC1_CODE.load(Ordering::Relaxed);
            let last_dac2 = CP_PERF_LATEST_DAC2_CODE.load(Ordering::Relaxed);
            let last_cur1 = CP_PERF_LATEST_CUR1_SNS_MV.load(Ordering::Relaxed);
            let last_cur2 = CP_PERF_LATEST_CUR2_SNS_MV.load(Ordering::Relaxed);
            let last_cur1_eff = CP_PERF_LATEST_CUR1_SNS_MV_EFF.load(Ordering::Relaxed);
            let last_cur2_eff = CP_PERF_LATEST_CUR2_SNS_MV_EFF.load(Ordering::Relaxed);
            let last_zero1 = CP_PERF_LATEST_CUR1_ZERO_MV.load(Ordering::Relaxed);
            let last_zero2 = CP_PERF_LATEST_CUR2_ZERO_MV.load(Ordering::Relaxed);
            info!(
                "cp_perf: last en={} v_main={}mV v_local={}mV i_total={}mA target_i={}mA dac1={} dac2={} cur1={}mV(cur_eff={}mV z={}mV) cur2={}mV(cur_eff={}mV z={}mV) lim(cur={},p={})",
                last_en,
                last.v_main_mv,
                last_v_local,
                last_i_total,
                last.target_i_total_ma,
                last_dac1,
                last_dac2,
                last_cur1,
                last_cur1_eff,
                last_zero1,
                last_cur2,
                last_cur2_eff,
                last_zero2,
                any_current_limited,
                any_power_limited,
            );
            if last_en == 0 && last_i_total > 200 {
                warn!(
                    "cp_perf: suspicious non-zero current while output disabled (i_total={}mA dac1={} dac2={})",
                    last_i_total, last_dac1, last_dac2
                );
            }

            match t_enter_1 {
                Some(t) => info!("cp_perf: enter_tol(1)={}ms", t),
                None => warn!("cp_perf: enter_tol(1)=n/a"),
            }
            match t_enter_3 {
                Some(t) => info!("cp_perf: enter_tol({})={}ms", CP_PERF_WINDOW_CONSECUTIVE, t),
                None => warn!("cp_perf: enter_tol({})=n/a", CP_PERF_WINDOW_CONSECUTIVE),
            }
            match t_enter_1_sm {
                Some(t) => info!(
                    "cp_perf: enter_tol_smoothed(1)={}ms window={}ms",
                    t, CP_PERF_SMOOTH_WINDOW_SAMPLES
                ),
                None => warn!("cp_perf: enter_tol_smoothed(1)=n/a"),
            }
            match t_enter_3_sm {
                Some(t) => info!(
                    "cp_perf: enter_tol_smoothed({})={}ms window={}ms",
                    CP_PERF_WINDOW_CONSECUTIVE, t, CP_PERF_SMOOTH_WINDOW_SAMPLES
                ),
                None => warn!(
                    "cp_perf: enter_tol_smoothed({})=n/a",
                    CP_PERF_WINDOW_CONSECUTIVE
                ),
            }
            if let Some((t10, t90)) = t10t90 {
                if target_p_mw >= p0_mw {
                    info!(
                        "cp_perf: t10={}ms t90={}ms t10_90={}ms",
                        t10,
                        t90,
                        t90 - t10
                    );
                } else {
                    info!(
                        "cp_perf: t90={}ms t10={}ms t90_10={}ms",
                        t10,
                        t90,
                        t90 - t10
                    );
                }
            } else {
                warn!("cp_perf: t10/t90=n/a");
            }

            if let Some(t) = t_enter_1 {
                if t <= 5 {
                    info!("cp_perf: quick_check pass (enter_tol(1)<=5ms)");
                } else {
                    warn!("cp_perf: quick_check fail (enter_tol(1)={}ms > 5ms)", t);
                }
            } else {
                warn!("cp_perf: quick_check n/a (enter_tol(1))");
            }
            if let Some(t) = t_enter_3 {
                if t <= 5 {
                    info!(
                        "cp_perf: quick_check pass (enter_tol({})<=5ms)",
                        CP_PERF_WINDOW_CONSECUTIVE
                    );
                } else {
                    warn!(
                        "cp_perf: quick_check fail (enter_tol({})={}ms > 5ms)",
                        CP_PERF_WINDOW_CONSECUTIVE, t
                    );
                }
            } else {
                warn!(
                    "cp_perf: quick_check n/a (enter_tol({}))",
                    CP_PERF_WINDOW_CONSECUTIVE
                );
            }
        }

        if is_status_tick {
            // DAC 头间裕度：VREF - max(V_DAC1, V_DAC2)（便于检查任一通道是否接近打满）。
            let dac_v1_mv = (dac_code_ch1 as u32) * vref_mv / ADC_FULL_SCALE;
            let dac_v2_mv = (dac_code_ch2 as u32) * vref_mv / ADC_FULL_SCALE;
            let dac_v_max_mv = dac_v1_mv.max(dac_v2_mv);
            let dac_headroom_mv = (vref_mv.saturating_sub(dac_v_max_mv)) as u16;

            // loop_error semantics:
            // - CC: current error (mA) = I_target_total - I_measured_total
            // - CV: voltage error (mV) = V_main - V_target
            // - CP: power error (mW) = P_calc - P_target
            let loop_error = if status_mode == FAST_STATUS_MODE_CV {
                v_main_mv - ctrl_snapshot.target_v_mv
            } else if status_mode == FAST_STATUS_MODE_CP {
                (calc_p_mw as i32).saturating_sub(ctrl_snapshot.target_p_mw as i32)
            } else {
                target_i_total_ma - i_total_ma
            };

            // Calibration-only smoothing for UI + capture: smooth the RAW fields that the
            // web UI captures, then derive a smoothed view of the displayed physical values.
            //
            // IMPORTANT: Protection/fault detection above uses the instantaneous values
            // (`v_local_mv`, `i_ch1_ma`, ...) and MUST remain unaffected by smoothing.
            let (raw_v_nr_100uv_sm, raw_v_rmt_100uv_sm, raw_cur1_100uv_sm, raw_cur2_100uv_sm) =
                cal_smoother.update(
                    cal_kind,
                    raw_v_nr_100uv_cal,
                    raw_v_rmt_100uv_cal,
                    raw_cur1_100uv_cal,
                    raw_cur2_100uv_cal,
                );

            let (status_v_local_mv, status_v_remote_mv, status_i_ch1_ma, status_i_ch2_ma) =
                if cal_kind == CalKind::Off {
                    (v_local_mv, v_remote_mv, i_ch1_ma, i_ch2_ma)
                } else {
                    let v_nr_sns_mv_sm = raw_100uv_to_mv(raw_v_nr_100uv_sm);
                    let v_rmt_sns_mv_sm = raw_100uv_to_mv(raw_v_rmt_100uv_sm);
                    let cur1_sns_mv_sm = raw_100uv_to_mv(raw_cur1_100uv_sm);
                    let cur2_sns_mv_sm = raw_100uv_to_mv(raw_cur2_100uv_sm);

                    let v_local_mv_uncal_sm =
                        (v_nr_sns_mv_sm * SENSE_GAIN_NUM / SENSE_GAIN_DEN) as i32;
                    let v_remote_mv_uncal_sm =
                        (v_rmt_sns_mv_sm * SENSE_GAIN_NUM / SENSE_GAIN_DEN) as i32;

                    let i_ch1_ma_uncal_sm = (2 * cur1_sns_mv_sm) as i32;
                    let i_ch2_ma_uncal_sm = (2 * cur2_sns_mv_sm) as i32;

                    let v_local_sm = if curves[CurveKind::VLocal.index()].is_empty() {
                        v_local_mv_uncal_sm
                    } else {
                        piecewise_linear(
                            curves[CurveKind::VLocal.index()].as_slice(),
                            raw_v_nr_100uv_sm,
                        )
                        .unwrap_or(v_local_mv_uncal_sm)
                    };
                    let v_remote_sm = if curves[CurveKind::VRemote.index()].is_empty() {
                        v_remote_mv_uncal_sm
                    } else {
                        piecewise_linear(
                            curves[CurveKind::VRemote.index()].as_slice(),
                            raw_v_rmt_100uv_sm,
                        )
                        .unwrap_or(v_remote_mv_uncal_sm)
                    };

                    let i_ch1_sm = if curves[CurveKind::CurrentCh1.index()].is_empty() {
                        i_ch1_ma_uncal_sm
                    } else {
                        piecewise_linear(
                            curves[CurveKind::CurrentCh1.index()].as_slice(),
                            raw_cur1_100uv_sm,
                        )
                        .unwrap_or(i_ch1_ma_uncal_sm)
                    };
                    let i_ch2_sm = if curves[CurveKind::CurrentCh2.index()].is_empty() {
                        i_ch2_ma_uncal_sm
                    } else {
                        piecewise_linear(
                            curves[CurveKind::CurrentCh2.index()].as_slice(),
                            raw_cur2_100uv_sm,
                        )
                        .unwrap_or(i_ch2_ma_uncal_sm)
                    };

                    (v_local_sm, v_remote_sm, i_ch1_sm, i_ch2_sm)
                };
            let status_i_total_ma = status_i_ch1_ma.saturating_add(status_i_ch2_ma);
            let status_v_main_mv = if remote_active {
                status_v_local_mv.max(status_v_remote_mv)
            } else {
                status_v_local_mv
            };
            let status_calc_p_mw = ((status_i_total_ma as i64 * (status_v_main_mv.max(0) as i64))
                / 1_000)
                .clamp(0, u32::MAX as i64) as u32;
            let status_loop_error = if status_mode == FAST_STATUS_MODE_CV {
                status_v_main_mv - ctrl_snapshot.target_v_mv
            } else if status_mode == FAST_STATUS_MODE_CP {
                (status_calc_p_mw as i32).saturating_sub(ctrl_snapshot.target_p_mw as i32)
            } else {
                target_i_total_ma - status_i_total_ma
            };

            if is_telemetry_log_tick {
                info!(
                    "sense: v_loc={}mV v_rmt={}mV v_5v={}mV i_ch1={}mA i_ch2={}mA i_total={}mA target_total={}mA ch1_target={}mA ch2_target={}mA dac1={} dac2={} loop_err={}",
                    v_local_mv,
                    v_remote_mv,
                    v_5v_mv,
                    i_ch1_ma,
                    i_ch2_ma,
                    i_total_ma,
                    target_i_total_ma,
                    target_ch1_ma,
                    target_ch2_ma,
                    dac_code_ch1,
                    dac_code_ch2,
                    loop_error
                );
            }

            // 将物理量打包为 FastStatus 帧，由数字板 UI 展示。
            let mut state_flags = 0u32;
            if remote_active {
                state_flags |= STATE_FLAG_REMOTE_ACTIVE;
            }
            if !link_fault {
                state_flags |= STATE_FLAG_LINK_GOOD;
            }
            if effective_output_enable {
                state_flags |= STATE_FLAG_ENABLED;
            }
            if uv_latched {
                state_flags |= STATE_FLAG_UV_LATCHED;
            }
            if power_limited {
                state_flags |= STATE_FLAG_POWER_LIMITED;
            }
            if current_limited {
                state_flags |= STATE_FLAG_CURRENT_LIMITED;
            }

            // Optional Raw telemetry fields during calibration.
            let (status_cal_kind, raw_v_nr_opt, raw_v_rmt_opt, raw_cur_opt, raw_dac_opt) =
                match cal_kind {
                    CalKind::Voltage => (
                        Some(u8::from(cal_kind)),
                        Some(raw_v_nr_100uv_sm),
                        Some(raw_v_rmt_100uv_sm),
                        None,
                        None,
                    ),
                    CalKind::CurrentCh1 => (
                        Some(u8::from(cal_kind)),
                        None,
                        None,
                        Some(raw_cur1_100uv_sm),
                        Some(dac_code_ch1),
                    ),
                    CalKind::CurrentCh2 => (
                        Some(u8::from(cal_kind)),
                        None,
                        None,
                        Some(raw_cur2_100uv_sm),
                        Some(dac_code_ch2),
                    ),
                    CalKind::Off => (None, None, None, None, None),
                };
            let status = FastStatus {
                uptime_ms: now_ms,
                mode: status_mode,
                state_flags,
                enable: effective_output_enable,
                // target_value 表示两通道合计目标电流（mA）。
                target_value: target_i_total_ma,
                // i_local_ma / i_remote_ma 对应通道 1 / 通道 2 实测电流。
                i_local_ma: status_i_ch1_ma,
                i_remote_ma: status_i_ch2_ma,
                v_local_mv: status_v_local_mv,
                v_remote_mv: status_v_remote_mv,
                calc_p_mw: status_calc_p_mw,
                dac_headroom_mv,
                loop_error: status_loop_error,
                sink_core_temp_mc,
                sink_exhaust_temp_mc,
                mcu_temp_mc,
                fault_flags,
                cal_kind: status_cal_kind,
                raw_v_nr_100uv: raw_v_nr_opt,
                raw_v_rmt_100uv: raw_v_rmt_opt,
                raw_cur_100uv: raw_cur_opt,
                raw_dac_code: raw_dac_opt,
            };

            if ENABLE_FAST_STATUS_TX {
                let now_ms = timestamp_ms() as u32;
                let quiet_until = QUIET_UNTIL_MS.load(Ordering::Relaxed);
                if now_ms >= quiet_until {
                    let _ = FAST_STATUS_TX_CH.try_send(status);
                }
            }
        }

        status_div = status_div.wrapping_add(1);
        if status_div >= CONTROL_TICKS_PER_STATUS {
            status_div = 0;
        }

        // Use absolute scheduling to reduce drift/jitter versus after_millis().
        Timer::at(next_tick).await;
        next_tick += control_period;
        let now = Instant::now();
        if next_tick <= now {
            // If we're late, resync to avoid running hot in a tight loop.
            next_tick = now + control_period;
        }
    }
}

// 旧版 mock FAST_STATUS 生成逻辑已被真实采样逻辑替代，保留占位以防回滚时需要参考。

async fn apply_soft_reset_safing(
    dac: &mut Dac<'static, stm32::peripherals::DAC1, embassy_stm32::mode::Blocking>,
    load_en_ctl: &mut Output<'static>,
    load_en_ts: &mut Output<'static>,
) {
    let reason = SoftResetReason::from(LAST_SOFT_RESET_REASON.load(Ordering::Relaxed));

    // Drop remote enable on soft reset; the digital side is expected to
    // explicitly re-arm via SetEnable after handshake completes.
    ENABLE_REQUESTED.store(false, Ordering::Relaxed);
    ACTIVE_MODE_SEEN.store(false, Ordering::Relaxed);
    LAST_SETMODE_SEQ_VALID.store(false, Ordering::Relaxed);
    active_control_reset();

    load_en_ctl.set_low();
    load_en_ts.set_low();

    // SOFT_RESET：清零总目标电流，等待数字板重新下发 SetPoint。
    let reset_target_total = 0;
    TARGET_I_LOCAL_MA.store(reset_target_total, Ordering::Relaxed);
    let reset_dac_code_ch1 = 0u16;
    let reset_dac_code_ch2 = 0u16;
    dac.ch1().set(DacValue::Bit12Right(reset_dac_code_ch1));
    dac.ch2().set(DacValue::Bit12Right(reset_dac_code_ch2));

    Timer::after_millis(5).await;

    // Clear any latched protection faults; digital side is expected to re-arm
    // the load explicitly after observing a clean state.
    FAULT_FLAGS.store(0, Ordering::Relaxed);
    info!("fault flags cleared on soft reset");

    info!(
        "soft reset applied: reason={:?}, total target set to {} mA (DAC1={} DAC2={}), load disabled",
        reason, reset_target_total, reset_dac_code_ch1, reset_dac_code_ch2
    );
}

async fn send_setpoint_ack(
    seq: u8,
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
    ack_raw: &mut [u8],
    ack_slip: &mut [u8],
) {
    let ack_len = match encode_ack_only_frame(seq, MSG_SET_POINT, false, ack_raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("setpoint ack encode error: {:?}", err);
            return;
        }
    };
    let slip_len = match slip_encode(&ack_raw[..ack_len], ack_slip) {
        Ok(len) => len,
        Err(err) => {
            warn!("setpoint ack slip encode error: {:?}", err);
            return;
        }
    };

    let mut tx = uart_tx.lock().await;
    match tx.write(&ack_slip[..slip_len]).await {
        Ok(_) => info!("setpoint ACK sent: seq={} len={}B", seq, slip_len),
        Err(err) => warn!("setpoint ack write error: {:?}", err),
    }
}

async fn send_setmode_ack(
    seq: u8,
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
    ack_raw: &mut [u8],
    ack_slip: &mut [u8],
) {
    let ack_len = match encode_ack_only_frame(seq, MSG_SET_MODE, false, ack_raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("setmode ack encode error: {:?}", err);
            return;
        }
    };
    let slip_len = match slip_encode(&ack_raw[..ack_len], ack_slip) {
        Ok(len) => len,
        Err(err) => {
            warn!("setmode ack slip encode error: {:?}", err);
            return;
        }
    };

    let mut tx = uart_tx.lock().await;
    match tx.write(&ack_slip[..slip_len]).await {
        Ok(_) => info!("setmode ACK sent: seq={} len={}B", seq, slip_len),
        Err(err) => warn!("setmode ack write error: {:?}", err),
    }
}

async fn handle_soft_reset_request(
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
    ack_raw: &mut [u8],
    ack_slip: &mut [u8],
    header: FrameHeader,
    reset: SoftReset,
) {
    if header.flags & FLAG_IS_ACK != 0 {
        info!(
            "soft_reset ack received on analog side (unexpected but ignored) seq={}",
            header.seq
        );
        return;
    }

    LAST_SOFT_RESET_REASON.store(u8::from(reset.reason), Ordering::Relaxed);
    SOFT_RESET_PENDING.store(true, Ordering::SeqCst);
    LAST_SETPOINT_SEQ_VALID.store(false, Ordering::Relaxed);
    LAST_SETMODE_SEQ_VALID.store(false, Ordering::Relaxed);
    ACTIVE_MODE_SEEN.store(false, Ordering::Relaxed);
    QUIET_UNTIL_MS.store(
        (timestamp_ms() as u32).saturating_add(500),
        Ordering::Relaxed,
    );
    LAST_SETPOINT_SEQ_VALID.store(false, Ordering::Relaxed);

    // Reset atomic SetMode active-control snapshot on soft reset; the digital side
    // is expected to re-send SetMode after re-arming.
    active_control_reset();

    info!(
        "soft_reset request received: seq={} reason={:?} ts_ms={}",
        header.seq, reset.reason, reset.timestamp_ms
    );

    let ack_len = match encode_soft_reset_frame(header.seq, &reset, true, ack_raw) {
        Ok(len) => len,
        Err(err) => {
            warn!("soft_reset ack encode error: {:?}", err);
            return;
        }
    };
    let slip_len = match slip_encode(&ack_raw[..ack_len], ack_slip) {
        Ok(len) => len,
        Err(err) => {
            warn!("soft_reset ack slip encode error: {:?}", err);
            return;
        }
    };

    let mut tx = uart_tx.lock().await;
    match tx.write(&ack_slip[..slip_len]).await {
        Ok(_) => info!(
            "soft_reset ACK sent: seq={} reason={:?} ts_ms={}",
            header.seq, reset.reason, reset.timestamp_ms
        ),
        Err(err) => warn!("soft_reset ack write error: {:?}", err),
    }
}

/// UART RX 任务：从数字板接收控制帧（SetMode/SetPoint/SoftReset/SetEnable/...）。
#[embassy_executor::task]
async fn uart_setpoint_rx_task(
    mut uart_rx: RingBufferedUartRx<'static>,
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
) {
    info!(
        "UART control RX task starting (SetMode=0x{:02x}, SetPoint=0x{:02x}, default_target={} mA, range={}..{} mA)",
        MSG_SET_MODE, MSG_SET_POINT, DEFAULT_TARGET_I_LOCAL_MA, TARGET_I_MIN_MA, TARGET_I_MAX_MA
    );

    let mut decoder: SlipDecoder<128> = SlipDecoder::new();
    decoder.reset(); // ensure clean state on startup
    let mut buf = [0u8; 32];
    let mut ack_raw = [0u8; 64];
    let mut ack_slip = [0u8; 96];
    let mut last_rx_err_log_ms: u32 = 0;
    let mut last_setmode_dup_ack_seq: u8 = 0;
    let mut last_setmode_dup_ack_ms: u32 = 0;
    const SETMODE_DUP_ACK_THROTTLE_MS: u32 = 100;

    // Startup quiet window: ignore traffic for a short period to align buffers.
    QUIET_UNTIL_MS.store(
        (timestamp_ms() as u32).saturating_add(500),
        Ordering::Relaxed,
    );

    // NOTE: Do not block on "syncing" to a specific SLIP boundary here.
    //
    // The digital side may burst calibration frames immediately on link-up; any
    // pre-decode sync loop risks dropping the first (and often most important)
    // CalWrite chunk, leaving CAL_READY false and output disabled.
    //
    // The SLIP decoder self-synchronizes on SLIP_END, and we already validate
    // minimum frame length + CRC, so starting decode immediately is safe.

    loop {
        match uart_rx.read(&mut buf).await {
            Ok(n) if n > 0 => {
                for &b in &buf[..n] {
                    match decoder.push(b) {
                        Ok(Some(frame)) => {
                            if frame.len() < HEADER_LEN + CRC_LEN {
                                warn!(
                                    "SetPoint RX: too-short frame len={} resetting decoder",
                                    frame.len()
                                );
                                decoder.reset();
                                continue;
                            }
                            trace!(
                                "uart rx: SLIP frame len={}, head={=[u8]:#04x}",
                                frame.len(),
                                &frame[..frame.len().min(16)]
                            );
                            match decode_set_mode_frame(&frame) {
                                Ok((hdr, cmd)) => {
                                    if hdr.flags & FLAG_IS_ACK != 0 {
                                        info!(
                                            "setmode ACK received on analog side (ignored) seq={}",
                                            hdr.seq
                                        );
                                        continue;
                                    } else {
                                        let last_seq = LAST_SETMODE_SEQ.load(Ordering::Relaxed);
                                        let last_valid =
                                            LAST_SETMODE_SEQ_VALID.load(Ordering::Relaxed);
                                        let is_dup = last_valid && hdr.seq == last_seq;

                                        if !last_valid || !is_dup {
                                            LAST_SETMODE_SEQ.store(hdr.seq, Ordering::Relaxed);
                                            LAST_SETMODE_SEQ_VALID.store(true, Ordering::Relaxed);

                                            let prev_enabled =
                                                ACTIVE_CTRL_OUTPUT_ENABLED.load(Ordering::Relaxed);
                                            let prev_uv_latched =
                                                ACTIVE_CTRL_UV_LATCHED.load(Ordering::Relaxed);

                                            let new_target_p_mw = cmd.target_p_mw.unwrap_or(0);

                                            ACTIVE_CTRL_SEQ.fetch_add(1, Ordering::Release);
                                            ACTIVE_CTRL_PRESET_ID
                                                .store(cmd.preset_id, Ordering::Relaxed);
                                            ACTIVE_CTRL_OUTPUT_ENABLED
                                                .store(cmd.output_enabled, Ordering::Relaxed);
                                            ACTIVE_CTRL_MODE_U8
                                                .store(u8::from(cmd.mode), Ordering::Relaxed);
                                            ACTIVE_CTRL_TARGET_I_MA
                                                .store(cmd.target_i_ma, Ordering::Relaxed);
                                            ACTIVE_CTRL_TARGET_V_MV
                                                .store(cmd.target_v_mv, Ordering::Relaxed);
                                            ACTIVE_CTRL_TARGET_P_MW
                                                .store(new_target_p_mw, Ordering::Relaxed);
                                            ACTIVE_CTRL_MIN_V_MV
                                                .store(cmd.min_v_mv, Ordering::Relaxed);
                                            ACTIVE_CTRL_MAX_I_MA_TOTAL
                                                .store(cmd.max_i_ma_total, Ordering::Relaxed);
                                            ACTIVE_CTRL_MAX_P_MW
                                                .store(cmd.max_p_mw, Ordering::Relaxed);

                                            if !prev_enabled && cmd.output_enabled {
                                                if prev_uv_latched {
                                                    info!(
                                                        "uv_latched cleared on output enable rising edge (preset_id={} seq={})",
                                                        cmd.preset_id, hdr.seq
                                                    );
                                                }
                                                ACTIVE_CTRL_UV_LATCHED
                                                    .store(false, Ordering::Relaxed);
                                            }
                                            ACTIVE_CTRL_SEQ.fetch_add(1, Ordering::Release);

                                            ACTIVE_MODE_SEEN.store(true, Ordering::Relaxed);

                                            info!(
                                                "SetMode received: preset_id={} enable={} mode={:?} target_i={}mA target_v={}mV target_p={}mW min_v={}mV max_i_total={}mA max_p={}mW seq={}",
                                                cmd.preset_id,
                                                cmd.output_enabled,
                                                cmd.mode,
                                                cmd.target_i_ma,
                                                cmd.target_v_mv,
                                                cmd.target_p_mw.unwrap_or(0),
                                                cmd.min_v_mv,
                                                cmd.max_i_ma_total,
                                                cmd.max_p_mw,
                                                hdr.seq
                                            );
                                        } else {
                                            let now_ms = timestamp_ms() as u32;
                                            if now_ms.wrapping_sub(last_setmode_dup_ack_ms)
                                                >= SETMODE_DUP_ACK_THROTTLE_MS
                                            {
                                                info!(
                                                    "SetMode duplicate received: seq={} (throttled ack)",
                                                    hdr.seq
                                                );
                                            }
                                        }

                                        // 任意有效 SetMode 帧均视作“通信正常”活动，用于链路健康统计。
                                        LAST_RX_GOOD_MS
                                            .store(timestamp_ms() as u32, Ordering::Relaxed);
                                        LINK_EVER_GOOD.store(true, Ordering::Relaxed);

                                        // Always ACK valid SetMode frames; throttle duplicate ACKs to
                                        // avoid starving FAST_STATUS on a noisy link.
                                        let should_ack = if is_dup {
                                            let now_ms = timestamp_ms() as u32;
                                            let ok = hdr.seq != last_setmode_dup_ack_seq
                                                || now_ms.wrapping_sub(last_setmode_dup_ack_ms)
                                                    >= SETMODE_DUP_ACK_THROTTLE_MS;
                                            if ok {
                                                last_setmode_dup_ack_seq = hdr.seq;
                                                last_setmode_dup_ack_ms = now_ms;
                                            }
                                            ok
                                        } else {
                                            true
                                        };
                                        if should_ack {
                                            send_setmode_ack(
                                                hdr.seq,
                                                uart_tx,
                                                &mut ack_raw,
                                                &mut ack_slip,
                                            )
                                            .await;
                                        }
                                    }
                                }
                                Err(ProtocolError::UnsupportedMessage(_)) => {
                                    match decode_set_point_frame(&frame) {
                                        Ok((hdr, sp)) => {
                                            let v = sp
                                                .target_i_ma
                                                .clamp(TARGET_I_MIN_MA, TARGET_I_MAX_MA);
                                            let last_seq =
                                                LAST_SETPOINT_SEQ.load(Ordering::Relaxed);
                                            let last_valid =
                                                LAST_SETPOINT_SEQ_VALID.load(Ordering::Relaxed);
                                            let is_dup = last_valid && hdr.seq == last_seq;

                                            if ACTIVE_MODE_SEEN.load(Ordering::Relaxed) {
                                                let now_ms = timestamp_ms() as u32;
                                                let last_log = LAST_SETPOINT_IGNORED_LOG_MS
                                                    .load(Ordering::Relaxed);
                                                if now_ms.wrapping_sub(last_log) > 1_000 {
                                                    LAST_SETPOINT_IGNORED_LOG_MS
                                                        .store(now_ms, Ordering::Relaxed);
                                                    warn!(
                                                        "SetPoint ignored (SetMode active): seq={} target_i_ma={}mA",
                                                        hdr.seq, v
                                                    );
                                                }
                                                if !last_valid || !is_dup {
                                                    LAST_SETPOINT_SEQ
                                                        .store(hdr.seq, Ordering::Relaxed);
                                                    LAST_SETPOINT_SEQ_VALID
                                                        .store(true, Ordering::Relaxed);
                                                }
                                            } else if !last_valid || !is_dup {
                                                let prev =
                                                    TARGET_I_LOCAL_MA.swap(v, Ordering::Relaxed);
                                                LAST_SETPOINT_SEQ.store(hdr.seq, Ordering::Relaxed);
                                                LAST_SETPOINT_SEQ_VALID
                                                    .store(true, Ordering::Relaxed);
                                                info!(
                                                    "SetPoint received: target_i_ma={} mA (prev={} mA, seq={})",
                                                    v, prev, hdr.seq
                                                );
                                            } else {
                                                info!(
                                                    "SetPoint duplicate received: seq={} target_i_ma={} mA (ignored, ack only)",
                                                    hdr.seq, v
                                                );
                                            }

                                            // 任意有效 SetPoint 帧均视作“通信正常”活动，用于链路健康统计。
                                            LAST_RX_GOOD_MS
                                                .store(timestamp_ms() as u32, Ordering::Relaxed);
                                            LINK_EVER_GOOD.store(true, Ordering::Relaxed);

                                            // ACK regardless of whether it was a duplicate to keep sender state in sync.
                                            send_setpoint_ack(
                                                hdr.seq,
                                                uart_tx,
                                                &mut ack_raw,
                                                &mut ack_slip,
                                            )
                                            .await;
                                        }
                                        Err(ProtocolError::UnsupportedMessage(_)) => {
                                            match decode_soft_reset_frame(&frame) {
                                                Ok((hdr, reset)) => {
                                                    handle_soft_reset_request(
                                                        uart_tx,
                                                        &mut ack_raw,
                                                        &mut ack_slip,
                                                        hdr,
                                                        reset,
                                                    )
                                                    .await;
                                                    // soft_reset 请求同样视为有效通信活动。
                                                    LAST_RX_GOOD_MS.store(
                                                        timestamp_ms() as u32,
                                                        Ordering::Relaxed,
                                                    );
                                                    LINK_EVER_GOOD.store(true, Ordering::Relaxed);
                                                }
                                                Err(ProtocolError::UnsupportedMessage(_)) => {
                                                    match decode_set_enable_frame(&frame) {
                                                        Ok((_hdr, cmd)) => {
                                                            let prev = ENABLE_REQUESTED.swap(
                                                                cmd.enable,
                                                                Ordering::Relaxed,
                                                            );
                                                            info!(
                                                                "SetEnable received: enable={} (prev={})",
                                                                cmd.enable, prev
                                                            );
                                                            LAST_RX_GOOD_MS.store(
                                                                timestamp_ms() as u32,
                                                                Ordering::Relaxed,
                                                            );
                                                            LINK_EVER_GOOD
                                                                .store(true, Ordering::Relaxed);
                                                        }
                                                        Err(ProtocolError::UnsupportedMessage(
                                                            _,
                                                        )) => {
                                                            if let Ok((hdr, _payload)) =
                                                                decode_frame(&frame)
                                                                && hdr.msg
                                                                    == pd::MSG_PD_SINK_REQUEST
                                                            {
                                                                if hdr.flags & FLAG_IS_ACK != 0 {
                                                                    info!(
                                                                        "PD_SINK_REQUEST ACK received on analog side (ignored) seq={}",
                                                                        hdr.seq
                                                                    );
                                                                    continue;
                                                                }

                                                                let (is_nack, reason) =
                                                                    match decode_pd_sink_request_frame(&frame) {
                                                                        Ok((_hdr2, req)) => {
                                                                            let mode = match req.mode {
                                                                                loadlynx_protocol::PdSinkMode::Fixed => {
                                                                                    Some(pd::PD_MODE_FIXED)
                                                                                }
                                                                                loadlynx_protocol::PdSinkMode::Pps => {
                                                                                    Some(pd::PD_MODE_PPS)
                                                                                }
                                                                                loadlynx_protocol::PdSinkMode::Unknown(_) => None,
                                                                            };

                                                                            match mode {
                                                                                None => (true, "unsupported mode"),
                                                                                Some(mode) => {
                                                                                    let object_pos = req.object_pos.max(1);
                                                                                    if object_pos == 0 || object_pos > 14 {
                                                                                        (true, "invalid object_pos")
                                                                                    } else if req.target_mv < 3_000
                                                                                        || req.target_mv > 21_000
                                                                                    {
                                                                                        (true, "invalid target_mv")
                                                                                    } else if req.i_req_ma > 10_000 {
                                                                                        (true, "invalid i_req_ma")
                                                                                    } else {
                                                                                        pd::PD_DESIRED_MODE.store(
                                                                                            mode,
                                                                                            Ordering::Relaxed,
                                                                                        );
                                                                                        pd::PD_DESIRED_OBJECT_POS.store(
                                                                                            object_pos,
                                                                                            Ordering::Relaxed,
                                                                                        );
                                                                                        pd::PD_DESIRED_TARGET_MV.store(
                                                                                            req.target_mv,
                                                                                            Ordering::Relaxed,
                                                                                        );
                                                                                        pd::PD_DESIRED_I_REQ_MA.store(
                                                                                            req.i_req_ma,
                                                                                            Ordering::Relaxed,
                                                                                        );
                                                                                        pd::PD_RENEGOTIATE_SIGNAL.signal(());
                                                                                        info!(
                                                                                            "PD_SINK_REQUEST received: mode={} object_pos={} target_mv={} i_req_ma={} seq={}",
                                                                                            mode,
                                                                                            object_pos,
                                                                                            req.target_mv,
                                                                                            req.i_req_ma,
                                                                                            hdr.seq
                                                                                        );
                                                                                        (false, "")
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        Err(_err) => {
                                                                            (true, "frame decode error")
                                                                        }
                                                                    };

                                                                if is_nack {
                                                                    warn!(
                                                                        "PD_SINK_REQUEST rejected ({}): seq={}",
                                                                        reason, hdr.seq
                                                                    );
                                                                }

                                                                LAST_RX_GOOD_MS.store(
                                                                    timestamp_ms() as u32,
                                                                    Ordering::Relaxed,
                                                                );
                                                                LINK_EVER_GOOD
                                                                    .store(true, Ordering::Relaxed);

                                                                let ack_len =
                                                                    match encode_ack_only_frame(
                                                                        hdr.seq,
                                                                        pd::MSG_PD_SINK_REQUEST,
                                                                        is_nack,
                                                                        &mut ack_raw,
                                                                    ) {
                                                                        Ok(len) => len,
                                                                        Err(err) => {
                                                                            warn!(
                                                                                "PD_SINK_REQUEST ack encode error: {:?}",
                                                                                err
                                                                            );
                                                                            continue;
                                                                        }
                                                                    };
                                                                let slip_len = match slip_encode(
                                                                    &ack_raw[..ack_len],
                                                                    &mut ack_slip,
                                                                ) {
                                                                    Ok(len) => len,
                                                                    Err(err) => {
                                                                        warn!(
                                                                            "PD_SINK_REQUEST ack slip encode error: {:?}",
                                                                            err
                                                                        );
                                                                        continue;
                                                                    }
                                                                };
                                                                let mut tx = uart_tx.lock().await;
                                                                if let Err(err) = tx
                                                                    .write(&ack_slip[..slip_len])
                                                                    .await
                                                                {
                                                                    warn!(
                                                                        "PD_SINK_REQUEST ack write error: {:?}",
                                                                        err
                                                                    );
                                                                }

                                                                continue;
                                                            }
                                                            match decode_limit_profile_frame(&frame) {
                                                        Ok((_hdr, profile)) => {
                                                            {
                                                                let mut limits =
                                                                    LIMIT_PROFILE.lock().await;
                                                                limits.max_i_ma = profile.max_i_ma;
                                                                limits.max_p_mw = profile.max_p_mw;
                                                                limits.ovp_mv = profile.ovp_mv;
                                                                limits.temp_trip_mc =
                                                                    profile.temp_trip_mc;
                                                                limits.thermal_derate_pct =
                                                                    profile.thermal_derate_pct;
                                                            }
                                                            info!(
                                                                "LimitProfile received: max_i={}mA max_p={}mW ovp={}mV temp_trip={}mC derate={}%",
                                                                profile.max_i_ma,
                                                                profile.max_p_mw,
                                                                profile.ovp_mv,
                                                                profile.temp_trip_mc,
                                                                profile.thermal_derate_pct
                                                            );
                                                            LAST_RX_GOOD_MS.store(
                                                                timestamp_ms() as u32,
                                                                Ordering::Relaxed,
                                                            );
                                                            LINK_EVER_GOOD
                                                                .store(true, Ordering::Relaxed);
                                                        }
                                                        Err(ProtocolError::UnsupportedMessage(
                                                            _,
                                                        )) => match decode_cal_mode_frame(&frame) {
                                                            Ok((hdr, mode)) => {
                                                                if hdr.flags & FLAG_IS_ACK != 0 {
                                                                    info!(
                                                                        "CalMode ACK received (ignored): seq={} kind={:?}",
                                                                        hdr.seq, mode.kind
                                                                    );
                                                                } else {
                                                                    let prev_raw = CAL_MODE_KIND
                                                                        .swap(
                                                                            u8::from(mode.kind),
                                                                            Ordering::Relaxed,
                                                                        );
                                                                    info!(
                                                                        "CalMode received: kind={:?} (prev_raw={}) seq={}",
                                                                        mode.kind,
                                                                        prev_raw,
                                                                        hdr.seq
                                                                    );

                                                                    LAST_RX_GOOD_MS.store(
                                                                        timestamp_ms() as u32,
                                                                        Ordering::Relaxed,
                                                                    );
                                                                    LINK_EVER_GOOD.store(
                                                                        true,
                                                                        Ordering::Relaxed,
                                                                    );

                                                                    let ack_len =
                                                                        match encode_ack_only_frame(
                                                                            hdr.seq,
                                                                            MSG_CAL_MODE,
                                                                            false,
                                                                            &mut ack_raw,
                                                                        ) {
                                                                            Ok(len) => len,
                                                                            Err(err) => {
                                                                                warn!(
                                                                                    "CalMode ack encode error: {:?}",
                                                                                    err
                                                                                );
                                                                                continue;
                                                                            }
                                                                        };
                                                                    let slip_len = match slip_encode(
                                                                        &ack_raw[..ack_len],
                                                                        &mut ack_slip,
                                                                    ) {
                                                                        Ok(len) => len,
                                                                        Err(err) => {
                                                                            warn!(
                                                                                "CalMode ack slip encode error: {:?}",
                                                                                err
                                                                            );
                                                                            continue;
                                                                        }
                                                                    };

                                                                    let mut tx =
                                                                        uart_tx.lock().await;
                                                                    if let Err(err) = tx
                                                                        .write(
                                                                            &ack_slip[..slip_len],
                                                                        )
                                                                        .await
                                                                    {
                                                                        warn!(
                                                                            "CalMode ack write error: {:?}",
                                                                            err
                                                                        );
                                                                    } else {
                                                                        info!(
                                                                            "CalMode ACK sent: seq={} len={}B",
                                                                            hdr.seq, slip_len
                                                                        );
                                                                    }
                                                                }
                                                            }
                                                            Err(
                                                                ProtocolError::UnsupportedMessage(
                                                                    _,
                                                                ),
                                                            ) => {
                                                                match decode_cal_write_frame(&frame)
                                                                {
                                                                    Ok((_hdr, cal)) => {
                                                                        let payload = cal.payload;
                                                                        let fmt_version =
                                                                            payload[0];
                                                                        let hw_rev = payload[1];
                                                                        let kind_raw = payload[2];
                                                                        let chunk_index =
                                                                            payload[3];
                                                                        let total_chunks =
                                                                            payload[4];
                                                                        let total_points =
                                                                            payload[5];

                                                                        info!(
                                                                            "CalWrite chunk received: kind_raw={} chunk={}/{} total_points={} fmt_version={} hw_rev={} outer_index={}",
                                                                            kind_raw,
                                                                            chunk_index,
                                                                            total_chunks,
                                                                            total_points,
                                                                            fmt_version,
                                                                            hw_rev,
                                                                            cal.index
                                                                        );

                                                                        let mut state =
                                                                            CAL_STATE.lock().await;
                                                                        let mut curves_changed = false;
                                                                        match state
                                                                            .ingest_cal_write(
                                                                                cal.index,
                                                                                &payload, cal.crc,
                                                                            ) {
                                                                            Ok(Some(done_kind)) => {
                                                                                info!(
                                                                                    "CalWrite curve completed: kind={:?}",
                                                                                    done_kind
                                                                                );
                                                                                curves_changed = true;
                                                                            }
                                                                            Ok(None) => {}
                                                                            Err(err) => {
                                                                                warn!(
                                                                                    "CalWrite rejected for kind_raw={} chunk_index={}: {:?}",
                                                                                    kind_raw,
                                                                                    chunk_index,
                                                                                    err
                                                                                );
                                                                            }
                                                                        }
                                                                        if curves_changed {
                                                                            cal_curves_publish(
                                                                                state.snapshot(),
                                                                            );
                                                                        }

                                                                        let all_valid =
                                                                            state.all_valid();
                                                                        let prev = CAL_READY.swap(
                                                                            all_valid,
                                                                            Ordering::Relaxed,
                                                                        );
                                                                        if all_valid != prev {
                                                                            info!(
                                                                                "CAL_READY updated: {} (prev={})",
                                                                                all_valid, prev
                                                                            );
                                                                        }

                                                                        LAST_RX_GOOD_MS.store(
                                                                            timestamp_ms() as u32,
                                                                            Ordering::Relaxed,
                                                                        );
                                                                        LINK_EVER_GOOD.store(
                                                                            true,
                                                                            Ordering::Relaxed,
                                                                        );
                                                                    }
                                                                    Err(err) => {
                                                                        warn!(
                                                                            "CalWrite decode error {:?} (len={}, head={=[u8]:#04x})",
                                                                            err,
                                                                            frame.len(),
                                                                            &frame[..frame
                                                                                .len()
                                                                                .min(8)],
                                                                        );
                                                                        decoder.reset();
                                                                    }
                                                                }
                                                            }
                                                            Err(err) => {
                                                                warn!(
                                                                    "CalMode decode error {:?} (len={}, head={=[u8]:#04x})",
                                                                    err,
                                                                    frame.len(),
                                                                    &frame[..frame.len().min(8)],
                                                                );
                                                                decoder.reset();
                                                            }
                                                        },
                                                        Err(err) => {
                                                            warn!(
                                                                "LimitProfile decode error {:?} (len={}, head={=[u8]:#04x})",
                                                                err,
                                                                frame.len(),
                                                                &frame[..frame.len().min(8)],
                                                            );
                                                            decoder.reset();
                                                        }
                                                    }
                                                        }
                                                        Err(err) => {
                                                            warn!(
                                                                "set_enable decode error {:?} (len={}, head={=[u8]:#04x})",
                                                                err,
                                                                frame.len(),
                                                                &frame[..frame.len().min(8)],
                                                            );
                                                            decoder.reset();
                                                        }
                                                    }
                                                }
                                                Err(err) => {
                                                    warn!(
                                                        "soft_reset decode error {:?} (len={}, head={=[u8]:#04x})",
                                                        err,
                                                        frame.len(),
                                                        &frame[..frame.len().min(8)],
                                                    );
                                                    decoder.reset();
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            warn!(
                                                "decode_set_point_frame error {:?} (len={}, head={=[u8]:#04x})",
                                                err,
                                                frame.len(),
                                                &frame[..frame.len().min(8)],
                                            );
                                            decoder.reset();
                                        }
                                    }
                                }
                                Err(err) => {
                                    warn!(
                                        "decode_set_mode_frame error {:?} (len={}, head={=[u8]:#04x})",
                                        err,
                                        frame.len(),
                                        &frame[..frame.len().min(8)],
                                    );
                                    decoder.reset();
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(_err) => {
                            warn!("SLIP decode error in SetPoint RX");
                            decoder.reset();
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(err) => {
                let now_ms = timestamp_ms() as u32;
                if now_ms.wrapping_sub(last_rx_err_log_ms) > 1_000 {
                    last_rx_err_log_ms = now_ms;
                    warn!("uart rx error in SetPoint task: {:?}", err);
                }
                // embassy-stm32 RingBufferedUartRx stops background reception on any UART error.
                // If bytes keep arriving while DMAR is off, ORE can re-trigger and the receiver
                // can get stuck in a permanent error loop. Re-start DMA-backed reception to
                // recover without requiring a board reset.
                uart_rx.start_uart();
                Timer::after_millis(1).await;
                decoder.reset();
            }
        }
    }
}

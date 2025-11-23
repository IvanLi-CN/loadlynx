#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, Ordering};
use embassy_executor::Spawner;
use embassy_stm32 as stm32;
use embassy_stm32::adc::{Adc, AdcChannel, SampleTime, Temperature as AdcTemperature};
use embassy_stm32::bind_interrupts;
use embassy_stm32::dac::{Dac, Value as DacValue};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::mode::Async as UartAsync;
use embassy_stm32::usart::{
    Config as UartConfig, DataBits as UartDataBits, Parity as UartParity, RingBufferedUartRx,
    StopBits as UartStopBits, Uart, UartRx, UartTx,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Instant, Timer};
use libm::logf;
use loadlynx_protocol::{
    CRC_LEN, Error as ProtocolError, FLAG_IS_ACK, FastStatus, FrameHeader, HEADER_LEN,
    MSG_SET_POINT, SLIP_END, SlipDecoder, SoftReset, SoftResetReason, decode_set_point_frame,
    decode_soft_reset_frame, encode_ack_only_frame, encode_fast_status_frame,
    encode_soft_reset_frame, slip_encode,
};
use static_cell::StaticCell;
use stm32_metapac::VREFBUF;
use stm32_metapac::vrefbuf::vals::{Hiz, Vrs};

bind_interrupts!(struct Irqs {
    USART3 => stm32::usart::InterruptHandler<stm32::peripherals::USART3>;
});

// 模拟板 FAST_STATUS 发送周期：20 Hz → 1000/20 ms = 50 ms
const FAST_STATUS_PERIOD_MS: u64 = 1000 / 20;
// 调试开关：如需只验证数字板→模拟板的 SetPoint 路径，可暂时关闭 FAST_STATUS TX。
const ENABLE_FAST_STATUS_TX: bool = true;
const STATE_FLAG_REMOTE_ACTIVE: u32 = 1 << 0;
const STATE_FLAG_LINK_GOOD: u32 = 1 << 1;

// DAC1 → CC1 环路：0.5 A 设计值对应的 DAC 码。
// 物理链路（来自网表与 INA193/OPA2365 手册）：
//   I_load → R_SHUNT=25 mΩ → INA193 (G=20 V/V) → V_CUR1_SP
//   V_CUR1_SP = 20 * 0.025 Ω * I = 0.5 * I [V/A]
//   CC 运放比较 V_CUR1_SP 与 DAC_CH1，经 100 Ω/100 kΩ 网络，直流近似 V_DAC ≈ V_CUR1_SP。
//
// VREFBUF 设为 2.9 V (Vrs::VREF2)，DAC 12bit 满量程 4095：
//   I = 0.5 A → V_CUR1_SP ≈ 0.25 V
//   CODE_0p5A ≈ 0.25 / 2.9 * 4095 ≈ 353
const CC_0P5A_DAC_CODE_CH1: u16 = 353;

// ADC 公共参数（G431 12bit ADC，以 VREFBUF 2.9 V 作为参考）。
const ADC_VREF_MV: u32 = 2900;
const ADC_FULL_SCALE: u32 = 4095;

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

// 默认恒流目标（mA）：0.5 A，用于未接收到任何远端 SetPoint 时的启动值。
const DEFAULT_TARGET_I_LOCAL_MA: i32 = 500;
// 可接受的目标电流范围（mA），用于防止异常指令导致过流。
const TARGET_I_MIN_MA: i32 = 0;
const TARGET_I_MAX_MA: i32 = 5_000;

// 由数字板通过 SetPoint 消息更新的电流设定（mA）。
//
// - 初始值为 DEFAULT_TARGET_I_LOCAL_MA（0.5 A）。
// - uart_setpoint_rx_task 解析 SetPoint 帧并写入该原子量。
// - 采样/遥测主循环在每次迭代中读取该值，用于计算 DAC 目标码与 loop_error。
static TARGET_I_LOCAL_MA: AtomicI32 = AtomicI32::new(DEFAULT_TARGET_I_LOCAL_MA);
static SOFT_RESET_PENDING: AtomicBool = AtomicBool::new(false);
static LAST_SOFT_RESET_REASON: AtomicU8 = AtomicU8::new(0);
static LAST_SETPOINT_SEQ_VALID: AtomicBool = AtomicBool::new(false);
static LAST_SETPOINT_SEQ: AtomicU8 = AtomicU8::new(0);
static QUIET_UNTIL_MS: AtomicU32 = AtomicU32::new(0);
const RAW_DUMP_BYTES: usize = 256;

static UART_TX_SHARED: StaticCell<Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>> =
    StaticCell::new();

fn timestamp_ms() -> u64 {
    Instant::now().as_millis() as u64
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
    // 为 ADC1/ADC2 选择稳定的时钟源，避免后续 ADC 初始化触发时钟相关异常。
    let mut config = stm32::Config::default();
    {
        use embassy_stm32::rcc::mux;
        config.rcc.mux.adc12sel = mux::Adcsel::SYS;
    }
    let p = stm32::init(config);

    info!("LoadLynx analog alive; init VREFBUF/ADC/DAC/UART (CC 0.5A, real telemetry)");

    // 配置内部基准缓冲（VREFBUF），使 VREF+ ≈ 2.9 V，供 ADC/DAC 共用。
    let vrefbuf = VREFBUF;
    vrefbuf.csr().modify(|csr| {
        csr.set_hiz(Hiz::CONNECTED);
        csr.set_vrs(Vrs::VREF2);
        csr.set_envr(true);
    });
    while !vrefbuf.csr().read().vrr() {
        // 等待基准缓冲稳定
    }

    // 暂时直接闭合 TPS22810 负载开关：PB13=LOAD_EN_CTL，PB14=LOAD_EN_TS。
    // 逻辑：LOAD_EN = LOAD_EN_CTL AND LOAD_EN_TS。为简单起见，两路都拉高，
    // 只启用硬件恒流通道 1（DAC_CH1 设为 0.5A 目标，DAC_CH2=0）。
    let mut load_en_ctl = Output::new(p.PB13, Level::High, Speed::Low);
    let mut load_en_ts = Output::new(p.PB14, Level::High, Speed::Low);
    load_en_ctl.set_high();
    load_en_ts.set_high();

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

    // 拆分为 TX/RX 两个半通道：主循环持续通过 TX 发送 FAST_STATUS，
    // 另起任务在 RX 上监听来自数字板的 SetPoint 控制帧。
    let (uart_tx, uart_rx): (UartTx<'static, UartAsync>, UartRx<'static, UartAsync>) = uart.split();

    let uart_tx_shared = UART_TX_SHARED.init(Mutex::new(uart_tx));

    // 将 RX 端转换为环形缓冲 UART，以避免在任务之间存在调度间隙时丢字节。
    static UART_RX_DMA_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    let uart_rx_ring: RingBufferedUartRx<'static> =
        uart_rx.into_ring_buffered(UART_RX_DMA_BUF.init([0; 256]));

    // 启动独立任务接收 SetPoint 控制消息。
    if let Err(e) = _spawner.spawn(uart_setpoint_rx_task(uart_rx_ring, uart_tx_shared)) {
        warn!("failed to spawn uart_setpoint_rx_task: {:?}", e);
    }

    // ADC1/ADC2：阻塞读取即可满足 30Hz 遥测。
    let mut adc1 = Adc::new(p.ADC1);
    let mut adc2 = Adc::new(p.ADC2);
    adc1.set_sample_time(SampleTime::CYCLES247_5);
    adc2.set_sample_time(SampleTime::CYCLES247_5);

    // 片内温度传感器（连接到 ADC1_IN16），用于 MCU die 温度遥测。
    let mut mcu_temp_ch: AdcTemperature = adc1.enable_temperature();

    // 通道映射（参考 loadlynx.ioc 与网表）：
    // - PA0/PA1:  V_RMT_SP / V_RMT_SN  → 远端电压差分放大后输出
    // - PA2/PA3:  V_NR_SP  / V_NR_SN   → 近端电压差分放大后输出
    // - PA6/PA7:  CUR1_SP  / CUR1_SN   → CH1 电流检测（INA193 输出）
    // - PB11/PB15:CUR2_SP  / CUR2_SN   → CH2 电流检测
    // - PB12:     _5V_SNS              → 模拟板 5V 轨电压分压
    // - PB0/PB1:  TS1 / TS2            → 温度传感器（暂不参与控制，仅打包）
    let mut v_rmt_sp = p.PA0.degrade_adc();
    let mut v_rmt_sn = p.PA1.degrade_adc();
    let mut v_nr_sp = p.PA2.degrade_adc();
    let mut v_nr_sn = p.PA3.degrade_adc();

    let mut cur1_sp = p.PA6.degrade_adc();
    let mut cur1_sn = p.PA7.degrade_adc();
    let mut cur2_sp = p.PB11.degrade_adc();
    let mut cur2_sn = p.PB15.degrade_adc();

    let mut sns_5v = p.PB12.degrade_adc();
    let mut ts1 = p.PB0.degrade_adc();
    let mut ts2 = p.PB1.degrade_adc();

    // DAC1：PA4/PA5 → CH1/CH2，当前仅启用 CH1 恒流，CH2 置零。
    let mut dac = Dac::new_blocking(p.DAC1, p.PA4, p.PA5);
    dac.ch1().set(DacValue::Bit12Right(CC_0P5A_DAC_CODE_CH1));
    dac.ch2().set(DacValue::Bit12Right(0));

    info!(
        "CC setpoint CH1: default target {} mA (DAC code = {})",
        DEFAULT_TARGET_I_LOCAL_MA, CC_0P5A_DAC_CODE_CH1
    );

    let mut seq: u8 = 0;
    let mut uptime_ms: u32 = 0;
    let mut raw_frame = [0u8; 192];
    let mut slip_frame = [0u8; 384];

    loop {
        if SOFT_RESET_PENDING.swap(false, Ordering::SeqCst) {
            apply_soft_reset_safing(&mut dac, &mut load_en_ctl, &mut load_en_ts).await;
        }

        // --- 采样所有相关 ADC 通道（阻塞读取） ---
        let v_rmt_sp_code = adc1.blocking_read(&mut v_rmt_sp);
        let v_rmt_sn_code = adc1.blocking_read(&mut v_rmt_sn);
        let v_nr_sp_code = adc1.blocking_read(&mut v_nr_sp);
        let v_nr_sn_code = adc1.blocking_read(&mut v_nr_sn);

        let cur1_sp_code = adc2.blocking_read(&mut cur1_sp);
        let cur1_sn_code = adc2.blocking_read(&mut cur1_sn);
        let cur2_sp_code = adc2.blocking_read(&mut cur2_sp);
        let cur2_sn_code = adc2.blocking_read(&mut cur2_sn);

        let sns_5v_code = adc1.blocking_read(&mut sns_5v);
        let ts1_code = adc1.blocking_read(&mut ts1);
        let ts2_code = adc1.blocking_read(&mut ts2);
        let mcu_temp_code = adc1.blocking_read(&mut mcu_temp_ch);

        // 节点电压（基于 VREF=2.9 V 的原始 ADC 电压，单位 mV）
        let adc_to_mv = |code: u16| -> u32 { (code as u32) * ADC_VREF_MV / ADC_FULL_SCALE };

        let v_rmt_sp_mv = adc_to_mv(v_rmt_sp_code);
        let v_nr_sp_mv = adc_to_mv(v_nr_sp_code);
        let _v_rmt_sn_mv = adc_to_mv(v_rmt_sn_code);
        let _v_nr_sn_mv = adc_to_mv(v_nr_sn_code);

        let cur1_sp_mv = adc_to_mv(cur1_sp_code);
        let _cur1_sn_mv = adc_to_mv(cur1_sn_code);
        let cur2_sp_mv = adc_to_mv(cur2_sp_code);
        let _cur2_sn_mv = adc_to_mv(cur2_sn_code);

        let v_5v_sns_mv = adc_to_mv(sns_5v_code);
        let ts1_mv = adc_to_mv(ts1_code);
        let ts2_mv = adc_to_mv(ts2_code);

        // 负载端电压（近端 / 远端），由差分放大器缩放关系反推：
        //   V_SP = (10/124) * V_load  →  V_load = (124/10) * V_SP
        let v_local_mv = (v_nr_sp_mv * SENSE_GAIN_NUM / SENSE_GAIN_DEN) as i32;
        let v_remote_mv = (v_rmt_sp_mv * SENSE_GAIN_NUM / SENSE_GAIN_DEN) as i32;

        // 模拟板 5V 轨电压：R25=75k (5V→5V_SNS)，R26=10k (5V_SNS→GND)
        //   V_5V_SNS = 5V * 10 / (75+10) = 5V * 10/85
        //   V_5V     = V_5V_SNS * (75+10)/10 = V_5V_SNS * 8.5
        let _v_5v_mv = (v_5v_sns_mv * 85 / 10) as i32;

        // 电流检测：INA193 + 25 mΩ
        //   V_shunt = I * 0.025 Ω
        //   V_out   = 20 * V_shunt = 0.5 * I
        //   I       = 2 * V_out
        let i_local_ma = (2 * cur1_sp_mv) as i32;
        let i_remote_ma = (2 * cur2_sp_mv) as i32;

        let calc_p_mw =
            ((i_local_ma as i64 * v_local_mv as i64) / 1_000).clamp(0, u32::MAX as i64) as u32;

        // 按目标电流线性缩放 DAC 码。标定点：0.5 A → CC_0P5A_DAC_CODE_CH1。
        let mut target_i_local_ma = TARGET_I_LOCAL_MA.load(Ordering::Relaxed);
        if target_i_local_ma < TARGET_I_MIN_MA {
            target_i_local_ma = TARGET_I_MIN_MA;
        }
        if target_i_local_ma > TARGET_I_MAX_MA {
            target_i_local_ma = TARGET_I_MAX_MA;
        }
        let dac_code = ((CC_0P5A_DAC_CODE_CH1 as i32) * target_i_local_ma
            / DEFAULT_TARGET_I_LOCAL_MA)
            .clamp(0, ADC_FULL_SCALE as i32) as u16;
        dac.ch1().set(DacValue::Bit12Right(dac_code));

        // DAC 头间裕度：VREF - V_DAC（便于检查 CC 裁剪空间）
        let dac_v_mv = (dac_code as u32) * ADC_VREF_MV / ADC_FULL_SCALE;
        let dac_headroom_mv = (ADC_VREF_MV.saturating_sub(dac_v_mv)) as u16;

        // 目标恒流（远端设定，单位 mA），用于 loop_error 与 UI 显示。
        let loop_error = target_i_local_ma - i_local_ma;

        // TS1: NTC on heatsink core between MOSFETs → MOS/sink core temperature.
        // TS2: NTC near heatsink exhaust/side wall → exhaust/case temperature.
        let sink_core_temp_mc: i32 = ntc_mv_to_mc(ts1_mv);
        let sink_exhaust_temp_mc: i32 = ntc_mv_to_mc(ts2_mv);
        let mcu_temp_mc: i32 = g4_internal_mcu_temp_to_mc(mcu_temp_code);

        // 将物理量打包为 FastStatus 帧，由数字板 UI 展示。
        let status = FastStatus {
            uptime_ms,
            mode: 1, // 简单视为 CC 模式
            state_flags: STATE_FLAG_REMOTE_ACTIVE | STATE_FLAG_LINK_GOOD,
            enable: true,
            target_value: target_i_local_ma,
            i_local_ma,
            i_remote_ma,
            v_local_mv,
            v_remote_mv,
            calc_p_mw,
            dac_headroom_mv,
            loop_error,
            sink_core_temp_mc,
            sink_exhaust_temp_mc,
            mcu_temp_mc,
            fault_flags: 0,
        };

        if ENABLE_FAST_STATUS_TX {
            let now_ms = timestamp_ms() as u32;
            let quiet_until = QUIET_UNTIL_MS.load(Ordering::Relaxed);
            if now_ms >= quiet_until {
                let frame_len = encode_fast_status_frame(seq, &status, &mut raw_frame)
                    .expect("encode fast_status frame");
                let slip_len =
                    slip_encode(&raw_frame[..frame_len], &mut slip_frame).expect("slip encode");
                let mut tx = uart_tx_shared.lock().await;
                if let Err(_err) = tx.write(&slip_frame[..slip_len]).await {
                    warn!("uart tx error; dropping frame");
                }
                seq = seq.wrapping_add(1);
            }
        }

        uptime_ms = uptime_ms.wrapping_add(FAST_STATUS_PERIOD_MS as u32);
        Timer::after_millis(FAST_STATUS_PERIOD_MS).await;
    }
}

// 旧版 mock FAST_STATUS 生成逻辑已被真实采样逻辑替代，保留占位以防回滚时需要参考。

async fn apply_soft_reset_safing(
    dac: &mut Dac<'static, stm32::peripherals::DAC1, embassy_stm32::mode::Blocking>,
    load_en_ctl: &mut Output<'static>,
    load_en_ts: &mut Output<'static>,
) {
    let reason = SoftResetReason::from(LAST_SOFT_RESET_REASON.load(Ordering::Relaxed));

    load_en_ctl.set_low();
    load_en_ts.set_low();

    TARGET_I_LOCAL_MA.store(0, Ordering::Relaxed);
    dac.ch1().set(DacValue::Bit12Right(0));
    dac.ch2().set(DacValue::Bit12Right(0));

    Timer::after_millis(5).await;

    load_en_ctl.set_high();
    load_en_ts.set_high();

    info!(
        "soft reset applied: reason={:?}, target reset to 0 mA, load re-enabled",
        reason
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
    QUIET_UNTIL_MS.store(
        (timestamp_ms() as u32).saturating_add(500),
        Ordering::Relaxed,
    );
    LAST_SETPOINT_SEQ_VALID.store(false, Ordering::Relaxed);

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

/// UART RX 任务：从数字板接收 SetPoint 帧并更新目标电流（mA）。
#[embassy_executor::task]
async fn uart_setpoint_rx_task(
    mut uart_rx: RingBufferedUartRx<'static>,
    uart_tx: &'static Mutex<CriticalSectionRawMutex, UartTx<'static, UartAsync>>,
) {
    info!(
        "UART setpoint RX task starting (msg_id=0x{:02x}, default_target={} mA, range={}..{} mA)",
        MSG_SET_POINT, DEFAULT_TARGET_I_LOCAL_MA, TARGET_I_MIN_MA, TARGET_I_MAX_MA
    );

    let mut decoder: SlipDecoder<128> = SlipDecoder::new();
    decoder.reset(); // ensure clean state on startup
    let mut buf = [0u8; 32];
    let mut byte_count: u32 = 0;
    let mut ack_raw = [0u8; 64];
    let mut ack_slip = [0u8; 96];

    // Startup quiet window: ignore traffic for a short period to align buffers.
    QUIET_UNTIL_MS.store(
        (timestamp_ms() as u32).saturating_add(500),
        Ordering::Relaxed,
    );

    // Drain any stale bytes in UART FIFO to avoid misaligned first frame, and
    // resynchronize to the first SLIP_END boundary before starting decode.
    if let Ok(drained) = uart_rx.read(&mut buf).await {
        if drained > 0 {
            info!("SetPoint RX: drained {} stale bytes before start", drained);
        }
    }

    // Wait until we see two consecutive SLIP_END to align to frame boundary.
    let mut end_seen = 0;
    loop {
        match uart_rx.read(&mut buf[..1]).await {
            Ok(1) if buf[0] == SLIP_END => {
                end_seen += 1;
                if end_seen >= 2 {
                    decoder.reset();
                    info!("SetPoint RX: synced to double SLIP_END boundary");
                    break;
                }
            }
            Ok(_) => end_seen = 0,
            Err(_) => break,
        }
    }

    // Dump first RAW_DUMP_BYTES of raw UART stream for debugging, without decoding.
    let mut raw_dump = [0u8; RAW_DUMP_BYTES];
    let mut raw_len = 0usize;
    let dump_start = timestamp_ms() as u32;
    while raw_len < RAW_DUMP_BYTES {
        match uart_rx.read(&mut buf).await {
            Ok(n) if n > 0 => {
                let take = core::cmp::min(n, RAW_DUMP_BYTES - raw_len);
                raw_dump[raw_len..raw_len + take].copy_from_slice(&buf[..take]);
                raw_len += take;
            }
            _ => {}
        }
        if (timestamp_ms() as u32).wrapping_sub(dump_start) > 300 {
            break;
        }
    }
    if raw_len > 0 {
        info!(
            "SetPoint RX raw_dump len={} data={=[u8]:#04x}",
            raw_len,
            &raw_dump[..raw_len]
        );
    }
    decoder.reset();

    loop {
        match uart_rx.read(&mut buf).await {
            Ok(n) if n > 0 => {
                for &b in &buf[..n] {
                    byte_count = byte_count.wrapping_add(1);
                    if byte_count <= 32 {
                        info!("SetPoint RX: byte[{}]=0x{:02x}", byte_count, b);
                    }
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
                            info!(
                                "SetPoint RX: SLIP frame len={}, head={=[u8]:#04x}",
                                frame.len(),
                                &frame[..frame.len().min(16)]
                            );
                            match decode_set_point_frame(&frame) {
                                Ok((hdr, sp)) => {
                                    let mut v = sp.target_i_ma;
                                    if v < TARGET_I_MIN_MA {
                                        v = TARGET_I_MIN_MA;
                                    }
                                    if v > TARGET_I_MAX_MA {
                                        v = TARGET_I_MAX_MA;
                                    }
                                    let last_seq = LAST_SETPOINT_SEQ.load(Ordering::Relaxed);
                                    let last_valid =
                                        LAST_SETPOINT_SEQ_VALID.load(Ordering::Relaxed);
                                    let is_dup = last_valid && hdr.seq == last_seq;

                                    if !is_dup {
                                        let prev = TARGET_I_LOCAL_MA.swap(v, Ordering::Relaxed);
                                        LAST_SETPOINT_SEQ.store(hdr.seq, Ordering::Relaxed);
                                        LAST_SETPOINT_SEQ_VALID.store(true, Ordering::Relaxed);
                                        info!(
                                            "SetPoint received: target_i_ma={} mA (prev={} mA, seq={})",
                                            v, prev, hdr.seq
                                        );
                                    } else if !last_valid {
                                        // First frame must establish seq; if not valid yet, reset decoder.
                                        warn!(
                                            "SetPoint RX: seq invalid on first frame, resetting decoder"
                                        );
                                        LAST_SETPOINT_SEQ_VALID.store(false, Ordering::Relaxed);
                                        decoder.reset();
                                        continue;
                                    } else {
                                        info!(
                                            "SetPoint duplicate received: seq={} target_i_ma={} mA (ignored, ack only)",
                                            hdr.seq, v
                                        );
                                    }

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
                warn!("uart rx error in SetPoint task: {:?}", err);
                decoder.reset();
            }
        }
    }
}

#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32 as stm32;
use embassy_stm32::bind_interrupts;
use embassy_stm32::usart::{Config as UartConfig, Uart};
use embassy_time::{Instant, Timer};
use loadlynx_protocol::{FastStatus, encode_fast_status_frame, slip_encode};

bind_interrupts!(struct Irqs {
    USART3 => stm32::usart::InterruptHandler<stm32::peripherals::USART3>;
});

const FAST_STATUS_PERIOD_MS: u64 = 1000 / 60; // ≈60 Hz
const STATE_FLAG_REMOTE_ACTIVE: u32 = 1 << 0;
const STATE_FLAG_LINK_GOOD: u32 = 1 << 1;

fn timestamp_ms() -> u64 {
    Instant::now().as_millis() as u64
}

defmt::timestamp!("{=u64:ms}", timestamp_ms());

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = stm32::init(Default::default());

    info!("LoadLynx analog alive; streaming mock FAST_STATUS frames");

    let mut uart_cfg = UartConfig::default();
    uart_cfg.baudrate = 230_400;

    let mut uart = Uart::new(
        p.USART3, p.PC11, p.PC10, Irqs, p.DMA1_CH1, p.DMA1_CH2, uart_cfg,
    )
    .unwrap();

    let mut seq: u8 = 0;
    let mut uptime_ms: u32 = 0;
    let mut raw_frame = [0u8; 192];
    let mut slip_frame = [0u8; 384];

    loop {
        let status = build_mock_status(uptime_ms, seq);
        let frame_len = encode_fast_status_frame(seq, &status, &mut raw_frame)
            .expect("encode fast_status frame");
        let slip_len = slip_encode(&raw_frame[..frame_len], &mut slip_frame).expect("slip encode");
        if let Err(_err) = uart.write(&slip_frame[..slip_len]).await {
            warn!("uart tx error; dropping frame");
        }

        seq = seq.wrapping_add(1);
        uptime_ms = uptime_ms.wrapping_add(FAST_STATUS_PERIOD_MS as u32);
        Timer::after_millis(FAST_STATUS_PERIOD_MS).await;
    }
}

fn build_mock_status(uptime_ms: u32, seq: u8) -> FastStatus {
    let ramp = triangle_wave(uptime_ms);
    let v_local_mv = 24_500 + ramp * 8;
    let v_remote_mv = v_local_mv + 18;
    let i_local_ma = 8_000 + ramp * 20;
    let i_remote_ma = 7_500 + ramp * 18;
    let calc_p_mw =
        ((i_local_ma as i64 * v_local_mv as i64) / 1_000).clamp(0, u32::MAX as i64) as u32;
    let sink_core_temp_mc = 40_000 + ramp * 120;
    let sink_exhaust_temp_mc = 37_000 + ramp * 60;
    let mcu_temp_mc = 34_000 + ramp * 45;
    let target_value = i_local_ma + 200;
    let loop_error = target_value - i_local_ma;

    FastStatus {
        uptime_ms,
        mode: 1,
        state_flags: STATE_FLAG_REMOTE_ACTIVE | STATE_FLAG_LINK_GOOD,
        enable: true,
        target_value,
        i_local_ma,
        i_remote_ma,
        v_local_mv,
        v_remote_mv,
        calc_p_mw,
        dac_headroom_mv: 180,
        loop_error,
        sink_core_temp_mc,
        sink_exhaust_temp_mc,
        mcu_temp_mc,
        fault_flags: if seq % 240 == 0 { 0x1 } else { 0 },
    }
}

fn triangle_wave(uptime_ms: u32) -> i32 {
    let phase = ((uptime_ms / 50) % 200) as i32;
    if phase < 100 { phase - 50 } else { 150 - phase }
}

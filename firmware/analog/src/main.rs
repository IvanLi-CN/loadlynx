#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32 as stm32;
use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::bind_interrupts;
use embassy_stm32::dac::{Dac, Value as DacValue};
use embassy_stm32::gpio::{Level, Output, Speed};
use stm32_metapac::vrefbuf::vals::{Hiz, Vrs};
use stm32_metapac::VREFBUF;
use embassy_stm32::usart::{Config as UartConfig, Uart};
use embassy_time::{Duration, Ticker};
use heapless::Vec;

// UART link protocol constants (see docs/interfaces/uart-link.md).
const PROTO_VER: u8 = 0x01;
const MSG_STATUS: u8 = 0x10;

bind_interrupts!(struct Irqs {
    USART3 => stm32::usart::InterruptHandler<stm32::peripherals::USART3>;
});

// The current monitor path is: I_load -> R_SHUNT=25 mΩ -> INA193 (gain=20 V/V),
// so V_CUR_SENSE = 20 * 0.025 Ω * I = 0.5 * I [V/A]. The CC error amp (U18)
// compares this sense voltage against the DAC setpoint at its + input
// (via R10=100 Ω), with the inverting input fed from CUR1_SP via R9=100 Ω.
// At DC the large 100 kΩ feedback (R31) mainly provides bias/stability, so
// the loop approximately enforces V_DAC ≈ V_CUR_SENSE.
//
// For I = 0.5 A:
//   V_CUR_SENSE ≈ 0.5 * 0.5 A = 0.25 V
// DAC is 12-bit referenced to VREF+ driven by VREFBUF. For STM32G431 with
// VREFBUF configured to 2.9 V (Vrs::VREF2):
//   CODE_0p5A ≈ 0.25 / 2.9 * (2^12 - 1) ≈ 353.
//
// This is a first-order design value; final accuracy still depends on shunt
// tolerance, INA193 gain error, and loop compensation, and should be trimmed
// in firmware if tighter than a few percent is required.
const CC_0P5A_DAC_CODE_CH1: u16 = 353;

fn crc16_ccitt_false(data: &[u8]) -> u16 {
    // poly 0x1021, init 0xFFFF, no reflection, no xorout
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

fn slip_encode(src: &[u8], dst: &mut Vec<u8, 256>) {
    const END: u8 = 0xC0;
    const ESC: u8 = 0xDB;
    const ESC_END: u8 = 0xDC;
    const ESC_ESC: u8 = 0xDD;

    dst.clear();
    let _ = dst.push(END);
    for &b in src {
        match b {
            END => {
                let _ = dst.push(ESC);
                let _ = dst.push(ESC_END);
            }
            ESC => {
                let _ = dst.push(ESC);
                let _ = dst.push(ESC_ESC);
            }
            _ => {
                let _ = dst.push(b);
            }
        }
    }
    let _ = dst.push(END);
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    // Use Embassy's default clock tree, but make sure ADC1/2 get a valid clock
    // source via the ADC12 mux. This avoids time-driver panics while still
    // providing sufficient performance for slow ADC sampling / CC bring-up.
    let mut config = stm32::Config::default();
    {
        use embassy_stm32::rcc::mux;
        config.rcc.mux.adc12sel = mux::Adcsel::SYS;
    }
    let p = stm32::init(config);

    info!("LoadLynx analog alive; init VREFBUF/ADC/DAC/UART (CC bring-up)");

    // Configure the internal voltage reference buffer (VREFBUF) to drive
    // VREF+ at approximately 2.9 V (VRS=VREF2) with low output impedance.
    // This VREF+ is used as reference for both ADC and DAC.
    //
    // Per RM0440 / AN5093 and stm32-metapac `vrefbuf_v2b`:
    //   VREF0 ≈ 2.048 V, VREF1 ≈ 2.5 V, VREF2 ≈ 2.9 V.
    let vrefbuf = VREFBUF;
    vrefbuf.csr().modify(|csr| {
        csr.set_hiz(Hiz::CONNECTED);
        csr.set_vrs(Vrs::VREF2);
        csr.set_envr(true);
    });
    while !vrefbuf.csr().read().vrr() {
        // wait for voltage reference buffer to become ready
    }

    // Temporary bring-up: force LOAD_EN high by driving both LOAD_EN_CTL (PB13)
    // and LOAD_EN_TS (PB14) high as plain GPIO outputs. This bypasses the COMP4
    // over-temperature gating and is only intended for lab testing of the CC
    // path (TPS22810 → OVCC → op-amps → MOSFETs).
    //
    // Hardware mapping (see docs/thermal/over-temperature-protection.md):
    // - PB13 = LOAD_EN_CTL (MCU intent)
    // - PB14 = LOAD_EN_TS / COMP4_OUT (COMP_OK)
    // - LOAD_EN = LOAD_EN_CTL AND LOAD_EN_TS → TPS22810 EN/UVLO
    //
    // For now we set both high so LOAD_EN = 1.
    let mut load_en_ctl = Output::new(p.PB13, Level::High, Speed::Low);
    let mut load_en_ts = Output::new(p.PB14, Level::High, Speed::Low);
    load_en_ctl.set_high();
    load_en_ts.set_high();

    // UART3: keep a minimal TX channel for debug/telemetry.
    let mut uart_cfg = UartConfig::default();
    uart_cfg.baudrate = 115_200;

    let mut uart = Uart::new(
        p.USART3,
        p.PC11, // RX
        p.PC10, // TX
        Irqs,
        p.DMA1_CH1,
        p.DMA1_CH2,
        uart_cfg,
    )
    .unwrap();

    // ADC1/ADC2 setup (blocking read, no DMA): we only need modest sample rate.
    let mut adc1 = Adc::new(p.ADC1);
    let mut adc2 = Adc::new(p.ADC2);

    adc1.set_sample_time(SampleTime::CYCLES247_5);
    adc2.set_sample_time(SampleTime::CYCLES247_5);

    // Channel mapping based on loadlynx.ioc labels:
    // - PA0/PA1: V_RMT_SP / V_RMT_SN (remote voltage sense)
    // - PA2/PA3: V_NR_SP / V_NR_SN (near voltage sense)
    // - PA6/PA7: CUR1_SP / CUR1_SN (channel 1 current sense, differential pair)
    // - PB11/PB15: CUR2_SP / CUR2_SN (channel 2 current sense, differential pair)
    // - PB12: _5V_SNS (5 V rail sense)
    // - PB0/PB1: TS1 / TS2 (temperature sensors)
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

    // DAC1: use CH1 (PA4) for channel 1 CC setpoint, explicitly force CH2 to 0
    // so only one load channel is active during bring-up. This avoids pushing
    // the bench supply into current limit when both channels try to pull 0.5 A.
    let mut dac = Dac::new_blocking(p.DAC1, p.PA4, p.PA5);
    dac.ch1().set(DacValue::Bit12Right(CC_0P5A_DAC_CODE_CH1));
    dac.ch2().set(DacValue::Bit12Right(0));

    info!(
        "CC setpoint CH1: target 0.5 A (DAC code = {}) – requires hardware calibration",
        CC_0P5A_DAC_CODE_CH1
    );

    // Periodic sampler: log raw ADC codes locally, and send derived voltages
    // and currents over UART3 to the digital board using the CBOR/SLIP-based
    // Status frame defined in docs/interfaces/uart-link.md.
    let mut ticker = Ticker::every(Duration::from_millis(200));
    let mut seq: u8 = 0;
    let mut ts_ms: u32 = 0;

    loop {
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

        info!(
            "ADC raw: VRMT+={} VRMT-={} VNR+={} VNR-={} I1+={} I1-={} I2+={} I2-={} 5V={} TS1={} TS2={}",
            v_rmt_sp_code,
            v_rmt_sn_code,
            v_nr_sp_code,
            v_nr_sn_code,
            cur1_sp_code,
            cur1_sn_code,
            cur2_sp_code,
            cur2_sn_code,
            sns_5v_code,
            ts1_code,
            ts2_code
        );

        // --- Derived quantities (purely from netlist + datasheet formulas) ---
        //
        // ADC reference: VREF = 2.9 V (VREFBUF VRS=VREF2).
        // 12-bit ADC: code ∈ [0, 4095], V = code/4095 * VREF.
        //
        // Current sense (per channel, INA193 + 25 mΩ shunt):
        //   V_shunt = I * R = I * 25 mΩ
        //   V_out   = G * V_shunt = 20 * 25 mΩ * I = 0.5 * I
        //   I       = 2 * V_out
        //
        // We express voltages in millivolts and currents in milliamps.

        const VREF_MV: u32 = 2900;
        const ADC_FULL_SCALE: u32 = 4095;

        // Helper closures for fixed-point conversions.
        let adc_to_mv = |code: u16| -> u32 {
            (code as u32) * VREF_MV / ADC_FULL_SCALE
        };
        let sense_code_to_shunt_mv = |code: u16| -> u32 {
            // V_shunt = V_out / 20, with V_out from ADC.
            adc_to_mv(code) / 20
        };
        let sense_code_to_ma = |code: u16| -> u32 {
            // I = 2 * V_out, V_out in volts. In mV/mA: I[mA] = 2 * V_out[mV].
            2 * adc_to_mv(code)
        };

        // Channel 1 load current and shunt voltage.
        let cur1_sp_mv = adc_to_mv(cur1_sp_code);
        let cur1_shunt_mv = sense_code_to_shunt_mv(cur1_sp_code);
        let cur1_ma = sense_code_to_ma(cur1_sp_code);

        // Channel 2 load current and shunt voltage.
        let cur2_sp_mv = adc_to_mv(cur2_sp_code);
        let cur2_shunt_mv = sense_code_to_shunt_mv(cur2_sp_code);
        let cur2_ma = sense_code_to_ma(cur2_sp_code);

        // Near / remote sense node voltages at ADC pins.
        let v_nr_sp_mv = adc_to_mv(v_nr_sp_code);
        let v_nr_sn_mv = adc_to_mv(v_nr_sn_code);
        let v_rmt_sp_mv = adc_to_mv(v_rmt_sp_code);
        let v_rmt_sn_mv = adc_to_mv(v_rmt_sn_code);

        // 5V rail monitor node.
        let v_5v_sns_mv = adc_to_mv(sns_5v_code);

        // Build CBOR payload for Status message:
        // array[6]: [ts_ms, ch1_i_mA, ch2_i_mA, vnr_sp_mV, vrmt_sp_mV, v5sns_mV]
        let mut payload: Vec<u8, 64> = Vec::new();
        // CBOR array(6) = 0x86
        let _ = payload.push(0x86);
        let mut push_u32 = |buf: &mut Vec<u8, 64>, val: u32| {
            // positive int, 32-bit: major type 0, additional 26 (0x1A), big-endian
            let _ = buf.push(0x1A);
            let _ = buf.push((val >> 24) as u8);
            let _ = buf.push((val >> 16) as u8);
            let _ = buf.push((val >> 8) as u8);
            let _ = buf.push(val as u8);
        };
        push_u32(&mut payload, ts_ms);
        push_u32(&mut payload, cur1_ma);
        push_u32(&mut payload, cur2_ma);
        push_u32(&mut payload, v_nr_sp_mv);
        push_u32(&mut payload, v_rmt_sp_mv);
        push_u32(&mut payload, v_5v_sns_mv);

        // Frame header (5 bytes) + payload + CRC16 (2 bytes)
        let mut frame: Vec<u8, 80> = Vec::new();
        let flags: u8 = 0; // Status: no ACK/RESP
        let msg: u8 = MSG_STATUS;
        let len: u16 = (payload.len() as u16) + 2; // payload + CRC16

        let _ = frame.push(PROTO_VER);
        let _ = frame.push(flags);
        let _ = frame.push(seq);
        let _ = frame.push(msg);
        let _ = frame.push((len & 0xFF) as u8);
        let _ = frame.push((len >> 8) as u8);
        // CBOR payload
        for b in payload.iter() {
            let _ = frame.push(*b);
        }
        // CRC16 over header + payload
        let crc = crc16_ccitt_false(&frame[..]);
        // Append CRC16 little-endian
        let _ = frame.push((crc & 0xFF) as u8);
        let _ = frame.push((crc >> 8) as u8);

        // SLIP-encode and send over UART3.
        let mut slip_buf: Vec<u8, 256> = Vec::new();
        slip_encode(&frame[..], &mut slip_buf);
        let _ = uart.write(&slip_buf[..]).await;

        // Advance sequence & timestamp.
        seq = seq.wrapping_add(1);
        ts_ms = ts_ms.wrapping_add(200);

        ticker.next().await;
    }
}

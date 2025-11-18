use loadlynx_protocol::{
    FastStatus, SlipDecoder, decode_fast_status_frame, encode_fast_status_frame, slip_encode,
};

fn main() {
    let status = FastStatus {
        uptime_ms: 123456,
        mode: 1,
        state_flags: 0x03,
        enable: true,
        target_value: 8_200,
        i_local_ma: 8_000,
        i_remote_ma: 7_500,
        v_local_mv: 24_700,
        v_remote_mv: 24_718,
        calc_p_mw: 198_000,
        dac_headroom_mv: 180,
        loop_error: 200,
        sink_core_temp_mc: 45_000,
        sink_exhaust_temp_mc: 39_000,
        mcu_temp_mc: 36_000,
        fault_flags: 0,
    };
    let mut raw = [0u8; 256];
    let len = encode_fast_status_frame(0x42, &status, &mut raw).unwrap();
    let mut slip = [0u8; 512];
    let slip_len = slip_encode(&raw[..len], &mut slip).unwrap();
    let mut decoder: SlipDecoder<512> = SlipDecoder::new();
    for b in &slip[..slip_len] {
        if let Some(frame) = decoder.push(*b).unwrap() {
            println!("frame recovered len {}", frame.len());
            let (_header, decoded) = decode_fast_status_frame(&frame).unwrap();
            println!("uptime {}", decoded.uptime_ms);
        }
    }
}

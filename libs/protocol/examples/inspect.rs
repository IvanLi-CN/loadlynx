use loadlynx_protocol::{FastStatus, encode_fast_status_frame, slip_encode};

fn main() {
    let status = FastStatus::default();
    let mut raw = [0u8; 256];
    let len = encode_fast_status_frame(0x63, &status, &mut raw).unwrap();
    println!("frame len {}", len);
    println!("raw head: {:02x?}", &raw[..len.min(16)]);
    let mut slip = [0u8; 512];
    let slip_len = slip_encode(&raw[..len], &mut slip).unwrap();
    println!("slip len {}", slip_len);
    println!("slip head: {:02x?}", &slip[..slip_len.min(32)]);
    let end_count = slip.iter().filter(|&&b| b == 0xC0).count();
    println!("slip END count {}", end_count);
}

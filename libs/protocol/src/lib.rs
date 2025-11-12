#![no_std]

use heapless::Vec;
use minicbor::decode::Error as CborDecodeError;
use minicbor::encode::{
    Error as CborEncodeError,
    write::{Cursor, EndOfSlice},
};
use minicbor::{Decode, Encode};

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 6;
pub const CRC_LEN: usize = 2;

pub const FLAG_ACK_REQ: u8 = 0x01;
pub const FLAG_IS_ACK: u8 = 0x02;
pub const FLAG_IS_NACK: u8 = 0x04;
pub const FLAG_IS_RESP: u8 = 0x08;

pub const MSG_FAST_STATUS: u8 = 0x10;

pub const SLIP_END: u8 = 0xC0;
pub const SLIP_ESC: u8 = 0xDB;
pub const SLIP_ESC_END: u8 = 0xDC;
pub const SLIP_ESC_ESC: u8 = 0xDD;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameHeader {
    pub version: u8,
    pub flags: u8,
    pub seq: u8,
    pub msg: u8,
    pub len: u16,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
#[cbor(map)]
pub struct FastStatus {
    #[n(0)]
    pub uptime_ms: u32,
    #[n(1)]
    pub mode: u8,
    #[n(2)]
    pub state_flags: u32,
    #[n(3)]
    pub enable: bool,
    #[n(4)]
    pub target_value: i32,
    #[n(5)]
    pub i_local_ma: i32,
    #[n(6)]
    pub i_remote_ma: i32,
    #[n(7)]
    pub v_local_mv: i32,
    #[n(8)]
    pub v_remote_mv: i32,
    #[n(9)]
    pub calc_p_mw: u32,
    #[n(10)]
    pub dac_headroom_mv: u16,
    #[n(11)]
    pub loop_error: i32,
    #[n(12)]
    pub sink_core_temp_mc: i32,
    #[n(13)]
    pub sink_exhaust_temp_mc: i32,
    #[n(14)]
    pub mcu_temp_mc: i32,
    #[n(15)]
    pub fault_flags: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    BufferTooSmall,
    PayloadTooLarge,
    InvalidVersion(u8),
    LengthMismatch,
    InvalidPayloadLength,
    UnsupportedMessage(u8),
    CborEncode,
    CborDecode,
    InvalidCrc,
    SlipFrameTooLarge,
    SlipInvalidEscape(u8),
}

pub fn encode_fast_status_frame(
    seq: u8,
    status: &FastStatus,
    out: &mut [u8],
) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = 0;
    out[2] = seq;
    out[3] = MSG_FAST_STATUS;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(status).map_err(map_encode_err)?;
        cursor.position()
    };
    if payload_len > u16::MAX as usize {
        return Err(Error::PayloadTooLarge);
    }

    let len_bytes = (payload_len as u16).to_le_bytes();
    out[4] = len_bytes[0];
    out[5] = len_bytes[1];

    let frame_len_without_crc = HEADER_LEN + payload_len;
    if frame_len_without_crc + CRC_LEN > out.len() {
        return Err(Error::BufferTooSmall);
    }

    let crc = crc16_ccitt_false(&out[..frame_len_without_crc]);
    let crc_bytes = crc.to_le_bytes();
    out[frame_len_without_crc] = crc_bytes[0];
    out[frame_len_without_crc + 1] = crc_bytes[1];
    Ok(frame_len_without_crc + CRC_LEN)
}

pub fn decode_fast_status_frame(frame: &[u8]) -> Result<(FrameHeader, FastStatus), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_FAST_STATUS {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let status: FastStatus = decoder.decode().map_err(map_decode_err)?;
    Ok((header, status))
}

pub fn decode_frame(buf: &[u8]) -> Result<(FrameHeader, &[u8]), Error> {
    if buf.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::LengthMismatch);
    }

    let version = buf[0];
    if version != PROTOCOL_VERSION {
        return Err(Error::InvalidVersion(version));
    }

    let flags = buf[1];
    let seq = buf[2];
    let msg = buf[3];
    let len = u16::from_le_bytes([buf[4], buf[5]]);
    let payload_len = len as usize;
    let expected_total = HEADER_LEN + payload_len + CRC_LEN;
    if expected_total != buf.len() {
        return Err(Error::InvalidPayloadLength);
    }

    let payload = &buf[HEADER_LEN..HEADER_LEN + payload_len];
    let crc_frame = u16::from_le_bytes([
        buf[HEADER_LEN + payload_len],
        buf[HEADER_LEN + payload_len + 1],
    ]);
    let crc_calc = crc16_ccitt_false(&buf[..HEADER_LEN + payload_len]);
    if crc_calc != crc_frame {
        return Err(Error::InvalidCrc);
    }

    Ok((
        FrameHeader {
            version,
            flags,
            seq,
            msg,
            len,
        },
        payload,
    ))
}

pub fn crc16_ccitt_false(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in bytes {
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

fn map_encode_err(err: CborEncodeError<EndOfSlice>) -> Error {
    if err.is_write() {
        Error::BufferTooSmall
    } else {
        Error::CborEncode
    }
}

fn map_decode_err(_: CborDecodeError) -> Error {
    Error::CborDecode
}

pub fn slip_encode(frame: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let mut idx = 0;
    ensure_capacity(out, idx)?;
    out[idx] = SLIP_END;
    idx += 1;

    for &byte in frame {
        match byte {
            SLIP_END => {
                ensure_capacity(out, idx + 1)?;
                out[idx] = SLIP_ESC;
                out[idx + 1] = SLIP_ESC_END;
                idx += 2;
            }
            SLIP_ESC => {
                ensure_capacity(out, idx + 1)?;
                out[idx] = SLIP_ESC;
                out[idx + 1] = SLIP_ESC_ESC;
                idx += 2;
            }
            _ => {
                ensure_capacity(out, idx)?;
                out[idx] = byte;
                idx += 1;
            }
        }
    }

    ensure_capacity(out, idx)?;
    out[idx] = SLIP_END;
    Ok(idx + 1)
}

pub struct SlipDecoder<const N: usize> {
    buffer: Vec<u8, N>,
    escaping: bool,
}

impl<const N: usize> SlipDecoder<N> {
    pub const fn new() -> Self {
        Self {
            buffer: Vec::new(),
            escaping: false,
        }
    }

    pub fn push(&mut self, byte: u8) -> Result<Option<Vec<u8, N>>, Error> {
        if self.escaping {
            self.escaping = false;
            let decoded = match byte {
                SLIP_ESC_END => SLIP_END,
                SLIP_ESC_ESC => SLIP_ESC,
                other => return Err(Error::SlipInvalidEscape(other)),
            };
            self.buffer
                .push(decoded)
                .map_err(|_| Error::SlipFrameTooLarge)?;
            return Ok(None);
        }

        match byte {
            SLIP_END => {
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                let mut frame = Vec::new();
                frame
                    .extend_from_slice(&self.buffer)
                    .map_err(|_| Error::SlipFrameTooLarge)?;
                self.buffer.clear();
                Ok(Some(frame))
            }
            SLIP_ESC => {
                self.escaping = true;
                Ok(None)
            }
            _ => {
                self.buffer
                    .push(byte)
                    .map_err(|_| Error::SlipFrameTooLarge)?;
                Ok(None)
            }
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.escaping = false;
    }
}

fn ensure_capacity(out: &[u8], idx: usize) -> Result<(), Error> {
    if idx >= out.len() {
        Err(Error::BufferTooSmall)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_status_roundtrip() {
        let status = FastStatus {
            uptime_ms: 1234,
            mode: 2,
            state_flags: 0x01,
            enable: true,
            target_value: 2500,
            i_local_ma: 2000,
            i_remote_ma: 1980,
            v_local_mv: 24500,
            v_remote_mv: 24600,
            calc_p_mw: 49_000,
            dac_headroom_mv: 120,
            loop_error: -12,
            sink_core_temp_mc: 45000,
            sink_exhaust_temp_mc: 41000,
            mcu_temp_mc: 38000,
            fault_flags: 0,
        };

        let mut raw = [0u8; 192];
        let len = encode_fast_status_frame(7, &status, &mut raw).unwrap();
        let (header, decoded) = decode_fast_status_frame(&raw[..len]).unwrap();
        assert_eq!(header.version, PROTOCOL_VERSION);
        assert_eq!(header.seq, 7);
        assert_eq!(decoded.v_remote_mv, status.v_remote_mv);

        let mut slip_buf = [0u8; 256];
        let slip_len = slip_encode(&raw[..len], &mut slip_buf).unwrap();
        let mut decoder: SlipDecoder<256> = SlipDecoder::new();
        let mut recovered = None;
        for byte in &slip_buf[..slip_len] {
            if let Some(frame) = decoder.push(*byte).unwrap() {
                recovered = Some(frame);
            }
        }
        let recovered = recovered.expect("frame not recovered");
        assert_eq!(&recovered[..], &raw[..len]);
    }

    #[test]
    fn crc_mismatch_detected() {
        let status = FastStatus::default();
        let mut raw = [0u8; 128];
        let len = encode_fast_status_frame(0, &status, &mut raw).unwrap();
        raw[HEADER_LEN] ^= 0xFF; // flip a payload byte
        let err = decode_fast_status_frame(&raw[..len]).unwrap_err();
        assert!(matches!(err, Error::InvalidCrc));
    }

    #[test]
    fn version_mismatch_detected() {
        let status = FastStatus::default();
        let mut raw = [0u8; 96];
        let len = encode_fast_status_frame(1, &status, &mut raw).unwrap();
        raw[0] = PROTOCOL_VERSION.wrapping_add(1);
        let err = decode_fast_status_frame(&raw[..len]).unwrap_err();
        assert!(matches!(err, Error::InvalidVersion(_)));
    }

    #[test]
    fn slip_roundtrip_with_escaping() {
        let frame = [SLIP_END, SLIP_ESC, 0x01, 0x02, SLIP_ESC];
        let mut encoded = [0u8; 32];
        let written = slip_encode(&frame, &mut encoded).unwrap();
        let mut decoder: SlipDecoder<32> = SlipDecoder::new();
        let mut recovered = None;
        for &byte in &encoded[..written] {
            if let Some(frame) = decoder.push(byte).unwrap() {
                recovered = Some(frame);
            }
        }
        let recovered = recovered.expect("frame not recovered");
        assert_eq!(&recovered[..], &frame[..]);
    }

    #[test]
    fn slip_encode_only_escapes_reserved_bytes() {
        let frame = [0x00, 0x11, SLIP_END, SLIP_ESC, 0xFF];
        let mut encoded = [0u8; 32];
        let written = slip_encode(&frame, &mut encoded).unwrap();

        // Frames must start and end with SLIP_END once encoded.
        assert_eq!(encoded[0], SLIP_END);
        assert_eq!(encoded[written - 1], SLIP_END);

        // Ensure reserved bytes are escaped and ordinary bytes are untouched.
        let mut saw_end_escape = false;
        let mut saw_esc_escape = false;
        for window in encoded[..written].windows(2) {
            match window {
                [SLIP_ESC, SLIP_ESC_END] => saw_end_escape = true,
                [SLIP_ESC, SLIP_ESC_ESC] => saw_esc_escape = true,
                [SLIP_ESC, other] => panic!("unexpected escape sequence 0x{:02X}", other),
                _ => {}
            }
        }

        assert!(saw_end_escape, "SLIP_END must be escaped");
        assert!(saw_esc_escape, "SLIP_ESC must be escaped");
    }

    #[test]
    fn slip_invalid_escape_is_error() {
        let mut decoder: SlipDecoder<8> = SlipDecoder::new();
        assert!(decoder.push(SLIP_ESC).unwrap().is_none());
        let err = decoder.push(0x00).unwrap_err();
        assert!(matches!(err, Error::SlipInvalidEscape(0x00)));
    }

    #[test]
    fn slip_overflow_detected() {
        let mut decoder: SlipDecoder<4> = SlipDecoder::new();
        for _ in 0..4 {
            decoder.push(0x11).unwrap();
        }
        let err = decoder.push(0x22).unwrap_err();
        assert!(matches!(err, Error::SlipFrameTooLarge));
    }

    #[test]
    fn encode_rejects_small_buffer() {
        let status = FastStatus::default();
        let mut raw = [0u8; HEADER_LEN];
        let err = encode_fast_status_frame(0, &status, &mut raw).unwrap_err();
        assert!(matches!(err, Error::BufferTooSmall));
    }

    #[test]
    fn decode_rejects_length_mismatch() {
        let status = FastStatus::default();
        let mut raw = [0u8; 96];
        let len = encode_fast_status_frame(0, &status, &mut raw).unwrap();
        let mut truncated = [0u8; 128];
        truncated[..len].copy_from_slice(&raw[..len]);
        truncated[4] = 0;
        truncated[5] = 0;
        let err = decode_frame(&truncated[..len]).unwrap_err();
        assert!(matches!(err, Error::InvalidPayloadLength));
    }
}

#![no_std]

use heapless::Vec;
use minicbor::decode::Error as CborDecodeError;
use minicbor::encode::{
    Error as CborEncodeError,
    write::{Cursor, EndOfSlice},
};
use minicbor::{Decode, Decoder, Encode, Encoder};

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 6;
pub const CRC_LEN: usize = 2;

pub const FLAG_ACK_REQ: u8 = 0x01;
pub const FLAG_IS_ACK: u8 = 0x02;
pub const FLAG_IS_NACK: u8 = 0x04;
pub const FLAG_IS_RESP: u8 = 0x08;

pub const MSG_HELLO: u8 = 0x01;
pub const MSG_FAST_STATUS: u8 = 0x10;
pub const MSG_FAULT: u8 = 0x11;
/// SetPoint message: S3 (digital) → G431 (analog)
///
/// This is a minimal control message used to steer the analog board's
/// constant-current setpoint. Units are milliamps (mA) and the value is
/// interpreted as a signed 32-bit integer to leave room for future
/// extensions (e.g. negative values for sink/source modes).
pub const MSG_SET_POINT: u8 = 0x22;
pub const MSG_SET_ENABLE: u8 = 0x20;
pub const MSG_SET_MODE: u8 = 0x21;
pub const MSG_SET_LIMITS: u8 = 0x23;
pub const MSG_GET_STATUS: u8 = 0x24;
/// Soft-reset request/ack handshake initiated by the digital side to reset
/// analog-side state without power-cycling.
pub const MSG_SOFT_RESET: u8 = 0x26;
/// Calibration write message: S3 (digital) → G431 (analog).
pub const MSG_CAL_WRITE: u8 = 0x30;
/// Reserved for future calibration readback support.
pub const MSG_CAL_READ: u8 = 0x31;

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

/// Minimal control payload for adjusting the analog board's current setpoint.
///
/// The value is expressed in milliamps (mA). The analog firmware treats this
/// as the target *local* sink current for channel 1. The digital side is
/// responsible for clamping the value to a sane range for the current
/// hardware (e.g. 0‒1000 mA).
#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
#[cbor(map)]
pub struct SetPoint {
    /// Target current in milliamps.
    #[n(0)]
    pub target_i_ma: i32,
}

/// Reason codes for a soft-reset request initiated by the digital side.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftResetReason {
    Manual,
    FirmwareUpdate,
    UiRecover,
    LinkRecover,
    Unknown(u8),
}

impl From<u8> for SoftResetReason {
    fn from(value: u8) -> Self {
        match value {
            0 => SoftResetReason::Manual,
            1 => SoftResetReason::FirmwareUpdate,
            2 => SoftResetReason::UiRecover,
            3 => SoftResetReason::LinkRecover,
            other => SoftResetReason::Unknown(other),
        }
    }
}

impl From<SoftResetReason> for u8 {
    fn from(reason: SoftResetReason) -> Self {
        match reason {
            SoftResetReason::Manual => 0,
            SoftResetReason::FirmwareUpdate => 1,
            SoftResetReason::UiRecover => 2,
            SoftResetReason::LinkRecover => 3,
            SoftResetReason::Unknown(raw) => raw,
        }
    }
}

impl Default for SoftResetReason {
    fn default() -> Self {
        SoftResetReason::Manual
    }
}

impl<C> Encode<C> for SoftResetReason {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.u8((*self).into())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for SoftResetReason {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let raw = d.u8()?;
        Ok(raw.into())
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
#[cbor(map)]
pub struct SoftReset {
    /// Enumerated reason for the reset, mapped to u8 for forward compatibility.
    #[n(0)]
    pub reason: SoftResetReason,
    /// Timestamp in milliseconds from the sender when the request was issued.
    #[n(1)]
    pub timestamp_ms: u32,
}

/// One-shot HELLO message sent from the analog side to announce protocol/firmware
/// version after power-on or soft-reset safing.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
#[cbor(map)]
pub struct Hello {
    /// Protocol version understood by the sender.
    #[n(0)]
    pub protocol_version: u8,
    /// Compact firmware version identifier (implementation-defined).
    #[n(1)]
    pub fw_version: u32,
}

/// Simple enable/disable control from the digital side to the analog side.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
#[cbor(map)]
pub struct SetEnable {
    #[n(0)]
    pub enable: bool,
}

/// Minimal single-block calibration write payload.
///
/// This is intentionally small and opaque for now; the digital side owns the
/// layout of `payload`. The analog side only gates enable based on successful
/// receipt of at least one `CalWrite` block.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
#[cbor(map)]
pub struct CalWrite {
    /// Chunk index for future multi-block support. For now only 0 is used.
    #[n(0)]
    pub index: u8,
    /// Opaque 32-byte payload owned by the digital side.
    #[n(1)]
    pub payload: [u8; 32],
    /// Optional inner CRC for the payload (e.g. CRC16 over index+payload).
    #[n(2)]
    pub crc: u16,
}

/// Optional GetStatus request used by the digital side to ask for an immediate
/// FastStatus update. The `request_id` field is reserved for correlating a
/// future reply; it is currently unused by the firmware.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
#[cbor(map)]
pub struct GetStatus {
    #[n(0)]
    pub request_id: u8,
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

/// Encode a HELLO frame announcing protocol/firmware version from the analog
/// side. This is typically sent once after power-on or soft-reset safing.
pub fn encode_hello_frame(seq: u8, hello: &Hello, out: &mut [u8]) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = 0;
    out[2] = seq;
    out[3] = MSG_HELLO;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(hello).map_err(map_encode_err)?;
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

/// Encode an ACK or NACK frame with no payload. This is intended for
/// lightweight confirmations such as SetPoint ACKs. `msg` is echoed from the
/// original request; `is_nack` selects between ACK (`FLAG_IS_ACK`) and NACK
/// (`FLAG_IS_NACK`).
pub fn encode_ack_only_frame(
    seq: u8,
    msg: u8,
    is_nack: bool,
    out: &mut [u8],
) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = if is_nack { FLAG_IS_NACK } else { FLAG_IS_ACK };
    out[2] = seq;
    out[3] = msg;
    out[4] = 0;
    out[5] = 0;

    let crc = crc16_ccitt_false(&out[..HEADER_LEN]);
    let crc_bytes = crc.to_le_bytes();
    out[HEADER_LEN] = crc_bytes[0];
    out[HEADER_LEN + 1] = crc_bytes[1];
    Ok(HEADER_LEN + CRC_LEN)
}

/// Encode a `SetPoint` payload into a binary frame with header and CRC,
/// ready for SLIP framing.
pub fn encode_set_point_frame(
    seq: u8,
    setpoint: &SetPoint,
    out: &mut [u8],
) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = FLAG_ACK_REQ; // control frames request ACKs in future revisions
    out[2] = seq;
    out[3] = MSG_SET_POINT;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(setpoint).map_err(map_encode_err)?;
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

/// Encode a `CalWrite` payload into a binary frame with header and CRC.
pub fn encode_cal_write_frame(
    seq: u8,
    cal: &CalWrite,
    out: &mut [u8],
) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = 0;
    out[2] = seq;
    out[3] = MSG_CAL_WRITE;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(cal).map_err(map_encode_err)?;
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

/// Encode a SetEnable control frame from the digital side to the analog side.
pub fn encode_set_enable_frame(seq: u8, cmd: &SetEnable, out: &mut [u8]) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = 0;
    out[2] = seq;
    out[3] = MSG_SET_ENABLE;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(cmd).map_err(map_encode_err)?;
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

/// Encode a GetStatus control frame from the digital side. The analog side may
/// respond by sending an immediate FastStatus frame.
pub fn encode_get_status_frame(seq: u8, req: &GetStatus, out: &mut [u8]) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = 0;
    out[2] = seq;
    out[3] = MSG_GET_STATUS;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(req).map_err(map_encode_err)?;
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

/// Encode a soft-reset frame. Requests set `is_ack=false`; acknowledgements
/// set `is_ack=true`. Requests automatically set `FLAG_ACK_REQ` to request a
/// reply from the analog side.
pub fn encode_soft_reset_frame(
    seq: u8,
    reset: &SoftReset,
    is_ack: bool,
    out: &mut [u8],
) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = if is_ack { FLAG_IS_ACK } else { FLAG_ACK_REQ };
    out[2] = seq;
    out[3] = MSG_SOFT_RESET;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(reset).map_err(map_encode_err)?;
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

/// Decode a HELLO frame and return its header and payload.
pub fn decode_hello_frame(frame: &[u8]) -> Result<(FrameHeader, Hello), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_HELLO {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let hello: Hello = decoder.decode().map_err(map_decode_err)?;
    Ok((header, hello))
}

/// Decode a `SetPoint` frame. Callers are expected to have obtained `frame`
/// either from `decode_frame` + message ID filtering, or from a SLIP decoder
/// that yields full binary frames.
pub fn decode_set_point_frame(frame: &[u8]) -> Result<(FrameHeader, SetPoint), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_SET_POINT {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let setpoint: SetPoint = decoder.decode().map_err(map_decode_err)?;
    Ok((header, setpoint))
}

/// Decode a SetEnable frame.
pub fn decode_set_enable_frame(frame: &[u8]) -> Result<(FrameHeader, SetEnable), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_SET_ENABLE {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let cmd: SetEnable = decoder.decode().map_err(map_decode_err)?;
    Ok((header, cmd))
}

/// Decode a `CalWrite` frame.
pub fn decode_cal_write_frame(frame: &[u8]) -> Result<(FrameHeader, CalWrite), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_CAL_WRITE {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let cal: CalWrite = decoder.decode().map_err(map_decode_err)?;
    Ok((header, cal))
}

/// Decode a GetStatus frame.
pub fn decode_get_status_frame(frame: &[u8]) -> Result<(FrameHeader, GetStatus), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_GET_STATUS {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let req: GetStatus = decoder.decode().map_err(map_decode_err)?;
    Ok((header, req))
}

/// Decode a soft-reset frame (request or ACK). Callers should inspect
/// `header.flags` to distinguish requests from acknowledgements.
pub fn decode_soft_reset_frame(frame: &[u8]) -> Result<(FrameHeader, SoftReset), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_SOFT_RESET {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let reset: SoftReset = decoder.decode().map_err(map_decode_err)?;
    Ok((header, reset))
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
    fn set_point_roundtrip() {
        let setpoint = SetPoint { target_i_ma: 600 };

        let mut raw = [0u8; 64];
        let len = encode_set_point_frame(3, &setpoint, &mut raw).unwrap();
        let (header, decoded) = decode_set_point_frame(&raw[..len]).unwrap();
        assert_eq!(header.version, PROTOCOL_VERSION);
        assert_eq!(header.seq, 3);
        assert_eq!(header.msg, MSG_SET_POINT);
        assert_eq!(decoded.target_i_ma, setpoint.target_i_ma);

        let mut slip_buf = [0u8; 96];
        let slip_len = slip_encode(&raw[..len], &mut slip_buf).unwrap();
        let mut decoder: SlipDecoder<96> = SlipDecoder::new();
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
    fn ack_only_frame_roundtrip() {
        let mut raw = [0u8; 16];
        let len = encode_ack_only_frame(5, MSG_SET_POINT, false, &mut raw).unwrap();
        assert_eq!(len, HEADER_LEN + CRC_LEN);

        let (hdr, payload) = decode_frame(&raw[..len]).unwrap();
        assert_eq!(hdr.seq, 5);
        assert_eq!(hdr.msg, MSG_SET_POINT);
        assert_eq!(hdr.flags & FLAG_IS_ACK, FLAG_IS_ACK);
        assert_eq!(payload.len(), 0);
    }

    #[test]
    fn soft_reset_roundtrip_and_ack_flag() {
        let reset = SoftReset {
            reason: SoftResetReason::FirmwareUpdate,
            timestamp_ms: 123_456,
        };

        let mut raw = [0u8; 64];
        let len = encode_soft_reset_frame(9, &reset, false, &mut raw).unwrap();
        let (hdr, decoded) = decode_soft_reset_frame(&raw[..len]).unwrap();
        assert_eq!(hdr.msg, MSG_SOFT_RESET);
        assert_eq!(hdr.flags & FLAG_ACK_REQ, FLAG_ACK_REQ);
        assert_eq!(decoded.timestamp_ms, reset.timestamp_ms);

        let mut ack_raw = [0u8; 64];
        let ack_len = encode_soft_reset_frame(9, &reset, true, &mut ack_raw).unwrap();
        let (ack_hdr, ack_decoded) = decode_soft_reset_frame(&ack_raw[..ack_len]).unwrap();
        assert_eq!(ack_hdr.flags & FLAG_IS_ACK, FLAG_IS_ACK);
        assert_eq!(ack_decoded.reason, reset.reason);
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

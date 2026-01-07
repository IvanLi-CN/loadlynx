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
pub const MSG_PD_STATUS: u8 = 0x13;
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
/// LimitProfile message: S3 (digital) → G431 (analog)
///
/// Carries software-configurable limits for current, power, voltage and
/// temperature, plus a simple thermal derate factor.
pub const MSG_LIMIT_PROFILE: u8 = MSG_SET_LIMITS;
pub const MSG_GET_STATUS: u8 = 0x24;
/// Calibration mode control: S3 (digital) → G431 (analog).
///
/// Used to request optional raw telemetry in FastStatus during user calibration.
pub const MSG_CAL_MODE: u8 = 0x25;
/// Soft-reset request/ack handshake initiated by the digital side to reset
/// analog-side state without power-cycling.
pub const MSG_SOFT_RESET: u8 = 0x26;
pub const MSG_PD_SINK_REQUEST: u8 = 0x27;
/// Calibration write message: S3 (digital) → G431 (analog).
pub const MSG_CAL_WRITE: u8 = 0x30;
/// Reserved for future calibration readback support.
pub const MSG_CAL_READ: u8 = 0x31;

pub const PD_MAX_FIXED_PDOS: usize = 8;
pub const PD_MAX_PPS_PDOS: usize = 4;

/// Wire-level load mode mapping shared between control messages and telemetry.
///
/// Values not listed here are reserved for future expansion.
pub const LOAD_MODE_CC: u8 = 1;
pub const LOAD_MODE_CV: u8 = 2;

/// FastStatus mode values.
///
/// This mapping is intentionally identical to [`LOAD_MODE_CC`] / [`LOAD_MODE_CV`].
pub const FAST_STATUS_MODE_CC: u8 = LOAD_MODE_CC;
pub const FAST_STATUS_MODE_CV: u8 = LOAD_MODE_CV;

/// `FastStatus.state_flags` shared bit definitions.
///
/// Bits 0..=2 are already in use in current firmware and MUST NOT be repurposed.
pub const STATE_FLAG_REMOTE_ACTIVE: u32 = 1 << 0;
pub const STATE_FLAG_LINK_GOOD: u32 = 1 << 1;
pub const STATE_FLAG_ENABLED: u32 = 1 << 2;
pub const STATE_FLAG_UV_LATCHED: u32 = 1 << 3;
pub const STATE_FLAG_POWER_LIMITED: u32 = 1 << 4;
pub const STATE_FLAG_CURRENT_LIMITED: u32 = 1 << 5;

/// Fault bitmask definitions shared between analog and digital firmware.
///
/// These bits live in `FastStatus.fault_flags` and represent latched protection
/// conditions detected on the analog board. Once set, they remain asserted
/// until cleared by a SoftReset handshake.
pub const FAULT_OVERCURRENT: u32 = 1 << 0;
pub const FAULT_OVERVOLTAGE: u32 = 1 << 1;
pub const FAULT_MCU_OVER_TEMP: u32 = 1 << 2;
pub const FAULT_SINK_OVER_TEMP: u32 = 1 << 3;

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
    /// Optional calibration kind currently active on the analog side.
    ///
    /// Present only when the link is in calibration mode.
    #[n(16)]
    pub cal_kind: Option<u8>,
    /// Optional raw near‑sense voltage in 100 µV units.
    #[n(17)]
    pub raw_v_nr_100uv: Option<i16>,
    /// Optional raw remote‑sense voltage in 100 µV units.
    #[n(18)]
    pub raw_v_rmt_100uv: Option<i16>,
    /// Optional raw current (selected channel per `cal_kind`) in 100 µA units.
    #[n(19)]
    pub raw_cur_100uv: Option<i16>,
    /// Optional raw DAC code used by the control loop.
    #[n(20)]
    pub raw_dac_code: Option<u16>,
}

/// Stable load mode contract carried in control frames and surfaced via telemetry.
///
/// Only CC and CV are currently defined for protocol v1; other values are reserved.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadMode {
    Cc,
    Cv,
    Reserved(u8),
}

impl From<u8> for LoadMode {
    fn from(value: u8) -> Self {
        match value {
            LOAD_MODE_CC => LoadMode::Cc,
            LOAD_MODE_CV => LoadMode::Cv,
            other => LoadMode::Reserved(other),
        }
    }
}

impl From<LoadMode> for u8 {
    fn from(mode: LoadMode) -> Self {
        match mode {
            LoadMode::Cc => LOAD_MODE_CC,
            LoadMode::Cv => LOAD_MODE_CV,
            LoadMode::Reserved(raw) => raw,
        }
    }
}

impl Default for LoadMode {
    fn default() -> Self {
        LoadMode::Cc
    }
}

impl<C> Encode<C> for LoadMode {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.u8((*self).into())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for LoadMode {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let raw = d.u8()?;
        Ok(raw.into())
    }
}

/// Atomic active-control message (digital → analog) carried in [`MSG_SET_MODE`].
///
/// This payload freezes the v1 wire contract for CC/CV mode selection plus a
/// complete set of safety limits and one active preset slot.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Eq)]
#[cbor(map)]
pub struct SetMode {
    /// 1..=5
    #[n(0)]
    pub preset_id: u8,
    /// User output switch (higher layers may force false when applying a preset).
    #[n(1)]
    pub output_enabled: bool,
    /// Active load mode (CC/CV).
    #[n(2)]
    pub mode: LoadMode,
    /// CC target (mA). Present for wire stability; ignored in CV mode.
    #[n(3)]
    pub target_i_ma: i32,
    /// CV target (mV). Present for wire stability; ignored in CC mode.
    #[n(4)]
    pub target_v_mv: i32,
    /// Minimum allowed voltage (mV) (e.g. undervoltage threshold).
    #[n(5)]
    pub min_v_mv: i32,
    /// Total current limit (mA) across all channels.
    #[n(6)]
    pub max_i_ma_total: i32,
    /// Power limit (mW).
    #[n(7)]
    pub max_p_mw: u32,
}

/// Minimal control payload for adjusting the analog board's current setpoint.
///
/// The value is expressed in milliamps (mA). The analog firmware treats this
/// as the target *total* sink current across all active channels. It is
/// responsible for internally distributing the total current between the
/// available channels (e.g. single‑channel below a threshold, dual‑channel
/// sharing above it). The digital side is responsible for clamping the value
/// to a sane range for the current hardware (e.g. 0‒5000 mA for a 5 A design).
#[derive(Debug, Clone, Copy, Encode, Decode, Default)]
#[cbor(map)]
pub struct SetPoint {
    /// Target current in milliamps.
    #[n(0)]
    pub target_i_ma: i32,
}

/// Software-configurable limits reported by the digital side.
///
/// Units:
/// - max_i_ma: mA (software current limit for total sink current across all channels)
/// - max_p_mw: mW (software power limit)
/// - ovp_mv: mV (soft overvoltage threshold)
/// - temp_trip_mc: milli-degrees Celsius for sink temperature trip
/// - thermal_derate_pct: 0–100 %, multiplicative derate factor for max_i_ma
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Eq)]
#[cbor(map)]
pub struct LimitProfile {
    #[n(0)]
    pub max_i_ma: i32,
    #[n(1)]
    pub max_p_mw: u32,
    #[n(2)]
    pub ovp_mv: i32,
    #[n(3)]
    pub temp_trip_mc: i32,
    #[n(4)]
    pub thermal_derate_pct: u8,
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

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdSinkMode {
    Fixed,
    Pps,
    Unknown(u8),
}

impl From<u8> for PdSinkMode {
    fn from(value: u8) -> Self {
        match value {
            0 => PdSinkMode::Fixed,
            1 => PdSinkMode::Pps,
            other => PdSinkMode::Unknown(other),
        }
    }
}

impl From<PdSinkMode> for u8 {
    fn from(value: PdSinkMode) -> Self {
        match value {
            PdSinkMode::Fixed => 0,
            PdSinkMode::Pps => 1,
            PdSinkMode::Unknown(raw) => raw,
        }
    }
}

impl Default for PdSinkMode {
    fn default() -> Self {
        PdSinkMode::Fixed
    }
}

impl<C> Encode<C> for PdSinkMode {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.u8((*self).into())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for PdSinkMode {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let raw = d.u8()?;
        Ok(raw.into())
    }
}

/// Digital → analog PD target request, persisted by the analog side.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Eq)]
#[cbor(map)]
pub struct PdSinkRequest {
    #[n(0)]
    pub mode: PdSinkMode,
    /// Desired target VBUS in millivolts (mV), e.g. 5000 or 20000.
    #[n(1)]
    pub target_mv: u32,
}

/// Source-provided fixed PDO capability summary: `[mv, max_ma]`.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Eq)]
#[cbor(array)]
pub struct FixedPdo {
    #[n(0)]
    pub mv: u32,
    #[n(1)]
    pub max_ma: u32,
}

/// Source-provided PPS APDO capability summary: `[min_mv, max_mv, max_ma]`.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Eq)]
#[cbor(array)]
pub struct PpsPdo {
    #[n(0)]
    pub min_mv: u32,
    #[n(1)]
    pub max_mv: u32,
    #[n(2)]
    pub max_ma: u32,
}

pub type FixedPdoList = Vec<FixedPdo, PD_MAX_FIXED_PDOS>;
pub type PpsPdoList = Vec<PpsPdo, PD_MAX_PPS_PDOS>;

/// Analog → digital PD status report (attach/contract + capability summary).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdStatus {
    pub attached: bool,
    pub contract_mv: u32,
    pub contract_ma: u32,
    pub fixed_pdos: FixedPdoList,
    pub pps_pdos: PpsPdoList,
}

impl<C> Encode<C> for PdStatus {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(5)?;
        e.u8(0)?;
        e.bool(self.attached)?;
        e.u8(1)?;
        e.u32(self.contract_mv)?;
        e.u8(2)?;
        e.u32(self.contract_ma)?;
        e.u8(3)?;
        e.array(self.fixed_pdos.len() as u64)?;
        for pdo in self.fixed_pdos.iter() {
            e.encode_with(*pdo, ctx)?;
        }
        e.u8(4)?;
        e.array(self.pps_pdos.len() as u64)?;
        for pdo in self.pps_pdos.iter() {
            e.encode_with(*pdo, ctx)?;
        }
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for PdStatus {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let Some(entries) = d.map()? else {
            return Err(minicbor::decode::Error::message(
                "indefinite maps not supported",
            ));
        };

        let mut status = PdStatus::default();
        for _ in 0..entries {
            let key = d.u8()?;
            match key {
                0 => status.attached = d.bool()?,
                1 => status.contract_mv = d.u32()?,
                2 => status.contract_ma = d.u32()?,
                3 => status.fixed_pdos = decode_fixed_pdo_list(d, ctx)?,
                4 => status.pps_pdos = decode_pps_pdo_list(d, ctx)?,
                _ => d.skip()?,
            }
        }
        Ok(status)
    }
}

fn decode_fixed_pdo_list<'b, C>(
    d: &mut Decoder<'b>,
    ctx: &mut C,
) -> Result<FixedPdoList, minicbor::decode::Error> {
    let Some(len) = d.array()? else {
        return Err(minicbor::decode::Error::message(
            "indefinite arrays not supported",
        ));
    };

    let mut out = FixedPdoList::new();
    for _ in 0..len {
        let pdo: FixedPdo = d.decode_with(ctx)?;
        let _ = out.push(pdo);
    }
    Ok(out)
}

fn decode_pps_pdo_list<'b, C>(
    d: &mut Decoder<'b>,
    ctx: &mut C,
) -> Result<PpsPdoList, minicbor::decode::Error> {
    let Some(len) = d.array()? else {
        return Err(minicbor::decode::Error::message(
            "indefinite arrays not supported",
        ));
    };

    let mut out = PpsPdoList::new();
    for _ in 0..len {
        let pdo: PpsPdo = d.decode_with(ctx)?;
        let _ = out.push(pdo);
    }
    Ok(out)
}

/// Calibration raw telemetry selection.
///
/// Unknown kinds received over the wire are mapped to `Off` to keep decoding
/// forward compatible while defaulting to a safe "no raw telemetry" state.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalKind {
    Off,
    Voltage,
    CurrentCh1,
    CurrentCh2,
}

impl From<u8> for CalKind {
    fn from(value: u8) -> Self {
        match value {
            0 => CalKind::Off,
            1 => CalKind::Voltage,
            2 => CalKind::CurrentCh1,
            3 => CalKind::CurrentCh2,
            _ => CalKind::Off,
        }
    }
}

impl From<CalKind> for u8 {
    fn from(kind: CalKind) -> Self {
        match kind {
            CalKind::Off => 0,
            CalKind::Voltage => 1,
            CalKind::CurrentCh1 => 2,
            CalKind::CurrentCh2 => 3,
        }
    }
}

impl Default for CalKind {
    fn default() -> Self {
        CalKind::Off
    }
}

impl<C> Encode<C> for CalKind {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.u8((*self).into())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for CalKind {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let raw = d.u8()?;
        Ok(raw.into())
    }
}

/// Calibration mode control payload.
///
/// Sent from the digital side with `FLAG_ACK_REQ`.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Eq)]
#[cbor(map)]
pub struct CalMode {
    #[n(0)]
    pub kind: CalKind,
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
    /// Chunk index for multi-block calibration writes.
    #[n(0)]
    pub index: u8,
    /// Opaque 32-byte payload owned by the digital side.
    #[n(1)]
    pub payload: [u8; 32],
    /// Optional inner CRC16 for the payload (e.g. CRC16 over index+payload).
    #[n(2)]
    pub crc: u16,
}

/// Preferred name for the multi-block calibration write chunk.
///
/// This is an alias to `CalWrite` to preserve API and wire compatibility.
pub type CalWriteChunk = CalWrite;

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

/// Encode a PD_STATUS frame reporting attach/contract + capability summary.
pub fn encode_pd_status_frame(seq: u8, status: &PdStatus, out: &mut [u8]) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = 0;
    out[2] = seq;
    out[3] = MSG_PD_STATUS;

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

/// Encode a `LimitProfile` payload into a binary frame with header and CRC,
/// ready for SLIP framing.
pub fn encode_limit_profile_frame(
    seq: u8,
    profile: &LimitProfile,
    out: &mut [u8],
) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    // Control/configuration frames request ACKs in future revisions.
    out[1] = FLAG_ACK_REQ;
    out[2] = seq;
    out[3] = MSG_LIMIT_PROFILE;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(profile).map_err(map_encode_err)?;
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
pub fn encode_cal_write_frame(seq: u8, cal: &CalWrite, out: &mut [u8]) -> Result<usize, Error> {
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

/// Encode an atomic `SetMode` control frame from the digital side to the analog side.
pub fn encode_set_mode_frame(seq: u8, cmd: &SetMode, out: &mut [u8]) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = FLAG_ACK_REQ;
    out[2] = seq;
    out[3] = MSG_SET_MODE;

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

/// Encode a CalMode control frame from the digital side to the analog side.
pub fn encode_cal_mode_frame(seq: u8, mode: &CalMode, out: &mut [u8]) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = FLAG_ACK_REQ;
    out[2] = seq;
    out[3] = MSG_CAL_MODE;

    let payload_len = {
        let payload_slice = &mut out[HEADER_LEN..];
        let mut cursor = Cursor::new(payload_slice);
        let mut encoder = minicbor::Encoder::new(&mut cursor);
        encoder.encode(mode).map_err(map_encode_err)?;
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

/// Encode a PD_SINK_REQUEST control frame from the digital side.
pub fn encode_pd_sink_request_frame(
    seq: u8,
    req: &PdSinkRequest,
    out: &mut [u8],
) -> Result<usize, Error> {
    if out.len() < HEADER_LEN + CRC_LEN {
        return Err(Error::BufferTooSmall);
    }

    out[0] = PROTOCOL_VERSION;
    out[1] = FLAG_ACK_REQ;
    out[2] = seq;
    out[3] = MSG_PD_SINK_REQUEST;

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

pub fn decode_fast_status_frame(frame: &[u8]) -> Result<(FrameHeader, FastStatus), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_FAST_STATUS {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let status: FastStatus = decoder.decode().map_err(map_decode_err)?;
    Ok((header, status))
}

pub fn decode_pd_status_frame(frame: &[u8]) -> Result<(FrameHeader, PdStatus), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_PD_STATUS {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let status: PdStatus = decoder.decode().map_err(map_decode_err)?;
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

/// Decode a `LimitProfile` frame. Callers are expected to have obtained
/// `frame` either from `decode_frame` + message ID filtering, or from a SLIP
/// decoder that yields full binary frames.
pub fn decode_limit_profile_frame(frame: &[u8]) -> Result<(FrameHeader, LimitProfile), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_LIMIT_PROFILE {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let profile: LimitProfile = decoder.decode().map_err(map_decode_err)?;
    Ok((header, profile))
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

/// Decode a `SetMode` frame.
pub fn decode_set_mode_frame(frame: &[u8]) -> Result<(FrameHeader, SetMode), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_SET_MODE {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let cmd: SetMode = decoder.decode().map_err(map_decode_err)?;
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

/// Decode a CalMode frame.
pub fn decode_cal_mode_frame(frame: &[u8]) -> Result<(FrameHeader, CalMode), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_CAL_MODE {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let mode: CalMode = decoder.decode().map_err(map_decode_err)?;
    Ok((header, mode))
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

pub fn decode_pd_sink_request_frame(frame: &[u8]) -> Result<(FrameHeader, PdSinkRequest), Error> {
    let (header, payload) = decode_frame(frame)?;
    if header.msg != MSG_PD_SINK_REQUEST {
        return Err(Error::UnsupportedMessage(header.msg));
    }
    let mut decoder = minicbor::Decoder::new(payload);
    let req: PdSinkRequest = decoder.decode().map_err(map_decode_err)?;
    Ok((header, req))
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
            ..FastStatus::default()
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
    fn cal_mode_roundtrip_and_ack_req() {
        let mode = CalMode {
            kind: CalKind::CurrentCh2,
        };
        let mut raw = [0u8; 64];
        let len = encode_cal_mode_frame(1, &mode, &mut raw).unwrap();
        let (hdr, decoded) = decode_cal_mode_frame(&raw[..len]).unwrap();
        assert_eq!(hdr.msg, MSG_CAL_MODE);
        assert_eq!(hdr.flags & FLAG_ACK_REQ, FLAG_ACK_REQ);
        assert_eq!(decoded, mode);
    }

    #[test]
    fn cal_mode_unknown_kind_maps_to_off() {
        let mut payload_buf = [0u8; 16];
        let payload_len = {
            let mut cursor = Cursor::new(&mut payload_buf[..]);
            let mut encoder = minicbor::Encoder::new(&mut cursor);
            encoder.map(1).unwrap();
            encoder.u8(0).unwrap();
            encoder.u8(99).unwrap();
            cursor.position()
        };

        let mut raw = [0u8; 64];
        raw[0] = PROTOCOL_VERSION;
        raw[1] = FLAG_ACK_REQ;
        raw[2] = 2;
        raw[3] = MSG_CAL_MODE;
        let len_bytes = (payload_len as u16).to_le_bytes();
        raw[4] = len_bytes[0];
        raw[5] = len_bytes[1];
        raw[HEADER_LEN..HEADER_LEN + payload_len].copy_from_slice(&payload_buf[..payload_len]);
        let frame_len_without_crc = HEADER_LEN + payload_len;
        let crc = crc16_ccitt_false(&raw[..frame_len_without_crc]);
        let crc_bytes = crc.to_le_bytes();
        raw[frame_len_without_crc] = crc_bytes[0];
        raw[frame_len_without_crc + 1] = crc_bytes[1];
        let total_len = frame_len_without_crc + CRC_LEN;

        let (_hdr, decoded) = decode_cal_mode_frame(&raw[..total_len]).unwrap();
        assert_eq!(decoded.kind, CalKind::Off);
    }

    #[test]
    fn set_mode_roundtrip_cc_and_header() {
        let cmd = SetMode {
            preset_id: 1,
            output_enabled: false,
            mode: LoadMode::Cc,
            target_i_ma: 2500,
            target_v_mv: 12_000,
            min_v_mv: 500,
            max_i_ma_total: 5000,
            max_p_mw: 60_000,
        };

        let mut raw = [0u8; 96];
        let len = encode_set_mode_frame(3, &cmd, &mut raw).unwrap();
        let (hdr, decoded) = decode_set_mode_frame(&raw[..len]).unwrap();
        assert_eq!(hdr.msg, MSG_SET_MODE);
        assert_eq!(hdr.seq, 3);
        assert_eq!(hdr.flags & FLAG_ACK_REQ, FLAG_ACK_REQ);
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn set_mode_roundtrip_cv() {
        let cmd = SetMode {
            preset_id: 5,
            output_enabled: true,
            mode: LoadMode::Cv,
            target_i_ma: 1234,
            target_v_mv: 42_000,
            min_v_mv: 1000,
            max_i_ma_total: 3000,
            max_p_mw: 120_000,
        };

        let mut raw = [0u8; 96];
        let len = encode_set_mode_frame(200, &cmd, &mut raw).unwrap();
        let (_hdr, decoded) = decode_set_mode_frame(&raw[..len]).unwrap();
        assert_eq!(decoded.mode, LoadMode::Cv);
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn pd_sink_request_roundtrip_and_ack_req() {
        let req = PdSinkRequest {
            mode: PdSinkMode::Fixed,
            target_mv: 20_000,
        };

        let mut raw = [0u8; 64];
        let len = encode_pd_sink_request_frame(7, &req, &mut raw).unwrap();
        let (hdr, decoded) = decode_pd_sink_request_frame(&raw[..len]).unwrap();
        assert_eq!(hdr.msg, MSG_PD_SINK_REQUEST);
        assert_eq!(hdr.seq, 7);
        assert_eq!(hdr.flags & FLAG_ACK_REQ, FLAG_ACK_REQ);
        assert_eq!(decoded, req);
    }

    #[test]
    fn pd_status_roundtrip_and_lists() {
        let mut fixed_pdos = FixedPdoList::new();
        fixed_pdos
            .push(FixedPdo {
                mv: 5_000,
                max_ma: 3_000,
            })
            .unwrap();
        fixed_pdos
            .push(FixedPdo {
                mv: 20_000,
                max_ma: 1_500,
            })
            .unwrap();

        let mut pps_pdos = PpsPdoList::new();
        pps_pdos
            .push(PpsPdo {
                min_mv: 3_300,
                max_mv: 11_000,
                max_ma: 3_000,
            })
            .unwrap();

        let status = PdStatus {
            attached: true,
            contract_mv: 20_000,
            contract_ma: 1_500,
            fixed_pdos,
            pps_pdos,
        };

        let mut raw = [0u8; 128];
        let len = encode_pd_status_frame(9, &status, &mut raw).unwrap();
        let (hdr, decoded) = decode_pd_status_frame(&raw[..len]).unwrap();
        assert_eq!(hdr.msg, MSG_PD_STATUS);
        assert_eq!(hdr.seq, 9);
        assert_eq!(decoded, status);
        assert_eq!(decoded.fixed_pdos.len(), 2);
        assert_eq!(decoded.fixed_pdos[1].mv, 20_000);
    }

    #[test]
    fn set_mode_decode_rejects_wrong_msg_id() {
        let cmd = SetMode {
            preset_id: 1,
            output_enabled: false,
            mode: LoadMode::Cc,
            target_i_ma: 0,
            target_v_mv: 0,
            min_v_mv: 0,
            max_i_ma_total: 0,
            max_p_mw: 0,
        };

        let mut raw = [0u8; 96];
        let len = encode_set_mode_frame(1, &cmd, &mut raw).unwrap();
        raw[3] = MSG_SET_ENABLE;
        let crc = crc16_ccitt_false(&raw[..len - CRC_LEN]);
        let crc_bytes = crc.to_le_bytes();
        raw[len - 2] = crc_bytes[0];
        raw[len - 1] = crc_bytes[1];

        let err = decode_set_mode_frame(&raw[..len]).unwrap_err();
        assert!(matches!(err, Error::UnsupportedMessage(MSG_SET_ENABLE)));
    }

    #[test]
    fn fast_status_no_raw_bytes_match_v0() {
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
            ..FastStatus::default()
        };

        let mut raw = [0u8; 192];
        let len = encode_fast_status_frame(7, &status, &mut raw).unwrap();

        const EXPECTED_FRAME_V0: [u8; 62] = [
            1, 0, 7, 16, 54, 0, 176, 0, 25, 4, 210, 1, 2, 2, 1, 3, 245, 4, 25, 9, 196, 5, 25, 7,
            208, 6, 25, 7, 188, 7, 25, 95, 180, 8, 25, 96, 24, 9, 25, 191, 104, 10, 24, 120, 11,
            43, 12, 25, 175, 200, 13, 25, 160, 40, 14, 25, 148, 112, 15, 0, 187, 146,
        ];

        assert_eq!(len, EXPECTED_FRAME_V0.len());
        assert_eq!(&raw[..len], &EXPECTED_FRAME_V0);
    }

    #[test]
    fn fast_status_with_raw_roundtrip_and_missing_fields_default_none() {
        let status = FastStatus {
            uptime_ms: 1,
            mode: 0,
            state_flags: 0,
            enable: false,
            target_value: 0,
            i_local_ma: 0,
            i_remote_ma: 0,
            v_local_mv: 0,
            v_remote_mv: 0,
            calc_p_mw: 0,
            dac_headroom_mv: 0,
            loop_error: 0,
            sink_core_temp_mc: 0,
            sink_exhaust_temp_mc: 0,
            mcu_temp_mc: 0,
            fault_flags: 0,
            cal_kind: Some(1),
            raw_v_nr_100uv: Some(-123),
            raw_v_rmt_100uv: None,
            raw_cur_100uv: Some(789),
            raw_dac_code: None,
        };

        let mut raw = [0u8; 192];
        let len = encode_fast_status_frame(1, &status, &mut raw).unwrap();
        let (_hdr, decoded) = decode_fast_status_frame(&raw[..len]).unwrap();
        assert_eq!(decoded.cal_kind, status.cal_kind);
        assert_eq!(decoded.raw_v_nr_100uv, status.raw_v_nr_100uv);
        assert_eq!(decoded.raw_v_rmt_100uv, None);
        assert_eq!(decoded.raw_cur_100uv, status.raw_cur_100uv);
        assert_eq!(decoded.raw_dac_code, None);
    }

    #[test]
    fn cal_write_single_chunk_roundtrip() {
        let chunk: CalWriteChunk = CalWrite {
            index: 0,
            payload: [0x11; 32],
            crc: 0x1234,
        };

        let mut raw = [0u8; 96];
        let len = encode_cal_write_frame(2, &chunk, &mut raw).unwrap();
        let (_hdr, decoded) = decode_cal_write_frame(&raw[..len]).unwrap();
        assert_eq!(decoded.index, chunk.index);
        assert_eq!(decoded.payload, chunk.payload);
        assert_eq!(decoded.crc, chunk.crc);
    }

    #[test]
    fn cal_write_multi_chunk_roundtrip() {
        let chunks: [CalWriteChunk; 3] = [
            CalWrite {
                index: 0,
                payload: [0x00; 32],
                crc: 0x0000,
            },
            CalWrite {
                index: 1,
                payload: [0x01; 32],
                crc: 0x1111,
            },
            CalWrite {
                index: 2,
                payload: [0x02; 32],
                crc: 0x2222,
            },
        ];

        for (seq, chunk) in chunks.iter().enumerate() {
            let mut raw = [0u8; 96];
            let len = encode_cal_write_frame(seq as u8, chunk, &mut raw).unwrap();
            let (_hdr, decoded) = decode_cal_write_frame(&raw[..len]).unwrap();
            assert_eq!(decoded.index, chunk.index);
            assert_eq!(decoded.payload, chunk.payload);
            assert_eq!(decoded.crc, chunk.crc);
        }
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
    fn limit_profile_roundtrip() {
        let profile = LimitProfile {
            max_i_ma: 5_000,
            max_p_mw: 250_000,
            ovp_mv: 55_000,
            temp_trip_mc: 100_000,
            thermal_derate_pct: 100,
        };

        let mut raw = [0u8; 64];
        let len = encode_limit_profile_frame(7, &profile, &mut raw).unwrap();
        let (header, decoded) = decode_limit_profile_frame(&raw[..len]).unwrap();
        assert_eq!(header.version, PROTOCOL_VERSION);
        assert_eq!(header.seq, 7);
        assert_eq!(header.msg, MSG_LIMIT_PROFILE);
        assert_eq!(decoded, profile);
    }

    #[test]
    fn limit_profile_wrong_msg_id_yields_unsupported_message() {
        // Build a valid SetPoint frame and attempt to decode it as LimitProfile.
        let setpoint = SetPoint { target_i_ma: 1234 };
        let mut raw = [0u8; 64];
        let len = encode_set_point_frame(1, &setpoint, &mut raw).unwrap();
        let err = decode_limit_profile_frame(&raw[..len]).unwrap_err();
        assert!(matches!(err, Error::UnsupportedMessage(id) if id == MSG_SET_POINT));
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

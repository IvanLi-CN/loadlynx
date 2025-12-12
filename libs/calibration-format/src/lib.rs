#![no_std]

use heapless::Vec;
use loadlynx_protocol::{CalWrite, crc16_ccitt_false};

#[cfg(test)]
extern crate std;

pub const CAL_FMT_VERSION: u8 = 1;

// v4.2 -> 42 (see docs/dev-notes/user-calibration.md examples).
pub const DIGITAL_HW_REV: u8 = 42;

// M24C64 (64 Kbit = 8 KiB) external EEPROM.
pub const EEPROM_I2C_ADDR_7BIT: u8 = 0x50;

// Fixed layout for the calibration profile blob.
pub const EEPROM_PROFILE_BASE_ADDR: u16 = 0x0000;
pub const EEPROM_PROFILE_LEN: usize = 256;
pub const EEPROM_PROFILE_CRC32_LEN: usize = 4;
pub const EEPROM_PROFILE_CRC32_OFFSET: usize = EEPROM_PROFILE_LEN - EEPROM_PROFILE_CRC32_LEN;

// ST M24C64 page write supports up to 32 bytes; we keep this constant explicit
// and never write across page boundaries.
pub const EEPROM_PAGE_SIZE_BYTES: usize = 32;

// Header (8 bytes) + 3 points (24 bytes) = 32 payload bytes.
pub const CALWRITE_PAYLOAD_LEN: usize = 32;
pub const CALWRITE_HEADER_LEN: usize = 8;
pub const CALWRITE_POINTS_REGION_LEN: usize = 24;
pub const CALWRITE_POINTS_PER_CHUNK: usize = 3;
pub const CALWRITE_POINT_LEN: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileSource {
    FactoryDefault,
    UserCalibrated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveKind {
    VLocal = 0,
    VRemote = 1,
    CurrentCh1 = 2,
    CurrentCh2 = 3,
}

impl CurveKind {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CalPoint {
    pub raw_100uv: i16,
    pub raw_dac_code: u16,
    pub meas_physical: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveProfile {
    pub source: ProfileSource,
    pub fmt_version: u8,
    pub hw_rev: u8,
    pub current_ch1: Vec<CalPoint, 5>,
    pub current_ch2: Vec<CalPoint, 5>,
    pub v_local: Vec<CalPoint, 5>,
    pub v_remote: Vec<CalPoint, 5>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileLoadError {
    InvalidLength,
    UnsupportedFmtVersion(u8),
    HwRevMismatch { stored: u8, expected: u8 },
    InvalidCounts,
    CrcMismatch { stored: u32, computed: u32 },
}

impl ActiveProfile {
    pub fn factory_default(hw_rev: u8) -> Self {
        // Defaults are chosen to approximate the current firmware's ideal
        // conversions (no user calibration), while still satisfying the analog
        // side requirement of non-empty curves.
        //
        // - Current: at least 2 points, raw_dac_code unused and set to 0.
        // - Voltage: at least 2 points, V_load ≈ (124/100) * raw_max(mV-equivalent).
        let mut current_ch1 = Vec::<CalPoint, 5>::new();
        let mut current_ch2 = Vec::<CalPoint, 5>::new();
        let mut v_local = Vec::<CalPoint, 5>::new();
        let mut v_remote = Vec::<CalPoint, 5>::new();

        let _ = current_ch1.push(CalPoint {
            raw_100uv: 0,
            raw_dac_code: 0,
            meas_physical: 0, // mA
        });
        let _ = current_ch1.push(CalPoint {
            raw_100uv: 25_000,
            raw_dac_code: 0,
            meas_physical: 5_000, // mA
        });
        let _ = current_ch2.extend_from_slice(&current_ch1);

        let raw_max: i16 = i16::MAX;
        let meas_mv_max: i32 = (raw_max as i32).saturating_mul(124).saturating_div(100);
        let _ = v_local.push(CalPoint {
            raw_100uv: 0,
            raw_dac_code: 0,
            meas_physical: 0, // mV
        });
        let _ = v_local.push(CalPoint {
            raw_100uv: raw_max,
            raw_dac_code: 0,
            meas_physical: meas_mv_max, // mV
        });
        let _ = v_remote.extend_from_slice(&v_local);

        Self {
            source: ProfileSource::FactoryDefault,
            fmt_version: CAL_FMT_VERSION,
            hw_rev,
            current_ch1,
            current_ch2,
            v_local,
            v_remote,
        }
    }

    pub fn points_for(&self, kind: CurveKind) -> &[CalPoint] {
        match kind {
            CurveKind::VLocal => self.v_local.as_slice(),
            CurveKind::VRemote => self.v_remote.as_slice(),
            CurveKind::CurrentCh1 => self.current_ch1.as_slice(),
            CurveKind::CurrentCh2 => self.current_ch2.as_slice(),
        }
    }

    pub fn points_for_mut(&mut self, kind: CurveKind) -> &mut Vec<CalPoint, 5> {
        match kind {
            CurveKind::VLocal => &mut self.v_local,
            CurveKind::VRemote => &mut self.v_remote,
            CurveKind::CurrentCh1 => &mut self.current_ch1,
            CurveKind::CurrentCh2 => &mut self.current_ch2,
        }
    }
}

pub fn normalize_points(mut points: Vec<CalPoint, 5>) -> Vec<CalPoint, 5> {
    // Small N (<=5): stable insertion sort by raw_100uv, then drop duplicates.
    let len = points.len();
    let slice = points.as_mut_slice();
    for i in 1..len {
        let mut j = i;
        while j > 0 && slice[j - 1].raw_100uv > slice[j].raw_100uv {
            slice.swap(j - 1, j);
            j -= 1;
        }
    }

    // Dedup by raw_100uv (keep last occurrence).
    let mut out = Vec::<CalPoint, 5>::new();
    for p in slice.iter().copied() {
        if let Some(last) = out.last() {
            if last.raw_100uv == p.raw_100uv {
                let _ = out.pop();
            }
        }
        let _ = out.push(p);
    }
    out
}

pub fn meas_is_strictly_increasing(points: &[CalPoint]) -> bool {
    for win in points.windows(2) {
        if win[1].meas_physical <= win[0].meas_physical {
            return false;
        }
    }
    true
}

pub fn encode_calwrite_chunks(
    fmt_version: u8,
    hw_rev: u8,
    kind: CurveKind,
    points: &[CalPoint],
) -> Vec<CalWrite, 2> {
    let total_points = points.len().min(5);
    let total_chunks =
        ((total_points + (CALWRITE_POINTS_PER_CHUNK - 1)) / CALWRITE_POINTS_PER_CHUNK).max(1);

    let mut chunks = Vec::<CalWrite, 2>::new();
    for chunk_index in 0..total_chunks {
        let mut payload = [0u8; CALWRITE_PAYLOAD_LEN];
        payload[0] = fmt_version;
        payload[1] = hw_rev;
        payload[2] = kind.as_u8();
        payload[3] = chunk_index as u8;
        payload[4] = total_chunks as u8;
        payload[5] = total_points as u8;
        payload[6] = 0;
        payload[7] = 0;

        let base_point = chunk_index * CALWRITE_POINTS_PER_CHUNK;
        for i in 0..CALWRITE_POINTS_PER_CHUNK {
            let point_index = base_point + i;
            let dst = CALWRITE_HEADER_LEN + i * CALWRITE_POINT_LEN;
            if point_index < total_points {
                let p = points[point_index];
                payload[dst..dst + 2].copy_from_slice(&p.raw_100uv.to_le_bytes());
                payload[dst + 2..dst + 4].copy_from_slice(&p.raw_dac_code.to_le_bytes());
                payload[dst + 4..dst + 8].copy_from_slice(&p.meas_physical.to_le_bytes());
            }
        }

        let index_u8 = chunk_index as u8;
        let mut crc_buf = [0u8; 1 + CALWRITE_PAYLOAD_LEN];
        crc_buf[0] = index_u8;
        crc_buf[1..].copy_from_slice(&payload);
        let crc = crc16_ccitt_false(&crc_buf);

        let _ = chunks.push(CalWrite {
            index: index_u8,
            payload,
            crc,
        });
    }
    chunks
}

pub fn serialize_profile(profile: &ActiveProfile) -> [u8; EEPROM_PROFILE_LEN] {
    // Layout:
    // 0: fmt_version (u8)
    // 1: hw_rev      (u8)
    // 2..6: counts   (u8 x4): current_ch1, current_ch2, v_local, v_remote
    // 6..8: reserved
    // 8..168: points (4 curves x 5 points x 8B)
    // 252..256: crc32 (u32 LE) over 0..252
    const OFF_FMT: usize = 0;
    const OFF_HW_REV: usize = 1;
    const OFF_COUNTS: usize = 2;
    const OFF_POINTS: usize = 8;

    let mut out = [0u8; EEPROM_PROFILE_LEN];
    out[OFF_FMT] = profile.fmt_version;
    out[OFF_HW_REV] = profile.hw_rev;

    let counts = [
        profile.current_ch1.len() as u8,
        profile.current_ch2.len() as u8,
        profile.v_local.len() as u8,
        profile.v_remote.len() as u8,
    ];
    out[OFF_COUNTS..OFF_COUNTS + 4].copy_from_slice(&counts);

    let mut write_curve = |curve_idx: usize, points: &[CalPoint]| {
        for i in 0..5 {
            let dst = OFF_POINTS + (curve_idx * 5 + i) * CALWRITE_POINT_LEN;
            if i < points.len() {
                let p = points[i];
                out[dst..dst + 2].copy_from_slice(&p.raw_100uv.to_le_bytes());
                out[dst + 2..dst + 4].copy_from_slice(&p.raw_dac_code.to_le_bytes());
                out[dst + 4..dst + 8].copy_from_slice(&p.meas_physical.to_le_bytes());
            }
        }
    };

    // Order: current_ch1, current_ch2, v_local, v_remote.
    write_curve(0, profile.current_ch1.as_slice());
    write_curve(1, profile.current_ch2.as_slice());
    write_curve(2, profile.v_local.as_slice());
    write_curve(3, profile.v_remote.as_slice());

    let crc = crc32_ieee(&out[..EEPROM_PROFILE_CRC32_OFFSET]);
    out[EEPROM_PROFILE_CRC32_OFFSET..].copy_from_slice(&crc.to_le_bytes());
    out
}

pub fn deserialize_profile(
    bytes: &[u8; EEPROM_PROFILE_LEN],
    expected_hw_rev: u8,
) -> Result<ActiveProfile, ProfileLoadError> {
    const OFF_FMT: usize = 0;
    const OFF_HW_REV: usize = 1;
    const OFF_COUNTS: usize = 2;
    const OFF_POINTS: usize = 8;

    let fmt_version = bytes[OFF_FMT];
    if fmt_version != CAL_FMT_VERSION {
        return Err(ProfileLoadError::UnsupportedFmtVersion(fmt_version));
    }
    let hw_rev = bytes[OFF_HW_REV];
    if hw_rev != expected_hw_rev {
        return Err(ProfileLoadError::HwRevMismatch {
            stored: hw_rev,
            expected: expected_hw_rev,
        });
    }

    let stored_crc = u32::from_le_bytes(bytes[EEPROM_PROFILE_CRC32_OFFSET..].try_into().unwrap());
    let computed_crc = crc32_ieee(&bytes[..EEPROM_PROFILE_CRC32_OFFSET]);
    if stored_crc != computed_crc {
        return Err(ProfileLoadError::CrcMismatch {
            stored: stored_crc,
            computed: computed_crc,
        });
    }

    let counts = &bytes[OFF_COUNTS..OFF_COUNTS + 4];
    if counts.iter().any(|&c| c == 0 || c > 5) {
        return Err(ProfileLoadError::InvalidCounts);
    }

    let read_curve = |curve_idx: usize, count: usize| -> Vec<CalPoint, 5> {
        let mut out = Vec::<CalPoint, 5>::new();
        for i in 0..count.min(5) {
            let src = OFF_POINTS + (curve_idx * 5 + i) * CALWRITE_POINT_LEN;
            let raw_100uv = i16::from_le_bytes([bytes[src], bytes[src + 1]]);
            let raw_dac_code = u16::from_le_bytes([bytes[src + 2], bytes[src + 3]]);
            let meas_physical = i32::from_le_bytes([
                bytes[src + 4],
                bytes[src + 5],
                bytes[src + 6],
                bytes[src + 7],
            ]);
            let _ = out.push(CalPoint {
                raw_100uv,
                raw_dac_code,
                meas_physical,
            });
        }
        out
    };

    let current_ch1 = read_curve(0, counts[0] as usize);
    let current_ch2 = read_curve(1, counts[1] as usize);
    let v_local = read_curve(2, counts[2] as usize);
    let v_remote = read_curve(3, counts[3] as usize);

    Ok(ActiveProfile {
        source: ProfileSource::UserCalibrated,
        fmt_version,
        hw_rev,
        current_ch1,
        current_ch2,
        v_local,
        v_remote,
    })
}

pub fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320u32 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(raw: i16, dac: u16, meas: i32) -> CalPoint {
        CalPoint {
            raw_100uv: raw,
            raw_dac_code: dac,
            meas_physical: meas,
        }
    }

    #[test]
    fn calwrite_chunk_count_matches_points() {
        let pts1 = [p(1, 2, 3)];
        let pts2 = [p(1, 2, 3), p(4, 5, 6)];
        let pts3 = [p(1, 2, 3), p(4, 5, 6), p(7, 8, 9)];
        let pts4 = [p(1, 2, 3), p(4, 5, 6), p(7, 8, 9), p(10, 11, 12)];
        let pts5 = [
            p(1, 2, 3),
            p(4, 5, 6),
            p(7, 8, 9),
            p(10, 11, 12),
            p(13, 14, 15),
        ];

        assert_eq!(
            encode_calwrite_chunks(CAL_FMT_VERSION, DIGITAL_HW_REV, CurveKind::VLocal, &pts1).len(),
            1
        );
        assert_eq!(
            encode_calwrite_chunks(CAL_FMT_VERSION, DIGITAL_HW_REV, CurveKind::VLocal, &pts2).len(),
            1
        );
        assert_eq!(
            encode_calwrite_chunks(CAL_FMT_VERSION, DIGITAL_HW_REV, CurveKind::VLocal, &pts3).len(),
            1
        );
        assert_eq!(
            encode_calwrite_chunks(CAL_FMT_VERSION, DIGITAL_HW_REV, CurveKind::VLocal, &pts4).len(),
            2
        );
        assert_eq!(
            encode_calwrite_chunks(CAL_FMT_VERSION, DIGITAL_HW_REV, CurveKind::VLocal, &pts5).len(),
            2
        );
    }

    #[test]
    fn calwrite_payload_layout_and_crc16_match() {
        let pts = [p(-123, 0x1234, -0x1020_3040)];
        let chunks = encode_calwrite_chunks(CAL_FMT_VERSION, 7, CurveKind::CurrentCh2, &pts);
        assert_eq!(chunks.len(), 1);
        let c = chunks[0];

        // Header bytes.
        assert_eq!(c.payload[0], CAL_FMT_VERSION);
        assert_eq!(c.payload[1], 7);
        assert_eq!(c.payload[2], CurveKind::CurrentCh2.as_u8());
        assert_eq!(c.payload[3], 0); // chunk_index
        assert_eq!(c.payload[4], 1); // total_chunks
        assert_eq!(c.payload[5], 1); // total_points

        // Point 0 @ offset 8.
        assert_eq!(&c.payload[8..10], &(-123i16).to_le_bytes());
        assert_eq!(&c.payload[10..12], &0x1234u16.to_le_bytes());
        assert_eq!(&c.payload[12..16], &(-0x1020_3040i32).to_le_bytes());

        // Inner CRC16 is crc16_ccitt_false(index + payload).
        let mut buf = [0u8; 33];
        buf[0] = c.index;
        buf[1..].copy_from_slice(&c.payload);
        assert_eq!(c.crc, crc16_ccitt_false(&buf));
    }

    #[test]
    fn eeprom_roundtrip_and_crc32_detects_corruption() {
        let mut prof = ActiveProfile::factory_default(DIGITAL_HW_REV);
        prof.source = ProfileSource::UserCalibrated;
        prof.current_ch1.clear();
        let _ = prof.current_ch1.push(p(10, 11, 12));
        let _ = prof.current_ch1.push(p(20, 21, 22));
        let _ = prof.current_ch1.push(p(30, 31, 32));
        let _ = prof.current_ch1.push(p(40, 41, 42));
        let _ = prof.current_ch1.push(p(50, 51, 52));

        let bytes = serialize_profile(&prof);
        let decoded = deserialize_profile((&bytes).try_into().unwrap(), DIGITAL_HW_REV).unwrap();
        assert_eq!(decoded.fmt_version, CAL_FMT_VERSION);
        assert_eq!(decoded.hw_rev, DIGITAL_HW_REV);
        assert_eq!(decoded.current_ch1.len(), 5);
        assert_eq!(decoded.current_ch1[2].raw_dac_code, 31);

        let mut corrupted = bytes;
        corrupted[17] ^= 0x01;
        let err =
            deserialize_profile((&corrupted).try_into().unwrap(), DIGITAL_HW_REV).unwrap_err();
        assert!(matches!(err, ProfileLoadError::CrcMismatch { .. }));
    }

    #[test]
    fn meas_monotonic_validation() {
        let ok = [p(10, 0, 100), p(20, 0, 200), p(30, 0, 300)];
        assert!(meas_is_strictly_increasing(&ok));

        let bad = [p(10, 0, 100), p(20, 0, 100)];
        assert!(!meas_is_strictly_increasing(&bad));

        let bad2 = [p(10, 0, 200), p(20, 0, 100)];
        assert!(!meas_is_strictly_increasing(&bad2));
    }
}

//! Calibration algorithms and runtime state for the analog (STM32G431) board.
//!
//! This module is `no_std` and designed to be host-testable via the package
//! library target (`src/lib.rs`).

use loadlynx_protocol::crc16_ccitt_false;

pub const MAX_POINTS: usize = 24;
pub const POINTS_PER_CHUNK: usize = 3;
pub const MAX_CHUNKS: usize = 8;

// NOTE: The firmware no longer uses a fixed 0.5 A reference point for output
// mapping; DAC codes are derived either from user calibration samples
// (`raw_100uv` ↔ `raw_dac_code`) or from the measured reference voltage
// (`raw_100uv_to_dac_code_vref`).

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct CalPoint {
    pub raw_100uv: i16,
    pub raw_dac_code: u16,
    pub meas_physical: i32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct CalCurve {
    pub points: [CalPoint; MAX_POINTS],
    pub len: u8,
}

impl CalCurve {
    pub const fn empty() -> Self {
        Self {
            points: [CalPoint {
                raw_100uv: 0,
                raw_dac_code: 0,
                meas_physical: 0,
            }; MAX_POINTS],
            len: 0,
        }
    }

    pub fn as_slice(&self) -> &[CalPoint] {
        &self.points[..self.len as usize]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format)]
pub enum CurveKind {
    VLocal = 0,
    VRemote = 1,
    CurrentCh1 = 2,
    CurrentCh2 = 3,
}

impl TryFrom<u8> for CurveKind {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CurveKind::VLocal),
            1 => Ok(CurveKind::VRemote),
            2 => Ok(CurveKind::CurrentCh1),
            3 => Ok(CurveKind::CurrentCh2),
            _ => Err(()),
        }
    }
}

impl CurveKind {
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format)]
pub enum CalError {
    EmptyPoints,
    NonMonotonicMeas,
    InvalidCurve,
    InvalidChunk,
    CrcMismatch,
    VersionMismatch,
    InconsistentHeader,
    TooManyPoints,
    TooManyChunks,
}

/// Prepare a curve in-place: sort by raw, dedup by raw, and validate meas monotonicity.
/// Returns the new length.
pub fn prepare_curve(points: &mut [CalPoint; MAX_POINTS], len: usize) -> Result<usize, CalError> {
    if len == 0 {
        return Err(CalError::EmptyPoints);
    }
    let slice = &mut points[..len];
    // Insertion sort by raw_100uv (len ≤ 7, no_std friendly).
    for i in 1..slice.len() {
        let key = slice[i];
        let mut j = i;
        while j > 0 && slice[j - 1].raw_100uv > key.raw_100uv {
            slice[j] = slice[j - 1];
            j -= 1;
        }
        slice[j] = key;
    }

    // Dedup by raw_100uv, keeping the last occurrence after sort (matches the
    // digital side + web preview behavior).
    let mut w = 0usize;
    for i in 0..slice.len() {
        if w == 0 || slice[i].raw_100uv != slice[w - 1].raw_100uv {
            slice[w] = slice[i];
            w += 1;
        } else {
            // Same raw: overwrite prior entry so the last occurrence wins.
            slice[w - 1] = slice[i];
        }
    }

    // Validate meas monotonic increasing.
    for i in 1..w {
        if slice[i].meas_physical <= slice[i - 1].meas_physical {
            return Err(CalError::NonMonotonicMeas);
        }
    }

    Ok(w)
}

/// Piecewise linear mapping from raw (100 µV units) to calibrated physical value.
///
/// The input points must be prepared (sorted, deduped, meas monotonic).
pub fn piecewise_linear(points: &[CalPoint], raw: i16) -> Result<i32, CalError> {
    if points.is_empty() {
        return Err(CalError::EmptyPoints);
    }
    // Validate raw ascending and meas monotonic to catch accidental misuse.
    for i in 1..points.len() {
        if points[i].raw_100uv <= points[i - 1].raw_100uv
            || points[i].meas_physical <= points[i - 1].meas_physical
        {
            return Err(CalError::InvalidCurve);
        }
    }

    let raw_i32 = raw as i32;
    if points.len() == 1 {
        let p0 = points[0];
        if p0.raw_100uv == 0 {
            return Ok(p0.meas_physical);
        }
        let num = raw_i32 as i64 * p0.meas_physical as i64;
        let den = p0.raw_100uv as i64;
        return Ok((num / den).clamp(i32::MIN as i64, i32::MAX as i64) as i32);
    }

    // Find segment by raw.
    if raw_i32 <= points[0].raw_100uv as i32 {
        return interpolate_segment(points[0], points[1], raw_i32);
    }
    let last = points.len() - 1;
    if raw_i32 >= points[last].raw_100uv as i32 {
        return interpolate_segment(points[last - 1], points[last], raw_i32);
    }

    for win in points.windows(2) {
        let a = win[0];
        let b = win[1];
        if raw_i32 >= a.raw_100uv as i32 && raw_i32 <= b.raw_100uv as i32 {
            return interpolate_segment(a, b, raw_i32);
        }
    }

    Err(CalError::InvalidCurve)
}

fn interpolate_segment(a: CalPoint, b: CalPoint, raw: i32) -> Result<i32, CalError> {
    let raw_a = a.raw_100uv as i32;
    let raw_b = b.raw_100uv as i32;
    let den = raw_b - raw_a;
    if den == 0 {
        return Err(CalError::InvalidCurve);
    }
    let t_num = (raw - raw_a) as i64;
    let den_i64 = den as i64;
    let meas_a = a.meas_physical as i64;
    let meas_b = b.meas_physical as i64;
    let out = meas_a + (meas_b - meas_a) * t_num / den_i64;
    Ok(out.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

/// Inverse mapping from desired physical value to raw (100 µV units).
///
/// The input points must be prepared (sorted, deduped, meas monotonic).
pub fn inverse_piecewise(points: &[CalPoint], meas_des: i32) -> Result<i16, CalError> {
    if points.is_empty() {
        return Err(CalError::EmptyPoints);
    }
    for i in 1..points.len() {
        if points[i].raw_100uv <= points[i - 1].raw_100uv
            || points[i].meas_physical <= points[i - 1].meas_physical
        {
            return Err(CalError::InvalidCurve);
        }
    }

    if points.len() == 1 {
        let p0 = points[0];
        if p0.meas_physical == 0 {
            return Ok(0);
        }
        let num = meas_des as i64 * p0.raw_100uv as i64;
        let den = p0.meas_physical as i64;
        let raw = num / den;
        return Ok(raw.clamp(i16::MIN as i64, i16::MAX as i64) as i16);
    }

    let meas_des_i64 = meas_des as i64;
    let first = points[0];
    let last = points[points.len() - 1];

    if meas_des_i64 <= first.meas_physical as i64 {
        return inverse_segment(first, points[1], meas_des_i64);
    }
    if meas_des_i64 >= last.meas_physical as i64 {
        return inverse_segment(points[points.len() - 2], last, meas_des_i64);
    }

    for win in points.windows(2) {
        let a = win[0];
        let b = win[1];
        if meas_des_i64 >= a.meas_physical as i64 && meas_des_i64 <= b.meas_physical as i64 {
            return inverse_segment(a, b, meas_des_i64);
        }
    }

    Err(CalError::InvalidCurve)
}

fn inverse_segment(a: CalPoint, b: CalPoint, meas_des: i64) -> Result<i16, CalError> {
    let meas_a = a.meas_physical as i64;
    let meas_b = b.meas_physical as i64;
    let den = meas_b - meas_a;
    if den == 0 {
        return Err(CalError::InvalidCurve);
    }
    let t_num = meas_des - meas_a;
    let raw_a = a.raw_100uv as i64;
    let raw_b = b.raw_100uv as i64;
    let raw = raw_a + (raw_b - raw_a) * t_num / den;
    Ok(raw.clamp(i16::MIN as i64, i16::MAX as i64) as i16)
}

pub fn mv_to_raw_100uv(mv: u32) -> i16 {
    let raw = (mv as i32) * 10;
    raw.clamp(0, i16::MAX as i32) as i16
}

/// Piecewise linear mapping from raw (100 µV units) to DAC code using the
/// current calibration curve's `raw_dac_code` field.
///
/// The input curve is expected to be the active curve produced by
/// [`prepare_curve`] (sorted by raw, deduped by raw, meas monotonic). This
/// helper additionally enforces strictly increasing `raw_dac_code` by dropping
/// non-monotonic DAC samples so interpolation remains well-defined.
pub fn raw_100uv_to_dac_code_calibrated(
    points: &[CalPoint],
    raw_100uv: i16,
) -> Result<u16, CalError> {
    if points.is_empty() {
        return Err(CalError::EmptyPoints);
    }

    let raw_i32 = raw_100uv as i32;
    if raw_i32 <= 0 {
        return Ok(0);
    }

    // Build a monotonic (raw -> dac) mapping from the received calibration curve.
    // Keep the first point, then only accept points whose DAC code is strictly
    // increasing. This drops spurious/noisy DAC samples without rejecting the
    // entire curve.
    let mut raws = [0i16; MAX_POINTS];
    let mut dacs = [0u16; MAX_POINTS];
    let mut n = 0usize;

    for (idx, p) in points.iter().enumerate() {
        if idx > 0 && p.raw_100uv <= points[idx - 1].raw_100uv {
            // Active curves should already be strictly increasing in raw.
            return Err(CalError::InvalidCurve);
        }

        if n == 0 {
            raws[0] = p.raw_100uv;
            dacs[0] = p.raw_dac_code;
            n = 1;
            continue;
        }

        // Enforce strictly increasing DAC codes.
        if p.raw_dac_code <= dacs[n - 1] {
            continue;
        }
        raws[n] = p.raw_100uv;
        dacs[n] = p.raw_dac_code;
        n += 1;
        if n >= MAX_POINTS {
            break;
        }
    }

    if n < 2 {
        return Err(CalError::InvalidCurve);
    }

    // Helper: interpolate/extrapolate within a segment.
    fn interp(raw_a: i16, raw_b: i16, dac_a: u16, dac_b: u16, raw: i32) -> Result<u16, CalError> {
        let raw_a_i32 = raw_a as i32;
        let raw_b_i32 = raw_b as i32;
        let den = (raw_b_i32 - raw_a_i32) as i64;
        if den == 0 {
            return Err(CalError::InvalidCurve);
        }
        let t_num = (raw - raw_a_i32) as i64;
        let dac_a_i64 = dac_a as i64;
        let dac_b_i64 = dac_b as i64;
        let out = dac_a_i64 + (dac_b_i64 - dac_a_i64) * t_num / den;
        Ok(out.clamp(0, 4095) as u16)
    }

    // Segment selection mirrors `piecewise_linear` (extrapolate using end segments).
    if raw_i32 <= raws[0] as i32 {
        return interp(raws[0], raws[1], dacs[0], dacs[1], raw_i32);
    }
    let last = n - 1;
    if raw_i32 >= raws[last] as i32 {
        return interp(
            raws[last - 1],
            raws[last],
            dacs[last - 1],
            dacs[last],
            raw_i32,
        );
    }

    for i in 0..(n - 1) {
        let a_raw = raws[i] as i32;
        let b_raw = raws[i + 1] as i32;
        if raw_i32 >= a_raw && raw_i32 <= b_raw {
            return interp(raws[i], raws[i + 1], dacs[i], dacs[i + 1], raw_i32);
        }
    }

    Err(CalError::InvalidCurve)
}

/// Fallback mapping from raw (100 µV units) to DAC code using the current ADC
/// reference voltage (`vref_mv`), assuming `V_DAC ≈ V_SNS`.
pub fn raw_100uv_to_dac_code_vref(raw_100uv: i16, vref_mv: u32) -> u16 {
    let raw_i32 = raw_100uv as i32;
    if raw_i32 <= 0 || vref_mv == 0 {
        return 0;
    }
    // raw_100uv -> mV: raw/10.
    // dac_code = V(mV) / vref_mv * 4095 = raw * 4095 / (vref_mv * 10).
    let num = raw_i32 as i64 * 4095;
    let den = (vref_mv as i64) * 10;
    (num / den).clamp(0, 4095) as u16
}

#[derive(Copy, Clone, Debug, Default)]
struct PendingCurve {
    fmt_version: u8,
    hw_rev: u8,
    total_chunks: u8,
    total_points: u8,
    received_chunks_mask: u8,
    filled: [bool; MAX_POINTS],
    points: [CalPoint; MAX_POINTS],
    active: bool,
}

impl PendingCurve {
    const fn empty() -> Self {
        Self {
            fmt_version: 0,
            hw_rev: 0,
            total_chunks: 0,
            total_points: 0,
            received_chunks_mask: 0,
            filled: [false; MAX_POINTS],
            points: [CalPoint {
                raw_100uv: 0,
                raw_dac_code: 0,
                meas_physical: 0,
            }; MAX_POINTS],
            active: false,
        }
    }

    fn reset(&mut self) {
        *self = PendingCurve::empty();
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CalibrationState {
    active_curves: [CalCurve; 4],
    active_valid: [bool; 4],
    pending: [PendingCurve; 4],
}

impl Default for CalibrationState {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationState {
    pub const fn new() -> Self {
        Self {
            active_curves: [CalCurve::empty(); 4],
            active_valid: [false; 4],
            pending: [PendingCurve::empty(); 4],
        }
    }

    pub fn snapshot(&self) -> [CalCurve; 4] {
        self.active_curves
    }

    pub fn all_valid(&self) -> bool {
        self.active_valid.iter().all(|v| *v)
    }

    /// Feed one decoded CalWrite chunk into the receiver.
    ///
    /// Returns `Ok(Some(kind))` when a kind completes and becomes active,
    /// `Ok(None)` when the chunk was accepted but not yet complete,
    /// or `Err` on fatal validation failure for this kind (old active kept).
    pub fn ingest_cal_write(
        &mut self,
        index: u8,
        payload: &[u8; 32],
        crc: u16,
    ) -> Result<Option<CurveKind>, CalError> {
        // Inner CRC check over index + payload.
        let mut buf = [0u8; 33];
        buf[0] = index;
        buf[1..].copy_from_slice(payload);
        let calc = crc16_ccitt_false(&buf);
        if calc != crc {
            return Err(CalError::CrcMismatch);
        }

        let fmt_version = payload[0];
        let hw_rev = payload[1];
        let kind_raw = payload[2];
        let chunk_index = payload[3];
        let total_chunks = payload[4];
        let total_points = payload[5];

        if fmt_version != 1 && fmt_version != 2 && fmt_version != 3 {
            return Err(CalError::VersionMismatch);
        }
        if total_points as usize > MAX_POINTS {
            return Err(CalError::TooManyPoints);
        }
        if total_chunks == 0 || total_chunks as usize > MAX_CHUNKS {
            return Err(CalError::TooManyChunks);
        }
        if chunk_index >= total_chunks {
            return Err(CalError::InvalidChunk);
        }
        if index != chunk_index {
            return Err(CalError::InvalidChunk);
        }

        let kind = CurveKind::try_from(kind_raw).map_err(|_| CalError::InvalidChunk)?;
        let pending_idx = kind.index();
        let pending = &mut self.pending[pending_idx];

        if !pending.active {
            pending.fmt_version = fmt_version;
            pending.hw_rev = hw_rev;
            pending.total_chunks = total_chunks;
            pending.total_points = total_points;
            pending.active = true;
        } else if pending.fmt_version != fmt_version
            || pending.hw_rev != hw_rev
            || pending.total_chunks != total_chunks
            || pending.total_points != total_points
        {
            pending.reset();
            return Err(CalError::InconsistentHeader);
        }

        // Unpack up to 3 points from this chunk.
        for point_off in 0..POINTS_PER_CHUNK {
            let overall = (chunk_index as usize) * POINTS_PER_CHUNK + point_off;
            if overall >= total_points as usize {
                continue;
            }
            let base = 8 + point_off * 8;
            let raw_100uv = i16::from_le_bytes([payload[base], payload[base + 1]]);
            let raw_dac_code = u16::from_le_bytes([payload[base + 2], payload[base + 3]]);
            let meas_physical = i32::from_le_bytes([
                payload[base + 4],
                payload[base + 5],
                payload[base + 6],
                payload[base + 7],
            ]);
            pending.points[overall] = CalPoint {
                raw_100uv,
                raw_dac_code,
                meas_physical,
            };
            pending.filled[overall] = true;
        }

        pending.received_chunks_mask |= 1u8 << chunk_index;

        // Check completion.
        let all_chunks = pending.received_chunks_mask.count_ones() as u8 == total_chunks;
        let all_points = pending.filled[..total_points as usize].iter().all(|b| *b);

        if all_chunks && all_points {
            let mut pts = pending.points;
            match prepare_curve(&mut pts, total_points as usize) {
                Ok(new_len) => {
                    let mut curve = CalCurve::empty();
                    curve.len = new_len as u8;
                    curve.points[..new_len].copy_from_slice(&pts[..new_len]);
                    self.active_curves[pending_idx] = curve;
                    self.active_valid[pending_idx] = true;
                    pending.reset();
                    return Ok(Some(kind));
                }
                Err(err) => {
                    pending.reset();
                    return Err(err);
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(raw: i16, meas: i32) -> CalPoint {
        CalPoint {
            raw_100uv: raw,
            raw_dac_code: 0,
            meas_physical: meas,
        }
    }

    fn pt_dac(raw: i16, dac: u16, meas: i32) -> CalPoint {
        CalPoint {
            raw_100uv: raw,
            raw_dac_code: dac,
            meas_physical: meas,
        }
    }

    #[test]
    fn piecewise_one_point_scale() {
        let points = [pt(1000, 2000)];
        let out = piecewise_linear(&points, 1500).unwrap();
        assert_eq!(out, 3000);
    }

    #[test]
    fn piecewise_two_points_linear() {
        let points = [pt(1000, 1000), pt(3000, 5000)];
        let out = piecewise_linear(&points, 2000).unwrap();
        assert_eq!(out, 3000);
    }

    #[test]
    fn piecewise_multi_points_segment() {
        let points = [pt(1000, 1000), pt(2000, 2000), pt(4000, 3000)];
        assert_eq!(piecewise_linear(&points, 1500).unwrap(), 1500);
        assert_eq!(piecewise_linear(&points, 3000).unwrap(), 2500);
    }

    #[test]
    fn raw_to_dac_calibrated_basic_interpolation() {
        let points = [pt_dac(0, 0, 0), pt_dac(2500, 353, 500)];
        assert_eq!(
            raw_100uv_to_dac_code_calibrated(&points, 1250).unwrap(),
            176
        );
    }

    #[test]
    fn raw_to_dac_calibrated_drops_non_monotonic_dac_points() {
        let points = [
            pt_dac(0, 0, 0),
            pt_dac(1000, 200, 100),
            pt_dac(2000, 150, 200), // non-monotonic DAC, should be dropped
            pt_dac(3000, 400, 300),
        ];
        // Interpolate between (1000->200) and (3000->400).
        assert_eq!(
            raw_100uv_to_dac_code_calibrated(&points, 2000).unwrap(),
            300
        );
    }

    #[test]
    fn raw_to_dac_calibrated_rejects_all_zero_dac_curve() {
        let points = [pt_dac(0, 0, 0), pt_dac(25_000, 0, 5000)];
        assert_eq!(
            raw_100uv_to_dac_code_calibrated(&points, 1000).unwrap_err(),
            CalError::InvalidCurve
        );
    }

    #[test]
    fn raw_to_dac_vref_matches_2p9v_expectation() {
        // 0.5A -> 0.25V -> 250mV -> raw=2500 (100uV).
        assert_eq!(raw_100uv_to_dac_code_vref(2500, 2900), 353);
    }

    #[test]
    fn piecewise_extrapolation() {
        let points = [pt(1000, 1000), pt(2000, 2000)];
        assert_eq!(piecewise_linear(&points, 0).unwrap(), 0);
        assert_eq!(piecewise_linear(&points, 3000).unwrap(), 3000);
    }

    #[test]
    fn inverse_in_range() {
        let points = [pt(1000, 1000), pt(3000, 5000)];
        let raw = inverse_piecewise(&points, 3000).unwrap();
        assert_eq!(raw, 2000);
    }

    #[test]
    fn inverse_extrapolation() {
        let points = [pt(1000, 1000), pt(2000, 2000)];
        assert_eq!(inverse_piecewise(&points, 0).unwrap(), 0);
        assert_eq!(inverse_piecewise(&points, 3000).unwrap(), 3000);
    }

    #[test]
    fn prepare_rejects_non_monotonic_meas() {
        let mut pts = [CalPoint::default(); MAX_POINTS];
        pts[0] = pt(1000, 1000);
        pts[1] = pt(2000, 900);
        let err = prepare_curve(&mut pts, 2).unwrap_err();
        assert_eq!(err, CalError::NonMonotonicMeas);
    }
}

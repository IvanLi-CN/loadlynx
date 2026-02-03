#![no_std]

/// Breathing animation helpers (pure logic).
///
/// Kept in `libs/` so it can be covered by host-side unit tests.
pub mod breathing {
    /// Triangle-wave "breathing" brightness.
    ///
    /// - `now_ms`: current time in milliseconds (wrapping is OK; only `mod period_ms` is used).
    /// - `period_ms`: full breathe period in milliseconds (up + down).
    /// - `max_brightness_pct`: upper clamp of the returned brightness (logical %, 0..=100).
    ///
    /// Returns brightness as a logical percent (0..=max_brightness_pct, clipped to 100).
    #[inline]
    pub fn triangle_breathe_pct(now_ms: u32, period_ms: u32, max_brightness_pct: u8) -> u8 {
        // Use u64 intermediates to avoid overflow when `period_ms` is large.
        let max = max_brightness_pct.min(100) as u64;
        if max == 0 || period_ms < 2 {
            return 0;
        }

        let period = period_ms as u64;
        let phase = (now_ms as u64) % period; // 0..period-1
        let half = period / 2;
        if half == 0 {
            return 0;
        }

        // Linear up (0..half) then linear down (half..period).
        let lin = if phase <= half {
            (max * phase) / half
        } else {
            (max * (period - phase)) / half
        };

        lin.min(max) as u8
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::breathing::triangle_breathe_pct;

    #[test]
    fn triangle_breathe_boundaries() {
        let period = 1000;
        let max = 50;
        assert_eq!(triangle_breathe_pct(0, period, max), 0);
        assert_eq!(triangle_breathe_pct(500, period, max), 50);
        assert_eq!(triangle_breathe_pct(1000, period, max), 0);
    }

    #[test]
    fn triangle_breathe_midpoints() {
        let period = 1000;
        let max = 60;
        assert_eq!(triangle_breathe_pct(250, period, max), 30);
        assert_eq!(triangle_breathe_pct(750, period, max), 30);
    }

    #[test]
    fn triangle_breathe_clamps_max_to_100() {
        let period = 1000;
        let max = 150;
        assert_eq!(triangle_breathe_pct(500, period, max), 100);
    }

    #[test]
    fn triangle_breathe_handles_zero_period() {
        assert_eq!(triangle_breathe_pct(123, 0, 50), 0);
        assert_eq!(triangle_breathe_pct(123, 1, 50), 0);
    }

    #[test]
    fn triangle_breathe_large_period_no_overflow() {
        let period = u32::MAX; // would overflow old u32 intermediates in debug builds
        let max = 100;
        assert_eq!(triangle_breathe_pct(0, period, max), 0);
        assert_eq!(triangle_breathe_pct(period / 2, period, max), 100);
    }
}

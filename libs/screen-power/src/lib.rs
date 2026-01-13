#![no_std]

#[cfg(test)]
extern crate std;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenPowerState {
    Active,
    Dim,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenPowerConfig {
    pub dim_after_ms: u32,
    pub off_after_ms: u32,
    pub dim_max_pct: u8,
}

impl ScreenPowerConfig {
    pub const fn new(dim_after_ms: u32, off_after_ms: u32, dim_max_pct: u8) -> Self {
        Self {
            dim_after_ms,
            off_after_ms,
            dim_max_pct,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenPowerTransition {
    pub from: ScreenPowerState,
    pub to: ScreenPowerState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenPowerTick {
    pub state: ScreenPowerState,
    pub target_backlight_pct: u8,
    pub idle_ms: u32,
    pub transition: Option<ScreenPowerTransition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenPowerModel {
    state: ScreenPowerState,
    /// "Normal"/active backlight brightness that should be restored on wake.
    active_backlight_pct: u8,
}

impl ScreenPowerModel {
    pub const fn new(active_backlight_pct: u8) -> Self {
        Self {
            state: ScreenPowerState::Active,
            active_backlight_pct,
        }
    }

    pub const fn state(&self) -> ScreenPowerState {
        self.state
    }

    pub const fn active_backlight_pct(&self) -> u8 {
        self.active_backlight_pct
    }

    pub fn set_active_backlight_pct(&mut self, pct: u8) {
        self.active_backlight_pct = pct;
    }

    pub fn dim_backlight_pct(&self, cfg: ScreenPowerConfig) -> u8 {
        self.active_backlight_pct.min(cfg.dim_max_pct)
    }

    pub fn tick(
        &mut self,
        cfg: ScreenPowerConfig,
        now_ms: u32,
        last_user_activity_ms: u32,
        load_enabled: bool,
    ) -> ScreenPowerTick {
        let idle_ms = now_ms.wrapping_sub(last_user_activity_ms);

        let desired = if load_enabled {
            ScreenPowerState::Active
        } else if idle_ms >= cfg.off_after_ms {
            ScreenPowerState::Off
        } else if idle_ms >= cfg.dim_after_ms {
            ScreenPowerState::Dim
        } else {
            ScreenPowerState::Active
        };

        let transition = if desired != self.state {
            let from = self.state;
            self.state = desired;
            Some(ScreenPowerTransition { from, to: desired })
        } else {
            None
        };

        let target_backlight_pct = match self.state {
            ScreenPowerState::Active => self.active_backlight_pct,
            ScreenPowerState::Dim => self.dim_backlight_pct(cfg),
            ScreenPowerState::Off => 0,
        };

        ScreenPowerTick {
            state: self.state,
            target_backlight_pct,
            idle_ms,
            transition,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dim_pct_never_brightens() {
        let cfg = ScreenPowerConfig::new(120_000, 300_000, 10);
        for &orig in &[0u8, 1, 8, 9, 10, 11, 49, 50, 80, 100] {
            let model = ScreenPowerModel::new(orig);
            assert_eq!(model.dim_backlight_pct(cfg), orig.min(10));
        }
    }

    #[test]
    fn transitions_respect_wraparound_idle() {
        let cfg = ScreenPowerConfig::new(120_000, 300_000, 10);
        let mut model = ScreenPowerModel::new(80);

        // Simulate wrap-around: last activity just before u32::MAX, now is small.
        let last = u32::MAX - 10;
        let now = 20;
        let tick = model.tick(cfg, now, last, false);
        assert_eq!(tick.idle_ms, 31);
        assert_eq!(tick.state, ScreenPowerState::Active);

        // Jump forward enough to exceed dim threshold.
        let now2 = last.wrapping_add(cfg.dim_after_ms + 1);
        let tick2 = model.tick(cfg, now2, last, false);
        assert_eq!(tick2.state, ScreenPowerState::Dim);

        // Jump forward enough to exceed off threshold.
        let now3 = last.wrapping_add(cfg.off_after_ms + 1);
        let tick3 = model.tick(cfg, now3, last, false);
        assert_eq!(tick3.state, ScreenPowerState::Off);

        // Any activity forces Active.
        let tick4 = model.tick(cfg, now3, now3, false);
        assert_eq!(tick4.state, ScreenPowerState::Active);

        // LOAD ON forces Active regardless of idle.
        let tick5 = model.tick(cfg, now3, last, true);
        assert_eq!(tick5.state, ScreenPowerState::Active);
    }
}

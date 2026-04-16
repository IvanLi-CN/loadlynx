use core::sync::atomic::Ordering;

use loadlynx_calibration_format as calfmt;
use loadlynx_protocol::{CalKind, LoadMode, PdStatus};

use crate::ui::preset_panel::{PresetPanelDigit, PresetPanelField};

pub const PRESET_COUNT: usize = 5;

pub const HARD_MAX_I_MA_TOTAL: i32 = 10_000;
pub const HARD_MAX_V_MV: i32 = 55_000;
pub const DEFAULT_MIN_V_MV: i32 = 0;
pub const DEFAULT_MAX_I_MA_TOTAL: i32 = HARD_MAX_I_MA_TOTAL;
pub const DEFAULT_MAX_P_MW: u32 = crate::HARD_MAX_P_MW;

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum AdjustDigit {
    Ones,
    Tenths,
    Hundredths,
    Thousandths,
}

impl AdjustDigit {
    pub const DEFAULT: Self = Self::Tenths;

    pub fn step_milli(self) -> i32 {
        match self {
            AdjustDigit::Ones => 1_000,    // 1.00
            AdjustDigit::Tenths => 100,    // 0.10
            AdjustDigit::Hundredths => 10, // 0.01
            AdjustDigit::Thousandths => 1, // 0.001
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preset {
    pub preset_id: u8, // 1..=5
    pub mode: LoadMode,
    pub target_p_mw: u32,
    pub target_i_ma: i32,
    pub target_v_mv: i32,
    pub min_v_mv: i32,
    pub max_i_ma_total: i32,
    pub max_p_mw: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalibrationCcOverride {
    pub output_enabled: bool,
    pub target_i_ma: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectiveOutputCommand {
    pub preset: Preset,
    pub output_enabled: bool,
}

pub const fn calibration_mode_uses_cc_override(cal_mode: CalKind) -> bool {
    matches!(cal_mode, CalKind::CurrentCh1 | CalKind::CurrentCh2)
}

impl Preset {
    pub fn clamp(mut self) -> Self {
        // preset_id is validated by callers; do not mutate it here.

        // Non-negative invariants.
        self.target_i_ma = self.target_i_ma.max(0);
        self.target_v_mv = self.target_v_mv.max(0).min(HARD_MAX_V_MV);
        self.min_v_mv = self.min_v_mv.max(0).min(HARD_MAX_V_MV);
        self.max_i_ma_total = self.max_i_ma_total.max(0);

        // Hard clamps.
        self.max_i_ma_total = self.max_i_ma_total.min(HARD_MAX_I_MA_TOTAL);
        let hard_max_p = crate::LIMIT_PROFILE_DEFAULT.max_p_mw;
        self.max_p_mw = self.max_p_mw.min(hard_max_p);
        self.target_p_mw = self.target_p_mw.min(hard_max_p);

        // Frozen UI invariants:
        // - CC:  TARGET_I <= OCP (max_i_ma_total)
        // - CV:  UVLO (min_v_mv) <= TARGET_V (target_v_mv)
        if self.mode == LoadMode::Cv {
            self.target_v_mv = self.target_v_mv.max(self.min_v_mv);
        }
        // - CP:  TARGET_P <= OPP (max_p_mw)
        if self.mode == LoadMode::Cp && self.target_p_mw > self.max_p_mw {
            self.target_p_mw = self.max_p_mw;
        }

        // Targets should never exceed the current caps.
        self.target_i_ma = self.target_i_ma.min(self.max_i_ma_total);
        self
    }
}

pub fn default_presets() -> [Preset; PRESET_COUNT] {
    // Safe defaults: output remains OFF on boot; targets are conservative and
    // can be updated by the user via HTTP.
    [
        Preset {
            preset_id: 1,
            mode: LoadMode::Cc,
            target_p_mw: 0,
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
        Preset {
            preset_id: 2,
            mode: LoadMode::Cc,
            target_p_mw: 0,
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
        Preset {
            preset_id: 3,
            mode: LoadMode::Cc,
            target_p_mw: 0,
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
        Preset {
            preset_id: 4,
            mode: LoadMode::Cc,
            target_p_mw: 0,
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
        Preset {
            preset_id: 5,
            mode: LoadMode::Cc,
            target_p_mw: 0,
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiView {
    Main,
    PresetPanel,
    PresetPanelBlocked,
    PdSettings,
    #[cfg(feature = "audio_menu")]
    AudioMenu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum PdMode {
    Fixed = 0,
    Pps = 1,
}

impl PdMode {
    pub const DEFAULT: Self = Self::Fixed;

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Fixed),
            1 => Some(Self::Pps),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum PdSettingsFocus {
    None = 0,
    Vreq = 1,
    Ireq = 2,
}

impl PdSettingsFocus {
    pub const DEFAULT: Self = Self::None;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub struct PdConfig {
    pub mode: PdMode,
    /// Selected Fixed PDO object position (1-based). `0` means "not explicitly selected".
    ///
    /// Note: capability lists may omit `pos` (legacy), in which case the effective position is
    /// derived from the list index on the digital side.
    pub fixed_object_pos: u8,
    /// Selected PPS APDO object position (1-based). `0` means "not selected" and disables Apply.
    pub pps_object_pos: u8,
    /// Active target voltage. In Fixed mode this mirrors the selected PDO voltage; in PPS mode it
    /// mirrors the currently edited PPS target.
    pub target_mv: u32,
    /// Sticky PPS target cache so browsing/saving Fixed PDOs does not overwrite the last PPS Vreq.
    pub pps_target_mv: u32,
    pub i_req_ma: u32,
}

impl PdConfig {
    pub const DEFAULT_TARGET_MV: u32 = 5_000;
    pub const DEFAULT_I_REQ_MA: u32 = 3_000;
    pub const MIN_AUGMENTED_TARGET_MV: u32 = 3_000;
    pub const MAX_PPS_TARGET_MV: u32 = 21_000;
    pub const MAX_FIXED_TARGET_MV: u32 = 48_000;

    pub const fn default() -> Self {
        Self {
            mode: PdMode::DEFAULT,
            fixed_object_pos: 0,
            pps_object_pos: 0,
            target_mv: Self::DEFAULT_TARGET_MV,
            pps_target_mv: Self::DEFAULT_TARGET_MV,
            i_req_ma: Self::DEFAULT_I_REQ_MA,
        }
    }

    pub const fn safe5v() -> Self {
        Self::safe5v_with_i(Self::DEFAULT_I_REQ_MA)
    }

    pub const fn safe5v_with_i(i_req_ma: u32) -> Self {
        Self {
            mode: PdMode::Fixed,
            fixed_object_pos: 0,
            pps_object_pos: 0,
            target_mv: Self::DEFAULT_TARGET_MV,
            pps_target_mv: Self::DEFAULT_TARGET_MV,
            i_req_ma,
        }
    }

    pub const fn effective(saved: Self, allow_extended_voltage: bool) -> Self {
        if allow_extended_voltage {
            saved
        } else {
            // Force 5V but preserve the user-saved current request.
            Self::safe5v_with_i(saved.i_req_ma)
        }
    }

    pub const fn allows_non_safe5v(self) -> bool {
        match self.mode {
            PdMode::Fixed => self.target_mv != Self::DEFAULT_TARGET_MV,
            PdMode::Pps => self.pps_target_mv != Self::DEFAULT_TARGET_MV,
        }
    }

    pub fn toggle_target(&mut self) -> bool {
        let next = if self.target_mv >= 28_000 {
            5_000
        } else if self.target_mv >= 20_000 {
            28_000
        } else {
            20_000
        };
        if next == self.target_mv {
            false
        } else {
            self.target_mv = next;
            true
        }
    }
}

pub const EPR_FIXED_28V_OBJECT_POS: u8 = 8;
pub const EPR_FIXED_28V_MV: u32 = 28_000;
pub const EPR_FIXED_28V_MAX_MA: u32 = 5_000;
pub const UNKNOWN_PDO_MAX_MA: u32 = 0;
pub const MAX_PD_OBJECT_POS: u8 = 16;
pub const MAX_SUPPORTED_FIXED_TARGET_MV: u32 = EPR_FIXED_28V_MV;

pub const fn supported_epr_fixed_selection(object_pos: u8) -> Option<(u32, u32)> {
    if object_pos == EPR_FIXED_28V_OBJECT_POS {
        Some((EPR_FIXED_28V_MV, UNKNOWN_PDO_MAX_MA))
    } else {
        None
    }
}

pub const fn supported_epr_fixed_target(object_pos: u8, target_mv: u32) -> Option<u32> {
    if let Some((mv, _max_ma)) = supported_epr_fixed_selection(object_pos) {
        if target_mv == mv {
            return Some(mv);
        }
    }
    None
}

/// USB PD R3.2 v1.1 Tables 10.12 / 10.13 make 28V Fixed the baseline EPR fixed rail once the
/// source advertises the SPR-side EPR-capable bit; before EPR entry the sink has to surface that
/// row synthetically because the real EPR Fixed PDOs are not visible yet.
pub fn can_advertise_synthetic_epr_fixed(status: Option<&PdStatus>) -> bool {
    status.map(|s| !s.attached || s.epr_capable).unwrap_or(true)
}

// Fixed/PPS paths report mV * mA and AVS exposes PDP in watts, so keep the common unit in uW.
pub fn source_max_power_uw(status: &PdStatus) -> u32 {
    let fixed_max = status
        .fixed_pdos
        .iter()
        .map(|pdo| pdo.mv.saturating_mul(pdo.max_ma))
        .max()
        .unwrap_or(0);
    let pps_max = status
        .pps_pdos
        .iter()
        .map(|pdo| pdo.max_mv.saturating_mul(pdo.max_ma))
        .max()
        .unwrap_or(0);
    let epr_max = status
        .epr_avs_pdos
        .iter()
        .map(|pdo| u32::from(pdo.pdp_w).saturating_mul(1_000_000))
        .max()
        .unwrap_or(0);
    fixed_max.max(pps_max).max(epr_max)
}

pub fn effective_pdo_i_req_limit(
    status: Option<&PdStatus>,
    object_pos: u8,
    target_mv: u32,
    advertised_max_ma: u32,
) -> Option<u32> {
    if advertised_max_ma != UNKNOWN_PDO_MAX_MA {
        return Some(advertised_max_ma);
    }

    let status = status?;
    if !status.attached || target_mv == 0 {
        return None;
    }
    if supported_epr_fixed_target(object_pos, target_mv).is_none() {
        return None;
    }

    let max_power_uw = source_max_power_uw(status);
    if max_power_uw == 0 {
        return None;
    }

    Some((max_power_uw / target_mv).clamp(50, EPR_FIXED_28V_MAX_MA))
}

pub fn i_req_within_effective_pdo_limit(
    status: Option<&PdStatus>,
    object_pos: u8,
    target_mv: u32,
    advertised_max_ma: u32,
    i_req_ma: u32,
) -> bool {
    i_req_ma >= 50
        && effective_pdo_i_req_limit(status, object_pos, target_mv, advertised_max_ma)
            .map(|limit| i_req_ma <= limit)
            .unwrap_or(true)
}

pub fn clamp_i_req_to_effective_pdo_limit(
    status: Option<&PdStatus>,
    object_pos: u8,
    target_mv: u32,
    advertised_max_ma: u32,
    i_req_ma: u32,
) -> u32 {
    effective_pdo_i_req_limit(status, object_pos, target_mv, advertised_max_ma)
        .map(|limit| i_req_ma.min(limit))
        .unwrap_or(i_req_ma)
        .max(50)
}

#[derive(Clone, Debug)]
pub struct ControlState {
    /// Mutable in-RAM working presets.
    pub presets: [Preset; PRESET_COUNT],
    /// Last successfully persisted snapshot (EEPROM baseline).
    pub saved: [Preset; PRESET_COUNT],
    /// Whether `presets[i] != saved[i]`.
    pub dirty: [bool; PRESET_COUNT],
    pub active_preset_id: u8,  // 1..=5
    pub editing_preset_id: u8, // 1..=5
    pub output_enabled: bool,
    pub calibration_cc_override: Option<CalibrationCcOverride>,
    calibration_cc_restore_output_enabled: Option<bool>,
    pub adjust_digit: AdjustDigit,
    pub ui_view: UiView,
    pub panel_selected_field: PresetPanelField,
    pub panel_selected_digit: PresetPanelDigit,
    /// Persisted PD policy (EEPROM-backed); used by the UART PD apply task.
    pub pd_saved: PdConfig,
    /// User-controlled gate that decides whether the runtime PD policy may leave Safe5V.
    pub allow_extended_voltage: bool,
    /// Draft PD policy edited in the PD settings UI; copied to `pd_saved` on Apply.
    pub pd_draft: PdConfig,
    pub pd_settings_focus: PdSettingsFocus,
    pub pd_settings_digit: AdjustDigit,
}

impl ControlState {
    pub fn new(
        presets: [Preset; PRESET_COUNT],
        pd: PdConfig,
        allow_extended_voltage: bool,
    ) -> Self {
        Self {
            presets,
            saved: presets,
            dirty: [false; PRESET_COUNT],
            active_preset_id: 1,
            editing_preset_id: 1,
            output_enabled: false,
            calibration_cc_override: None,
            calibration_cc_restore_output_enabled: None,
            adjust_digit: AdjustDigit::DEFAULT,
            ui_view: UiView::Main,
            panel_selected_field: PresetPanelField::Target,
            panel_selected_digit: PresetPanelDigit::Tenths,
            pd_saved: pd,
            allow_extended_voltage,
            pd_draft: pd,
            pd_settings_focus: PdSettingsFocus::DEFAULT,
            pd_settings_digit: AdjustDigit::Tenths,
        }
    }

    pub fn active_preset(&self) -> Preset {
        let idx = self.active_preset_id.saturating_sub(1) as usize;
        self.presets.get(idx).copied().unwrap_or(self.presets[0])
    }

    fn set_live_output_enabled(&mut self, output_enabled: bool) {
        self.output_enabled = output_enabled;
        crate::DESIRED_OUTPUT_ENABLED.store(output_enabled, Ordering::Relaxed);
    }

    pub fn set_normal_output_enabled(&mut self, output_enabled: bool) {
        if self.calibration_cc_override.is_some() {
            self.calibration_cc_restore_output_enabled = Some(output_enabled);
            return;
        }
        self.set_live_output_enabled(output_enabled);
    }

    pub fn set_calibration_restore_output_enabled(&mut self, output_enabled: bool) {
        self.calibration_cc_restore_output_enabled = Some(output_enabled);
    }

    pub fn calibration_restore_output_enabled(&self) -> Option<bool> {
        self.calibration_cc_restore_output_enabled
    }

    pub fn restore_calibration_output_state(
        &mut self,
        output_enabled: bool,
        calibration_cc_override: Option<CalibrationCcOverride>,
        calibration_restore_output_enabled: Option<bool>,
    ) {
        self.calibration_cc_override = calibration_cc_override;
        self.calibration_cc_restore_output_enabled = calibration_restore_output_enabled;
        self.set_live_output_enabled(output_enabled);
    }

    pub fn force_output_off(&mut self) {
        if let Some(mut override_state) = self.calibration_cc_override {
            override_state.output_enabled = false;
            self.calibration_cc_override = Some(override_state);
        }
        self.calibration_cc_restore_output_enabled = None;
        self.set_live_output_enabled(false);
    }

    pub fn set_calibration_cc_override(&mut self, target_i_ma: i32, output_enabled: bool) {
        let target_i_ma = target_i_ma
            .max(0)
            .min(crate::LIMIT_PROFILE_DEFAULT.max_i_ma)
            .min(HARD_MAX_I_MA_TOTAL);
        let effective_output_enabled = output_enabled && target_i_ma != 0;
        self.calibration_cc_restore_output_enabled
            .get_or_insert(self.output_enabled);
        self.calibration_cc_override = Some(CalibrationCcOverride {
            output_enabled: effective_output_enabled,
            target_i_ma,
        });
        self.set_live_output_enabled(effective_output_enabled);
    }

    pub fn sync_live_output_for_mode(&mut self, cal_mode: CalKind) -> bool {
        if !calibration_mode_uses_cc_override(cal_mode) {
            return false;
        }

        let restore_output_enabled = self
            .calibration_cc_restore_output_enabled
            .or(Some(self.output_enabled));
        let effective_output_enabled = self.effective_output_command(cal_mode).output_enabled;
        let changed = self.calibration_cc_restore_output_enabled != restore_output_enabled
            || self.output_enabled != effective_output_enabled;
        if changed {
            self.restore_calibration_output_state(
                effective_output_enabled,
                self.calibration_cc_override,
                restore_output_enabled,
            );
        }
        changed
    }

    pub fn apply_calibration_mode_transition(
        &mut self,
        prev_kind: CalKind,
        next_kind: CalKind,
        allow_restore_output: bool,
    ) -> bool {
        if prev_kind == next_kind {
            return false;
        }

        let prev_uses_override = calibration_mode_uses_cc_override(prev_kind);
        let next_uses_override = calibration_mode_uses_cc_override(next_kind);

        if !prev_uses_override && next_uses_override {
            return self.sync_live_output_for_mode(next_kind);
        }

        if prev_uses_override && !next_uses_override {
            if self.calibration_restore_output_enabled().unwrap_or(false) && !allow_restore_output {
                self.set_calibration_restore_output_enabled(false);
            }
            return self.clear_calibration_cc_override(true);
        }

        false
    }

    pub fn set_calibration_output_enabled(&mut self, output_enabled: bool) -> bool {
        self.calibration_cc_restore_output_enabled
            .get_or_insert(self.output_enabled);

        if let Some(mut override_state) = self.calibration_cc_override {
            if output_enabled && override_state.target_i_ma == 0 {
                return false;
            }
            override_state.output_enabled = output_enabled && override_state.target_i_ma != 0;
            self.calibration_cc_override = Some(override_state);
            self.set_live_output_enabled(override_state.output_enabled);
            return true;
        }

        if output_enabled {
            return false;
        }

        self.set_live_output_enabled(false);
        true
    }

    pub fn enable_output_for_mode(&mut self, cal_mode: CalKind) -> bool {
        if calibration_mode_uses_cc_override(cal_mode) {
            return self.set_calibration_output_enabled(true);
        }

        self.set_normal_output_enabled(true);
        true
    }

    pub fn disable_output_for_mode(&mut self, cal_mode: CalKind) {
        if calibration_mode_uses_cc_override(cal_mode) {
            let _ = self.set_calibration_output_enabled(false);
            return;
        }

        self.force_output_off();
    }

    pub fn clear_calibration_cc_override(&mut self, restore_normal_output: bool) -> bool {
        let had_override = self.calibration_cc_override.take().is_some();
        if !had_override {
            if restore_normal_output {
                if let Some(output_enabled) = self.calibration_cc_restore_output_enabled.take() {
                    self.set_live_output_enabled(output_enabled);
                }
            }
            return false;
        }

        if restore_normal_output {
            if let Some(output_enabled) = self.calibration_cc_restore_output_enabled.take() {
                self.set_live_output_enabled(output_enabled);
            } else {
                self.set_live_output_enabled(false);
            }
        } else {
            self.set_live_output_enabled(false);
        }
        true
    }

    pub fn effective_output_command(&self, cal_mode: CalKind) -> EffectiveOutputCommand {
        if calibration_mode_uses_cc_override(cal_mode) {
            let active = self.active_preset();
            let override_state = self
                .calibration_cc_override
                .unwrap_or(CalibrationCcOverride {
                    output_enabled: false,
                    target_i_ma: 0,
                });
            let mut preset = active;
            let max_i_ma_total = preset.max_i_ma_total.min(HARD_MAX_I_MA_TOTAL);
            preset.mode = LoadMode::Cc;
            preset.target_p_mw = 0;
            preset.target_i_ma = override_state.target_i_ma.min(max_i_ma_total);
            return EffectiveOutputCommand {
                preset,
                output_enabled: override_state.output_enabled && override_state.target_i_ma != 0,
            };
        }

        EffectiveOutputCommand {
            preset: self.active_preset(),
            output_enabled: self.output_enabled,
        }
    }

    fn preset_idx(preset_id: u8) -> Option<usize> {
        if preset_id == 0 || preset_id > PRESET_COUNT as u8 {
            return None;
        }
        Some((preset_id - 1) as usize)
    }

    fn update_dirty_for_idx(&mut self, idx: usize) {
        if idx < PRESET_COUNT {
            self.dirty[idx] = self.presets[idx] != self.saved[idx];
        }
    }

    pub fn update_dirty_for_preset_id(&mut self, preset_id: u8) {
        let Some(idx) = Self::preset_idx(preset_id) else {
            return;
        };
        self.update_dirty_for_idx(idx);
    }

    pub fn commit_saved_for_preset_id(&mut self, preset_id: u8) {
        let Some(idx) = Self::preset_idx(preset_id) else {
            return;
        };
        self.saved[idx] = self.presets[idx];
        self.dirty[idx] = false;
    }

    /// Discard dirty changes for all *non-active* presets.
    ///
    /// Frozen rule: closing the preset panel reverts non-active dirty presets
    /// back to the last saved snapshot, but preserves the active preset.
    pub fn close_panel_discard(&mut self) {
        for idx in 0..PRESET_COUNT {
            let preset_id = (idx + 1) as u8;
            if preset_id == self.active_preset_id {
                continue;
            }
            self.presets[idx] = self.saved[idx];
            self.dirty[idx] = false;
        }
    }

    /// Activate `preset_id` as the new active preset.
    ///
    /// Frozen rule: when switching active presets, discard dirty changes for the
    /// old active preset (revert working <- saved) before switching, and always
    /// force output OFF for safety.
    pub fn activate_preset(&mut self, preset_id: u8) {
        if preset_id != self.active_preset_id {
            if let Some(old_idx) = Self::preset_idx(self.active_preset_id) {
                self.presets[old_idx] = self.saved[old_idx];
                self.dirty[old_idx] = false;
            }
            self.active_preset_id = preset_id;
        }

        // Safety: activation always forces output OFF.
        self.force_output_off();
    }

    /// Set the mode for the current `editing_preset_id`.
    ///
    /// Frozen rule: if editing the active preset and the mode actually changes,
    /// force output OFF. Editing a non-active preset must not affect output.
    pub fn set_mode_for_editing_preset(&mut self, mode: LoadMode) {
        let Some(idx) = Self::preset_idx(self.editing_preset_id) else {
            return;
        };

        let prev_mode = self.presets[idx].mode;
        if prev_mode == mode {
            return;
        }

        self.presets[idx].mode = mode;
        self.presets[idx] = self.presets[idx].clamp();
        self.update_dirty_for_idx(idx);

        if self.editing_preset_id == self.active_preset_id {
            self.force_output_off();
        }
    }
}

// ---- EEPROM presets blob ----------------------------------------------------

const PRESETS_MAGIC: [u8; 4] = *b"LLXP";
const PRESETS_FMT_VERSION: u8 = 1;
const PRESETS_HEADER_LEN: usize = 8;
const PRESET_RECORD_LEN: usize = 28;

fn put_u16_le(out: &mut [u8], offset: usize, v: u16) {
    out[offset..offset + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32_le(out: &mut [u8], offset: usize, v: u32) {
    out[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_i32_le(out: &mut [u8], offset: usize, v: i32) {
    out[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

fn get_u32_le(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

fn get_i32_le(input: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetsBlobError {
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidCount(u8),
    CrcMismatch { stored: u32, computed: u32 },
    InvalidLayout,
    InvalidPresetId(u8),
    InvalidMode(u8),
}

pub fn encode_presets_blob(
    presets: &[Preset; PRESET_COUNT],
) -> [u8; crate::eeprom::EEPROM_PRESETS_LEN] {
    let mut out = [0u8; crate::eeprom::EEPROM_PRESETS_LEN];
    out[0..4].copy_from_slice(&PRESETS_MAGIC);
    out[4] = PRESETS_FMT_VERSION;
    out[5] = PRESET_COUNT as u8;
    out[6] = 0;
    out[7] = 0;

    for (idx, p) in presets.iter().copied().enumerate() {
        let base = PRESETS_HEADER_LEN + idx * PRESET_RECORD_LEN;
        out[base] = p.preset_id;
        out[base + 1] = u8::from(p.mode);
        put_u16_le(&mut out, base + 2, 0);
        put_i32_le(&mut out, base + 4, p.target_i_ma);
        put_i32_le(&mut out, base + 8, p.target_v_mv);
        put_i32_le(&mut out, base + 12, p.min_v_mv);
        put_i32_le(&mut out, base + 16, p.max_i_ma_total);
        put_u32_le(&mut out, base + 20, p.max_p_mw);
        // v1 reserved field repurposed for CP target power (mW).
        put_u32_le(&mut out, base + 24, p.target_p_mw);
    }

    let crc_offset = crate::eeprom::EEPROM_PRESETS_LEN - 4;
    let crc = calfmt::crc32_ieee(&out[..crc_offset]);
    put_u32_le(&mut out, crc_offset, crc);
    out
}

pub fn decode_presets_blob(
    bytes: &[u8; crate::eeprom::EEPROM_PRESETS_LEN],
) -> Result<[Preset; PRESET_COUNT], PresetsBlobError> {
    if bytes[0..4] != PRESETS_MAGIC {
        return Err(PresetsBlobError::InvalidMagic);
    }
    let ver = bytes[4];
    if ver != PRESETS_FMT_VERSION {
        return Err(PresetsBlobError::UnsupportedVersion(ver));
    }
    let count = bytes[5];
    if count != PRESET_COUNT as u8 {
        return Err(PresetsBlobError::InvalidCount(count));
    }

    let crc_offset = crate::eeprom::EEPROM_PRESETS_LEN - 4;
    let stored_crc = get_u32_le(bytes, crc_offset);
    let computed_crc = calfmt::crc32_ieee(&bytes[..crc_offset]);
    if stored_crc != computed_crc {
        return Err(PresetsBlobError::CrcMismatch {
            stored: stored_crc,
            computed: computed_crc,
        });
    }

    let expected_end = PRESETS_HEADER_LEN + PRESET_COUNT * PRESET_RECORD_LEN;
    if expected_end > crc_offset {
        return Err(PresetsBlobError::InvalidLayout);
    }

    let mut out = default_presets();
    for idx in 0..PRESET_COUNT {
        let base = PRESETS_HEADER_LEN + idx * PRESET_RECORD_LEN;
        let preset_id = bytes[base];
        if preset_id == 0 || preset_id > PRESET_COUNT as u8 {
            return Err(PresetsBlobError::InvalidPresetId(preset_id));
        }
        let mode_raw = bytes[base + 1];
        let mode = match LoadMode::from(mode_raw) {
            LoadMode::Cc => LoadMode::Cc,
            LoadMode::Cv => LoadMode::Cv,
            LoadMode::Cp => LoadMode::Cp,
            LoadMode::Reserved(raw) => return Err(PresetsBlobError::InvalidMode(raw)),
        };

        let target_i_ma = get_i32_le(bytes, base + 4);
        let target_v_mv = get_i32_le(bytes, base + 8);
        let min_v_mv = get_i32_le(bytes, base + 12);
        let max_i_ma_total = get_i32_le(bytes, base + 16);
        let max_p_mw = get_u32_le(bytes, base + 20);
        let target_p_mw = get_u32_le(bytes, base + 24);

        out[(preset_id - 1) as usize] = Preset {
            preset_id,
            mode,
            target_p_mw,
            target_i_ma,
            target_v_mv,
            min_v_mv,
            max_i_ma_total,
            max_p_mw,
        }
        .clamp();
    }

    // Validate that all 1..=5 slots exist after mapping.
    for (i, p) in out.iter().enumerate() {
        let expected = (i + 1) as u8;
        if p.preset_id != expected {
            return Err(PresetsBlobError::InvalidPresetId(p.preset_id));
        }
    }

    Ok(out)
}

// ---- EEPROM PD config blob -------------------------------------------------

const PD_MAGIC: [u8; 4] = *b"LLPD";
const PD_FMT_VERSION: u8 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdBlobError {
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidMode(u8),
    InvalidTarget(u32),
    CrcMismatch { stored: u32, computed: u32 },
    InvalidLayout,
}

pub fn encode_pd_blob(
    cfg: &PdConfig,
    allow_extended_voltage: bool,
) -> [u8; crate::eeprom::EEPROM_PD_LEN] {
    let mut out = [0u8; crate::eeprom::EEPROM_PD_LEN];
    out[0..4].copy_from_slice(&PD_MAGIC);
    out[4] = PD_FMT_VERSION;
    out[5] = cfg.mode as u8;
    out[6] = cfg.fixed_object_pos;
    out[7] = cfg.pps_object_pos;
    put_u32_le(&mut out, 8, cfg.target_mv);
    put_u32_le(&mut out, 12, cfg.i_req_ma);
    out[16] = u8::from(allow_extended_voltage);
    put_u32_le(&mut out, 17, cfg.pps_target_mv);
    // out[21..28] reserved = 0

    let crc_offset = crate::eeprom::EEPROM_PD_LEN - 4;
    let crc = calfmt::crc32_ieee(&out[..crc_offset]);
    put_u32_le(&mut out, crc_offset, crc);
    out
}

pub fn decode_pd_blob(
    bytes: &[u8; crate::eeprom::EEPROM_PD_LEN],
) -> Result<(PdConfig, bool), PdBlobError> {
    if bytes[0..4] != PD_MAGIC {
        return Err(PdBlobError::InvalidMagic);
    }
    let ver = bytes[4];
    if ver != 1 && ver != 2 && ver != 3 && ver != 4 && ver != 5 && ver != PD_FMT_VERSION {
        return Err(PdBlobError::UnsupportedVersion(ver));
    }

    let crc_offset = crate::eeprom::EEPROM_PD_LEN - 4;
    if crc_offset < 12 {
        return Err(PdBlobError::InvalidLayout);
    }
    let stored_crc = get_u32_le(bytes, crc_offset);
    let computed_crc = calfmt::crc32_ieee(&bytes[..crc_offset]);
    if stored_crc != computed_crc {
        return Err(PdBlobError::CrcMismatch {
            stored: stored_crc,
            computed: computed_crc,
        });
    }

    let mode_raw = bytes[5];
    let mode = PdMode::from_u8(mode_raw).ok_or(PdBlobError::InvalidMode(mode_raw))?;

    let fixed_object_pos = if ver >= 3 { bytes[6] } else { 0 };
    let pps_object_pos = if ver >= 3 { bytes[7] } else { 0 };
    let target_mv = get_u32_le(bytes, 8);
    if ver == 1 && target_mv != 5_000 && target_mv != 20_000 {
        return Err(PdBlobError::InvalidTarget(target_mv));
    }
    let max_target_mv = match mode {
        PdMode::Fixed => MAX_SUPPORTED_FIXED_TARGET_MV,
        PdMode::Pps => PdConfig::MAX_PPS_TARGET_MV,
    };
    if ver >= 2 && (target_mv < PdConfig::MIN_AUGMENTED_TARGET_MV || target_mv > max_target_mv) {
        return Err(PdBlobError::InvalidTarget(target_mv));
    }

    let i_req_ma = if ver >= 2 {
        get_u32_le(bytes, 12)
    } else {
        PdConfig::DEFAULT_I_REQ_MA
    };
    let allow_extended_voltage = if ver >= 4 { bytes[16] != 0 } else { false };
    let (target_mv, pps_target_mv) = if ver >= 5 {
        let cached = get_u32_le(bytes, 17);
        let cached = if (PdConfig::MIN_AUGMENTED_TARGET_MV..=PdConfig::MAX_PPS_TARGET_MV)
            .contains(&cached)
        {
            cached
        } else {
            PdConfig::DEFAULT_TARGET_MV
        };
        (target_mv, cached)
    } else if matches!(mode, PdMode::Pps) {
        (target_mv, target_mv)
    } else {
        // Legacy fixed-mode blobs reused `target_mv` inconsistently (fixed display vs PPS cache).
        // Keep it for both fields so the UI/API stays stable after upgrade; fixed-mode requests
        // still derive their voltage from the selected PDO when status is available.
        (target_mv, target_mv)
    };

    Ok((
        PdConfig {
            mode,
            fixed_object_pos,
            pps_object_pos,
            target_mv,
            pps_target_mv,
            i_req_ma,
        },
        allow_extended_voltage,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_v3_blob(cfg: &PdConfig) -> [u8; crate::eeprom::EEPROM_PD_LEN] {
        let mut out = [0u8; crate::eeprom::EEPROM_PD_LEN];
        out[0..4].copy_from_slice(&PD_MAGIC);
        out[4] = 3;
        out[5] = cfg.mode as u8;
        out[6] = cfg.fixed_object_pos;
        out[7] = cfg.pps_object_pos;
        put_u32_le(&mut out, 8, cfg.target_mv);
        put_u32_le(&mut out, 12, cfg.i_req_ma);
        let crc_offset = crate::eeprom::EEPROM_PD_LEN - 4;
        let crc = calfmt::crc32_ieee(&out[..crc_offset]);
        put_u32_le(&mut out, crc_offset, crc);
        out
    }

    fn encode_v4_blob(
        cfg: &PdConfig,
        allow_extended_voltage: bool,
    ) -> [u8; crate::eeprom::EEPROM_PD_LEN] {
        let mut out = [0u8; crate::eeprom::EEPROM_PD_LEN];
        out[0..4].copy_from_slice(&PD_MAGIC);
        out[4] = 4;
        out[5] = cfg.mode as u8;
        out[6] = cfg.fixed_object_pos;
        out[7] = cfg.pps_object_pos;
        put_u32_le(&mut out, 8, cfg.target_mv);
        put_u32_le(&mut out, 12, cfg.i_req_ma);
        out[16] = u8::from(allow_extended_voltage);
        let crc_offset = crate::eeprom::EEPROM_PD_LEN - 4;
        let crc = calfmt::crc32_ieee(&out[..crc_offset]);
        put_u32_le(&mut out, crc_offset, crc);
        out
    }

    #[test]
    fn pd_blob_roundtrip_preserves_allow_extended_voltage() {
        let cfg = PdConfig {
            mode: PdMode::Pps,
            fixed_object_pos: 4,
            pps_object_pos: 2,
            target_mv: 9_000,
            pps_target_mv: 9_000,
            i_req_ma: 2_000,
        };

        let blob = encode_pd_blob(&cfg, true);
        let (decoded, allow_extended_voltage) = decode_pd_blob(&blob).expect("decode v5 blob");
        assert_eq!(decoded, cfg);
        assert!(allow_extended_voltage);
    }

    #[test]
    fn pd_blob_roundtrip_preserves_28v_fixed_target() {
        let cfg = PdConfig {
            mode: PdMode::Fixed,
            fixed_object_pos: 8,
            pps_object_pos: 0,
            target_mv: 28_000,
            pps_target_mv: 9_000,
            i_req_ma: 5_000,
        };

        let blob = encode_pd_blob(&cfg, true);
        let (decoded, allow_extended_voltage) = decode_pd_blob(&blob).expect("decode v6 blob");
        assert_eq!(decoded, cfg);
        assert!(allow_extended_voltage);
    }

    #[test]
    fn pd_blob_rejects_pps_target_above_21v() {
        let cfg = PdConfig {
            mode: PdMode::Pps,
            fixed_object_pos: 0,
            pps_object_pos: 2,
            target_mv: 28_000,
            pps_target_mv: 28_000,
            i_req_ma: 3_000,
        };

        let blob = encode_pd_blob(&cfg, true);
        let err = decode_pd_blob(&blob).unwrap_err();
        assert_eq!(err, PdBlobError::InvalidTarget(28_000));
    }

    #[test]
    fn pd_blob_rejects_fixed_target_above_28v() {
        let cfg = PdConfig {
            mode: PdMode::Fixed,
            fixed_object_pos: 9,
            pps_object_pos: 0,
            target_mv: 36_000,
            pps_target_mv: 9_000,
            i_req_ma: 3_000,
        };

        let blob = encode_pd_blob(&cfg, true);
        let err = decode_pd_blob(&blob).unwrap_err();
        assert_eq!(err, PdBlobError::InvalidTarget(36_000));
    }

    #[test]
    fn pd_blob_v3_fixed_keeps_target_voltage() {
        let legacy = PdConfig {
            mode: PdMode::Fixed,
            fixed_object_pos: 4,
            pps_object_pos: 2,
            target_mv: 20_000,
            pps_target_mv: PdConfig::DEFAULT_TARGET_MV,
            i_req_ma: 3_000,
        };

        let blob = encode_v3_blob(&legacy);
        let (decoded, allow_extended_voltage) = decode_pd_blob(&blob).expect("decode v3 blob");
        assert_eq!(decoded.mode, PdMode::Fixed);
        assert_eq!(decoded.fixed_object_pos, 4);
        assert_eq!(decoded.target_mv, 20_000);
        assert_eq!(decoded.pps_target_mv, 20_000);
        assert!(!allow_extended_voltage);
    }

    #[test]
    fn pd_blob_v4_fixed_keeps_target_voltage() {
        let legacy = PdConfig {
            mode: PdMode::Fixed,
            fixed_object_pos: 4,
            pps_object_pos: 2,
            target_mv: 20_000,
            pps_target_mv: PdConfig::DEFAULT_TARGET_MV,
            i_req_ma: 3_000,
        };

        let blob = encode_v4_blob(&legacy, true);
        let (decoded, allow_extended_voltage) = decode_pd_blob(&blob).expect("decode v4 blob");
        assert_eq!(decoded.mode, PdMode::Fixed);
        assert_eq!(decoded.fixed_object_pos, 4);
        assert_eq!(decoded.target_mv, 20_000);
        assert_eq!(decoded.pps_target_mv, 20_000);
        assert!(allow_extended_voltage);
    }

    #[test]
    fn supported_epr_fixed_selection_only_advertises_28v() {
        assert_eq!(
            supported_epr_fixed_selection(EPR_FIXED_28V_OBJECT_POS),
            Some((EPR_FIXED_28V_MV, UNKNOWN_PDO_MAX_MA))
        );
        assert_eq!(supported_epr_fixed_selection(9), None);
    }

    #[test]
    fn supported_epr_fixed_target_requires_matching_voltage() {
        assert_eq!(
            supported_epr_fixed_target(EPR_FIXED_28V_OBJECT_POS, EPR_FIXED_28V_MV),
            Some(EPR_FIXED_28V_MV)
        );
        assert_eq!(
            supported_epr_fixed_target(EPR_FIXED_28V_OBJECT_POS, 20_000),
            None
        );
    }

    #[test]
    fn synthetic_epr_fixed_advertises_when_detached_or_epr_capable() {
        let detached = PdStatus {
            attached: false,
            ..PdStatus::default()
        };
        let attached_spr_only = PdStatus {
            attached: true,
            epr_capable: false,
            ..PdStatus::default()
        };
        let attached_epr_capable = PdStatus {
            attached: true,
            epr_capable: true,
            ..PdStatus::default()
        };

        assert!(can_advertise_synthetic_epr_fixed(None));
        assert!(can_advertise_synthetic_epr_fixed(Some(&detached)));
        assert!(!can_advertise_synthetic_epr_fixed(Some(&attached_spr_only)));
        assert!(can_advertise_synthetic_epr_fixed(Some(
            &attached_epr_capable
        )));
    }

    #[test]
    fn effective_pdo_i_req_limit_uses_attached_source_power_for_synthetic_28v() {
        let mut status = PdStatus {
            attached: true,
            epr_capable: true,
            ..PdStatus::default()
        };
        let _ = status.fixed_pdos.push(loadlynx_protocol::FixedPdo {
            pos: 4,
            mv: 20_000,
            max_ma: 5_000,
        });

        assert_eq!(
            effective_pdo_i_req_limit(
                Some(&status),
                EPR_FIXED_28V_OBJECT_POS,
                EPR_FIXED_28V_MV,
                UNKNOWN_PDO_MAX_MA
            ),
            Some(3_571)
        );
        assert!(!i_req_within_effective_pdo_limit(
            Some(&status),
            EPR_FIXED_28V_OBJECT_POS,
            EPR_FIXED_28V_MV,
            UNKNOWN_PDO_MAX_MA,
            3_600
        ));
        assert_eq!(
            clamp_i_req_to_effective_pdo_limit(
                Some(&status),
                EPR_FIXED_28V_OBJECT_POS,
                EPR_FIXED_28V_MV,
                UNKNOWN_PDO_MAX_MA,
                3_600
            ),
            3_571
        );
    }

    #[test]
    fn effective_pdo_i_req_limit_caps_synthetic_28v_at_fixed_5a() {
        let mut status = PdStatus {
            attached: true,
            epr_capable: true,
            ..PdStatus::default()
        };
        let _ = status.epr_avs_pdos.push(loadlynx_protocol::EprAvsPdo {
            pos: 10,
            min_mv: 15_000,
            max_mv: 28_000,
            pdp_w: 240,
        });

        assert_eq!(
            effective_pdo_i_req_limit(
                Some(&status),
                EPR_FIXED_28V_OBJECT_POS,
                EPR_FIXED_28V_MV,
                UNKNOWN_PDO_MAX_MA
            ),
            Some(EPR_FIXED_28V_MAX_MA)
        );
    }

    #[test]
    fn effective_output_command_uses_calibration_override_only_in_current_mode() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        crate::DESIRED_OUTPUT_ENABLED.store(true, Ordering::Relaxed);
        state.output_enabled = true;
        state.presets[0].target_i_ma = 900;
        state.presets[0].target_v_mv = 12_500;
        state.presets[0].min_v_mv = 11_200;
        state.presets[0].max_i_ma_total = 1_200;
        state.presets[0].max_p_mw = 18_500;
        state.set_calibration_cc_override(2_000, true);

        let current = state.effective_output_command(CalKind::CurrentCh1);
        assert_eq!(current.preset.mode, LoadMode::Cc);
        assert_eq!(current.preset.target_i_ma, 1_200);
        assert_eq!(current.preset.target_v_mv, 12_500);
        assert_eq!(current.preset.min_v_mv, 11_200);
        assert_eq!(current.preset.max_i_ma_total, 1_200);
        assert_eq!(current.preset.max_p_mw, 18_500);
        assert!(current.output_enabled);
        assert!(state.output_enabled);
        assert!(crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        let normal = state.effective_output_command(CalKind::Off);
        assert_eq!(normal.preset.target_i_ma, 900);
        assert!(normal.output_enabled);

        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn clearing_calibration_override_restores_normal_control() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        state.presets[0].target_i_ma = 1_100;
        state.output_enabled = true;
        crate::DESIRED_OUTPUT_ENABLED.store(true, Ordering::Relaxed);
        state.set_calibration_cc_override(1_500, true);
        state.set_calibration_cc_override(0, true);
        assert!(!state.output_enabled);
        assert!(!crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));
        assert!(state.clear_calibration_cc_override(true));
        assert!(!state.clear_calibration_cc_override(true));

        let current = state.effective_output_command(CalKind::CurrentCh2);
        assert_eq!(current.preset.target_i_ma, 0);
        assert!(!current.output_enabled);

        let normal = state.effective_output_command(CalKind::Off);
        assert_eq!(normal.preset.target_i_ma, 1_100);
        assert!(normal.output_enabled);
        assert!(state.output_enabled);
        assert!(crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn normal_output_updates_during_calibration_are_restored_on_exit() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
        state.set_calibration_cc_override(1_500, true);
        state.set_normal_output_enabled(false);

        assert!(state.output_enabled);
        assert!(crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        assert!(state.clear_calibration_cc_override(true));
        assert!(!state.output_enabled);
        assert!(!crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));
    }

    #[test]
    fn clearing_calibration_override_without_restore_keeps_channel_switch_off() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        state.output_enabled = true;
        crate::DESIRED_OUTPUT_ENABLED.store(true, Ordering::Relaxed);
        state.set_calibration_cc_override(1_500, true);

        assert!(state.clear_calibration_cc_override(false));
        assert!(!state.output_enabled);
        assert!(!crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        state.set_calibration_restore_output_enabled(false);
        state.set_calibration_cc_override(900, true);
        assert!(state.clear_calibration_cc_override(true));
        assert!(!state.output_enabled);
        assert!(!crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn enabling_output_in_current_calibration_reuses_override_target() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
        state.set_calibration_cc_override(1_500, false);

        assert!(state.enable_output_for_mode(CalKind::CurrentCh1));
        assert!(state.output_enabled);
        assert!(crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        let current = state.effective_output_command(CalKind::CurrentCh1);
        assert_eq!(current.preset.target_i_ma, 1_500);
        assert!(current.output_enabled);

        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn enabling_output_in_current_calibration_without_target_is_blocked() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);

        assert!(!state.enable_output_for_mode(CalKind::CurrentCh2));
        assert!(!state.output_enabled);
        assert!(!crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));
    }

    #[test]
    fn entering_current_calibration_syncs_live_output_and_keeps_restore_state() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        state.output_enabled = true;
        crate::DESIRED_OUTPUT_ENABLED.store(true, Ordering::Relaxed);

        assert!(state.sync_live_output_for_mode(CalKind::CurrentCh1));
        assert!(!state.output_enabled);
        assert!(!crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));
        assert_eq!(state.calibration_restore_output_enabled(), Some(true));
        assert!(
            !state
                .effective_output_command(CalKind::CurrentCh1)
                .output_enabled
        );

        assert!(!state.clear_calibration_cc_override(true));
        assert!(state.output_enabled);
        assert!(crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn disabling_current_calibration_output_preserves_normal_restore_state() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        state.output_enabled = true;
        crate::DESIRED_OUTPUT_ENABLED.store(true, Ordering::Relaxed);

        assert!(state.sync_live_output_for_mode(CalKind::CurrentCh1));
        state.set_calibration_cc_override(1_500, true);
        state.disable_output_for_mode(CalKind::CurrentCh1);

        assert!(!state.output_enabled);
        assert!(!crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));
        assert_eq!(state.calibration_restore_output_enabled(), Some(true));
        assert_eq!(
            state.calibration_cc_override,
            Some(CalibrationCcOverride {
                output_enabled: false,
                target_i_ma: 1_500,
            })
        );

        assert!(state.clear_calibration_cc_override(true));
        assert!(state.output_enabled);
        assert!(crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn switching_between_current_calibration_modes_keeps_override_target() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        state.output_enabled = true;
        crate::DESIRED_OUTPUT_ENABLED.store(true, Ordering::Relaxed);
        state.set_calibration_cc_override(1_500, true);

        assert!(!state.apply_calibration_mode_transition(
            CalKind::CurrentCh1,
            CalKind::CurrentCh2,
            true,
        ));
        assert_eq!(
            state.calibration_cc_override,
            Some(CalibrationCcOverride {
                output_enabled: true,
                target_i_ma: 1_500,
            })
        );
        assert_eq!(state.calibration_restore_output_enabled(), Some(true));
        assert!(state.output_enabled);
        assert!(
            state
                .effective_output_command(CalKind::CurrentCh2)
                .output_enabled
        );
        assert!(crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn leaving_current_calibration_transition_obeys_restore_gate() {
        let mut state = ControlState::new(default_presets(), PdConfig::default(), false);
        state.output_enabled = true;
        crate::DESIRED_OUTPUT_ENABLED.store(true, Ordering::Relaxed);
        assert!(state.sync_live_output_for_mode(CalKind::CurrentCh1));
        state.set_calibration_cc_override(1_500, true);

        assert!(state.apply_calibration_mode_transition(CalKind::CurrentCh1, CalKind::Off, false,));
        assert_eq!(state.calibration_cc_override, None);
        assert_eq!(state.calibration_restore_output_enabled(), None);
        assert!(!state.output_enabled);
        assert!(!crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        state.output_enabled = true;
        crate::DESIRED_OUTPUT_ENABLED.store(true, Ordering::Relaxed);
        assert!(state.sync_live_output_for_mode(CalKind::CurrentCh2));
        state.set_calibration_cc_override(1_500, true);

        assert!(state.apply_calibration_mode_transition(
            CalKind::CurrentCh2,
            CalKind::Voltage,
            true,
        ));
        assert_eq!(state.calibration_cc_override, None);
        assert_eq!(state.calibration_restore_output_enabled(), None);
        assert!(state.output_enabled);
        assert!(crate::DESIRED_OUTPUT_ENABLED.load(Ordering::Relaxed));

        crate::DESIRED_OUTPUT_ENABLED.store(false, Ordering::Relaxed);
    }
}

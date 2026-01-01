use loadlynx_calibration_format as calfmt;
use loadlynx_protocol::LoadMode;

use crate::ui::preset_panel::{PresetPanelDigit, PresetPanelField};

pub const PRESET_COUNT: usize = 5;

pub const HARD_MAX_I_MA_TOTAL: i32 = 10_000;
pub const HARD_MAX_V_MV: i32 = 55_000;
pub const DEFAULT_MIN_V_MV: i32 = 0;
pub const DEFAULT_MAX_I_MA_TOTAL: i32 = HARD_MAX_I_MA_TOTAL;
pub const DEFAULT_MAX_P_MW: u32 = 150_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum AdjustDigit {
    Ones,
    Tenths,
    Hundredths,
}

impl AdjustDigit {
    pub const DEFAULT: Self = Self::Tenths;

    pub fn step_milli(self) -> i32 {
        match self {
            AdjustDigit::Ones => 1_000,    // 1.00
            AdjustDigit::Tenths => 100,    // 0.10
            AdjustDigit::Hundredths => 10, // 0.01
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preset {
    pub preset_id: u8, // 1..=5
    pub mode: LoadMode,
    pub target_i_ma: i32,
    pub target_v_mv: i32,
    pub min_v_mv: i32,
    pub max_i_ma_total: i32,
    pub max_p_mw: u32,
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

        // Targets should never exceed the current/power caps.
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
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
        Preset {
            preset_id: 2,
            mode: LoadMode::Cc,
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
        Preset {
            preset_id: 3,
            mode: LoadMode::Cc,
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
        Preset {
            preset_id: 4,
            mode: LoadMode::Cc,
            target_i_ma: 0,
            target_v_mv: 12_000,
            min_v_mv: DEFAULT_MIN_V_MV,
            max_i_ma_total: DEFAULT_MAX_I_MA_TOTAL,
            max_p_mw: DEFAULT_MAX_P_MW,
        },
        Preset {
            preset_id: 5,
            mode: LoadMode::Cc,
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
    pub adjust_digit: AdjustDigit,
    pub ui_view: UiView,
    pub panel_selected_field: PresetPanelField,
    pub panel_selected_digit: PresetPanelDigit,
}

impl ControlState {
    pub fn new(presets: [Preset; PRESET_COUNT]) -> Self {
        Self {
            presets,
            saved: presets,
            dirty: [false; PRESET_COUNT],
            active_preset_id: 1,
            editing_preset_id: 1,
            output_enabled: false,
            adjust_digit: AdjustDigit::DEFAULT,
            ui_view: UiView::Main,
            panel_selected_field: PresetPanelField::Target,
            panel_selected_digit: PresetPanelDigit::Tenths,
        }
    }

    pub fn active_preset(&self) -> Preset {
        let idx = self.active_preset_id.saturating_sub(1) as usize;
        self.presets.get(idx).copied().unwrap_or(self.presets[0])
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
        self.output_enabled = false;
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
            self.output_enabled = false;
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
        put_u32_le(&mut out, base + 24, 0);
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
            LoadMode::Reserved(raw) => return Err(PresetsBlobError::InvalidMode(raw)),
        };

        let target_i_ma = get_i32_le(bytes, base + 4);
        let target_v_mv = get_i32_le(bytes, base + 8);
        let min_v_mv = get_i32_le(bytes, base + 12);
        let max_i_ma_total = get_i32_le(bytes, base + 16);
        let max_p_mw = get_u32_le(bytes, base + 20);
        // reserved: base+24..+28

        out[(preset_id - 1) as usize] = Preset {
            preset_id,
            mode,
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

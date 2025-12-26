use loadlynx_calibration_format as calfmt;
use loadlynx_protocol::LoadMode;

pub const PRESET_COUNT: usize = 5;

pub const HARD_MAX_I_MA_TOTAL: i32 = 10_000;
pub const DEFAULT_MIN_V_MV: i32 = 0;
pub const DEFAULT_MAX_I_MA_TOTAL: i32 = HARD_MAX_I_MA_TOTAL;
pub const DEFAULT_MAX_P_MW: u32 = 150_000;

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
        self.target_v_mv = self.target_v_mv.max(0);
        self.min_v_mv = self.min_v_mv.max(0);
        self.max_i_ma_total = self.max_i_ma_total.max(0);

        // Hard clamps.
        self.max_i_ma_total = self.max_i_ma_total.min(HARD_MAX_I_MA_TOTAL);
        let hard_max_p = crate::LIMIT_PROFILE_DEFAULT.max_p_mw;
        self.max_p_mw = self.max_p_mw.min(hard_max_p);

        // Targets should never exceed the current/power caps. Voltage cap is
        // not defined at the digital layer (only non-negative is enforced).
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

#[derive(Clone, Debug)]
pub struct ControlState {
    pub presets: [Preset; PRESET_COUNT],
    pub active_preset_id: u8, // 1..=5
    pub output_enabled: bool,
}

impl ControlState {
    pub fn new(presets: [Preset; PRESET_COUNT]) -> Self {
        Self {
            presets,
            active_preset_id: 1,
            output_enabled: false,
        }
    }

    pub fn active_preset(&self) -> Preset {
        let idx = self.active_preset_id.saturating_sub(1) as usize;
        self.presets.get(idx).copied().unwrap_or(self.presets[0])
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

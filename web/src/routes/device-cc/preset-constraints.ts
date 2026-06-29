import type { LoadMode, Preset } from "../../api/types.ts";

export const PRESET_HARD_MAX_I_MA_TOTAL = 10_000;
export const PRESET_HARD_MAX_V_MV = 55_000;
export const PRESET_HARD_MAX_P_MW = 200_000;

export type PresetDraft = Pick<
  Preset,
  | "mode"
  | "target_i_ma"
  | "target_v_mv"
  | "target_p_mw"
  | "min_v_mv"
  | "max_i_ma_total"
  | "max_p_mw"
>;

function clampNumber(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min;
  }
  return Math.min(Math.max(value, min), max);
}

function normalizeMode(mode: LoadMode): LoadMode {
  if (mode === "cv" || mode === "cp" || mode === "cr") {
    return mode;
  }
  return "cc";
}

export function clampPresetDraft(input: PresetDraft): PresetDraft {
  const next: PresetDraft = {
    mode: normalizeMode(input.mode),
    target_i_ma: Math.max(0, Math.round(input.target_i_ma)),
    target_v_mv: clampNumber(
      Math.round(input.target_v_mv),
      0,
      PRESET_HARD_MAX_V_MV,
    ),
    target_p_mw: clampNumber(
      Math.round(input.target_p_mw),
      0,
      PRESET_HARD_MAX_P_MW,
    ),
    min_v_mv: clampNumber(Math.round(input.min_v_mv), 0, PRESET_HARD_MAX_V_MV),
    max_i_ma_total: clampNumber(
      Math.round(input.max_i_ma_total),
      0,
      PRESET_HARD_MAX_I_MA_TOTAL,
    ),
    max_p_mw: clampNumber(Math.round(input.max_p_mw), 0, PRESET_HARD_MAX_P_MW),
  };

  if (next.mode === "cv" && next.target_v_mv < next.min_v_mv) {
    next.target_v_mv = next.min_v_mv;
  }

  if (next.mode === "cp" && next.target_p_mw > next.max_p_mw) {
    next.target_p_mw = next.max_p_mw;
  }

  if (next.target_i_ma > next.max_i_ma_total) {
    next.target_i_ma = next.max_i_ma_total;
  }

  return next;
}

export function getPresetDraftBounds(input: PresetDraft) {
  const draft = clampPresetDraft(input);

  return {
    targetCurrent: {
      min: 0,
      max: Math.min(PRESET_HARD_MAX_I_MA_TOTAL, draft.max_i_ma_total),
      step: 50,
    },
    targetVoltage: {
      min: draft.mode === "cv" ? draft.min_v_mv : 0,
      max: PRESET_HARD_MAX_V_MV,
      step: 100,
    },
    targetPower: {
      min: 0,
      max: Math.min(PRESET_HARD_MAX_P_MW, draft.max_p_mw),
      step: 500,
    },
    minVoltage: {
      min: 0,
      max: PRESET_HARD_MAX_V_MV,
      step: 100,
    },
    maxCurrent: {
      min: 0,
      max: PRESET_HARD_MAX_I_MA_TOTAL,
      step: 50,
    },
    maxPower: {
      min: 0,
      max: PRESET_HARD_MAX_P_MW,
      step: 500,
    },
  } as const;
}

import {
  getPresetDraftBounds,
  type PresetDraft,
} from "../../routes/device-cc/preset-constraints.ts";

export type PresetEditorScaleInput = {
  draftPresetMode: PresetDraft["mode"];
  draftPresetTargetIMa: number;
  draftPresetTargetVMv: number;
  draftPresetTargetPMw: number;
  draftPresetMinVMv: number;
  draftPresetMaxIMaTotal: number;
  draftPresetMaxPMw: number;
};

export function getPresetEditorScales(input: PresetEditorScaleInput) {
  return getPresetDraftBounds({
    mode: input.draftPresetMode,
    target_i_ma: input.draftPresetTargetIMa,
    target_v_mv: input.draftPresetTargetVMv,
    target_p_mw: input.draftPresetTargetPMw,
    min_v_mv: input.draftPresetMinVMv,
    max_i_ma_total: input.draftPresetMaxIMaTotal,
    max_p_mw: input.draftPresetMaxPMw,
  });
}

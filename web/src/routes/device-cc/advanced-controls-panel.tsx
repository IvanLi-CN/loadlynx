import { useTranslation } from "react-i18next";
import { AdvancedPanel } from "../../components/instrument/advanced-panel.tsx";
import { ModeSliderSelector } from "../../components/instrument/mode-slider-selector.tsx";
import {
  BlockControlRow,
  BlockControlSliderRow,
} from "../../components/ui/block-control-row.tsx";
import type { MainDisplayCanvasProps } from "./main-display-canvas.tsx";
import { MainDisplayCanvas } from "./main-display-canvas.tsx";
import { getPresetDraftBounds } from "./preset-constraints.ts";

type PresetMode = "cc" | "cv" | "cp" | "cr";
type EditablePresetMode = Exclude<PresetMode, "cr">;
type PresetEditableField =
  | "target_i_ma"
  | "target_v_mv"
  | "target_p_mw"
  | "min_v_mv"
  | "max_i_ma_total"
  | "max_p_mw";
type PresetInputUnitKind = "current" | "voltage" | "power";

export interface AdvancedControlsPanelProps {
  collapsed: boolean;
  selectedPresetId: number | null;
  activePresetId: number | null;
  cpSupported: boolean;
  cpDraftOutOfRange: boolean;
  display: MainDisplayCanvasProps;
  draftPresetMode: PresetMode;
  draftPresetTargetIMa: number;
  draftPresetTargetVMv: number;
  draftPresetTargetPMw: number;
  draftPresetMinVMv: number;
  draftPresetMaxIMaTotal: number;
  draftPresetMaxPMw: number;
  getDisplayValue: (field: PresetEditableField, rawValue: number) => string;
  setDisplayDraft: (field: PresetEditableField, text: string) => void;
  commitDisplayDraft: (
    field: PresetEditableField,
    unitKind: PresetInputUnitKind,
    fallbackValue: number,
    setValue: (value: number) => void,
  ) => void;
  fieldError: (
    field: PresetEditableField,
    fallbackError?: string | null,
  ) => string | null;
  baseUrl: string | undefined;
  mockDevtoolsEnabled: boolean;
  saveDisabled: boolean;
  saveError: string | null;
  applyError: string | null;
  savePending: boolean;
  applyPending: boolean;
  onApplyPreset: () => void;
  onSavePreset: () => void;
  onSetCollapsed: (collapsed: boolean) => void;
  onModeChange: (mode: EditablePresetMode) => void;
  onTargetCurrentChange: (value: number) => void;
  onTargetVoltageChange: (value: number) => void;
  onTargetPowerChange: (value: number) => void;
  onMinVoltageChange: (value: number) => void;
  onMaxCurrentChange: (value: number) => void;
  onMaxPowerChange: (value: number) => void;
  onToggleMockUvLatch: () => void;
}

export function AdvancedControlsPanel({
  collapsed,
  selectedPresetId,
  activePresetId,
  cpSupported,
  cpDraftOutOfRange,
  display,
  draftPresetMode,
  draftPresetTargetIMa,
  draftPresetTargetVMv,
  draftPresetTargetPMw,
  draftPresetMinVMv,
  draftPresetMaxIMaTotal,
  draftPresetMaxPMw,
  getDisplayValue,
  setDisplayDraft,
  commitDisplayDraft,
  fieldError,
  baseUrl,
  mockDevtoolsEnabled,
  saveDisabled,
  saveError,
  applyError,
  savePending,
  applyPending,
  onApplyPreset,
  onSavePreset,
  onSetCollapsed,
  onModeChange,
  onTargetCurrentChange,
  onTargetVoltageChange,
  onTargetPowerChange,
  onMinVoltageChange,
  onMaxCurrentChange,
  onMaxPowerChange,
  onToggleMockUvLatch,
}: AdvancedControlsPanelProps) {
  const { t } = useTranslation();
  const isEditingActivePreset =
    selectedPresetId != null &&
    activePresetId != null &&
    selectedPresetId === activePresetId;

  const scales = getPresetDraftBounds({
    mode: draftPresetMode,
    target_i_ma: draftPresetTargetIMa,
    target_v_mv: draftPresetTargetVMv,
    target_p_mw: draftPresetTargetPMw,
    min_v_mv: draftPresetMinVMv,
    max_i_ma_total: draftPresetMaxIMaTotal,
    max_p_mw: draftPresetMaxPMw,
  });

  return (
    <AdvancedPanel
      summary="Transient · List · Battery · Trigger"
      collapsed={collapsed}
      onToggle={onSetCollapsed}
    >
      <div className="grid gap-4">
        <div>
          <div className="instrument-label">
            {t("dashboard.advanced.presetEditor")}
          </div>
          <div className="mt-3 flex flex-col gap-3">
            <BlockControlRow label={t("dashboard.presets.mode")}>
              <ModeSliderSelector
                availableModes={cpSupported ? ["CC", "CV", "CP"] : ["CC", "CV"]}
                activeMode={
                  draftPresetMode.toUpperCase() as "CC" | "CV" | "CP" | "CR"
                }
                onModeChange={(mode) =>
                  onModeChange(mode.toLowerCase() as EditablePresetMode)
                }
                size="sm"
                widthMode="fit"
                ariaLabel={t("dashboard.presets.advancedModeSelectorAria")}
              />
              {!cpSupported ? (
                <div className="text-[11px] text-slate-200/55">
                  {t("dashboard.presets.cpUnsupportedIdentity")}
                </div>
              ) : null}
            </BlockControlRow>

            {draftPresetMode === "cc" ? (
              <BlockControlSliderRow
                id="preset-target-i"
                label={t("dashboard.presets.targetCurrent")}
                value={draftPresetTargetIMa}
                displayValue={getDisplayValue(
                  "target_i_ma",
                  draftPresetTargetIMa,
                )}
                min={scales.targetCurrent.min}
                max={scales.targetCurrent.max}
                step={scales.targetCurrent.step}
                error={fieldError("target_i_ma")}
                onValueChange={onTargetCurrentChange}
                onDisplayValueChange={(text) =>
                  setDisplayDraft("target_i_ma", text)
                }
                onDisplayValueCommit={() =>
                  commitDisplayDraft(
                    "target_i_ma",
                    "current",
                    draftPresetTargetIMa,
                    onTargetCurrentChange,
                  )
                }
              />
            ) : draftPresetMode === "cv" ? (
              <BlockControlSliderRow
                id="preset-target-v"
                label={t("dashboard.presets.targetVoltage")}
                value={draftPresetTargetVMv}
                displayValue={getDisplayValue(
                  "target_v_mv",
                  draftPresetTargetVMv,
                )}
                min={scales.targetVoltage.min}
                max={scales.targetVoltage.max}
                step={scales.targetVoltage.step}
                error={fieldError("target_v_mv")}
                onValueChange={onTargetVoltageChange}
                onDisplayValueChange={(text) =>
                  setDisplayDraft("target_v_mv", text)
                }
                onDisplayValueCommit={() =>
                  commitDisplayDraft(
                    "target_v_mv",
                    "voltage",
                    draftPresetTargetVMv,
                    onTargetVoltageChange,
                  )
                }
              />
            ) : draftPresetMode === "cp" ? (
              <BlockControlSliderRow
                id="preset-target-p"
                label={t("dashboard.presets.targetPower")}
                value={draftPresetTargetPMw}
                displayValue={getDisplayValue(
                  "target_p_mw",
                  draftPresetTargetPMw,
                )}
                min={scales.targetPower.min}
                max={scales.targetPower.max}
                step={scales.targetPower.step}
                inputClassName={
                  cpDraftOutOfRange ? "border-red-400/25" : undefined
                }
                error={fieldError(
                  "target_p_mw",
                  cpDraftOutOfRange ? "target_p_mw must be ≤ max_p_mw" : null,
                )}
                onValueChange={onTargetPowerChange}
                onDisplayValueChange={(text) =>
                  setDisplayDraft("target_p_mw", text)
                }
                onDisplayValueCommit={() =>
                  commitDisplayDraft(
                    "target_p_mw",
                    "power",
                    draftPresetTargetPMw,
                    onTargetPowerChange,
                  )
                }
              />
            ) : (
              <BlockControlRow label="CR">
                <div className="rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-200/65">
                  {t("dashboard.presets.crValuesShown")}
                </div>
              </BlockControlRow>
            )}

            <BlockControlSliderRow
              id="preset-min-v"
              label={t("dashboard.presets.minVoltage")}
              value={draftPresetMinVMv}
              displayValue={getDisplayValue("min_v_mv", draftPresetMinVMv)}
              min={scales.minVoltage.min}
              max={scales.minVoltage.max}
              step={scales.minVoltage.step}
              error={fieldError("min_v_mv")}
              onValueChange={onMinVoltageChange}
              onDisplayValueChange={(text) => setDisplayDraft("min_v_mv", text)}
              onDisplayValueCommit={() =>
                commitDisplayDraft(
                  "min_v_mv",
                  "voltage",
                  draftPresetMinVMv,
                  onMinVoltageChange,
                )
              }
            />

            <BlockControlSliderRow
              id="preset-max-i"
              label={t("dashboard.presets.maxCurrentTotal")}
              value={draftPresetMaxIMaTotal}
              displayValue={getDisplayValue(
                "max_i_ma_total",
                draftPresetMaxIMaTotal,
              )}
              min={scales.maxCurrent.min}
              max={scales.maxCurrent.max}
              step={scales.maxCurrent.step}
              error={fieldError("max_i_ma_total")}
              onValueChange={onMaxCurrentChange}
              onDisplayValueChange={(text) =>
                setDisplayDraft("max_i_ma_total", text)
              }
              onDisplayValueCommit={() =>
                commitDisplayDraft(
                  "max_i_ma_total",
                  "current",
                  draftPresetMaxIMaTotal,
                  onMaxCurrentChange,
                )
              }
            />

            <BlockControlSliderRow
              id="preset-max-p"
              label={t("dashboard.presets.maxPower")}
              value={draftPresetMaxPMw}
              displayValue={getDisplayValue("max_p_mw", draftPresetMaxPMw)}
              min={scales.maxPower.min}
              max={scales.maxPower.max}
              step={scales.maxPower.step}
              error={fieldError("max_p_mw")}
              onValueChange={onMaxPowerChange}
              onDisplayValueChange={(text) => setDisplayDraft("max_p_mw", text)}
              onDisplayValueCommit={() =>
                commitDisplayDraft(
                  "max_p_mw",
                  "power",
                  draftPresetMaxPMw,
                  onMaxPowerChange,
                )
              }
            />

            <div className="mt-2 flex flex-col gap-2">
              {isEditingActivePreset ? (
                <button
                  type="button"
                  className="h-9 rounded-lg border border-amber-400/25 bg-amber-500/10 px-3 text-xs font-semibold tracking-[0.14em] text-amber-100 uppercase disabled:opacity-50"
                  disabled={saveDisabled}
                  onClick={onSavePreset}
                >
                  {savePending
                    ? t("dashboard.presets.saving")
                    : t("dashboard.presets.saveActiveSlot")}
                </button>
              ) : (
                <>
                  <button
                    type="button"
                    className="h-9 rounded-lg border border-amber-400/25 bg-amber-500/10 px-3 text-xs font-semibold tracking-[0.14em] text-amber-100 uppercase disabled:opacity-50"
                    disabled={saveDisabled}
                    onClick={onSavePreset}
                  >
                    {savePending
                      ? t("dashboard.presets.saving")
                      : t("dashboard.presets.saveSlot")}
                  </button>
                  <button
                    type="button"
                    className="h-9 rounded-lg border border-sky-400/25 bg-sky-500/10 px-3 text-xs font-semibold tracking-[0.14em] text-sky-100 uppercase disabled:opacity-50"
                    disabled={!baseUrl || applyPending}
                    onClick={onApplyPreset}
                  >
                    {applyPending
                      ? t("dashboard.presets.applying")
                      : t("dashboard.presets.applyPreset")}
                  </button>
                </>
              )}
            </div>
          </div>
        </div>

        {mockDevtoolsEnabled ? (
          <button
            type="button"
            className="h-9 rounded-lg border border-slate-400/10 bg-black/20 px-3 text-xs font-semibold text-slate-200/70 disabled:opacity-50"
            onClick={onToggleMockUvLatch}
          >
            {t("dashboard.advanced.toggleUvLatch")}
          </button>
        ) : null}

        {saveError ? (
          <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
            {saveError}
          </div>
        ) : null}

        {!isEditingActivePreset && applyError ? (
          <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
            {applyError}
          </div>
        ) : null}

        <section aria-label={t("dashboard.advanced.hardwareMainDisplay")}>
          <div className="instrument-label">
            {t("dashboard.advanced.hardwareDisplay")}
          </div>
          <div className="mt-3 flex justify-center">
            <MainDisplayCanvas {...display} />
          </div>
        </section>
      </div>
    </AdvancedPanel>
  );
}

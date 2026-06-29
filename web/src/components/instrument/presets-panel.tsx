import { useTranslation } from "react-i18next";
import { getPresetDraftBounds } from "../../routes/device-cc/preset-constraints.ts";
import {
  BlockControlRow,
  BlockControlSliderRow,
} from "../ui/block-control-row.tsx";
import { ModeSliderSelector } from "./mode-slider-selector.tsx";

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

export type PresetsPanelProps = {
  presets: Array<{
    id: number;
    label: string;
    active: boolean;
    disabled?: boolean;
  }>;
  selectedPresetId: number | null;
  activePresetId: number | null;
  onPresetSelect: (id: number) => void;
  onApply: () => void;
  onSave: () => void;
  applyDisabled: boolean;
  saveDisabled: boolean;
  applying?: boolean;
  saving?: boolean;
  cpSupported: boolean;
  cpDraftOutOfRange: boolean;
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
  onModeChange: (mode: EditablePresetMode) => void;
  onTargetCurrentChange: (value: number) => void;
  onTargetVoltageChange: (value: number) => void;
  onTargetPowerChange: (value: number) => void;
  onMinVoltageChange: (value: number) => void;
  onMaxCurrentChange: (value: number) => void;
  onMaxPowerChange: (value: number) => void;
  saveError?: string | null;
  applyError?: string | null;
  actionNotice?: string | null;
};

export function PresetsPanel({
  presets,
  selectedPresetId,
  activePresetId,
  onPresetSelect,
  onApply,
  onSave,
  applyDisabled,
  saveDisabled,
  applying = false,
  saving = false,
  cpSupported,
  cpDraftOutOfRange,
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
  onModeChange,
  onTargetCurrentChange,
  onTargetVoltageChange,
  onTargetPowerChange,
  onMinVoltageChange,
  onMaxCurrentChange,
  onMaxPowerChange,
  saveError = null,
  applyError = null,
  actionNotice = null,
}: PresetsPanelProps) {
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
    <section
      aria-label={t("dashboard.presets.title")}
      className="instrument-card p-5"
    >
      <div className="instrument-label">{t("dashboard.presets.title")}</div>

      <div className="mt-4 grid grid-cols-4 gap-2">
        {presets.map((preset) => {
          const isSelected = selectedPresetId === preset.id;
          const isDisabled = Boolean(preset.disabled);
          return (
            <button
              key={preset.id}
              type="button"
              disabled={isDisabled}
              className={[
                "h-9 rounded-lg border text-xs font-semibold tracking-wide transition-colors",
                isSelected
                  ? "border-[rgba(111,234,249,0.28)] bg-[rgba(111,234,249,0.10)] text-slate-100"
                  : preset.active
                    ? "border-slate-200/20 bg-white/5 text-slate-100/90"
                    : "border-slate-400/10 bg-black/20 text-slate-200/70",
                isDisabled ? "cursor-not-allowed opacity-35" : "cursor-pointer",
              ].join(" ")}
              onClick={() => onPresetSelect(preset.id)}
            >
              {preset.label}
            </button>
          );
        })}
      </div>

      <div className="mt-5 rounded-xl border border-slate-400/10 bg-black/16 p-4">
        <div className="flex items-start justify-between gap-4">
          <div>
            <div className="instrument-label">
              {t("dashboard.presets.slotEditor")}
            </div>
            <div className="mt-2 text-sm font-semibold text-slate-100">
              {t("dashboard.presets.currentSlot", {
                slot: selectedPresetId ? `#${selectedPresetId}` : "—",
              })}
            </div>
            <div className="mt-1 text-[11px] text-slate-200/48">
              {t("dashboard.presets.description")}
            </div>
          </div>
          <div className="rounded-full border border-cyan-400/16 bg-cyan-400/8 px-2.5 py-1 text-[10px] font-semibold tracking-[0.14em] text-cyan-100/90 uppercase">
            {draftPresetMode.toUpperCase()}
          </div>
        </div>

        <div className="mt-4 flex flex-col gap-3">
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
              widthMode="fill"
              ariaLabel={t("dashboard.presets.modeSelectorAria")}
            />
            {!cpSupported ? (
              <div className="text-[11px] text-slate-200/55">
                {t("dashboard.presets.cpUnsupported")}
              </div>
            ) : null}
          </BlockControlRow>

          {draftPresetMode === "cc" ? (
            <BlockControlSliderRow
              id="drawer-preset-target-i"
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
              id="drawer-preset-target-v"
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
              id="drawer-preset-target-p"
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
            id="drawer-preset-min-v"
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
            id="drawer-preset-max-i"
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
            id="drawer-preset-max-p"
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
        </div>
      </div>

      <div className="mt-4 flex items-center justify-between gap-3">
        {isEditingActivePreset ? (
          <button
            type="button"
            className="h-9 w-full cursor-pointer rounded-lg border border-[rgba(253,212,94,0.28)] bg-[rgba(253,212,94,0.10)] px-3 text-xs font-semibold tracking-[0.14em] text-slate-100 uppercase transition-[transform,background-color,border-color,box-shadow] duration-150 hover:border-[rgba(253,212,94,0.48)] hover:bg-[rgba(253,212,94,0.16)] hover:shadow-[0_0_0_1px_rgba(253,212,94,0.12)] active:translate-y-[1px] active:bg-[rgba(253,212,94,0.22)] disabled:cursor-not-allowed disabled:opacity-50"
            disabled={saveDisabled}
            onClick={onSave}
          >
            {saving
              ? t("dashboard.presets.saving")
              : t("dashboard.presets.saveActiveSlot")}
          </button>
        ) : (
          <>
            <button
              type="button"
              className="h-9 flex-1 cursor-pointer rounded-lg border border-[rgba(111,234,249,0.28)] bg-[rgba(111,234,249,0.10)] px-3 text-xs font-semibold tracking-[0.14em] text-slate-100 uppercase transition-[transform,background-color,border-color,box-shadow] duration-150 hover:border-[rgba(111,234,249,0.48)] hover:bg-[rgba(111,234,249,0.16)] hover:shadow-[0_0_0_1px_rgba(111,234,249,0.16)] active:translate-y-[1px] active:bg-[rgba(111,234,249,0.22)] disabled:cursor-not-allowed disabled:opacity-50"
              disabled={applyDisabled}
              onClick={onApply}
            >
              {applying
                ? t("dashboard.presets.applying")
                : t("dashboard.presets.applyPreset")}
            </button>
            <button
              type="button"
              className="h-9 flex-1 cursor-pointer rounded-lg border border-[rgba(253,212,94,0.28)] bg-[rgba(253,212,94,0.10)] px-3 text-xs font-semibold tracking-[0.14em] text-slate-100 uppercase transition-[transform,background-color,border-color,box-shadow] duration-150 hover:border-[rgba(253,212,94,0.48)] hover:bg-[rgba(253,212,94,0.16)] hover:shadow-[0_0_0_1px_rgba(253,212,94,0.12)] active:translate-y-[1px] active:bg-[rgba(253,212,94,0.22)] disabled:cursor-not-allowed disabled:opacity-50"
              disabled={saveDisabled}
              onClick={onSave}
            >
              {saving
                ? t("dashboard.presets.saving")
                : t("dashboard.presets.saveSlot")}
            </button>
          </>
        )}
      </div>

      <div className="mt-3 space-y-2">
        <div className="text-[11px] text-slate-200/55">
          {isEditingActivePreset
            ? t("dashboard.presets.activeSlotHint")
            : t("dashboard.presets.applyTurnsOffHint")}
        </div>
        <div
          aria-live="polite"
          className="min-h-[1.25rem] text-[11px] font-medium text-cyan-100/82"
        >
          {actionNotice ?? ""}
        </div>
      </div>

      {saveError ? (
        <div className="mt-3 rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
          {saveError}
        </div>
      ) : null}

      {!isEditingActivePreset && applyError ? (
        <div className="mt-3 rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
          {applyError}
        </div>
      ) : null}
    </section>
  );
}

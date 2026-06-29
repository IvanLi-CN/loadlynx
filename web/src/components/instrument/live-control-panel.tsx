import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { PresetId } from "../../api/types.ts";
import {
  clampPresetDraft,
  getPresetDraftBounds,
  type PresetDraft,
} from "../../routes/device-cc/preset-constraints.ts";
import {
  formatPresetRawValue,
  makePresetInputMemoryKey,
  type PresetEditableField,
  type PresetInputMemoryStore,
  type PresetInputUnitKind,
  parsePresetInputValue,
  readPresetInputMemory,
  writePresetInputMemory,
} from "../../routes/device-cc/preset-input-memory.ts";
import { BlockControlSliderRow } from "../ui/block-control-row.tsx";
import { LoadOutputSwitch } from "./load-output-switch.tsx";
import { ModeSliderSelector } from "./mode-slider-selector.tsx";

type EditablePresetMode = "cc" | "cv" | "cp";

const EMPTY_PRESET_DRAFT: PresetDraft = {
  mode: "cc",
  target_i_ma: 0,
  target_v_mv: 0,
  target_p_mw: 0,
  min_v_mv: 0,
  max_i_ma_total: 10_000,
  max_p_mw: 150_000,
};

export type LiveControlPanelProps = {
  deviceId: string;
  baseUrl: string | undefined;
  activePresetId: PresetId | null;
  preset: PresetDraft | null;
  availableModes: Array<"CC" | "CV" | "CP">;
  cpSupported: boolean;
  outputEnabled: boolean;
  outputToggleDisabled?: boolean;
  showOutputReenableHint?: boolean;
  savePending?: boolean;
  saveError?: string | null;
  actionNotice?: string | null;
  onOutputToggle: (nextEnabled: boolean) => void;
  onSaveDraft: (presetId: PresetId, draft: PresetDraft) => void;
};

export function LiveControlPanel({
  deviceId,
  baseUrl,
  activePresetId,
  preset,
  availableModes,
  cpSupported,
  outputEnabled,
  outputToggleDisabled = false,
  showOutputReenableHint = false,
  savePending = false,
  saveError = null,
  actionNotice = null,
  onOutputToggle,
  onSaveDraft,
}: LiveControlPanelProps) {
  const { t } = useTranslation();
  const presetSignature = [
    preset?.mode ?? EMPTY_PRESET_DRAFT.mode,
    preset?.target_i_ma ?? EMPTY_PRESET_DRAFT.target_i_ma,
    preset?.target_v_mv ?? EMPTY_PRESET_DRAFT.target_v_mv,
    preset?.target_p_mw ?? EMPTY_PRESET_DRAFT.target_p_mw,
    preset?.min_v_mv ?? EMPTY_PRESET_DRAFT.min_v_mv,
    preset?.max_i_ma_total ?? EMPTY_PRESET_DRAFT.max_i_ma_total,
    preset?.max_p_mw ?? EMPTY_PRESET_DRAFT.max_p_mw,
  ].join("|");
  const normalizedPreset = clampPresetDraft(preset ?? EMPTY_PRESET_DRAFT);
  const [draft, setDraft] = useState<PresetDraft>(normalizedPreset);
  const lastPresetSignatureRef = useRef(presetSignature);
  const [inputMemory, setInputMemory] = useState<PresetInputMemoryStore>(() =>
    readPresetInputMemory(window.localStorage),
  );
  const [inputDrafts, setInputDrafts] = useState<
    Partial<Record<PresetEditableField, string>>
  >({});
  const [inputErrors, setInputErrors] = useState<
    Partial<Record<PresetEditableField, string | null>>
  >({});

  useEffect(() => {
    if (lastPresetSignatureRef.current === presetSignature) {
      return;
    }
    lastPresetSignatureRef.current = presetSignature;
    setDraft(normalizedPreset);
    setInputDrafts({});
    setInputErrors({});
  }, [normalizedPreset, presetSignature]);

  const scales = getPresetDraftBounds(draft);
  const cpDraftOutOfRange =
    draft.mode === "cp" && draft.target_p_mw > draft.max_p_mw;
  const persistInputMemory = (
    updater: (prev: PresetInputMemoryStore) => PresetInputMemoryStore,
  ) => {
    setInputMemory((prev) => {
      const next = updater(prev);
      writePresetInputMemory(window.localStorage, next);
      return next;
    });
  };

  const memoryKeyFor = (field: PresetEditableField) =>
    makePresetInputMemoryKey({
      deviceId,
      baseUrl,
      presetId: activePresetId ?? 1,
      field,
    });

  const getDisplayValue = (
    field: PresetEditableField,
    rawValue: number,
  ): string => {
    const liveDraft = inputDrafts[field];
    if (liveDraft !== undefined) {
      return liveDraft;
    }

    const memory = inputMemory[memoryKeyFor(field)];
    if (memory && memory.value === rawValue) {
      return memory.text;
    }

    return formatPresetRawValue(rawValue);
  };

  const applyDraftPatch = (patch: Partial<PresetDraft>) => {
    setDraft((prev) => clampPresetDraft({ ...prev, ...patch }));
  };

  const setDisplayDraft = (field: PresetEditableField, text: string) => {
    setInputDrafts((prev) => ({
      ...prev,
      [field]: text,
    }));
    setInputErrors((prev) => ({
      ...prev,
      [field]: null,
    }));
  };

  const commitDisplayDraft = (
    field: PresetEditableField,
    unitKind: PresetInputUnitKind,
    fallbackValue: number,
  ) => {
    const currentRaw =
      inputDrafts[field] ?? getDisplayValue(field, fallbackValue);
    const parsed = parsePresetInputValue(currentRaw, unitKind);
    if (!parsed.ok) {
      setInputErrors((prev) => ({
        ...prev,
        [field]: parsed.error,
      }));
      setInputDrafts((prev) => ({
        ...prev,
        [field]: currentRaw,
      }));
      return;
    }

    const nextDraft = clampPresetDraft({
      ...draft,
      [field]: parsed.value,
    } as PresetDraft);
    const nextValue = nextDraft[field];
    setDraft(nextDraft);
    setInputErrors((prev) => ({
      ...prev,
      [field]: null,
    }));
    setInputDrafts((prev) => {
      const next = { ...prev };
      delete next[field];
      return next;
    });

    const memoryKey = memoryKeyFor(field);
    if (nextValue === parsed.value) {
      persistInputMemory((prev) => ({
        ...prev,
        [memoryKey]: {
          value: nextValue,
          text: parsed.displayText,
        },
      }));
      return;
    }

    persistInputMemory((prev) => {
      const next = { ...prev };
      delete next[memoryKey];
      return next;
    });
  };

  const fieldError = (
    field: PresetEditableField,
    fallbackError: string | null = null,
  ) => inputErrors[field] ?? fallbackError;

  const handleSave = () => {
    if (!activePresetId) {
      return;
    }
    onSaveDraft(activePresetId, draft);
  };

  const liveControlSaveDisabled =
    !activePresetId ||
    !preset ||
    savePending ||
    cpDraftOutOfRange ||
    draft.mode === "cr";

  const targetRows = [
    {
      field: "target_i_ma" as const,
      mode: "cc" as const,
      label: t("dashboard.presets.targetCurrent"),
      value: draft.target_i_ma,
      min: scales.targetCurrent.min,
      max: scales.targetCurrent.max,
      step: scales.targetCurrent.step,
      unitKind: "current" as const,
      error: fieldError("target_i_ma"),
    },
    {
      field: "target_v_mv" as const,
      mode: "cv" as const,
      label: t("dashboard.presets.targetVoltage"),
      value: draft.target_v_mv,
      min: scales.targetVoltage.min,
      max: scales.targetVoltage.max,
      step: scales.targetVoltage.step,
      unitKind: "voltage" as const,
      error: fieldError("target_v_mv"),
    },
    {
      field: "target_p_mw" as const,
      mode: "cp" as const,
      label: t("dashboard.presets.targetPower"),
      value: draft.target_p_mw,
      min: scales.targetPower.min,
      max: scales.targetPower.max,
      step: scales.targetPower.step,
      unitKind: "power" as const,
      error: fieldError(
        "target_p_mw",
        cpDraftOutOfRange ? "target_p_mw must be ≤ max_p_mw" : null,
      ),
    },
  ];

  const orderedTargetRows = targetRows.sort((left, right) => {
    if (left.mode === draft.mode) return -1;
    if (right.mode === draft.mode) return 1;
    return 0;
  });

  const activeTargetRow = orderedTargetRows[0];

  const limitRows = [
    {
      field: "min_v_mv" as const,
      label: t("dashboard.presets.minVoltage"),
      value: draft.min_v_mv,
      min: scales.minVoltage.min,
      max: scales.minVoltage.max,
      step: scales.minVoltage.step,
      unitKind: "voltage" as const,
      error: fieldError("min_v_mv"),
    },
    {
      field: "max_i_ma_total" as const,
      label: t("dashboard.presets.maxCurrentTotal"),
      value: draft.max_i_ma_total,
      min: scales.maxCurrent.min,
      max: scales.maxCurrent.max,
      step: scales.maxCurrent.step,
      unitKind: "current" as const,
      error: fieldError("max_i_ma_total"),
    },
    {
      field: "max_p_mw" as const,
      label: t("dashboard.presets.maxPower"),
      value: draft.max_p_mw,
      min: scales.maxPower.min,
      max: scales.maxPower.max,
      step: scales.maxPower.step,
      unitKind: "power" as const,
      error: fieldError("max_p_mw"),
    },
  ];

  return (
    <section
      aria-label={t("dashboard.liveControl.title")}
      className="instrument-card p-5"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        {activePresetId ? (
          <div className="instrument-pill instrument-pill-cyan">
            {t("dashboard.liveControl.activeSlot", { slot: activePresetId })}
          </div>
        ) : null}
      </div>

      <div className="mt-4 grid gap-5">
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] lg:items-start">
          <div className="ll-live-control__top-control grid gap-2">
            <div className="text-[10px] tracking-[0.14em] uppercase text-slate-200/50">
              {t("dashboard.presets.mode")}
            </div>
            <ModeSliderSelector
              availableModes={availableModes}
              activeMode={draft.mode.toUpperCase() as "CC" | "CV" | "CP" | "CR"}
              onModeChange={(mode) =>
                applyDraftPatch({
                  mode: mode.toLowerCase() as EditablePresetMode,
                })
              }
              size="sm"
              widthMode="fill"
              ariaLabel={t("dashboard.liveControl.modeSelectorAria")}
              className="ll-live-control__mode-group"
            />
            {!cpSupported ? (
              <div className="text-[11px] text-slate-200/55">
                {t("dashboard.presets.cpUnsupported")}
              </div>
            ) : null}
          </div>

          <div className="ll-live-control__top-control grid gap-2">
            <div className="text-[10px] tracking-[0.14em] uppercase text-slate-200/50">
              {t("dashboard.liveControl.loadLabel")}
            </div>

            <LoadOutputSwitch
              checked={outputEnabled}
              disabled={outputToggleDisabled}
              onCheckedChange={onOutputToggle}
              offLabel={t("dashboard.liveControl.loadOff")}
              onLabel={t("dashboard.liveControl.loadOn")}
              offHint={t("dashboard.liveControl.loadOffHint")}
              onHint={t("dashboard.liveControl.loadOnHint")}
              ariaLabel={t("dashboard.liveControl.loadSwitchAria")}
              size="sm"
              className="ll-live-control__load-switch"
            />

            {showOutputReenableHint ? (
              <div className="text-[11px] text-amber-200">
                {t("dashboard.liveControl.outputReenableHint")}
              </div>
            ) : null}
          </div>
        </div>

        <div className="border-t border-slate-400/10 pt-4">
          <div className="grid gap-3">
            <div className="flex items-center justify-between gap-3">
              <div className="instrument-label">
                {t("dashboard.liveControl.setpoint")}
              </div>
              <div className="instrument-pill instrument-pill-cyan">
                {draft.mode.toUpperCase()}
              </div>
            </div>

            <BlockControlSliderRow
              id={`live-${activeTargetRow.field}`}
              className="ll-live-control__primary"
              label={activeTargetRow.label}
              value={activeTargetRow.value}
              displayValue={getDisplayValue(
                activeTargetRow.field,
                activeTargetRow.value,
              )}
              min={activeTargetRow.min}
              max={activeTargetRow.max}
              step={activeTargetRow.step}
              error={activeTargetRow.error}
              onValueChange={(value) =>
                applyDraftPatch({
                  [activeTargetRow.field]: value,
                } as Partial<PresetDraft>)
              }
              onDisplayValueChange={(text) =>
                setDisplayDraft(activeTargetRow.field, text)
              }
              onDisplayValueCommit={() =>
                commitDisplayDraft(
                  activeTargetRow.field,
                  activeTargetRow.unitKind,
                  activeTargetRow.value,
                )
              }
            />
          </div>
        </div>

        <div className="border-t border-slate-400/10 pt-4">
          <div className="instrument-label">
            {t("dashboard.liveControl.limits")}
          </div>
          <div className="mt-3 grid gap-3">
            {limitRows.map((row) => (
              <BlockControlSliderRow
                key={row.field}
                id={`live-${row.field}`}
                label={row.label}
                value={row.value}
                displayValue={getDisplayValue(row.field, row.value)}
                min={row.min}
                max={row.max}
                step={row.step}
                error={row.error}
                onValueChange={(value) =>
                  applyDraftPatch({
                    [row.field]: value,
                  } as Partial<PresetDraft>)
                }
                onDisplayValueChange={(text) =>
                  setDisplayDraft(row.field, text)
                }
                onDisplayValueCommit={() =>
                  commitDisplayDraft(row.field, row.unitKind, row.value)
                }
              />
            ))}
          </div>
        </div>

        <div className="border-t border-slate-400/10 pt-4">
          <div>
            <button
              type="button"
              className="ll-button ll-button-sm ll-button-primary w-full"
              disabled={liveControlSaveDisabled}
              onClick={handleSave}
            >
              {savePending
                ? t("dashboard.liveControl.saving")
                : t("dashboard.liveControl.save")}
            </button>
          </div>

          {actionNotice ? (
            <div
              aria-live="polite"
              className="mt-3 min-h-[1.25rem] text-[11px] font-medium text-cyan-100/82"
            >
              {actionNotice}
            </div>
          ) : null}
        </div>
      </div>

      {saveError ? (
        <div className="mt-3 rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
          {saveError}
        </div>
      ) : null}
    </section>
  );
}

import { AdvancedPanel } from "../../components/instrument/advanced-panel.tsx";
import type { MainDisplayCanvasProps } from "./main-display-canvas.tsx";
import { MainDisplayCanvas } from "./main-display-canvas.tsx";

type PresetMode = "cc" | "cv" | "cp" | "cr";
type EditablePresetMode = Exclude<PresetMode, "cr">;

export interface AdvancedControlsPanelProps {
  collapsed: boolean;
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
  return (
    <AdvancedPanel
      summary="Transient · List · Battery · Trigger"
      collapsed={collapsed}
      onToggle={onSetCollapsed}
    >
      <div className="grid gap-4">
        <div>
          <div className="instrument-label">Preset editor</div>
          <div className="mt-3 grid gap-3">
            <div>
              <label
                htmlFor="preset-mode"
                className="block text-[11px] text-slate-200/60"
              >
                Mode
              </label>
              <select
                id="preset-mode"
                className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                value={draftPresetMode}
                onChange={(event) =>
                  onModeChange(event.target.value as EditablePresetMode)
                }
              >
                <option value="cc">cc</option>
                <option value="cv">cv</option>
                {cpSupported ? <option value="cp">cp</option> : null}
                {draftPresetMode === "cr" ? (
                  <option value="cr" disabled hidden>
                    cr
                  </option>
                ) : null}
              </select>
              {!cpSupported ? (
                <div className="mt-2 text-[11px] text-slate-200/55">
                  CP: 固件不支持（identity.capabilities.cp_supported=false）
                </div>
              ) : null}
            </div>

            {draftPresetMode === "cc" ? (
              <div>
                <label
                  htmlFor="preset-target-i"
                  className="block text-[11px] text-slate-200/60"
                >
                  Target current (mA)
                </label>
                <input
                  id="preset-target-i"
                  type="number"
                  className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                  value={draftPresetTargetIMa}
                  onChange={(event) =>
                    onTargetCurrentChange(
                      Number.parseInt(event.target.value || "0", 10),
                    )
                  }
                />
              </div>
            ) : draftPresetMode === "cv" ? (
              <div>
                <label
                  htmlFor="preset-target-v"
                  className="block text-[11px] text-slate-200/60"
                >
                  Target voltage (mV)
                </label>
                <input
                  id="preset-target-v"
                  type="number"
                  className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                  value={draftPresetTargetVMv}
                  onChange={(event) =>
                    onTargetVoltageChange(
                      Number.parseInt(event.target.value || "0", 10),
                    )
                  }
                />
              </div>
            ) : draftPresetMode === "cp" ? (
              <div>
                <label
                  htmlFor="preset-target-p"
                  className="block text-[11px] text-slate-200/60"
                >
                  Target power (mW)
                </label>
                <input
                  id="preset-target-p"
                  type="number"
                  className={[
                    "mt-1 w-full rounded-lg border bg-black/20 px-3 py-2 text-[12px] text-slate-100",
                    cpDraftOutOfRange
                      ? "border-red-400/25"
                      : "border-slate-400/10",
                  ].join(" ")}
                  value={draftPresetTargetPMw}
                  onChange={(event) =>
                    onTargetPowerChange(
                      Number.parseInt(event.target.value || "0", 10),
                    )
                  }
                />
                {cpDraftOutOfRange ? (
                  <div className="mt-2 text-[11px] text-red-200/85">
                    target_p_mw must be ≤ max_p_mw
                  </div>
                ) : null}
              </div>
            ) : (
              <div className="rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-200/65">
                CR is a legacy preset mode and is read-only in this editor.
              </div>
            )}

            <div>
              <label
                htmlFor="preset-min-v"
                className="block text-[11px] text-slate-200/60"
              >
                Min voltage (mV)
              </label>
              <input
                id="preset-min-v"
                type="number"
                className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                value={draftPresetMinVMv}
                onChange={(event) =>
                  onMinVoltageChange(
                    Number.parseInt(event.target.value || "0", 10),
                  )
                }
              />
            </div>

            <div>
              <label
                htmlFor="preset-max-i"
                className="block text-[11px] text-slate-200/60"
              >
                Max current total (mA)
              </label>
              <input
                id="preset-max-i"
                type="number"
                className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                value={draftPresetMaxIMaTotal}
                onChange={(event) =>
                  onMaxCurrentChange(
                    Number.parseInt(event.target.value || "0", 10),
                  )
                }
              />
            </div>

            <div>
              <label
                htmlFor="preset-max-p"
                className="block text-[11px] text-slate-200/60"
              >
                Max power (mW)
              </label>
              <input
                id="preset-max-p"
                type="number"
                className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                value={draftPresetMaxPMw}
                onChange={(event) =>
                  onMaxPowerChange(
                    Number.parseInt(event.target.value || "0", 10),
                  )
                }
              />
            </div>

            <div className="mt-2 flex flex-col gap-2">
              <button
                type="button"
                className="h-9 rounded-lg border border-amber-400/25 bg-amber-500/10 px-3 text-xs font-semibold tracking-[0.14em] text-amber-100 uppercase disabled:opacity-50"
                disabled={saveDisabled}
                onClick={onSavePreset}
              >
                {savePending ? "Saving…" : "Save Draft"}
              </button>
              <button
                type="button"
                className="h-9 rounded-lg border border-sky-400/25 bg-sky-500/10 px-3 text-xs font-semibold tracking-[0.14em] text-sky-100 uppercase disabled:opacity-50"
                disabled={!baseUrl || applyPending}
                onClick={onApplyPreset}
              >
                {applyPending ? "Applying…" : "Apply Preset"}
              </button>
            </div>
          </div>
        </div>

        {mockDevtoolsEnabled ? (
          <button
            type="button"
            className="h-9 rounded-lg border border-slate-400/10 bg-black/20 px-3 text-xs font-semibold text-slate-200/70 disabled:opacity-50"
            onClick={onToggleMockUvLatch}
          >
            Toggle UV latch (mock)
          </button>
        ) : null}

        {saveError ? (
          <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
            {saveError}
          </div>
        ) : null}

        {applyError ? (
          <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
            {applyError}
          </div>
        ) : null}

        <section aria-label="Hardware main display">
          <div className="instrument-label">Hardware display</div>
          <div className="mt-3 flex justify-center">
            <MainDisplayCanvas {...display} />
          </div>
        </section>
      </div>
    </AdvancedPanel>
  );
}

import { LoadOutputSwitch } from "./load-output-switch.tsx";
import { ModeSliderSelector } from "./mode-slider-selector.tsx";

type EditableControlMode = "CC" | "CV" | "CP";
type VisibleControlMode = EditableControlMode | "CR";

export type ControlModePanelProps = {
  availableModes: EditableControlMode[];
  activeMode: VisibleControlMode;
  onModeChange: (mode: EditableControlMode) => void;
  outputEnabled: boolean;
  outputToggleDisabled?: boolean;
  onOutputToggle: (nextEnabled: boolean) => void;
  outputHint: string | null;
  showOutputReenableHint?: boolean;
};

export function ControlModePanel({
  availableModes,
  activeMode,
  onModeChange,
  outputEnabled,
  outputToggleDisabled = false,
  onOutputToggle,
  outputHint,
  showOutputReenableHint = false,
}: ControlModePanelProps) {
  return (
    <section aria-label="Mode and output" className="instrument-card p-5">
      <div className="instrument-label">Mode &amp; Output</div>

      <div className="mt-4">
        <ModeSliderSelector
          availableModes={availableModes}
          activeMode={activeMode}
          onModeChange={onModeChange}
          size="sm"
          widthMode="fit"
          ariaLabel="Control mode selector"
        />
      </div>

      <div className="mt-5 border-t border-slate-400/10 pt-4">
        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <div className="text-[10px] tracking-[0.14em] uppercase text-slate-200/50">
              Output
            </div>
            <div className="mt-2 text-sm font-semibold text-slate-100">
              {outputEnabled ? "Enabled" : "Disabled"}
            </div>
            <div className="mt-1 text-[11px] text-slate-200/55">
              {outputHint ?? "—"}
            </div>
          </div>

          <div className="w-full max-w-[18rem]">
            <LoadOutputSwitch
              checked={outputEnabled}
              disabled={outputToggleDisabled}
              onCheckedChange={onOutputToggle}
              offLabel="Off"
              onLabel="On"
              offHint="Standby"
              onHint="Armed"
              ariaLabel="Load output switch"
              size="sm"
            />
          </div>
        </div>

        {showOutputReenableHint ? (
          <div className="mt-3 border-t border-amber-400/20 pt-3 text-[11px] text-amber-200">
            Preset applied while output was enabled — please re‑enable Output.
          </div>
        ) : null}
      </div>
    </section>
  );
}

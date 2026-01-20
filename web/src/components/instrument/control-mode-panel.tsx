export type ControlModePanelProps = {
  availableModes: Array<"CC" | "CV" | "CP" | "CR">;
  activeMode: "CC" | "CV" | "CP" | "CR";
  onModeChange: (mode: "CC" | "CV" | "CP" | "CR") => void;
  outputEnabled: boolean;
  outputToggleDisabled?: boolean;
  onOutputToggle: (nextEnabled: boolean) => void;
  outputHint: string | null;
  showOutputReenableHint?: boolean;
};

function ModeButton({
  mode,
  active,
  disabled,
  onClick,
}: {
  mode: "CC" | "CV" | "CP" | "CR";
  active: boolean;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={[
        "rounded-lg border px-3 py-1 text-xs font-semibold tracking-wide",
        active
          ? "border-[rgba(111,234,249,0.28)] bg-[rgba(111,234,249,0.10)] text-slate-100"
          : "border-slate-400/10 bg-black/20 text-slate-200/60",
        disabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer",
      ].join(" ")}
      disabled={disabled}
      onClick={onClick}
    >
      {mode}
    </button>
  );
}

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
  const allModes: Array<"CC" | "CV" | "CP" | "CR"> = ["CC", "CV", "CP", "CR"];
  return (
    <section aria-label="Mode and output" className="instrument-card p-5">
      <div className="instrument-label">Mode &amp; Output</div>

      <div className="mt-4 flex flex-wrap gap-2">
        {allModes.map((mode) => {
          const supported = availableModes.includes(mode);
          return (
            <ModeButton
              key={mode}
              mode={mode}
              active={mode === activeMode}
              disabled={!supported}
              onClick={() => onModeChange(mode)}
            />
          );
        })}
      </div>

      <div className="mt-4 rounded-xl border border-slate-400/10 bg-black/20 px-4 py-3">
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

          <label className="relative inline-flex h-6 w-11 cursor-pointer items-center">
            <input
              type="checkbox"
              className="absolute inset-0 z-10 h-full w-full cursor-pointer opacity-0"
              aria-label="Output enabled"
              checked={outputEnabled}
              disabled={outputToggleDisabled}
              onChange={(event) => onOutputToggle(event.target.checked)}
            />
            <span
              className={[
                "pointer-events-none relative inline-flex h-6 w-11 items-center rounded-full border transition-colors",
                outputEnabled
                  ? "border-[rgba(131,255,210,0.26)] bg-[rgba(131,255,210,0.10)]"
                  : "border-slate-400/15 bg-slate-500/10",
                outputToggleDisabled ? "opacity-50" : "",
              ].join(" ")}
            >
              <span
                className={[
                  "inline-block h-5 w-5 transform rounded-full shadow transition-transform",
                  outputEnabled
                    ? "bg-[rgba(131,255,210,0.92)] shadow-[0_0_18px_rgba(131,255,210,0.35)]"
                    : "bg-slate-100/90",
                  outputEnabled ? "translate-x-5" : "translate-x-1",
                ].join(" ")}
              />
            </span>
          </label>
        </div>

        {showOutputReenableHint ? (
          <div className="mt-3 rounded-lg border border-amber-400/20 bg-amber-500/10 px-3 py-2 text-[11px] text-amber-200">
            Preset applied while output was enabled — please re‑enable Output.
          </div>
        ) : null}
      </div>
    </section>
  );
}

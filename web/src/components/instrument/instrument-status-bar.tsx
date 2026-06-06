export type InstrumentStatusBarProps = {
  deviceName: string;
  deviceIp: string | null;
  firmwareVersion: string | null;
  modeLabel: "CC" | "CV" | "CP" | "CR" | "UNKNOWN";
  linkState: "up" | "down" | "unknown";
  outputState: { enabled: boolean; setpointLabel: string | null };
  protectionState: { summary: string; level: "ok" | "warn" | "danger" };
  faultSummary: string | null;
  stale?: boolean;
};

export function InstrumentStatusBar({
  deviceName,
  deviceIp,
  firmwareVersion,
  modeLabel,
  linkState,
  outputState,
  protectionState,
  faultSummary,
  stale = false,
}: InstrumentStatusBarProps) {
  const runStateText =
    modeLabel === "CC"
      ? "Constant Current"
      : modeLabel === "CV"
        ? "Constant Voltage"
        : modeLabel === "CP"
          ? "Constant Power"
          : modeLabel === "CR"
            ? "Constant Resistance"
            : "Unknown";

  const pillBase = "instrument-pill";

  const linkPillClass =
    linkState === "up"
      ? "instrument-pill-green"
      : linkState === "down"
        ? "instrument-pill-danger"
        : "";

  const outputText = outputState.enabled
    ? `ON${outputState.setpointLabel ? ` · ${outputState.setpointLabel}` : ""}`
    : "OFF";

  const outputPillClass = outputState.enabled ? "instrument-pill-cyan" : "";

  const protectTitleText =
    protectionState.level === "danger"
      ? "Fault"
      : protectionState.level === "warn"
        ? "Attention"
        : "All Clear";

  const protectPillText =
    protectionState.level === "ok" && !faultSummary
      ? "UV LATCH READY"
      : protectionState.level === "ok" && faultSummary
        ? "FAULT PRESENT"
        : protectionState.summary.replaceAll("_", " ");

  const protectPillClass =
    protectionState.level === "danger"
      ? "instrument-pill-danger"
      : protectionState.level === "warn"
        ? "instrument-pill-amber"
        : "instrument-pill-amber";

  return (
    <header className="instrument-card px-6 py-4">
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-4 lg:gap-6">
        <div className="min-w-0">
          <div className="instrument-label">Device</div>
          <div className="mt-1 truncate text-sm font-semibold text-slate-100">
            {deviceName}
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-slate-200/55">
            <span className="font-mono">{deviceIp ?? "—"}</span>
            <span aria-hidden="true">•</span>
            <span className="truncate">
              Firmware{" "}
              <span className="font-mono">{firmwareVersion ?? "—"}</span>
            </span>
            {stale ? (
              <span className="ml-1 rounded-full border border-amber-400/20 bg-amber-500/10 px-2 py-0.5 text-[10px] font-semibold tracking-[0.14em] text-amber-200">
                STALE
              </span>
            ) : null}
          </div>
        </div>

        <div>
          <div className="instrument-label">Run State</div>
          <div className="mt-1 text-sm font-semibold text-slate-100">
            {runStateText}
          </div>
          <div className="mt-2">
            <span className={`${pillBase} ${linkPillClass} w-full`}>
              {linkState === "up"
                ? "LINK UP"
                : linkState === "down"
                  ? "LINK DOWN"
                  : "LINK UNKNOWN"}
            </span>
          </div>
        </div>

        <div>
          <div className="instrument-label">Output</div>
          <div className="mt-1 text-sm font-semibold text-slate-100">
            {outputText}
          </div>
          <div className="mt-2">
            <span className={`${pillBase} ${outputPillClass} w-full`}>
              {outputState.enabled ? "REMOTE ACTIVE" : "OUTPUT DISABLED"}
            </span>
          </div>
        </div>

        <div>
          <div className="instrument-label">Protection</div>
          <div className="mt-1 text-sm font-semibold text-slate-100">
            {protectTitleText}
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <span className={`${pillBase} ${protectPillClass} w-full`}>
              {protectPillText}
            </span>
            {faultSummary ? (
              <span className="text-[11px] text-red-200/80 truncate">
                {faultSummary}
              </span>
            ) : null}
          </div>
        </div>
      </div>
    </header>
  );
}

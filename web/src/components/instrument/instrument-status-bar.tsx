import { useTranslation } from "react-i18next";

export type InstrumentStatusBarProps = {
  modeLabel: "CC" | "CV" | "CP" | "CR" | "UNKNOWN";
  linkState: "up" | "down" | "unknown";
  outputState: { enabled: boolean; setpointLabel: string | null };
  protectionState: { summary: string; level: "ok" | "warn" | "danger" };
  faultSummary: string | null;
  stale?: boolean;
};

export function InstrumentStatusBar({
  modeLabel,
  linkState,
  outputState,
  protectionState,
  faultSummary,
  stale = false,
}: InstrumentStatusBarProps) {
  const { t } = useTranslation();
  const runStateText =
    modeLabel === "CC"
      ? t("dashboard.statusBar.constantCurrent")
      : modeLabel === "CV"
        ? t("dashboard.statusBar.constantVoltage")
        : modeLabel === "CP"
          ? t("dashboard.statusBar.constantPower")
          : modeLabel === "CR"
            ? t("dashboard.statusBar.constantResistance")
            : t("dashboard.statusBar.unknown");

  const pillBase = "instrument-pill";

  const linkPillClass =
    linkState === "up"
      ? "instrument-pill-green"
      : linkState === "down"
        ? "instrument-pill-danger"
        : "";

  const outputText = outputState.enabled
    ? `${t("dashboard.statusBar.outputOn")}${outputState.setpointLabel ? ` · ${outputState.setpointLabel}` : ""}`
    : t("dashboard.statusBar.outputOff");

  const outputPillClass = outputState.enabled ? "instrument-pill-cyan" : "";

  const protectTitleText =
    protectionState.level === "danger"
      ? t("dashboard.statusBar.fault")
      : protectionState.level === "warn"
        ? t("dashboard.statusBar.attention")
        : t("dashboard.statusBar.allClear");

  const protectPillText =
    protectionState.level === "ok" && !faultSummary
      ? t("dashboard.statusBar.uvReady")
      : protectionState.level === "ok" && faultSummary
        ? t("dashboard.statusBar.faultPresent")
        : protectionState.summary.replaceAll("_", " ");

  const protectPillClass =
    protectionState.level === "danger"
      ? "instrument-pill-danger"
      : protectionState.level === "warn"
        ? "instrument-pill-amber"
        : "instrument-pill-amber";

  return (
    <header className="instrument-card px-6 py-4">
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3 lg:gap-6">
        <div>
          <div className="flex items-start justify-between gap-3">
            <div>
              <div className="instrument-label">
                {t("dashboard.statusBar.runState")}
              </div>
              <div className="mt-1 text-sm font-semibold text-slate-100">
                {runStateText}
              </div>
            </div>
            {stale ? (
              <span className="rounded-full border border-amber-400/20 bg-amber-500/10 px-2 py-0.5 text-[10px] font-semibold tracking-[0.14em] text-amber-200">
                {t("dashboard.statusBar.stale")}
              </span>
            ) : null}
          </div>
          <div className="mt-2">
            <span className={`${pillBase} ${linkPillClass} w-full`}>
              {linkState === "up"
                ? t("dashboard.statusBar.linkUp")
                : linkState === "down"
                  ? t("dashboard.statusBar.linkDown")
                  : t("dashboard.statusBar.linkUnknown")}
            </span>
          </div>
        </div>

        <div>
          <div className="instrument-label">
            {t("dashboard.statusBar.output")}
          </div>
          <div className="mt-1 text-sm font-semibold text-slate-100">
            {outputText}
          </div>
          <div className="mt-2">
            <span className={`${pillBase} ${outputPillClass} w-full`}>
              {outputState.enabled
                ? t("dashboard.statusBar.remoteActive")
                : t("dashboard.statusBar.outputDisabled")}
            </span>
          </div>
        </div>

        <div>
          <div className="instrument-label">
            {t("dashboard.statusBar.protection")}
          </div>
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

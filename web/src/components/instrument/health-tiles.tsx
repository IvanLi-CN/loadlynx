export type HealthTilesProps = {
  analogState: string;
  faultLabel: string;
  linkLatencyMs: number | null;
};

function Tile({
  label,
  value,
  tone = "neutral",
}: {
  label: string;
  value: string;
  tone?: "neutral" | "ok" | "warn" | "danger";
}) {
  const toneClass =
    tone === "ok"
      ? "text-emerald-200"
      : tone === "warn"
        ? "text-amber-200"
        : tone === "danger"
          ? "text-red-200"
          : "text-slate-100";

  return (
    <div className="instrument-card px-4 py-3">
      <div className="instrument-label">{label}</div>
      <div className={`mt-2 text-sm font-semibold ${toneClass}`}>{value}</div>
    </div>
  );
}

export function HealthTiles({
  analogState,
  faultLabel,
  linkLatencyMs,
}: HealthTilesProps) {
  const analogValue =
    analogState === "ready"
      ? "Online"
      : analogState === "offline"
        ? "Offline"
        : analogState === "cal_missing"
          ? "Cal missing"
          : analogState === "faulted"
            ? "Faulted"
            : analogState;

  const faultValue = faultLabel === "OK" ? "None" : faultLabel;
  const faultTone = faultLabel === "OK" ? "ok" : "danger";
  const latencyText =
    linkLatencyMs != null ? `${Math.round(linkLatencyMs)} ms` : "—";

  return (
    <section aria-label="Health tiles" className="grid grid-cols-3 gap-3">
      <Tile label="Analog State" value={analogValue} tone="neutral" />
      <Tile label="Fault Flags" value={faultValue} tone={faultTone} />
      <Tile label="Latency" value={latencyText} tone="neutral" />
    </section>
  );
}

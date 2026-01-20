export type LimitsPanelProps = {
  limits: Array<{
    label: string;
    value: string;
    tone?: "ok" | "warn" | "danger";
  }>;
};

function toneClass(tone: LimitsPanelProps["limits"][number]["tone"]): string {
  if (tone === "danger") return "text-red-200";
  if (tone === "warn") return "text-amber-200";
  if (tone === "ok") return "text-slate-100";
  return "text-slate-100";
}

export function LimitsPanel({ limits }: LimitsPanelProps) {
  return (
    <section aria-label="Limits" className="instrument-card p-5">
      <div className="instrument-label">Limits</div>
      <div className="mt-4 grid grid-cols-2 gap-3">
        {limits.map((row) => (
          <div
            key={row.label}
            className="rounded-xl border border-slate-400/10 bg-black/20 px-4 py-3"
          >
            <div className="text-[10px] tracking-[0.14em] uppercase text-slate-200/50">
              {row.label}
            </div>
            <div
              className={`mt-2 text-sm font-semibold ${toneClass(row.tone)}`}
            >
              {row.value}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

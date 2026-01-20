export type SetpointsPanelProps = {
  setpoints: Array<{
    label: string;
    value: string;
    readback: string | null;
    active?: boolean;
  }>;
};

export function SetpointsPanel({ setpoints }: SetpointsPanelProps) {
  return (
    <section aria-label="Setpoints" className="instrument-card p-5">
      <div className="instrument-label">Setpoints</div>
      <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-2">
        {setpoints.map((row) => {
          const rawReadback = row.readback ?? "Read: —";
          const readbackValue = rawReadback.startsWith("Read:")
            ? rawReadback.slice("Read:".length).trim()
            : rawReadback;

          return (
            <div
              key={row.label}
              className={[
                "rounded-xl border px-4 py-3",
                row.active
                  ? "border-[rgba(111,234,249,0.22)] bg-[rgba(111,234,249,0.08)]"
                  : "border-slate-400/10 bg-black/20",
              ].join(" ")}
            >
              <div className="text-[10px] tracking-[0.14em] uppercase text-slate-200/50">
                {row.label}
              </div>
              <div className="mt-2 text-sm font-semibold text-slate-100">
                {row.value}
              </div>
              <div className="mt-1 flex flex-wrap items-baseline gap-x-2 gap-y-1 text-[11px]">
                <span className="text-slate-200/55">Read:</span>
                <span className="instrument-glow-cyan font-mono">
                  {readbackValue}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

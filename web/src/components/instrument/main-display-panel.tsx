import { FixedNumber } from "./fixed-number.tsx";
import { Sparkline } from "./sparkline.tsx";

export type MainDisplayPanelProps = {
  headline: { value: number | null; unit: "V" | "A" | "W" };
  modeLabel: "CC" | "CV" | "CP" | "CR";
  setpointLabel: string;
  uptimeLabel: string;
  trend: { points: number[]; min: number; max: number };
  stale?: boolean;
};

export function MainDisplayPanel({
  headline,
  modeLabel,
  setpointLabel,
  uptimeLabel,
  trend,
  stale = false,
}: MainDisplayPanelProps) {
  const digits = headline.unit === "W" ? 2 : 3;

  return (
    <section
      aria-label="Main display"
      className="instrument-card p-5"
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="instrument-label">Main display</div>
          <div className="mt-2 flex items-baseline gap-3">
            <div className="instrument-glow-green text-6xl font-bold tracking-tight">
              <FixedNumber value={headline.value} digits={digits} />
            </div>
            <div className="text-sm font-semibold text-slate-200/70">
              {headline.unit}
            </div>
            {stale ? (
              <span className="rounded-full border border-amber-400/20 bg-amber-500/10 px-2 py-0.5 text-[10px] font-semibold tracking-[0.14em] text-amber-200">
                STALE
              </span>
            ) : null}
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px] text-slate-200/55">
            <span>
              Mode: <span className="text-slate-100/90">{modeLabel}</span>
            </span>
            <span>
              Set: <span className="font-mono text-slate-100/90">{setpointLabel}</span>
            </span>
            <span>
              Uptime: <span className="font-mono text-slate-100/90">{uptimeLabel}</span>
            </span>
          </div>
        </div>
      </div>

      <div className="mt-4 rounded-xl border border-slate-400/10 bg-black/20 p-2">
        <Sparkline
          points={trend.points}
          min={trend.min}
          max={trend.max}
          height={82}
          tone="cyan"
          variant="line"
        />
      </div>
    </section>
  );
}

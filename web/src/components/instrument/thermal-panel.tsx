import { formatFixed } from "./format.ts";
import { FixedNumber } from "./fixed-number.tsx";
import { Sparkline } from "./sparkline.tsx";

export type ThermalPanelProps = {
  sinkCoreC: number | null;
  sinkExhaustC: number | null;
  mcuC: number | null;
  faults: string[];
  trend?: { points: number[]; min: number; max: number };
};

export function ThermalPanel({
  sinkCoreC,
  sinkExhaustC,
  mcuC,
  faults,
  trend,
}: ThermalPanelProps) {
  const hasFaults = faults.length > 0;
  return (
    <section
      aria-label="Thermal and faults"
      className="instrument-card p-5"
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="instrument-label">Thermal / Faults</div>
          <div className="mt-2 flex items-baseline gap-3">
            <div className="instrument-glow-amber text-6xl font-bold tracking-tight">
              <FixedNumber value={sinkCoreC} digits={1} />
            </div>
            <div className="text-sm font-semibold text-slate-200/70">°C</div>
          </div>

          <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px] text-slate-200/55">
            <span>
              Sink Core{" "}
              <span className="font-mono text-slate-100/90">
                {formatFixed(sinkCoreC, 1)}°C
              </span>
            </span>
            <span>
              Exhaust{" "}
              <span className="font-mono text-slate-100/90">
                {formatFixed(sinkExhaustC, 1)}°C
              </span>
            </span>
            <span>
              MCU{" "}
              <span className="font-mono text-slate-100/90">
                {formatFixed(mcuC, 1)}°C
              </span>
            </span>
          </div>
        </div>

        <div className="text-right">
          <div className="instrument-label">Faults</div>
          {hasFaults ? (
            <div className="mt-2 space-y-1 text-[11px] text-red-200/85">
              {faults.slice(0, 3).map((f) => (
                <div key={f} className="font-semibold">
                  {f}
                </div>
              ))}
              {faults.length > 3 ? (
                <div className="text-[10px] text-red-200/60">
                  +{faults.length - 3} more
                </div>
              ) : null}
            </div>
          ) : (
            <div className="mt-2 text-[11px] text-emerald-200/80">None</div>
          )}
        </div>
      </div>

      <div className="mt-4 rounded-xl border border-slate-400/10 bg-black/20 p-2">
        <Sparkline
          points={trend?.points ?? []}
          min={trend?.min ?? 0}
          max={trend?.max ?? 1}
          height={82}
          tone="amber"
          variant="line"
        />
      </div>
    </section>
  );
}

import { FixedNumber } from "./fixed-number.tsx";
import { formatFixed } from "./format.ts";

export type MonitorReadoutsProps = {
  voltage: {
    read: number | null;
    local?: number | null;
    remote?: number | null;
  };
  current: {
    read: number | null;
    local?: number | null;
    remote?: number | null;
  };
  power: { read: number | null; ripplePct?: number | null };
  resistance: { read: number | null };
};

function ReadoutTile({
  label,
  value,
  digits,
  unit,
  subline,
}: {
  label: string;
  value: number | null;
  digits: number;
  unit: string;
  subline?: string | null;
}) {
  return (
    <div className="instrument-card px-5 py-4">
      <div className="instrument-label">{label}</div>
      <div className="mt-1 flex items-baseline gap-2">
        <div className="instrument-glow-cyan text-3xl font-bold tracking-tight">
          <FixedNumber value={value} digits={digits} />
        </div>
        <div className="text-sm text-slate-200/65">{unit}</div>
      </div>
      {subline ? (
        <div className="mt-1 text-[11px] text-slate-200/45">{subline}</div>
      ) : null}
    </div>
  );
}

export function MonitorReadouts({
  voltage,
  current,
  power,
  resistance,
}: MonitorReadoutsProps) {
  const vSub =
    voltage.local != null || voltage.remote != null
      ? `Local ${formatFixed(voltage.local, 3)} V · Remote ${formatFixed(voltage.remote, 3)} V`
      : null;

  const iSub =
    current.local != null || current.remote != null
      ? `Local ${formatFixed(current.local, 3)} A · Remote ${formatFixed(current.remote, 3)} A`
      : null;

  return (
    <section
      aria-label="Monitor readouts"
      className="grid grid-cols-1 sm:grid-cols-2 gap-3"
    >
      <ReadoutTile
        label="Voltage (read)"
        value={voltage.read}
        digits={3}
        unit="V"
        subline={vSub}
      />
      <ReadoutTile
        label="Current (read)"
        value={current.read}
        digits={3}
        unit="A"
        subline={iSub}
      />
      <ReadoutTile
        label="Power (read)"
        value={power.read}
        digits={2}
        unit="W"
      />
      <ReadoutTile
        label="Resistance (read)"
        value={resistance.read}
        digits={2}
        unit="Ω"
      />
    </section>
  );
}

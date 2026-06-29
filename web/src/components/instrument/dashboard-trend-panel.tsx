import { memo, useMemo, useState } from "react";
import type { TooltipContentProps, TooltipValueType } from "recharts";
import {
  Area,
  CartesianGrid,
  ComposedChart,
  Line,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { FixedNumber } from "./fixed-number.tsx";

type MetricKey = "voltage" | "current" | "power" | "resistance";
type TrendMetricKey = "voltage" | "current" | "power";

type TrendMetricSeries = {
  key: TrendMetricKey;
  label: string;
  shortLabel: string;
  unit: "V" | "A" | "W";
  digits: number;
  points: number[];
  times: number[];
  currentValue: number | null;
  referenceValue: number;
  referenceLabel: string;
  domainMin: number;
  domainMax: number;
};

type TrendRow = {
  time: number;
  voltage: number | null;
  current: number | null;
  power: number | null;
};

type TrendTone = {
  stroke: string;
  glow: string;
  dotFill: string;
  referenceStroke: string;
  areaFill?: string;
  axisTick?: string;
  axisDigits?: number;
  labelClassName: string;
};

export type DashboardTrendPanelProps = {
  headline: { value: number | null; unit: "V" | "A" | "W" | "Ω" };
  modeLabel: "CC" | "CV" | "CP" | "CR";
  setpointLabel: string;
  uptimeLabel: string;
  stale?: boolean;
  metrics: Array<{
    key: MetricKey;
    label: string;
    value: number | null;
    unit: "V" | "A" | "W" | "Ω";
    digits: number;
    detail: string | null;
    emphasized: boolean;
  }>;
  trendSeries: {
    voltage: TrendMetricSeries;
    current: TrendMetricSeries;
    power: TrendMetricSeries;
  };
};

const METRIC_TONES: Record<TrendMetricKey, TrendTone> = {
  voltage: {
    stroke: "#83ffd2",
    glow: "rgba(131,255,210,0.18)",
    dotFill: "#e8fff7",
    referenceStroke: "rgba(131,255,210,0.28)",
    axisTick: "rgba(131,255,210,0.72)",
    axisDigits: 1,
    labelClassName: "text-emerald-200/90",
  },
  current: {
    stroke: "#6feaf9",
    glow: "rgba(111,234,249,0.20)",
    dotFill: "#ecfbff",
    referenceStroke: "rgba(111,234,249,0.28)",
    axisTick: "rgba(111,234,249,0.72)",
    axisDigits: 2,
    labelClassName: "text-cyan-100/92",
  },
  power: {
    stroke: "#fdd45e",
    glow: "rgba(253,212,94,0.10)",
    dotFill: "#fff6d8",
    referenceStroke: "rgba(253,212,94,0.18)",
    areaFill: "rgba(253,212,94,0.08)",
    labelClassName: "text-amber-200/90",
  },
};

function formatTimeLabel(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) {
    return "—";
  }
  const total = Math.floor(seconds);
  const mm = Math.floor(total / 60);
  const ss = total % 60;
  return `${String(mm).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
}

function formatMetricValue(
  value: number | null | undefined,
  digits: number,
  unit: string,
): string {
  if (value == null || !Number.isFinite(value)) {
    return `— ${unit}`;
  }
  return `${value.toFixed(digits)} ${unit}`;
}

function formatAxisTick(value: number | string, digits: number): string {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return "";
  }
  return value.toFixed(digits);
}

function buildTrendRows(
  series: DashboardTrendPanelProps["trendSeries"],
): TrendRow[] {
  const byTime = new Map<number, TrendRow>();
  const entries: Array<[TrendMetricKey, TrendMetricSeries]> = [
    ["voltage", series.voltage],
    ["current", series.current],
    ["power", series.power],
  ];

  for (const [metricKey, metricSeries] of entries) {
    for (let index = 0; index < metricSeries.times.length; index += 1) {
      const time = metricSeries.times[index];
      const value = metricSeries.points[index] ?? null;
      if (!Number.isFinite(time)) {
        continue;
      }
      const existing = byTime.get(time) ?? {
        time,
        voltage: null,
        current: null,
        power: null,
      };
      existing[metricKey] = value;
      byTime.set(time, existing);
    }
  }

  return [...byTime.values()].sort((left, right) => left.time - right.time);
}

function TrendTooltip({
  active,
  payload,
  label,
  series,
}: TooltipContentProps<TooltipValueType, string | number> & {
  series: DashboardTrendPanelProps["trendSeries"];
}) {
  if (!active || !payload || payload.length === 0) {
    return null;
  }

  const row = payload[0]?.payload as TrendRow | undefined;
  if (!row) {
    return null;
  }

  const entries: Array<[TrendMetricKey, TrendMetricSeries]> = [
    ["voltage", series.voltage],
    ["current", series.current],
    ["power", series.power],
  ];

  return (
    <div className="ll-trend-tooltip">
      <div className="ll-trend-tooltip__time">
        {formatTimeLabel(typeof label === "number" ? label : row.time)}
      </div>
      <div className="ll-trend-tooltip__list">
        {entries.map(([metricKey, metricSeries]) => (
          <div key={metricKey} className="ll-trend-tooltip__item">
            <span
              className="ll-trend-tooltip__swatch"
              style={{ backgroundColor: METRIC_TONES[metricKey].stroke }}
            />
            <span className="ll-trend-tooltip__label">
              {metricSeries.label}
            </span>
            <span className="ll-trend-tooltip__value">
              {formatMetricValue(
                row[metricKey],
                metricSeries.digits,
                metricSeries.unit,
              )}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

type DashboardTrendPlotProps = {
  rows: TrendRow[];
  trendSeries: DashboardTrendPanelProps["trendSeries"];
  pinnedIndex: number | null;
  onPinnedIndexChange: (nextIndex: number | null) => void;
};

const DashboardTrendPlot = memo(function DashboardTrendPlot({
  rows,
  trendSeries,
  pinnedIndex,
  onPinnedIndexChange,
}: DashboardTrendPlotProps) {
  return rows.length >= 2 ? (
    <ResponsiveContainer width="100%" height="100%">
      <ComposedChart
        data={rows}
        margin={{ top: 12, right: 28, bottom: 8, left: 22 }}
        onClick={(state) => {
          const nextIndex =
            typeof state?.activeTooltipIndex === "number"
              ? state.activeTooltipIndex
              : null;
          if (nextIndex == null) {
            return;
          }
          onPinnedIndexChange(nextIndex);
        }}
      >
        <CartesianGrid
          vertical={false}
          stroke="rgba(148,163,184,0.08)"
          strokeDasharray="3 8"
        />
        <XAxis
          dataKey="time"
          type="number"
          domain={["dataMin", "dataMax"]}
          tickFormatter={(value: number) => formatTimeLabel(value)}
          tickLine={false}
          axisLine={false}
          minTickGap={36}
          tick={{
            fill: "rgba(226,232,240,0.42)",
            fontSize: 10,
          }}
        />

        <YAxis
          yAxisId="voltage"
          width={56}
          domain={[
            trendSeries.voltage.domainMin,
            trendSeries.voltage.domainMax,
          ]}
          axisLine={false}
          tickLine={false}
          tickMargin={10}
          tickFormatter={(value) =>
            formatAxisTick(value, METRIC_TONES.voltage.axisDigits ?? 1)
          }
          tick={{
            fill: METRIC_TONES.voltage.axisTick,
            fontSize: 11,
            fontWeight: 600,
            dx: 28,
          }}
        />
        <YAxis
          yAxisId="current"
          orientation="right"
          width={60}
          domain={[
            trendSeries.current.domainMin,
            trendSeries.current.domainMax,
          ]}
          axisLine={false}
          tickLine={false}
          tickMargin={10}
          tickFormatter={(value) =>
            formatAxisTick(value, METRIC_TONES.current.axisDigits ?? 2)
          }
          tick={{
            fill: METRIC_TONES.current.axisTick,
            fontSize: 11,
            fontWeight: 600,
          }}
        />
        <YAxis
          yAxisId="power"
          hide
          domain={[trendSeries.power.domainMin, trendSeries.power.domainMax]}
        />

        <Tooltip
          shared
          trigger={pinnedIndex != null ? "click" : "hover"}
          defaultIndex={pinnedIndex ?? undefined}
          cursor={{
            stroke: "rgba(226,232,240,0.22)",
            strokeWidth: 1,
            strokeDasharray: "4 6",
          }}
          content={(props) => <TrendTooltip {...props} series={trendSeries} />}
          wrapperStyle={{ outline: "none" }}
        />

        <Area
          yAxisId="power"
          type="monotone"
          dataKey="power"
          stroke="none"
          fill={METRIC_TONES.power.areaFill}
          isAnimationActive={false}
          connectNulls
        />

        {(
          Object.entries(trendSeries) as Array<
            [TrendMetricKey, TrendMetricSeries]
          >
        ).map(([metricKey, entry]) => (
          <ReferenceLine
            key={`${metricKey}-reference`}
            yAxisId={metricKey}
            y={entry.referenceValue}
            stroke={METRIC_TONES[metricKey].referenceStroke}
            strokeWidth={1}
            strokeDasharray="3 11"
            ifOverflow="extendDomain"
          />
        ))}

        {(
          Object.entries(trendSeries) as Array<
            [TrendMetricKey, TrendMetricSeries]
          >
        ).map(([metricKey, entry]) => (
          <Line
            key={metricKey}
            yAxisId={metricKey}
            type="monotone"
            dataKey={metricKey}
            stroke={METRIC_TONES[metricKey].stroke}
            strokeWidth={metricKey === "power" ? 1.35 : 2.4}
            strokeOpacity={metricKey === "power" ? 0.72 : 1}
            dot={false}
            activeDot={
              metricKey === "power"
                ? false
                : {
                    r: 6,
                    strokeWidth: 2,
                    stroke: METRIC_TONES[metricKey].stroke,
                    fill: METRIC_TONES[metricKey].dotFill,
                  }
            }
            isAnimationActive={false}
            connectNulls
            name={`${entry.label} line`}
          />
        ))}
      </ComposedChart>
    </ResponsiveContainer>
  ) : (
    <div className="flex h-full items-center justify-center text-sm text-slate-200/42">
      No data
    </div>
  );
});

export function DashboardTrendPanel({
  headline,
  modeLabel,
  setpointLabel,
  uptimeLabel,
  stale = false,
  metrics,
  trendSeries,
}: DashboardTrendPanelProps) {
  const latestTime =
    trendSeries.current.times.at(-1) ??
    trendSeries.voltage.times.at(-1) ??
    trendSeries.power.times.at(-1) ??
    null;
  const earliestTime =
    trendSeries.current.times.at(0) ??
    trendSeries.voltage.times.at(0) ??
    trendSeries.power.times.at(0) ??
    null;
  const focusDigits = headline.unit === "W" || headline.unit === "Ω" ? 2 : 3;
  const rows = useMemo(() => buildTrendRows(trendSeries), [trendSeries]);
  const [pinnedIndex, setPinnedIndex] = useState<number | null>(null);

  return (
    <section
      aria-label="Primary dashboard monitor"
      className="instrument-card overflow-hidden p-5 sm:p-6"
    >
      <div className="flex flex-col gap-5 border-b border-slate-400/10 pb-5 xl:flex-row xl:items-start xl:justify-between">
        <div className="min-w-0 flex-1">
          <div className="instrument-label">Main display</div>
          <div className="mt-3 flex flex-wrap items-baseline gap-x-3 gap-y-2">
            <div className="instrument-glow-green text-5xl font-bold tracking-tight sm:text-6xl">
              <FixedNumber value={headline.value} digits={focusDigits} />
            </div>
            <div className="text-base font-semibold text-slate-200/70 sm:text-lg">
              {headline.unit}
            </div>
            <span className="rounded-full border border-cyan-400/16 bg-cyan-400/8 px-2.5 py-1 text-[10px] font-semibold tracking-[0.14em] text-cyan-100/90 uppercase">
              {modeLabel}
            </span>
            {stale ? (
              <span className="rounded-full border border-amber-400/20 bg-amber-500/10 px-2.5 py-1 text-[10px] font-semibold tracking-[0.14em] text-amber-200 uppercase">
                Stale
              </span>
            ) : null}
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px] text-slate-200/55">
            <span>
              Setpoint:{" "}
              <span className="font-semibold text-slate-100/90">
                {setpointLabel}
              </span>
            </span>
            <span>
              Uptime:{" "}
              <span className="font-semibold text-slate-100/90">
                {uptimeLabel}
              </span>
            </span>
            <span>
              Window:{" "}
              <span className="font-semibold text-slate-100/90">
                {formatTimeLabel(earliestTime)} → {formatTimeLabel(latestTime)}
              </span>
            </span>
          </div>
        </div>

        <div className="min-w-0 xl:min-w-[16rem] xl:pl-6 xl:text-right">
          <div className="instrument-label">Live focus</div>
          <div className="mt-2 text-sm font-semibold text-slate-100">
            Voltage / Current / Power
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-slate-200/52 xl:justify-end">
            <span>Last 30 s</span>
            {pinnedIndex != null ? (
              <>
                <span className="text-slate-200/30">•</span>
                <span className="font-semibold text-slate-100/80">
                  Pinned sample
                </span>
                <button
                  type="button"
                  className="text-cyan-100/78 transition hover:text-cyan-100"
                  onClick={() => setPinnedIndex(null)}
                >
                  Clear
                </button>
              </>
            ) : null}
          </div>
        </div>
      </div>

      <div className="mt-4 grid min-w-0 grid-cols-2 gap-2 lg:grid-cols-4">
        {metrics.map((metric) => (
          <div
            key={metric.key}
            className={[
              "rounded-xl border px-4 py-3",
              metric.emphasized
                ? "border-cyan-300/28 bg-cyan-400/10 shadow-[0_0_22px_rgba(111,234,249,0.10)]"
                : "border-slate-400/10 bg-black/16",
            ].join(" ")}
          >
            <div className="text-[10px] font-semibold tracking-[0.14em] text-slate-200/54 uppercase">
              {metric.label}
            </div>
            <div className="mt-2 flex items-baseline gap-2">
              <div
                className={
                  metric.emphasized
                    ? "instrument-glow-cyan text-3xl font-bold tracking-tight"
                    : "text-2xl font-bold tracking-tight text-slate-100"
                }
              >
                <FixedNumber value={metric.value} digits={metric.digits} />
              </div>
              <div className="text-sm text-slate-200/62">{metric.unit}</div>
            </div>
            {metric.detail ? (
              <div className="mt-1 text-[11px] text-slate-200/42">
                {metric.detail}
              </div>
            ) : null}
          </div>
        ))}
      </div>

      <div className="mt-5 border-t border-slate-400/10 pt-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <div className="instrument-label">Trend</div>
            <div className="mt-1 text-sm font-semibold text-slate-100">
              Voltage, current and power over last 30 s
            </div>
          </div>
          <div className="grid gap-x-6 gap-y-3 text-right text-[11px] text-slate-200/54 sm:grid-cols-3 sm:text-left xl:min-w-[32rem]">
            {[trendSeries.voltage, trendSeries.current, trendSeries.power].map(
              (entry) => (
                <div key={entry.key} className="ll-dashboard-trend-stat">
                  <div
                    className={`text-[10px] font-semibold tracking-[0.14em] uppercase ${METRIC_TONES[entry.key].labelClassName}`}
                  >
                    {entry.label}
                  </div>
                  <div className="mt-1 font-semibold text-slate-100">
                    <FixedNumber
                      value={entry.currentValue}
                      digits={entry.digits}
                    />{" "}
                    {entry.unit}
                  </div>
                  <div className="mt-1 text-[10px] text-slate-200/48">
                    {entry.referenceLabel}:{" "}
                    <span className="font-semibold text-slate-100/80">
                      <FixedNumber
                        value={entry.referenceValue}
                        digits={entry.digits}
                      />{" "}
                      {entry.unit}
                    </span>
                  </div>
                </div>
              ),
            )}
          </div>
        </div>

        <div className="ll-dashboard-trend-plot mt-4 pt-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex flex-wrap items-center gap-3 text-[11px] text-slate-200/54">
              {(
                Object.entries(trendSeries) as Array<
                  [TrendMetricKey, TrendMetricSeries]
                >
              ).map(([metricKey, entry]) => (
                <div
                  key={metricKey}
                  className="flex items-center gap-2"
                  style={{
                    opacity: metricKey === "power" ? 0.68 : 1,
                  }}
                >
                  <span
                    className="h-2.5 w-2.5 rounded-full"
                    style={{
                      backgroundColor: METRIC_TONES[metricKey].stroke,
                      boxShadow:
                        metricKey === "power"
                          ? "none"
                          : `0 0 10px ${METRIC_TONES[metricKey].glow}`,
                    }}
                  />
                  <span className="font-semibold text-slate-100/88">
                    {entry.label}
                  </span>
                  <span>{entry.referenceLabel}</span>
                </div>
              ))}
            </div>
            <div className="text-[10px] text-slate-200/46">
              Hover to inspect. Click to pin.
            </div>
          </div>

          <div className="h-[19rem] min-h-[19rem] pt-3 lg:h-[21rem] lg:min-h-[21rem]">
            <DashboardTrendPlot
              rows={rows}
              trendSeries={trendSeries}
              pinnedIndex={pinnedIndex}
              onPinnedIndexChange={(nextIndex) =>
                setPinnedIndex((current) =>
                  current === nextIndex ? null : nextIndex,
                )
              }
            />
          </div>
        </div>
      </div>
    </section>
  );
}

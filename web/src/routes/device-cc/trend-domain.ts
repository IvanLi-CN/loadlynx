import type { LoadMode } from "../../api/types.ts";

export type TrendMetricKey = "voltage" | "current" | "power";

export type TrendSample = {
  time: number;
  voltage: number | null;
  current: number | null;
  power: number | null;
  resistance?: number | null;
  thermal?: number | null;
};

export type TrendReferenceKind =
  | "target_v"
  | "min_v"
  | "target_i"
  | "max_i"
  | "target_p"
  | "max_p";

export type TrendReference = {
  value: number;
  kind: TrendReferenceKind;
  label: string;
};

export type TrendSeriesView = {
  key: TrendMetricKey;
  label: string;
  shortLabel: string;
  unit: "V" | "A" | "W";
  digits: number;
  points: number[];
  times: number[];
  currentValue: number | null;
  referenceValue: number;
  referenceKind: TrendReferenceKind;
  referenceLabel: string;
  domainMin: number;
  domainMax: number;
};

export type TrendSeriesRecord = Record<TrendMetricKey, TrendSeriesView>;

export type TrendDomainMemory = {
  domainMin: number;
  domainMax: number;
  referenceKind: TrendReferenceKind;
  referenceValue: number;
  latestTime: number | null;
};

export type TrendDomainMemoryMap = Partial<
  Record<TrendMetricKey, TrendDomainMemory>
>;

type MetricConfig = {
  minSpan: number;
  topPadFloor: number;
  bottomPadFloor: number;
  topInsetPct: number;
  bottomInsetPct: number;
  zeroAnchored: boolean;
};

export const TREND_WINDOW_SECONDS = 30;
export const ELECTRICAL_TREND_SAMPLE_INTERVAL_MS = 500;
export const THERMAL_TREND_SAMPLE_INTERVAL_MS = 1_000;
export const TREND_SAMPLE_INTERVAL_MS = THERMAL_TREND_SAMPLE_INTERVAL_MS;

const METRIC_CONFIG: Record<TrendMetricKey, MetricConfig> = {
  voltage: {
    minSpan: 1.0,
    topPadFloor: 0.6,
    bottomPadFloor: 0.4,
    topInsetPct: 0.08,
    bottomInsetPct: 0.06,
    zeroAnchored: true,
  },
  current: {
    minSpan: 0.5,
    topPadFloor: 0.25,
    bottomPadFloor: 0.15,
    topInsetPct: 0.08,
    bottomInsetPct: 0.06,
    zeroAnchored: true,
  },
  power: {
    minSpan: 5,
    topPadFloor: 2,
    bottomPadFloor: 1,
    topInsetPct: 0.08,
    bottomInsetPct: 0.06,
    zeroAnchored: true,
  },
};

function isFiniteNumber(value: number | null | undefined): value is number {
  return value != null && Number.isFinite(value);
}

export function trimTrendSamplesToWindow(
  samples: TrendSample[],
  windowSeconds = TREND_WINDOW_SECONDS,
): TrendSample[] {
  if (samples.length === 0) {
    return samples;
  }
  const latestTime = samples.at(-1)?.time;
  if (!isFiniteNumber(latestTime)) {
    return samples;
  }
  const earliestAllowed = latestTime - windowSeconds;
  return samples.filter(
    (sample) => isFiniteNumber(sample.time) && sample.time >= earliestAllowed,
  );
}

export function shouldAppendTrendSample(
  lastRecordedUptimeMs: number | null,
  nextUptimeMs: number | null,
  sampleIntervalMs = TREND_SAMPLE_INTERVAL_MS,
): boolean {
  if (!isFiniteNumber(nextUptimeMs)) {
    return false;
  }

  if (lastRecordedUptimeMs == null) {
    return true;
  }

  if (nextUptimeMs < lastRecordedUptimeMs) {
    return true;
  }

  return nextUptimeMs - lastRecordedUptimeMs >= sampleIntervalMs;
}

function getSeriesValue(sample: TrendSample, metric: TrendMetricKey) {
  switch (metric) {
    case "voltage":
      return sample.voltage;
    case "current":
      return sample.current;
    case "power":
      return sample.power;
  }
}

export function buildTrendSeriesSamples(
  samples: TrendSample[],
  metric: TrendMetricKey,
) {
  const points: number[] = [];
  const times: number[] = [];
  for (const sample of samples) {
    const value = getSeriesValue(sample, metric);
    if (!isFiniteNumber(sample.time) || !isFiniteNumber(value)) {
      continue;
    }
    times.push(sample.time);
    points.push(value);
  }
  return { points, times };
}

export function getTrendReferenceByMetric(params: {
  metric: TrendMetricKey;
  mode: LoadMode;
  targetVoltageV: number;
  minVoltageV: number;
  targetCurrentA: number;
  maxCurrentA: number;
  targetPowerW: number;
  maxPowerW: number;
}): TrendReference {
  const {
    metric,
    mode,
    targetVoltageV,
    minVoltageV,
    targetCurrentA,
    maxCurrentA,
    targetPowerW,
    maxPowerW,
  } = params;

  switch (metric) {
    case "voltage":
      if (mode === "cv") {
        return {
          value: Math.max(0, targetVoltageV),
          kind: "target_v",
          label: "Target V",
        };
      }
      return {
        value: Math.max(0, minVoltageV),
        kind: "min_v",
        label: "Min V",
      };
    case "current":
      if (mode === "cc") {
        return {
          value: Math.max(0, targetCurrentA),
          kind: "target_i",
          label: "Target I",
        };
      }
      return {
        value: Math.max(0, maxCurrentA),
        kind: "max_i",
        label: "Max I",
      };
    case "power":
      if (mode === "cp") {
        return {
          value: Math.max(0, targetPowerW),
          kind: "target_p",
          label: "Target P",
        };
      }
      return {
        value: Math.max(0, maxPowerW),
        kind: "max_p",
        label: "Max P",
      };
  }
}

export function computeTrendDomain(params: {
  metric: TrendMetricKey;
  points: number[];
  referenceValue: number;
}) {
  const { metric, points, referenceValue } = params;
  const config = METRIC_CONFIG[metric];
  const dataMin = points.length > 0 ? Math.min(...points) : referenceValue;
  const dataMax = points.length > 0 ? Math.max(...points) : referenceValue;
  const zeroAnchor = config.zeroAnchored ? 0 : undefined;
  const unclampedBaseMin = Math.min(
    dataMin,
    referenceValue,
    zeroAnchor ?? dataMin,
  );
  const baseMin = Math.max(0, unclampedBaseMin);
  const baseMax = Math.max(dataMax, referenceValue);
  const span = Math.max(baseMax - baseMin, config.minSpan);
  const topPad = Math.max(span * 0.12, config.topPadFloor);
  const bottomPad = Math.max(span * 0.08, config.bottomPadFloor);
  let domainMin = Math.max(0, baseMin - bottomPad);
  let domainMax = Math.max(baseMax + topPad, domainMin + config.minSpan);

  const domainSpan = Math.max(domainMax - domainMin, config.minSpan);
  const plotSpan =
    domainSpan / (1 - config.topInsetPct - config.bottomInsetPct);
  domainMin = Math.max(0, domainMin - plotSpan * config.bottomInsetPct);
  domainMax = domainMin + plotSpan;

  if (referenceValue > domainMax) {
    domainMax = referenceValue + config.topPadFloor;
  }

  return {
    min: domainMin,
    max: domainMax,
  };
}

export function stabilizeTrendSeriesDomains(
  series: TrendSeriesRecord,
  previous: TrendDomainMemoryMap,
): TrendSeriesRecord {
  const nextEntries = Object.entries(series).map(([key, entry]) => {
    const metric = key as TrendMetricKey;
    const latestTime = entry.times.at(-1) ?? null;
    const prev = previous[metric];
    const referenceChanged =
      !prev ||
      prev.referenceKind !== entry.referenceKind ||
      Math.abs(prev.referenceValue - entry.referenceValue) > 1e-9;
    const timelineReset =
      prev?.latestTime != null &&
      latestTime != null &&
      latestTime < prev.latestTime;

    if (!prev || referenceChanged || timelineReset) {
      return [metric, entry] as const;
    }

    return [
      metric,
      {
        ...entry,
        domainMin: Math.min(prev.domainMin, entry.domainMin),
        domainMax: Math.max(prev.domainMax, entry.domainMax),
      },
    ] as const;
  });

  return Object.fromEntries(nextEntries) as TrendSeriesRecord;
}

export function snapshotTrendSeriesDomains(
  series: TrendSeriesRecord,
): TrendDomainMemoryMap {
  return Object.fromEntries(
    Object.entries(series).map(([key, entry]) => [
      key,
      {
        domainMin: entry.domainMin,
        domainMax: entry.domainMax,
        referenceKind: entry.referenceKind,
        referenceValue: entry.referenceValue,
        latestTime: entry.times.at(-1) ?? null,
      },
    ]),
  ) as TrendDomainMemoryMap;
}

export function buildTrendSeries(params: {
  samples: TrendSample[];
  mode: LoadMode;
  targetVoltageV: number;
  minVoltageV: number;
  targetCurrentA: number;
  maxCurrentA: number;
  targetPowerW: number;
  maxPowerW: number;
}): TrendSeriesRecord {
  const { samples, mode } = params;
  const refs = {
    voltage: getTrendReferenceByMetric({
      metric: "voltage",
      mode,
      targetVoltageV: params.targetVoltageV,
      minVoltageV: params.minVoltageV,
      targetCurrentA: params.targetCurrentA,
      maxCurrentA: params.maxCurrentA,
      targetPowerW: params.targetPowerW,
      maxPowerW: params.maxPowerW,
    }),
    current: getTrendReferenceByMetric({
      metric: "current",
      mode,
      targetVoltageV: params.targetVoltageV,
      minVoltageV: params.minVoltageV,
      targetCurrentA: params.targetCurrentA,
      maxCurrentA: params.maxCurrentA,
      targetPowerW: params.targetPowerW,
      maxPowerW: params.maxPowerW,
    }),
    power: getTrendReferenceByMetric({
      metric: "power",
      mode,
      targetVoltageV: params.targetVoltageV,
      minVoltageV: params.minVoltageV,
      targetCurrentA: params.targetCurrentA,
      maxCurrentA: params.maxCurrentA,
      targetPowerW: params.targetPowerW,
      maxPowerW: params.maxPowerW,
    }),
  } as const;

  const voltageSeries = buildTrendSeriesSamples(samples, "voltage");
  const currentSeries = buildTrendSeriesSamples(samples, "current");
  const powerSeries = buildTrendSeriesSamples(samples, "power");

  const voltageDomain = computeTrendDomain({
    metric: "voltage",
    points: voltageSeries.points,
    referenceValue: refs.voltage.value,
  });
  const currentDomain = computeTrendDomain({
    metric: "current",
    points: currentSeries.points,
    referenceValue: refs.current.value,
  });
  const powerDomain = computeTrendDomain({
    metric: "power",
    points: powerSeries.points,
    referenceValue: refs.power.value,
  });

  return {
    voltage: {
      key: "voltage",
      label: "Voltage",
      shortLabel: "V",
      unit: "V",
      digits: 3,
      points: voltageSeries.points,
      times: voltageSeries.times,
      currentValue: voltageSeries.points.at(-1) ?? null,
      referenceValue: refs.voltage.value,
      referenceKind: refs.voltage.kind,
      referenceLabel: refs.voltage.label,
      domainMin: voltageDomain.min,
      domainMax: voltageDomain.max,
    },
    current: {
      key: "current",
      label: "Current",
      shortLabel: "I",
      unit: "A",
      digits: 3,
      points: currentSeries.points,
      times: currentSeries.times,
      currentValue: currentSeries.points.at(-1) ?? null,
      referenceValue: refs.current.value,
      referenceKind: refs.current.kind,
      referenceLabel: refs.current.label,
      domainMin: currentDomain.min,
      domainMax: currentDomain.max,
    },
    power: {
      key: "power",
      label: "Power",
      shortLabel: "P",
      unit: "W",
      digits: 2,
      points: powerSeries.points,
      times: powerSeries.times,
      currentValue: powerSeries.points.at(-1) ?? null,
      referenceValue: refs.power.value,
      referenceKind: refs.power.kind,
      referenceLabel: refs.power.label,
      domainMin: powerDomain.min,
      domainMax: powerDomain.max,
    },
  };
}

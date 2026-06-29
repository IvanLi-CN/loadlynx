import { describe, expect, it } from "vitest";
import {
  buildTrendSeries,
  computeTrendDomain,
  getTrendReferenceByMetric,
  shouldAppendTrendSample,
  snapshotTrendSeriesDomains,
  stabilizeTrendSeriesDomains,
  type TrendSample,
  trimTrendSamplesToWindow,
} from "./trend-domain.ts";

describe("trend-domain", () => {
  it("trims samples to the latest 30 second window", () => {
    const samples: TrendSample[] = [
      { time: 10, voltage: 12, current: 1, power: 12 },
      { time: 25, voltage: 12.1, current: 1.1, power: 13.31 },
      { time: 39, voltage: 12.2, current: 1.2, power: 14.64 },
      { time: 41, voltage: 12.3, current: 1.3, power: 15.99 },
    ];

    expect(
      trimTrendSamplesToWindow(samples).map((sample) => sample.time),
    ).toEqual([25, 39, 41]);
  });

  it("only records trend points when a new sample window is due", () => {
    expect(shouldAppendTrendSample(null, 1_000)).toBe(true);
    expect(shouldAppendTrendSample(1_000, 1_300)).toBe(false);
    expect(shouldAppendTrendSample(1_000, 2_000)).toBe(true);
    expect(shouldAppendTrendSample(2_000, 1_500)).toBe(true);
  });

  it("uses CC/CV/CP specific reference rules", () => {
    expect(
      getTrendReferenceByMetric({
        metric: "current",
        mode: "cc",
        targetVoltageV: 12,
        minVoltageV: 0,
        targetCurrentA: 1.5,
        maxCurrentA: 10,
        targetPowerW: 20,
        maxPowerW: 150,
      }),
    ).toMatchObject({ kind: "target_i", value: 1.5 });

    expect(
      getTrendReferenceByMetric({
        metric: "voltage",
        mode: "cv",
        targetVoltageV: 12,
        minVoltageV: 0,
        targetCurrentA: 1.5,
        maxCurrentA: 10,
        targetPowerW: 20,
        maxPowerW: 150,
      }),
    ).toMatchObject({ kind: "target_v", value: 12 });

    expect(
      getTrendReferenceByMetric({
        metric: "power",
        mode: "cp",
        targetVoltageV: 12,
        minVoltageV: 0,
        targetCurrentA: 1.5,
        maxCurrentA: 10,
        targetPowerW: 20,
        maxPowerW: 150,
      }),
    ).toMatchObject({ kind: "target_p", value: 20 });
  });

  it("keeps zero-based domain with extra breathing room", () => {
    const domain = computeTrendDomain({
      metric: "current",
      points: [0.02, 0.03, 0.01],
      referenceValue: 0,
    });

    expect(domain.min).toBe(0);
    expect(domain.max).toBeGreaterThanOrEqual(0.5);
  });

  it("expands domain when data exceeds the current reference", () => {
    const domain = computeTrendDomain({
      metric: "power",
      points: [40, 50, 62],
      referenceValue: 20,
    });

    expect(domain.max).toBeGreaterThan(62);
    expect(domain.min).toBe(0);
  });

  it("builds three aligned series with reference labels", () => {
    const samples: TrendSample[] = [
      { time: 0, voltage: 11.9, current: 1.3, power: 15 },
      { time: 10, voltage: 12.0, current: 1.4, power: 16.8 },
      { time: 20, voltage: 12.1, current: 1.5, power: 18.2 },
    ];

    const series = buildTrendSeries({
      samples,
      mode: "cc",
      targetVoltageV: 12,
      minVoltageV: 0,
      targetCurrentA: 1.5,
      maxCurrentA: 10,
      targetPowerW: 20,
      maxPowerW: 150,
    });

    expect(series.current.referenceLabel).toBe("Target I");
    expect(series.voltage.referenceLabel).toBe("Min V");
    expect(series.power.referenceLabel).toBe("Max P");
    expect(series.current.times).toEqual([0, 10, 20]);
    expect(series.current.points).toEqual([1.3, 1.4, 1.5]);
  });

  it("does not shrink domains immediately when recent data calms down", () => {
    const expanded = buildTrendSeries({
      samples: [
        { time: 0, voltage: 12, current: 1.2, power: 18 },
        { time: 10, voltage: 12.2, current: 4.8, power: 60 },
        { time: 20, voltage: 12.1, current: 1.3, power: 16 },
      ],
      mode: "cc",
      targetVoltageV: 12,
      minVoltageV: 0,
      targetCurrentA: 1.5,
      maxCurrentA: 10,
      targetPowerW: 20,
      maxPowerW: 150,
    });

    const calm = buildTrendSeries({
      samples: [
        { time: 5, voltage: 12, current: 1.45, power: 17 },
        { time: 15, voltage: 12.01, current: 1.48, power: 18 },
        { time: 25, voltage: 12.02, current: 1.5, power: 18.2 },
      ],
      mode: "cc",
      targetVoltageV: 12,
      minVoltageV: 0,
      targetCurrentA: 1.5,
      maxCurrentA: 10,
      targetPowerW: 20,
      maxPowerW: 150,
    });

    const stabilized = stabilizeTrendSeriesDomains(
      calm,
      snapshotTrendSeriesDomains(expanded),
    );

    expect(stabilized.current.domainMax).toBeGreaterThanOrEqual(
      expanded.current.domainMax,
    );
    expect(stabilized.power.domainMax).toBeGreaterThanOrEqual(
      expanded.power.domainMax,
    );
  });

  it("recomputes domains when the active reference changes", () => {
    const ccSeries = buildTrendSeries({
      samples: [
        { time: 0, voltage: 11.9, current: 1.2, power: 15 },
        { time: 10, voltage: 12.0, current: 1.4, power: 16 },
      ],
      mode: "cc",
      targetVoltageV: 12,
      minVoltageV: 0,
      targetCurrentA: 1.5,
      maxCurrentA: 10,
      targetPowerW: 20,
      maxPowerW: 150,
    });

    const cvSeries = buildTrendSeries({
      samples: [
        { time: 0, voltage: 11.9, current: 1.2, power: 15 },
        { time: 10, voltage: 12.0, current: 1.4, power: 16 },
      ],
      mode: "cv",
      targetVoltageV: 12,
      minVoltageV: 0,
      targetCurrentA: 1.5,
      maxCurrentA: 10,
      targetPowerW: 20,
      maxPowerW: 150,
    });

    const stabilized = stabilizeTrendSeriesDomains(
      cvSeries,
      snapshotTrendSeriesDomains(ccSeries),
    );

    expect(stabilized.voltage.referenceKind).toBe("target_v");
    expect(stabilized.voltage.domainMax).toBe(cvSeries.voltage.domainMax);
    expect(stabilized.current.referenceKind).toBe("max_i");
    expect(stabilized.current.domainMax).toBe(cvSeries.current.domainMax);
  });
});

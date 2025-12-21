import type {
  CalibrationPointCurrent,
  CalibrationPointVoltage,
  CalibrationProfile,
} from "../api/types.ts";

export const CALIBRATION_MAX_POINTS = 24;

export interface ValidationIssue {
  path: string;
  message: string;
}

type RepresentativeKind = "mode" | "median";

function computeRepresentativeInt(values: number[]): {
  value: number;
  kind: RepresentativeKind;
} {
  if (values.length === 0) {
    return { value: 0, kind: "median" };
  }

  // Prefer a unique mode when it exists (i.e., we actually have repeated
  // samples). Otherwise fall back to the median which is robust against
  // outliers.
  const counts = new Map<number, number>();
  let maxCount = 0;
  for (const value of values) {
    const next = (counts.get(value) ?? 0) + 1;
    counts.set(value, next);
    if (next > maxCount) {
      maxCount = next;
    }
  }

  if (maxCount > 1) {
    let winner: number | null = null;
    for (const [value, count] of counts.entries()) {
      if (count !== maxCount) continue;
      if (winner != null) {
        winner = null;
        break;
      }
      winner = value;
    }
    if (winner != null) {
      return { value: winner, kind: "mode" };
    }
  }

  const sorted = values.slice().sort((a, b) => a - b);
  return { value: sorted[Math.floor((sorted.length - 1) / 2)], kind: "median" };
}

function normalizeByRaw100uv<T>(
  points: T[],
  getRaw: (point: T) => number,
): T[] {
  // Small N (<=24): stable insertion sort by raw, then drop duplicates (keep
  // last occurrence). Mirrors libs/calibration-format and firmware behavior.
  const sorted = points.slice();
  for (let i = 1; i < sorted.length; i++) {
    let j = i;
    while (j > 0 && getRaw(sorted[j - 1]) > getRaw(sorted[j])) {
      const tmp = sorted[j - 1];
      sorted[j - 1] = sorted[j];
      sorted[j] = tmp;
      j -= 1;
    }
  }

  const deduped: T[] = [];
  for (const point of sorted) {
    const last = deduped[deduped.length - 1];
    if (last && getRaw(last) === getRaw(point)) {
      deduped[deduped.length - 1] = point;
    } else {
      deduped.push(point);
    }
  }
  return deduped;
}

function addIssue(
  issues: ValidationIssue[],
  path: string,
  message: string,
): void {
  issues.push({ path, message });
}

function sanitizeVoltagePoints(
  issues: ValidationIssue[],
  kind: "v_local" | "v_remote",
  points: CalibrationPointVoltage[],
): CalibrationPointVoltage[] {
  const basePath = `${kind}_points`;
  const valid: CalibrationPointVoltage[] = [];
  for (let i = 0; i < points.length; i++) {
    const raw = points[i].raw;
    const mv = points[i].mv;
    if (!Number.isFinite(raw) || !Number.isInteger(raw)) {
      addIssue(issues, `${basePath}[${i}].raw`, "raw_100uv must be an integer");
      continue;
    }
    if (raw < -32768 || raw > 32767) {
      addIssue(
        issues,
        `${basePath}[${i}].raw`,
        "raw_100uv out of range for i16",
      );
      continue;
    }
    if (!Number.isFinite(mv) || !Number.isInteger(mv)) {
      addIssue(issues, `${basePath}[${i}].mv`, "mv must be an integer");
      continue;
    }
    valid.push({ raw, mv });
  }

  // Allow repeated samples for the same measured value (mv). We collapse them
  // at apply time using mode/median on raw_100uv, then enforce firmware
  // constraints afterwards.
  const byMv = new Map<number, number[]>();
  for (const point of valid) {
    const list = byMv.get(point.mv) ?? [];
    list.push(point.raw);
    byMv.set(point.mv, list);
  }

  const aggregatedByMv: CalibrationPointVoltage[] = [];
  for (const [mv, raws] of byMv.entries()) {
    const rep = computeRepresentativeInt(raws);
    if (raws.length > 1) {
      const raw0 = raws[0];
      const isExactDuplicate = raws.every((raw) => raw === raw0);
      if (!isExactDuplicate) {
        addIssue(
          issues,
          basePath,
          `duplicate mv=${mv} (${raws.length} samples): using ${rep.kind}`,
        );
      }
    }
    aggregatedByMv.push({ raw: rep.value, mv });
  }

  // Also handle cases where multiple measurements land on the same raw_100uv
  // (ADC quantization). Collapse by raw using mode/median on mv.
  const byRaw = new Map<number, number[]>();
  for (const point of aggregatedByMv) {
    const list = byRaw.get(point.raw) ?? [];
    list.push(point.mv);
    byRaw.set(point.raw, list);
  }

  const aggregatedByRaw: CalibrationPointVoltage[] = [];
  for (const [raw, mvs] of byRaw.entries()) {
    const rep = computeRepresentativeInt(mvs);
    if (mvs.length > 1) {
      addIssue(
        issues,
        basePath,
        `duplicate raw=${raw} (${mvs.length} samples): using ${rep.kind}`,
      );
    }
    aggregatedByRaw.push({ raw, mv: rep.value });
  }

  // Sort by raw and enforce strictly increasing meas (drop conflicts).
  const normalized = normalizeByRaw100uv(aggregatedByRaw, (p) => p.raw);
  const strictlyIncreasing: CalibrationPointVoltage[] = [];
  let droppedNonMonotonic = 0;
  for (const point of normalized) {
    const last = strictlyIncreasing[strictlyIncreasing.length - 1];
    if (last && point.mv <= last.mv) {
      droppedNonMonotonic += 1;
      continue;
    }
    strictlyIncreasing.push(point);
  }
  if (droppedNonMonotonic > 0) {
    addIssue(
      issues,
      basePath,
      `non-monotonic mv after sort: dropped ${droppedNonMonotonic} point(s)`,
    );
  }

  if (strictlyIncreasing.length === 0) {
    addIssue(
      issues,
      basePath,
      `points must contain 1..${CALIBRATION_MAX_POINTS} items`,
    );
    return [];
  }

  if (strictlyIncreasing.length > CALIBRATION_MAX_POINTS) {
    addIssue(
      issues,
      basePath,
      `too many points (max ${CALIBRATION_MAX_POINTS}); keeping first ${CALIBRATION_MAX_POINTS}`,
    );
    return strictlyIncreasing.slice(0, CALIBRATION_MAX_POINTS);
  }

  return strictlyIncreasing;
}

function sanitizeCurrentPoints(
  issues: ValidationIssue[],
  kind: "current_ch1" | "current_ch2",
  points: CalibrationPointCurrent[],
): CalibrationPointCurrent[] {
  const basePath = `${kind}_points`;
  const valid: CalibrationPointCurrent[] = [];
  for (let i = 0; i < points.length; i++) {
    const raw = points[i].raw;
    const ua = points[i].ua;
    const dac = points[i].dac_code;
    if (!Number.isFinite(raw) || !Number.isInteger(raw)) {
      addIssue(issues, `${basePath}[${i}].raw`, "raw_100uv must be an integer");
      continue;
    }
    if (raw < -32768 || raw > 32767) {
      addIssue(
        issues,
        `${basePath}[${i}].raw`,
        "raw_100uv out of range for i16",
      );
      continue;
    }
    if (!Number.isFinite(dac) || !Number.isInteger(dac)) {
      addIssue(
        issues,
        `${basePath}[${i}].dac_code`,
        "raw_dac_code must be an integer",
      );
      continue;
    }
    if (dac < 0 || dac > 65535) {
      addIssue(
        issues,
        `${basePath}[${i}].dac_code`,
        "raw_dac_code out of range for u16",
      );
      continue;
    }
    if (!Number.isFinite(ua) || !Number.isInteger(ua)) {
      addIssue(issues, `${basePath}[${i}].ua`, "ua must be an integer");
      continue;
    }
    valid.push({ raw, ua, dac_code: dac });
  }

  const byUa = new Map<number, { raws: number[]; dacs: number[] }>();
  for (const point of valid) {
    const entry = byUa.get(point.ua) ?? { raws: [], dacs: [] };
    entry.raws.push(point.raw);
    entry.dacs.push(point.dac_code);
    byUa.set(point.ua, entry);
  }

  const aggregatedByMa: CalibrationPointCurrent[] = [];
  for (const [ua, entry] of byUa.entries()) {
    const repRaw = computeRepresentativeInt(entry.raws);
    const repDac = computeRepresentativeInt(entry.dacs);
    const samples = entry.raws.length;
    if (samples > 1) {
      const raw0 = entry.raws[0];
      const dac0 = entry.dacs[0];
      const isExactDuplicate =
        entry.raws.every((raw) => raw === raw0) &&
        entry.dacs.every((dac) => dac === dac0);
      if (!isExactDuplicate) {
        addIssue(
          issues,
          basePath,
          `duplicate ua=${ua} (${samples} samples): raw uses ${repRaw.kind}, dac uses ${repDac.kind}`,
        );
      }
    }
    aggregatedByMa.push({ raw: repRaw.value, ua, dac_code: repDac.value });
  }

  const byRaw = new Map<number, { uas: number[]; dacs: number[] }>();
  for (const point of aggregatedByMa) {
    const entry = byRaw.get(point.raw) ?? { uas: [], dacs: [] };
    entry.uas.push(point.ua);
    entry.dacs.push(point.dac_code);
    byRaw.set(point.raw, entry);
  }

  const aggregatedByRaw: CalibrationPointCurrent[] = [];
  for (const [raw, entry] of byRaw.entries()) {
    const repUa = computeRepresentativeInt(entry.uas);
    const repDac = computeRepresentativeInt(entry.dacs);
    const samples = entry.uas.length;
    if (samples > 1) {
      addIssue(
        issues,
        basePath,
        `duplicate raw=${raw} (${samples} samples): ua uses ${repUa.kind}, dac uses ${repDac.kind}`,
      );
    }
    aggregatedByRaw.push({ raw, ua: repUa.value, dac_code: repDac.value });
  }

  const normalized = normalizeByRaw100uv(aggregatedByRaw, (p) => p.raw);
  const strictlyIncreasing: CalibrationPointCurrent[] = [];
  let droppedNonMonotonic = 0;
  for (const point of normalized) {
    const last = strictlyIncreasing[strictlyIncreasing.length - 1];
    if (last && point.ua <= last.ua) {
      droppedNonMonotonic += 1;
      continue;
    }
    strictlyIncreasing.push(point);
  }
  if (droppedNonMonotonic > 0) {
    addIssue(
      issues,
      basePath,
      `non-monotonic ua after sort: dropped ${droppedNonMonotonic} point(s)`,
    );
  }

  if (strictlyIncreasing.length === 0) {
    addIssue(
      issues,
      basePath,
      `points must contain 1..${CALIBRATION_MAX_POINTS} items`,
    );
    return [];
  }
  if (strictlyIncreasing.length > CALIBRATION_MAX_POINTS) {
    addIssue(
      issues,
      basePath,
      `too many points (max ${CALIBRATION_MAX_POINTS}); keeping first ${CALIBRATION_MAX_POINTS}`,
    );
    return strictlyIncreasing.slice(0, CALIBRATION_MAX_POINTS);
  }

  return strictlyIncreasing;
}

// Legacy helper retained for potential future consumers (exported to avoid
// TypeScript noUnusedLocals errors when building the web bundle).
export function _validateCommonRawAndMeas<T>(
  issues: ValidationIssue[],
  kind: string,
  points: T[],
  options: {
    measKey: string;
    getRaw: (point: T) => number;
    getMeas: (point: T) => number;
    basePath: string;
  },
): T[] {
  // Legacy helper retained for any future consumers.
  for (let i = 0; i < points.length; i++) {
    const point = points[i];
    const raw = options.getRaw(point);
    if (!Number.isFinite(raw) || !Number.isInteger(raw)) {
      addIssue(
        issues,
        `${options.basePath}[${i}].raw`,
        "raw_100uv must be an integer",
      );
    } else if (raw < -32768 || raw > 32767) {
      addIssue(
        issues,
        `${options.basePath}[${i}].raw`,
        "raw_100uv out of range for i16",
      );
    }

    const meas = options.getMeas(point);
    if (!Number.isFinite(meas) || !Number.isInteger(meas)) {
      addIssue(
        issues,
        `${options.basePath}[${i}].${options.measKey}`,
        `${options.measKey} must be an integer`,
      );
    }
  }

  const normalized = normalizeByRaw100uv(points, options.getRaw);
  if (normalized.length === 0) {
    addIssue(
      issues,
      options.basePath,
      `points must contain 1..${CALIBRATION_MAX_POINTS} items`,
    );
  } else if (normalized.length > CALIBRATION_MAX_POINTS) {
    addIssue(
      issues,
      options.basePath,
      `too many points (max ${CALIBRATION_MAX_POINTS})`,
    );
  }
  for (let i = 1; i < normalized.length; i++) {
    if (options.getMeas(normalized[i]) <= options.getMeas(normalized[i - 1])) {
      addIssue(
        issues,
        options.basePath,
        `meas must be strictly increasing for ${kind}`,
      );
      break;
    }
  }
  return normalized;
}

export function validateAndNormalizeVoltagePoints(
  kind: "v_local" | "v_remote",
  points: CalibrationPointVoltage[],
): { normalized: CalibrationPointVoltage[]; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];
  if (points.length === 0) {
    return { normalized: [], issues };
  }
  const normalized = sanitizeVoltagePoints(issues, kind, points);
  return { normalized, issues };
}

export function validateAndNormalizeCurrentPoints(
  kind: "current_ch1" | "current_ch2",
  points: CalibrationPointCurrent[],
): { normalized: CalibrationPointCurrent[]; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];
  if (points.length === 0) {
    return { normalized: [], issues };
  }
  const normalized = sanitizeCurrentPoints(issues, kind, points);

  return { normalized, issues };
}

export function validateCalibrationProfile(
  profile: CalibrationProfile,
): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  issues.push(
    ...validateAndNormalizeVoltagePoints("v_local", profile.v_local_points)
      .issues,
  );
  issues.push(
    ...validateAndNormalizeVoltagePoints("v_remote", profile.v_remote_points)
      .issues,
  );
  issues.push(
    ...validateAndNormalizeCurrentPoints(
      "current_ch1",
      profile.current_ch1_points,
    ).issues,
  );
  issues.push(
    ...validateAndNormalizeCurrentPoints(
      "current_ch2",
      profile.current_ch2_points,
    ).issues,
  );
  return issues;
}

function voltagePointsEqualNormalized(
  a: CalibrationPointVoltage[],
  b: CalibrationPointVoltage[],
): boolean {
  const na = sanitizeVoltagePoints([], "v_local", a);
  const nb = sanitizeVoltagePoints([], "v_local", b);
  if (na.length !== nb.length) return false;
  for (let i = 0; i < na.length; i++) {
    if (na[i].raw !== nb[i].raw || na[i].mv !== nb[i].mv) return false;
  }
  return true;
}

function currentPointsEqualNormalized(
  a: CalibrationPointCurrent[],
  b: CalibrationPointCurrent[],
): boolean {
  const na = sanitizeCurrentPoints([], "current_ch1", a);
  const nb = sanitizeCurrentPoints([], "current_ch1", b);
  if (na.length !== nb.length) return false;
  for (let i = 0; i < na.length; i++) {
    if (
      na[i].raw !== nb[i].raw ||
      na[i].ua !== nb[i].ua ||
      na[i].dac_code !== nb[i].dac_code
    ) {
      return false;
    }
  }
  return true;
}

export function calibrationProfilesPointsEqual(
  a: CalibrationProfile,
  b: CalibrationProfile,
): boolean {
  return (
    voltagePointsEqualNormalized(a.v_local_points, b.v_local_points) &&
    voltagePointsEqualNormalized(a.v_remote_points, b.v_remote_points) &&
    currentPointsEqualNormalized(a.current_ch1_points, b.current_ch1_points) &&
    currentPointsEqualNormalized(a.current_ch2_points, b.current_ch2_points)
  );
}

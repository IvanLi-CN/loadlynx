import type {
  CalibrationPointCurrent,
  CalibrationPointVoltage,
  CalibrationProfile,
} from "../api/types.ts";

export interface ValidationIssue {
  path: string;
  message: string;
}

function normalizeByRaw100uv<T>(
  points: T[],
  getRaw: (point: T) => number,
): T[] {
  // Small N (<=5): stable insertion sort by raw, then drop duplicates (keep
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

function validateCommonRawAndMeas<T>(
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
  if (points.length === 0) {
    addIssue(issues, options.basePath, "points must contain 1..5 items");
  }
  if (points.length > 5) {
    addIssue(issues, options.basePath, "too many points (max 5)");
  }

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
  const normalized = validateCommonRawAndMeas(issues, kind, points, {
    measKey: "mv",
    basePath: `${kind}_points`,
    getRaw: (p) => p.raw,
    getMeas: (p) => p.mv,
  });
  return { normalized, issues };
}

export function validateAndNormalizeCurrentPoints(
  kind: "current_ch1" | "current_ch2",
  points: CalibrationPointCurrent[],
): { normalized: CalibrationPointCurrent[]; issues: ValidationIssue[] } {
  const issues: ValidationIssue[] = [];

  for (let i = 0; i < points.length; i++) {
    const dac = points[i].dac_code;
    if (!Number.isFinite(dac) || !Number.isInteger(dac)) {
      addIssue(
        issues,
        `${kind}_points[${i}].dac_code`,
        "raw_dac_code must be an integer",
      );
    } else if (dac < 0 || dac > 65535) {
      addIssue(
        issues,
        `${kind}_points[${i}].dac_code`,
        "raw_dac_code out of range for u16",
      );
    }
  }

  const normalized = validateCommonRawAndMeas(issues, kind, points, {
    measKey: "ma",
    basePath: `${kind}_points`,
    getRaw: (p) => p.raw,
    getMeas: (p) => p.ma,
  });

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
    ...validateAndNormalizeCurrentPoints("current_ch1", profile.current_ch1_points)
      .issues,
  );
  issues.push(
    ...validateAndNormalizeCurrentPoints("current_ch2", profile.current_ch2_points)
      .issues,
  );
  return issues;
}

function voltagePointsEqualNormalized(
  a: CalibrationPointVoltage[],
  b: CalibrationPointVoltage[],
): boolean {
  const na = normalizeByRaw100uv(a, (p) => p.raw);
  const nb = normalizeByRaw100uv(b, (p) => p.raw);
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
  const na = normalizeByRaw100uv(a, (p) => p.raw);
  const nb = normalizeByRaw100uv(b, (p) => p.raw);
  if (na.length !== nb.length) return false;
  for (let i = 0; i < na.length; i++) {
    if (
      na[i].raw !== nb[i].raw ||
      na[i].ma !== nb[i].ma ||
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


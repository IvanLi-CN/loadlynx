import type {
  CalibrationActiveProfile,
  CalibrationPointCurrent,
  CalibrationPointVoltage,
  CalibrationProfile,
} from "../../api/types.ts";
import {
  type ValidationIssue,
  validateAndNormalizeCurrentPoints,
  validateAndNormalizeVoltagePoints,
} from "../../calibration/validation.ts";
import { isDraftEmpty } from "./shared.ts";

type CalibrationDraftImportSuccess = {
  ok: true;
  profile: CalibrationProfile;
};

type CalibrationDraftImportFailure = {
  ok: false;
  error: string;
  issues: ValidationIssue[] | null;
};

export type CalibrationDraftImportResult =
  | CalibrationDraftImportSuccess
  | CalibrationDraftImportFailure;

export function parseCalibrationDraftImport(
  text: string,
  activeFallback: CalibrationActiveProfile,
): CalibrationDraftImportResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text) as unknown;
  } catch {
    return {
      ok: false,
      error: "Invalid JSON file.",
      issues: null,
    };
  }

  const root =
    typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, unknown>)
      : null;
  const schemaVersion =
    root && typeof root.schema_version === "number"
      ? root.schema_version
      : root && typeof root.version === "number"
        ? root.version
        : null;
  const curvesCandidate =
    root && typeof root.curves === "object" && root.curves !== null
      ? (root.curves as Record<string, unknown>)
      : root && typeof root.profile === "object" && root.profile !== null
        ? (root.profile as Record<string, unknown>)
        : root;

  if (!curvesCandidate || typeof curvesCandidate !== "object") {
    return {
      ok: false,
      error: "Missing curves object in JSON.",
      issues: null,
    };
  }

  const curves = curvesCandidate as Record<string, unknown>;
  const issues: ValidationIssue[] = [];

  const readNumber = (value: unknown): number | null => {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      return null;
    }
    return value;
  };

  const readArray = (value: unknown): unknown[] | null => {
    return Array.isArray(value) ? value : null;
  };

  const parseVoltagePoint = (
    value: unknown,
    path: string,
  ): CalibrationPointVoltage | null => {
    if (Array.isArray(value) && value.length >= 2) {
      const raw = readNumber(value[0]);
      const mv = readNumber(value[1]);
      if (raw == null) {
        issues.push({
          path: `${path}[0]`,
          message: "raw must be a number",
        });
      }
      if (mv == null) {
        issues.push({ path: `${path}[1]`, message: "mv must be a number" });
      }
      return raw == null || mv == null ? null : { raw, mv };
    }
    if (typeof value !== "object" || value === null) {
      issues.push({ path, message: "point must be an object" });
      return null;
    }
    const record = value as Record<string, unknown>;
    const raw = readNumber(record.raw ?? record.raw_100uv);
    const mv = readNumber(record.mv ?? record.meas_mv);
    if (raw == null) {
      issues.push({ path: `${path}.raw`, message: "raw must be a number" });
    }
    if (mv == null) {
      issues.push({ path: `${path}.mv`, message: "mv must be a number" });
    }
    return raw == null || mv == null ? null : { raw, mv };
  };

  const parseCurrentPoint = (
    value: unknown,
    path: string,
  ): CalibrationPointCurrent | null => {
    if (Array.isArray(value)) {
      if (value.length >= 2 && Array.isArray(value[0])) {
        const raw = readNumber(value[0][0]);
        const dac = readNumber(value[0][1]);
        const stored = readNumber(value[1]);
        if (raw == null) {
          issues.push({
            path: `${path}[0][0]`,
            message: "raw must be a number",
          });
        }
        if (dac == null) {
          issues.push({
            path: `${path}[0][1]`,
            message: "dac_code must be a number",
          });
        }
        if (stored == null) {
          issues.push({
            path: `${path}[1]`,
            message: "measured current must be a number",
          });
        }
        if (raw == null || dac == null || stored == null) {
          return null;
        }
        return {
          raw,
          ua:
            schemaVersion != null && schemaVersion >= 3
              ? stored
              : stored * 1000,
          dac_code: dac,
        };
      }
      if (value.length >= 3) {
        const raw = readNumber(value[0]);
        const stored = readNumber(value[1]);
        const dac = readNumber(value[2]);
        if (raw == null) {
          issues.push({
            path: `${path}[0]`,
            message: "raw must be a number",
          });
        }
        if (stored == null) {
          issues.push({
            path: `${path}[1]`,
            message: "measured current must be a number",
          });
        }
        if (dac == null) {
          issues.push({
            path: `${path}[2]`,
            message: "dac_code must be a number",
          });
        }
        if (raw == null || stored == null || dac == null) {
          return null;
        }
        return {
          raw,
          ua:
            schemaVersion != null && schemaVersion >= 3
              ? stored
              : stored * 1000,
          dac_code: dac,
        };
      }
    }
    if (typeof value !== "object" || value === null) {
      issues.push({ path, message: "point must be an object" });
      return null;
    }
    const record = value as Record<string, unknown>;
    const raw = readNumber(record.raw ?? record.raw_100uv);
    const ua = readNumber(record.ua ?? record.meas_ua);
    const ma = readNumber(record.ma ?? record.meas_ma);
    const dac = readNumber(record.dac_code ?? record.raw_dac_code);
    if (raw == null) {
      issues.push({ path: `${path}.raw`, message: "raw must be a number" });
    }
    if (ua == null && ma == null) {
      issues.push({
        path: `${path}.ua`,
        message: "measured current must be a number",
      });
    }
    if (dac == null) {
      issues.push({
        path: `${path}.dac_code`,
        message: "dac_code must be a number",
      });
    }
    if (raw == null || dac == null) {
      return null;
    }
    if (ua != null) {
      return { raw, ua, dac_code: dac };
    }
    if (ma != null) {
      return { raw, ua: ma * 1000, dac_code: dac };
    }
    return null;
  };

  const parseVoltagePoints = (
    value: unknown,
    path: string,
  ): CalibrationPointVoltage[] => {
    const array = readArray(value);
    if (!array) {
      issues.push({ path, message: "must be an array" });
      return [];
    }
    return array.flatMap((entry, index) => {
      const point = parseVoltagePoint(entry, `${path}[${index}]`);
      return point ? [point] : [];
    });
  };

  const parseCurrentPoints = (
    value: unknown,
    path: string,
  ): CalibrationPointCurrent[] => {
    const array = readArray(value);
    if (!array) {
      issues.push({ path, message: "must be an array" });
      return [];
    }
    return array.flatMap((entry, index) => {
      const point = parseCurrentPoint(entry, `${path}[${index}]`);
      return point ? [point] : [];
    });
  };

  const parsedProfile: CalibrationProfile = {
    active: activeFallback,
    v_local_points: parseVoltagePoints(curves.v_local_points, "v_local_points"),
    v_remote_points: parseVoltagePoints(
      curves.v_remote_points,
      "v_remote_points",
    ),
    current_ch1_points: parseCurrentPoints(
      curves.current_ch1_points,
      "current_ch1_points",
    ),
    current_ch2_points: parseCurrentPoints(
      curves.current_ch2_points,
      "current_ch2_points",
    ),
  };

  if (issues.length > 0) {
    return {
      ok: false,
      error: "Import validation failed (shape/types).",
      issues,
    };
  }

  const normalizedLocal = validateAndNormalizeVoltagePoints(
    "v_local",
    parsedProfile.v_local_points,
  );
  const normalizedRemote = validateAndNormalizeVoltagePoints(
    "v_remote",
    parsedProfile.v_remote_points,
  );
  const normalizedCh1 = validateAndNormalizeCurrentPoints(
    "current_ch1",
    parsedProfile.current_ch1_points,
  );
  const normalizedCh2 = validateAndNormalizeCurrentPoints(
    "current_ch2",
    parsedProfile.current_ch2_points,
  );
  const normalizedProfile: CalibrationProfile = {
    active: activeFallback,
    v_local_points: normalizedLocal.normalized,
    v_remote_points: normalizedRemote.normalized,
    current_ch1_points: normalizedCh1.normalized,
    current_ch2_points: normalizedCh2.normalized,
  };
  const validationIssues = [
    ...normalizedLocal.issues,
    ...normalizedRemote.issues,
    ...normalizedCh1.issues,
    ...normalizedCh2.issues,
  ];

  if (validationIssues.length > 0) {
    return {
      ok: false,
      error: "Import validation failed.",
      issues: validationIssues,
    };
  }

  if (isDraftEmpty(normalizedProfile)) {
    return {
      ok: false,
      error: "Empty drafts are not supported for import.",
      issues: null,
    };
  }

  return {
    ok: true,
    profile: normalizedProfile,
  };
}

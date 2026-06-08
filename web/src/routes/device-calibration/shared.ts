import type { QueryObserverResult } from "@tanstack/react-query";
import Decimal from "decimal.js";
import { type HttpApiError, isHttpApiError } from "../../api/client.ts";
import type {
  CalibrationActiveProfile,
  CalibrationPointCurrent,
  CalibrationPointVoltage,
  CalibrationProfile,
  FastStatusView,
} from "../../api/types.ts";

export type RefetchProfile = () => Promise<
  QueryObserverResult<CalibrationProfile, HttpApiError>
>;

export type CalibrationTab = "voltage" | "current_ch1" | "current_ch2";
export type WithStatusStreamPaused = <T>(op: () => Promise<T>) => Promise<T>;
export type CurrentInputUnit = "A" | "mA";
export type VoltageInputUnit = "V";
type VoltagePair = [raw: number, mv: number];
type CurrentPair = [[raw: number, dac_code: number], value: number];

export type UndoAction =
  | {
      kind: "voltage_points";
      local?: CalibrationPointVoltage;
      remote?: CalibrationPointVoltage;
    }
  | {
      kind: "current_point";
      curve: "current_ch1" | "current_ch2";
      point: CalibrationPointCurrent;
    };

export const DEFAULT_ACTIVE_PROFILE: CalibrationActiveProfile = {
  source: "factory-default",
  fmt_version: 1,
  hw_rev: 1,
};

const CALIBRATION_DRAFT_STORAGE_VERSION = 4;
const CALIBRATION_CURRENT_OPTIONS_STORAGE_VERSION = 2;
const CALIBRATION_VOLTAGE_OPTIONS_STORAGE_VERSION = 2;

interface StoredCalibrationDraftV2 {
  version: 2;
  saved_at: string;
  device_id: string;
  base_url: string;
  active_tab: "voltage" | "current";
  draft_profile: {
    v_local_points: VoltagePair[];
    v_remote_points: VoltagePair[];
    current_ch1_points: CurrentPair[];
    current_ch2_points: CurrentPair[];
  };
}

interface StoredCalibrationDraftV3 {
  version: 3;
  saved_at: string;
  device_id: string;
  base_url: string;
  active_tab: CalibrationTab;
  draft_profile: {
    v_local_points: VoltagePair[];
    v_remote_points: VoltagePair[];
    current_ch1_points: CurrentPair[];
    current_ch2_points: CurrentPair[];
  };
}

export interface StoredCalibrationDraftV4 {
  version: 4;
  saved_at: string;
  device_id: string;
  base_url: string;
  active_tab: CalibrationTab;
  draft_profile: {
    v_local_points: VoltagePair[];
    v_remote_points: VoltagePair[];
    current_ch1_points: CurrentPair[];
    current_ch2_points: CurrentPair[];
  };
}

export interface ParsedCalibrationDraft {
  version: 4;
  saved_at: string;
  device_id: string;
  base_url: string;
  active_tab: CalibrationTab;
  draft_profile: {
    v_local_points: CalibrationPointVoltage[];
    v_remote_points: CalibrationPointVoltage[];
    current_ch1_points: CalibrationPointCurrent[];
    current_ch2_points: CalibrationPointCurrent[];
  };
}

export interface ParsedCalibrationCurrentOptions {
  baselineUa: number | null;
  unit: CurrentInputUnit | null;
}

export interface ParsedCalibrationVoltageOptions {
  inputUv: number | null;
  unit: VoltageInputUnit | null;
}

export function getCalibrationDraftStorageKey(
  deviceId: string,
  baseUrl: string,
  version = CALIBRATION_DRAFT_STORAGE_VERSION,
): string {
  const encodedBase = encodeURIComponent(baseUrl);
  return `loadlynx:calibration-draft:v${version}:${deviceId}:${encodedBase}`;
}

export function getCalibrationCurrentOptionsStorageKey(
  deviceId: string,
  baseUrl: string,
  curve: "current_ch1" | "current_ch2",
  version = CALIBRATION_CURRENT_OPTIONS_STORAGE_VERSION,
): string {
  const encodedBase = encodeURIComponent(baseUrl);
  return `loadlynx:calibration-current-options:v${version}:${deviceId}:${encodedBase}:${curve}`;
}

export function getCalibrationVoltageOptionsStorageKey(
  deviceId: string,
  baseUrl: string,
  version = CALIBRATION_VOLTAGE_OPTIONS_STORAGE_VERSION,
): string {
  const encodedBase = encodeURIComponent(baseUrl);
  return `loadlynx:calibration-voltage-options:v${version}:${deviceId}:${encodedBase}`;
}

export function readCalibrationCurrentOptionsFromStorage(
  storage: Pick<Storage, "getItem">,
  deviceId: string,
  baseUrl: string,
  curve: "current_ch1" | "current_ch2",
): ParsedCalibrationCurrentOptions {
  const readOptionsV2 = (): ParsedCalibrationCurrentOptions => {
    const key = getCalibrationCurrentOptionsStorageKey(
      deviceId,
      baseUrl,
      curve,
    );
    const raw = storage.getItem(key);
    if (!raw) {
      return { baselineUa: null, unit: null };
    }
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed !== "object" || parsed === null) {
      return { baselineUa: null, unit: null };
    }
    const obj = parsed as Record<string, unknown>;
    const baselineRaw = obj.baseline_ua;
    const unitRaw = obj.unit;
    const baselineUa =
      typeof baselineRaw === "number" &&
      Number.isFinite(baselineRaw) &&
      Number.isInteger(baselineRaw) &&
      baselineRaw >= 0
        ? baselineRaw
        : null;
    const unit =
      unitRaw === "A" || unitRaw === "mA"
        ? (unitRaw as CurrentInputUnit)
        : null;
    return { baselineUa, unit };
  };

  const readBaselineV1 = (): number | null => {
    const key = getCalibrationCurrentOptionsStorageKey(
      deviceId,
      baseUrl,
      curve,
      1,
    );
    const raw = storage.getItem(key);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed !== "object" || parsed === null) {
      return null;
    }
    const obj = parsed as Record<string, unknown>;
    return typeof obj.baseline_a === "string"
      ? parseNonNegativeDecimalToScaledInt(obj.baseline_a, 6)
      : null;
  };

  try {
    const current = readOptionsV2();
    return {
      baselineUa: current.baselineUa ?? readBaselineV1(),
      unit: current.unit,
    };
  } catch {
    return { baselineUa: null, unit: null };
  }
}

export function writeCalibrationCurrentOptionsToStorage(
  storage: Pick<Storage, "setItem" | "removeItem">,
  deviceId: string,
  baseUrl: string,
  curve: "current_ch1" | "current_ch2",
  options: { baselineUa: number; unit: CurrentInputUnit },
): void {
  const key = getCalibrationCurrentOptionsStorageKey(deviceId, baseUrl, curve);
  const keyV1 = getCalibrationCurrentOptionsStorageKey(
    deviceId,
    baseUrl,
    curve,
    1,
  );
  storage.setItem(
    key,
    JSON.stringify({
      baseline_ua: options.baselineUa,
      unit: options.unit,
    }),
  );
  storage.removeItem(keyV1);
}

export function readCalibrationVoltageOptionsFromStorage(
  storage: Pick<Storage, "getItem">,
  deviceId: string,
  baseUrl: string,
): ParsedCalibrationVoltageOptions {
  const readOptionsV2 = (): ParsedCalibrationVoltageOptions => {
    const raw = storage.getItem(
      getCalibrationVoltageOptionsStorageKey(deviceId, baseUrl),
    );
    if (!raw) {
      return { inputUv: null, unit: null };
    }
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed !== "object" || parsed === null) {
      return { inputUv: null, unit: null };
    }
    const obj = parsed as Record<string, unknown>;
    const inputUvRaw = obj.input_uv;
    const unitRaw = obj.unit;
    const inputUv =
      typeof inputUvRaw === "number" &&
      Number.isFinite(inputUvRaw) &&
      Number.isInteger(inputUvRaw) &&
      inputUvRaw >= 0
        ? inputUvRaw
        : null;
    const unit = unitRaw === "V" ? "V" : null;
    return { inputUv, unit };
  };

  const readInputV1 = (): number | null => {
    const raw = storage.getItem(
      getCalibrationVoltageOptionsStorageKey(deviceId, baseUrl, 1),
    );
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed !== "object" || parsed === null) {
      return null;
    }
    const obj = parsed as Record<string, unknown>;
    return typeof obj.input_v === "string"
      ? parseVoltageInputToUv(obj.input_v, "V")
      : null;
  };

  try {
    const current = readOptionsV2();
    return {
      inputUv: current.inputUv ?? readInputV1(),
      unit: current.unit ?? "V",
    };
  } catch {
    return { inputUv: null, unit: null };
  }
}

export function writeCalibrationVoltageOptionsToStorage(
  storage: Pick<Storage, "setItem" | "removeItem">,
  deviceId: string,
  baseUrl: string,
  options: { inputUv: number; unit: VoltageInputUnit },
): void {
  const key = getCalibrationVoltageOptionsStorageKey(deviceId, baseUrl);
  const keyV1 = getCalibrationVoltageOptionsStorageKey(deviceId, baseUrl, 1);
  storage.setItem(
    key,
    JSON.stringify({
      input_uv: options.inputUv,
      unit: options.unit,
    }),
  );
  storage.removeItem(keyV1);
}

export function readCalibrationDraftFromStorage(
  storage: Pick<Storage, "getItem">,
  deviceId: string,
  baseUrl: string,
): ParsedCalibrationDraft | null {
  try {
    const tryRead = (version: 2 | 3 | 4): unknown | null => {
      const key = getCalibrationDraftStorageKey(deviceId, baseUrl, version);
      const raw = storage.getItem(key);
      if (!raw) return null;
      return JSON.parse(raw) as unknown;
    };

    const parsedV4 = tryRead(4);
    const parsedV3 = parsedV4 ? null : tryRead(3);
    const parsedV2 = parsedV4 || parsedV3 ? null : tryRead(2);
    const parsed = parsedV4 ?? parsedV3 ?? parsedV2;
    if (!parsed || typeof parsed !== "object") return null;

    const obj = parsed as
      | StoredCalibrationDraftV2
      | StoredCalibrationDraftV3
      | StoredCalibrationDraftV4;
    const objRecord = parsed as Record<string, unknown>;
    if (obj.version !== 2 && obj.version !== 3 && obj.version !== 4) {
      return null;
    }
    if (obj.device_id !== deviceId || obj.base_url !== baseUrl) {
      return null;
    }

    const storedTab = objRecord.active_tab;
    const active_tab: CalibrationTab | null =
      storedTab === "voltage"
        ? "voltage"
        : storedTab === "current_ch1"
          ? "current_ch1"
          : storedTab === "current_ch2"
            ? "current_ch2"
            : storedTab === "current"
              ? "current_ch1"
              : null;
    if (!active_tab) return null;

    const readFiniteNumber = (value: unknown): number | null => {
      if (typeof value !== "number" || !Number.isFinite(value)) return null;
      return value;
    };

    const parseVoltagePoints = (value: unknown): CalibrationPointVoltage[] => {
      if (!Array.isArray(value)) return [];
      const out: CalibrationPointVoltage[] = [];
      for (const entry of value) {
        if (Array.isArray(entry) && entry.length >= 2) {
          const raw = readFiniteNumber(entry[0]);
          const mv = readFiniteNumber(entry[1]);
          if (raw != null && mv != null) {
            out.push({ raw, mv });
          }
          continue;
        }
        if (typeof entry !== "object" || entry === null) continue;
        const record = entry as Record<string, unknown>;
        const raw = readFiniteNumber(record.raw ?? record.raw_100uv);
        const mv = readFiniteNumber(record.mv ?? record.meas_mv);
        if (raw != null && mv != null) {
          out.push({ raw, mv });
        }
      }
      return out;
    };

    const parseCurrentPoints = (
      value: unknown,
      storedVersion: 2 | 3 | 4,
    ): CalibrationPointCurrent[] => {
      if (!Array.isArray(value)) return [];
      const out: CalibrationPointCurrent[] = [];
      for (const entry of value) {
        if (
          Array.isArray(entry) &&
          entry.length >= 2 &&
          Array.isArray(entry[0])
        ) {
          const raw = readFiniteNumber(entry[0][0]);
          const dac_code = readFiniteNumber(entry[0][1]);
          const stored = readFiniteNumber(entry[1]);
          if (raw != null && dac_code != null && stored != null) {
            out.push({
              raw,
              ua: storedVersion >= 4 ? stored : stored * 1000,
              dac_code,
            });
          }
          continue;
        }
        if (typeof entry !== "object" || entry === null) continue;
        const record = entry as Record<string, unknown>;
        const raw = readFiniteNumber(record.raw ?? record.raw_100uv);
        const ua = readFiniteNumber(record.ua ?? record.meas_ua);
        const ma = readFiniteNumber(record.ma ?? record.meas_ma);
        const dac_code = readFiniteNumber(
          record.dac_code ?? record.raw_dac_code,
        );
        if (raw == null || dac_code == null) continue;
        if (ua != null) {
          out.push({ raw, ua, dac_code });
        } else if (ma != null) {
          out.push({ raw, ua: ma * 1000, dac_code });
        }
      }
      return out;
    };

    const profileCandidate =
      typeof objRecord.draft_profile === "object" &&
      objRecord.draft_profile !== null
        ? (objRecord.draft_profile as Record<string, unknown>)
        : null;

    return {
      version: 4,
      saved_at:
        typeof objRecord.saved_at === "string" ? objRecord.saved_at : "",
      device_id: deviceId,
      base_url: baseUrl,
      active_tab,
      draft_profile: {
        v_local_points: parseVoltagePoints(profileCandidate?.v_local_points),
        v_remote_points: parseVoltagePoints(profileCandidate?.v_remote_points),
        current_ch1_points: parseCurrentPoints(
          profileCandidate?.current_ch1_points,
          obj.version,
        ),
        current_ch2_points: parseCurrentPoints(
          profileCandidate?.current_ch2_points,
          obj.version,
        ),
      },
    };
  } catch {
    return null;
  }
}

export function writeCalibrationDraftToStorage(
  storage: Pick<Storage, "setItem" | "removeItem">,
  deviceId: string,
  baseUrl: string,
  draft: StoredCalibrationDraftV4 | null,
): void {
  try {
    const keyV2 = getCalibrationDraftStorageKey(deviceId, baseUrl, 2);
    const keyV3 = getCalibrationDraftStorageKey(deviceId, baseUrl, 3);
    const keyV4 = getCalibrationDraftStorageKey(deviceId, baseUrl, 4);
    if (!draft) {
      storage.removeItem(keyV2);
      storage.removeItem(keyV3);
      storage.removeItem(keyV4);
      return;
    }
    storage.removeItem(keyV2);
    storage.removeItem(keyV3);
    storage.setItem(keyV4, JSON.stringify(draft));
  } catch {
    // best-effort
  }
}

export function parseNonNegativeDecimalToScaledInt(
  input: string,
  decimals: number,
): number | null {
  const trimmed = input.trim();
  if (trimmed === "") return null;
  if (!/^\d+(?:\.\d*)?$/.test(trimmed)) return null;

  try {
    const value = new Decimal(trimmed);
    if (!value.isFinite() || value.isNeg()) return null;

    const scale = new Decimal(10).pow(decimals);
    const scaled = value.mul(scale).toDecimalPlaces(0, Decimal.ROUND_HALF_UP);
    if (!scaled.isFinite() || !scaled.isInteger()) return null;
    if (scaled.gt(Number.MAX_SAFE_INTEGER)) return null;

    return scaled.toNumber();
  } catch {
    return null;
  }
}

export function currentUnitDecimals(unit: CurrentInputUnit): number {
  return unit === "A" ? 6 : 3;
}

export function voltageUnitDecimals(_unit: VoltageInputUnit): number {
  return 6;
}

export function parseCurrentInputToUa(
  input: string,
  unit: CurrentInputUnit,
): number | null {
  return parseNonNegativeDecimalToScaledInt(input, currentUnitDecimals(unit));
}

export function parseVoltageInputToUv(
  input: string,
  unit: VoltageInputUnit,
): number | null {
  return parseNonNegativeDecimalToScaledInt(input, voltageUnitDecimals(unit));
}

export function parseVoltageInputToMv(
  input: string,
  unit: VoltageInputUnit,
): number | null {
  const uv = parseVoltageInputToUv(input, unit);
  if (uv == null) return null;
  return Math.floor((uv + 500) / 1000);
}

export function formatIntAsFixedDecimal(
  numerator: number,
  denominator: number,
  decimals: number,
): string {
  return new Decimal(numerator).div(denominator).toFixed(decimals);
}

export function formatMvAsV(mv: number, decimals = 3): string {
  return formatIntAsFixedDecimal(mv, 1000, decimals);
}

export function formatMaAsA(ma: number, decimals = 4): string {
  return formatIntAsFixedDecimal(ma, 1000, decimals);
}

export function formatUaAsA(ua: number | Decimal, decimals = 6): string {
  return new Decimal(ua).div(1_000_000).toFixed(decimals);
}

export function formatUaToUnit(ua: number, unit: CurrentInputUnit): string {
  const decimals = currentUnitDecimals(unit);
  const scale = 10 ** decimals;
  const abs = Math.max(0, Math.trunc(ua));
  const intPart = Math.floor(abs / scale);
  const fracPart = abs % scale;
  return `${intPart}.${fracPart.toString().padStart(decimals, "0")}`;
}

export function formatUvToUnit(uv: number, unit: VoltageInputUnit): string {
  const decimals = voltageUnitDecimals(unit);
  const scale = 10 ** decimals;
  const abs = Math.max(0, Math.trunc(uv));
  const intPart = Math.floor(abs / scale);
  const fracPart = abs % scale;
  return `${intPart}.${fracPart.toString().padStart(decimals, "0")}`;
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function retryDeviceCall<T>(
  op: () => Promise<T>,
  opts?: { attempts?: number; firstDelayMs?: number; maxDelayMs?: number },
): Promise<T> {
  const attempts = Math.max(1, opts?.attempts ?? 6);
  let delayMs = Math.max(0, opts?.firstDelayMs ?? 150);
  const maxDelayMs = Math.max(delayMs, opts?.maxDelayMs ?? 1200);

  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      return await op();
    } catch (error) {
      const retryable =
        isHttpApiError(error) &&
        (error.retryable ?? (error.status === 0 || error.status >= 500));
      if (!retryable || attempt === attempts) {
        throw error;
      }
      await sleep(delayMs);
      delayMs = Math.min(maxDelayMs, delayMs * 2);
    }
  }

  throw new Error("unreachable");
}

export function mergeVoltageCandidatesByMv(
  localPoints: CalibrationPointVoltage[],
  remotePoints: CalibrationPointVoltage[],
): Array<{ mv: number; rawLocal?: number; rawRemote?: number }> {
  const byMv = new Map<
    number,
    { mv: number; rawLocal?: number; rawRemote?: number }
  >();

  for (const point of localPoints) {
    const entry = byMv.get(point.mv) ?? { mv: point.mv };
    entry.rawLocal = point.raw;
    byMv.set(point.mv, entry);
  }

  for (const point of remotePoints) {
    const entry = byMv.get(point.mv) ?? { mv: point.mv };
    entry.rawRemote = point.raw;
    byMv.set(point.mv, entry);
  }

  return Array.from(byMv.values()).sort((a, b) => a.mv - b.mv);
}

export function mergeVoltageCandidatesByIndex(
  localPoints: CalibrationPointVoltage[],
  remotePoints: CalibrationPointVoltage[],
): Array<{
  index: number;
  mv: number | null;
  mvLocal?: number;
  mvRemote?: number;
  rawLocal?: number;
  rawRemote?: number;
}> {
  const len = Math.max(localPoints.length, remotePoints.length);
  const out: Array<{
    index: number;
    mv: number | null;
    mvLocal?: number;
    mvRemote?: number;
    rawLocal?: number;
    rawRemote?: number;
  }> = [];

  for (let index = 0; index < len; index++) {
    const local = localPoints[index];
    const remote = remotePoints[index];
    out.push({
      index,
      mv: local?.mv ?? remote?.mv ?? null,
      mvLocal: local?.mv,
      mvRemote: remote?.mv,
      rawLocal: local?.raw,
      rawRemote: remote?.raw,
    });
  }

  return out;
}

export function formatLocalTimestamp(ms: number): string {
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return String(ms);
  }
}

export function makeEmptyDraftProfile(
  active?: CalibrationActiveProfile,
): CalibrationProfile {
  return {
    active: active ?? DEFAULT_ACTIVE_PROFILE,
    v_local_points: [],
    v_remote_points: [],
    current_ch1_points: [],
    current_ch2_points: [],
  };
}

export function isDraftEmpty(profile: CalibrationProfile): boolean {
  return (
    profile.v_local_points.length === 0 &&
    profile.v_remote_points.length === 0 &&
    profile.current_ch1_points.length === 0 &&
    profile.current_ch2_points.length === 0
  );
}

export function formatDeviceCalKind(kind: number | null | undefined): string {
  if (kind == null) return "off";
  switch (kind) {
    case 1:
      return "voltage";
    case 2:
      return "current_ch1";
    case 3:
      return "current_ch2";
    default:
      return `unknown(${kind})`;
  }
}

export function expectedCalKindForTab(tab: CalibrationTab): number {
  switch (tab) {
    case "voltage":
      return 1;
    case "current_ch1":
      return 2;
    case "current_ch2":
      return 3;
  }
}

export function isDeviceSubroutePath(
  pathname: string,
  deviceId: string,
): boolean {
  const prefix = `/${deviceId}`;
  return pathname === prefix || pathname.startsWith(`${prefix}/`);
}

export function statusInExpectedCalMode(
  status: FastStatusView | null,
  expectedCalKind: number,
): boolean {
  return (status?.raw.cal_kind ?? null) === expectedCalKind;
}

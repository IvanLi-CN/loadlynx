import type { QueryObserverResult } from "@tanstack/react-query";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import {
  getCalibrationProfile,
  postCalibrationApply,
  postCalibrationCommit,
  postCalibrationMode,
  postCalibrationReset,
  subscribeStatusStream,
  updateCc,
} from "../api/client.ts";
import type {
  CalibrationActiveProfile,
  CalibrationPointCurrent,
  CalibrationPointVoltage,
  CalibrationProfile,
  FastStatusView,
} from "../api/types.ts";
import { piecewiseLinear } from "../calibration/piecewise.ts";
import {
  CALIBRATION_MAX_POINTS,
  calibrationProfilesPointsEqual,
  type ValidationIssue,
  validateAndNormalizeCurrentPoints,
  validateAndNormalizeVoltagePoints,
} from "../calibration/validation.ts";
import { useDevicesQuery } from "../devices/hooks.ts";

type RefetchProfile = () => Promise<
  QueryObserverResult<CalibrationProfile, HttpApiError>
>;

type VoltagePair = [raw: number, mv: number];
type CurrentPair = [[raw: number, dac_code: number], ma: number];

type CalibrationTab = "voltage" | "current_ch1" | "current_ch2";

interface StoredCalibrationDraftV2 {
  version: 2;
  saved_at: string;
  device_id: string;
  base_url: string;
  // Legacy two-tab UI ("current" mapped to "current_ch1" during migration).
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

interface ParsedCalibrationDraftV3 {
  version: 3;
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

const CALIBRATION_DRAFT_STORAGE_VERSION = 3;

function getCalibrationDraftStorageKey(
  deviceId: string,
  baseUrl: string,
  version = CALIBRATION_DRAFT_STORAGE_VERSION,
): string {
  const encodedBase = encodeURIComponent(baseUrl);
  return `loadlynx:calibration-draft:v${version}:${deviceId}:${encodedBase}`;
}

function readCalibrationDraftFromStorage(
  deviceId: string,
  baseUrl: string,
): ParsedCalibrationDraftV3 | null {
  if (typeof window === "undefined") return null;
  try {
    const tryRead = (version: 2 | 3): unknown | null => {
      const key = getCalibrationDraftStorageKey(deviceId, baseUrl, version);
      const raw = window.localStorage.getItem(key);
      if (!raw) return null;
      return JSON.parse(raw) as unknown;
    };

    const parsedV3 = tryRead(3);
    const parsedV2 = parsedV3 ? null : tryRead(2);
    const parsed = parsedV3 ?? parsedV2;
    if (!parsed) return null;

    if (typeof parsed !== "object" || parsed === null) return null;
    const obj = parsed as StoredCalibrationDraftV2 | StoredCalibrationDraftV3;
    if (obj.version !== 2 && obj.version !== 3) return null;
    if (obj.device_id !== deviceId) return null;
    if (obj.base_url !== baseUrl) return null;
    const storedTab = (obj as unknown as Record<string, unknown>).active_tab;
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
          if (mv == null || raw == null) continue;
          out.push({ raw, mv });
          continue;
        }
        if (typeof entry !== "object" || entry === null) continue;
        const e = entry as Record<string, unknown>;
        const raw = readFiniteNumber(e.raw ?? e.raw_100uv);
        const mv = readFiniteNumber(e.mv ?? e.meas_mv);
        if (mv == null || raw == null) continue;
        out.push({ raw, mv });
      }
      return out;
    };

    const parseCurrentPoints = (value: unknown): CalibrationPointCurrent[] => {
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
          const ma = readFiniteNumber(entry[1]);
          if (raw == null || ma == null || dac_code == null) continue;
          out.push({ raw, ma, dac_code });
          continue;
        }
        if (typeof entry !== "object" || entry === null) continue;
        const e = entry as Record<string, unknown>;
        const raw = readFiniteNumber(e.raw ?? e.raw_100uv);
        const ma = readFiniteNumber(e.ma ?? e.meas_ma);
        const dac_code = readFiniteNumber(e.dac_code ?? e.raw_dac_code);
        if (raw == null || ma == null || dac_code == null) continue;
        out.push({ raw, ma, dac_code });
      }
      return out;
    };

    const profileCandidate =
      typeof (obj as unknown as Record<string, unknown>).draft_profile ===
        "object" &&
      (obj as unknown as Record<string, unknown>).draft_profile !== null
        ? ((obj as unknown as Record<string, unknown>).draft_profile as Record<
            string,
            unknown
          >)
        : null;

    return {
      version: 3,
      saved_at:
        typeof (obj as unknown as Record<string, unknown>).saved_at === "string"
          ? ((obj as unknown as Record<string, unknown>).saved_at as string)
          : "",
      device_id: deviceId,
      base_url: baseUrl,
      active_tab,
      draft_profile: {
        v_local_points: parseVoltagePoints(profileCandidate?.v_local_points),
        v_remote_points: parseVoltagePoints(profileCandidate?.v_remote_points),
        current_ch1_points: parseCurrentPoints(
          profileCandidate?.current_ch1_points,
        ),
        current_ch2_points: parseCurrentPoints(
          profileCandidate?.current_ch2_points,
        ),
      },
    };
  } catch {
    return null;
  }
}

function writeCalibrationDraftToStorage(
  deviceId: string,
  baseUrl: string,
  draft: StoredCalibrationDraftV3 | null,
): void {
  if (typeof window === "undefined") return;
  try {
    const keyV2 = getCalibrationDraftStorageKey(deviceId, baseUrl, 2);
    const keyV3 = getCalibrationDraftStorageKey(deviceId, baseUrl, 3);
    if (!draft) {
      window.localStorage.removeItem(keyV2);
      window.localStorage.removeItem(keyV3);
      return;
    }
    // Migrate to v3 and drop the legacy key.
    window.localStorage.removeItem(keyV2);
    window.localStorage.setItem(keyV3, JSON.stringify(draft));
  } catch {
    // best-effort (quota exceeded, storage disabled, etc.)
  }
}

type UndoAction =
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

interface UndoToastEntry {
  id: string;
  message: string;
  action: UndoAction;
  expiresAt: number;
  timeoutId: number;
}

interface InfoToastEntry {
  id: string;
  message: string;
  timeoutId: number;
}

function makeUndoId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `undo-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function mergeVoltageCandidatesByMv(
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

function formatLocalTimestamp(ms: number): string {
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return String(ms);
  }
}

function downloadJson(filename: string, data: unknown): void {
  const blob = new Blob([JSON.stringify(data, null, 2)], {
    type: "application/json; charset=utf-8",
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 250);
}

const DEFAULT_ACTIVE_PROFILE: CalibrationActiveProfile = {
  source: "factory-default",
  fmt_version: 1,
  hw_rev: 1,
};

function makeEmptyDraftProfile(
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

function isDraftEmpty(profile: CalibrationProfile): boolean {
  return (
    profile.v_local_points.length === 0 &&
    profile.v_remote_points.length === 0 &&
    profile.current_ch1_points.length === 0 &&
    profile.current_ch2_points.length === 0
  );
}

export function DeviceCalibrationRoute() {
  const { deviceId } = useParams({
    from: "/$deviceId/calibration",
  }) as {
    deviceId: string;
  };

  const devicesQuery = useDevicesQuery();
  const device = useMemo(
    () => devicesQuery.data?.find((entry) => entry.id === deviceId),
    [devicesQuery.data, deviceId],
  );

  if (devicesQuery.isLoading) {
    return (
      <div className="max-w-5xl mx-auto p-8 text-center text-base-content/60">
        Loading device...
      </div>
    );
  }

  if (!device) {
    return (
      <div className="max-w-5xl mx-auto space-y-4">
        <h2 className="text-2xl font-bold">Calibration</h2>
        <div role="alert" className="alert alert-error text-sm py-2">
          <span>
            Device not found. Please add the device first in{" "}
            <code className="code">Devices</code>.
          </span>
        </div>
        <Link to="/devices" className="btn btn-sm btn-outline">
          Back to devices
        </Link>
      </div>
    );
  }

  return <DeviceCalibrationPage deviceId={deviceId} baseUrl={device.baseUrl} />;
}

function DeviceCalibrationPage({
  deviceId,
  baseUrl,
}: {
  deviceId: string;
  baseUrl: string;
}) {
  const [activeTab, setActiveTab] = useState<CalibrationTab>("voltage");

  // Live status stream (includes optional RAW fields in calibration mode).
  const [status, setStatus] = useState<FastStatusView | null>(null);

  useEffect(() => {
    // Reset state while switching devices/URLs.
    setStatus(null);

    const unsubscribe = subscribeStatusStream(
      baseUrl,
      (view) => setStatus(view),
      () => setStatus(null),
    );

    return () => unsubscribe();
  }, [baseUrl]);

  const isOffline =
    status === null ||
    status.analog_state === "offline" ||
    status.analog_state === "faulted";

  const profileQuery = useQuery<CalibrationProfile, HttpApiError>({
    queryKey: ["device", deviceId, "calibration", "profile"],
    queryFn: () => getCalibrationProfile(baseUrl),
    enabled: Boolean(baseUrl),
    staleTime: Number.POSITIVE_INFINITY,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });

  const [draftProfile, setDraftProfile] = useState<CalibrationProfile>(() =>
    makeEmptyDraftProfile(),
  );
  const [previewProfile, setPreviewProfile] =
    useState<CalibrationProfile | null>(null);
  const [previewAppliedAt, setPreviewAppliedAt] = useState<number | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const [importIssues, setImportIssues] = useState<ValidationIssue[] | null>(
    null,
  );
  const [undoToasts, setUndoToasts] = useState<UndoToastEntry[]>([]);
  const [infoToasts, setInfoToasts] = useState<InfoToastEntry[]>([]);
  const [undoNow, setUndoNow] = useState(() => Date.now());
  const [draftStorageReady, setDraftStorageReady] = useState(false);

  const clearToasts = useCallback(() => {
    setUndoToasts((prev) => {
      for (const toast of prev) {
        window.clearTimeout(toast.timeoutId);
      }
      return [];
    });
    setInfoToasts((prev) => {
      for (const toast of prev) {
        window.clearTimeout(toast.timeoutId);
      }
      return [];
    });
  }, []);

  const enqueueInfoToast = (message: string) => {
    const id = makeUndoId();
    const timeoutId = window.setTimeout(() => {
      setInfoToasts((prev) => prev.filter((t) => t.id !== id));
    }, 2_500);
    setInfoToasts((prev) => [...prev, { id, message, timeoutId }]);
  };

  const upsertVoltagePoint = (
    points: CalibrationPointVoltage[],
    point: CalibrationPointVoltage,
  ): CalibrationPointVoltage[] => {
    const idx = points.findIndex((p) => p.raw === point.raw);
    if (idx < 0) return [...points, point];
    const next = points.slice();
    next[idx] = point;
    return next;
  };

  const upsertCurrentPoint = (
    points: CalibrationPointCurrent[],
    point: CalibrationPointCurrent,
  ): CalibrationPointCurrent[] => {
    const idx = points.findIndex((p) => p.raw === point.raw);
    if (idx < 0) return [...points, point];
    const next = points.slice();
    next[idx] = point;
    return next;
  };

  const applyUndoAction = (action: UndoAction) => {
    if (action.kind === "voltage_points") {
      setDraftProfile((prev) => ({
        ...prev,
        v_local_points: action.local
          ? upsertVoltagePoint(prev.v_local_points, action.local)
          : prev.v_local_points,
        v_remote_points: action.remote
          ? upsertVoltagePoint(prev.v_remote_points, action.remote)
          : prev.v_remote_points,
      }));
      return;
    }

    setDraftProfile((prev) => ({
      ...prev,
      current_ch1_points:
        action.curve === "current_ch1"
          ? upsertCurrentPoint(prev.current_ch1_points, action.point)
          : prev.current_ch1_points,
      current_ch2_points:
        action.curve === "current_ch2"
          ? upsertCurrentPoint(prev.current_ch2_points, action.point)
          : prev.current_ch2_points,
    }));
  };

  const undoToast = (toast: UndoToastEntry) => {
    window.clearTimeout(toast.timeoutId);
    setUndoToasts((prev) => prev.filter((t) => t.id !== toast.id));
    applyUndoAction(toast.action);
  };

  const enqueueUndo = (action: UndoAction, message: string) => {
    const id = makeUndoId();
    const expiresAt = Date.now() + 5_000;
    const timeoutId = window.setTimeout(() => {
      setUndoToasts((prev) => prev.filter((t) => t.id !== id));
    }, 5_000);

    setUndoToasts((prev) => [
      ...prev,
      {
        id,
        message,
        action,
        expiresAt,
        timeoutId,
      },
    ]);
  };

  const resetDraftToEmpty = (message = "Draft cleared.") => {
    clearToasts();
    writeCalibrationDraftToStorage(deviceId, baseUrl, null);
    setDraftProfile(makeEmptyDraftProfile(profileQuery.data?.active));
    setPreviewProfile(null);
    setPreviewAppliedAt(null);
    setImportError(null);
    setImportIssues(null);
    enqueueInfoToast(message);
  };

  useEffect(() => {
    setDraftProfile((prev) => ({
      ...prev,
      active:
        profileQuery.data?.active ?? prev.active ?? DEFAULT_ACTIVE_PROFILE,
    }));
  }, [profileQuery.data?.active]);

  useEffect(() => {
    if (undoToasts.length === 0) return;
    const id = window.setInterval(() => setUndoNow(Date.now()), 250);
    return () => window.clearInterval(id);
  }, [undoToasts.length]);

  const handleExportDraft = () => {
    if (isDraftEmpty(draftProfile)) {
      return;
    }

    const now = new Date();
    const stamp = now.toISOString().replaceAll(":", "-");
    const payload = {
      schema_version: 2,
      generated_at: now.toISOString(),
      device_id: deviceId,
      active_snapshot: profileQuery.data?.active ?? draftProfile.active,
      curves: {
        v_local_points: draftProfile.v_local_points.map((p) => [p.raw, p.mv]),
        v_remote_points: draftProfile.v_remote_points.map((p) => [p.raw, p.mv]),
        current_ch1_points: draftProfile.current_ch1_points.map((p) => [
          [p.raw, p.dac_code],
          p.ma,
        ]),
        current_ch2_points: draftProfile.current_ch2_points.map((p) => [
          [p.raw, p.dac_code],
          p.ma,
        ]),
      },
    };

    downloadJson(
      `loadlynx-calibration-draft-${deviceId}-${stamp}.json`,
      payload,
    );
  };

  const handleImportDraftFile = async (file: File | null) => {
    if (!file) return;

    setImportError(null);
    setImportIssues(null);
    clearToasts();

    let parsed: unknown;
    try {
      parsed = JSON.parse(await file.text());
    } catch {
      setImportError("Invalid JSON file.");
      return;
    }

    const root =
      typeof parsed === "object" && parsed !== null
        ? (parsed as Record<string, unknown>)
        : null;

    const curvesCandidate =
      root && typeof root.curves === "object" && root.curves !== null
        ? (root.curves as Record<string, unknown>)
        : root && typeof root.profile === "object" && root.profile !== null
          ? (root.profile as Record<string, unknown>)
          : root;

    if (!curvesCandidate || typeof curvesCandidate !== "object") {
      setImportError("Missing curves object in JSON.");
      return;
    }

    const curves = curvesCandidate as Record<string, unknown>;

    const issues: ValidationIssue[] = [];
    const readNumber = (value: unknown): number | null => {
      if (typeof value !== "number" || !Number.isFinite(value)) return null;
      return value;
    };

    const readArray = (value: unknown): unknown[] | null => {
      if (!Array.isArray(value)) return null;
      return value;
    };

    const parseVoltagePoint = (
      value: unknown,
      path: string,
    ): CalibrationPointVoltage | null => {
      if (Array.isArray(value) && value.length >= 2) {
        const raw = readNumber(value[0]);
        const mv = readNumber(value[1]);
        if (raw == null)
          issues.push({ path: `${path}[0]`, message: "raw must be a number" });
        if (mv == null)
          issues.push({ path: `${path}[1]`, message: "mv must be a number" });
        if (raw == null || mv == null) return null;
        return { raw, mv };
      }
      if (typeof value !== "object" || value === null) {
        issues.push({ path, message: "point must be an object" });
        return null;
      }
      const obj = value as Record<string, unknown>;
      const raw = readNumber(obj.raw ?? obj.raw_100uv);
      const mv = readNumber(obj.mv ?? obj.meas_mv);
      if (raw == null)
        issues.push({ path: `${path}.raw`, message: "raw must be a number" });
      if (mv == null)
        issues.push({ path: `${path}.mv`, message: "mv must be a number" });
      if (raw == null || mv == null) return null;
      return { raw, mv };
    };

    const parseCurrentPoint = (
      value: unknown,
      path: string,
    ): CalibrationPointCurrent | null => {
      if (Array.isArray(value)) {
        if (value.length >= 2 && Array.isArray(value[0])) {
          const raw = readNumber(value[0][0]);
          const dac = readNumber(value[0][1]);
          const ma = readNumber(value[1]);
          if (raw == null)
            issues.push({
              path: `${path}[0][0]`,
              message: "raw must be a number",
            });
          if (dac == null)
            issues.push({
              path: `${path}[0][1]`,
              message: "dac_code must be a number",
            });
          if (ma == null)
            issues.push({ path: `${path}[1]`, message: "ma must be a number" });
          if (raw == null || ma == null || dac == null) return null;
          return { raw, ma, dac_code: dac };
        }
        if (value.length >= 3) {
          const raw = readNumber(value[0]);
          const ma = readNumber(value[1]);
          const dac = readNumber(value[2]);
          if (raw == null)
            issues.push({
              path: `${path}[0]`,
              message: "raw must be a number",
            });
          if (ma == null)
            issues.push({ path: `${path}[1]`, message: "ma must be a number" });
          if (dac == null)
            issues.push({
              path: `${path}[2]`,
              message: "dac_code must be a number",
            });
          if (raw == null || ma == null || dac == null) return null;
          return { raw, ma, dac_code: dac };
        }
      }
      if (typeof value !== "object" || value === null) {
        issues.push({ path, message: "point must be an object" });
        return null;
      }
      const obj = value as Record<string, unknown>;
      const raw = readNumber(obj.raw ?? obj.raw_100uv);
      const ma = readNumber(obj.ma ?? obj.meas_ma);
      const dac = readNumber(obj.dac_code ?? obj.raw_dac_code);
      if (raw == null)
        issues.push({ path: `${path}.raw`, message: "raw must be a number" });
      if (ma == null)
        issues.push({ path: `${path}.ma`, message: "ma must be a number" });
      if (dac == null)
        issues.push({
          path: `${path}.dac_code`,
          message: "dac_code must be a number",
        });
      if (raw == null || ma == null || dac == null) return null;
      return { raw, ma, dac_code: dac };
    };

    const parseVoltagePoints = (
      value: unknown,
      path: string,
    ): CalibrationPointVoltage[] => {
      const arr = readArray(value);
      if (!arr) {
        issues.push({ path, message: "must be an array" });
        return [];
      }
      return arr.flatMap((entry, idx) => {
        const point = parseVoltagePoint(entry, `${path}[${idx}]`);
        return point ? [point] : [];
      });
    };

    const parseCurrentPoints = (
      value: unknown,
      path: string,
    ): CalibrationPointCurrent[] => {
      const arr = readArray(value);
      if (!arr) {
        issues.push({ path, message: "must be an array" });
        return [];
      }
      return arr.flatMap((entry, idx) => {
        const point = parseCurrentPoint(entry, `${path}[${idx}]`);
        return point ? [point] : [];
      });
    };

    const activeFallback =
      profileQuery.data?.active ??
      draftProfile.active ??
      DEFAULT_ACTIVE_PROFILE;

    const nextProfile: CalibrationProfile = {
      active: activeFallback,
      v_local_points: parseVoltagePoints(
        curves.v_local_points,
        "v_local_points",
      ),
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
      setImportError("Import validation failed (shape/types).");
      setImportIssues(issues);
      return;
    }

    if (isDraftEmpty(nextProfile)) {
      setImportError("Empty drafts are not supported for import.");
      setImportIssues(null);
      return;
    }

    // Normalize to match firmware behavior (raw-sorted + dedup).
    const vLocal =
      nextProfile.v_local_points.length > 0
        ? validateAndNormalizeVoltagePoints(
            "v_local",
            nextProfile.v_local_points,
          )
        : { normalized: [], issues: [] };
    const vRemote =
      nextProfile.v_remote_points.length > 0
        ? validateAndNormalizeVoltagePoints(
            "v_remote",
            nextProfile.v_remote_points,
          )
        : { normalized: [], issues: [] };
    const c1 =
      nextProfile.current_ch1_points.length > 0
        ? validateAndNormalizeCurrentPoints(
            "current_ch1",
            nextProfile.current_ch1_points,
          )
        : { normalized: [], issues: [] };
    const c2 =
      nextProfile.current_ch2_points.length > 0
        ? validateAndNormalizeCurrentPoints(
            "current_ch2",
            nextProfile.current_ch2_points,
          )
        : { normalized: [], issues: [] };

    const fullIssues = [
      ...vLocal.issues,
      ...vRemote.issues,
      ...c1.issues,
      ...c2.issues,
    ];
    if (fullIssues.length > 0) {
      setImportError("Import validation failed (firmware constraints).");
      setImportIssues(fullIssues);
      return;
    }

    const normalized: CalibrationProfile = {
      active: nextProfile.active,
      v_local_points: vLocal.normalized,
      v_remote_points: vRemote.normalized,
      current_ch1_points: c1.normalized,
      current_ch2_points: c2.normalized,
    };

    setDraftProfile(normalized);
    setPreviewProfile(structuredClone(normalized));
    setPreviewAppliedAt(Date.now());
    setImportError(null);
    setImportIssues(null);
  };

  const draftEmpty = useMemo(() => isDraftEmpty(draftProfile), [draftProfile]);
  const [confirmReadDeviceToDraft, setConfirmReadDeviceToDraft] =
    useState(false);
  const [readDeviceToDraftPending, setReadDeviceToDraftPending] =
    useState(false);
  const [alertDialog, setAlertDialog] = useState<{
    title: string;
    body: string;
    details: string[];
  } | null>(null);

  const showAlert = (title: string, body: string, details: string[] = []) => {
    setAlertDialog({ title, body, details });
  };

  const performReadDeviceToDraft = async () => {
    if (readDeviceToDraftPending) return;
    if (isOffline) {
      setAlertDialog({
        title: "Cannot Read Device Profile",
        body: "Device is offline/faulted; cannot read calibration profile.",
        details: [],
      });
      return;
    }

    clearToasts();
    setImportError(null);
    setImportIssues(null);
    setReadDeviceToDraftPending(true);

    try {
      const result = await profileQuery.refetch();
      const deviceProfile = result.data;
      if (!deviceProfile) throw new Error("No device profile loaded.");

      setDraftProfile(deviceProfile);

      setPreviewProfile(null);
      setPreviewAppliedAt(null);

      enqueueInfoToast(
        isDraftEmpty(deviceProfile)
          ? "Device profile is empty; draft cleared."
          : "Loaded device profile into draft.",
      );
    } catch (err) {
      console.error(err);
      setAlertDialog({
        title: "Failed to Read Device Profile",
        body: String(err),
        details: [],
      });
    } finally {
      setReadDeviceToDraftPending(false);
    }
  };

  const requestReadDeviceToDraft = () => {
    if (isOffline) {
      setAlertDialog({
        title: "Cannot Read Device Profile",
        body: "Device is offline/faulted; cannot read calibration profile.",
        details: [],
      });
      return;
    }
    if (!draftEmpty) {
      setConfirmReadDeviceToDraft(true);
      return;
    }
    void performReadDeviceToDraft();
  };

  const draftIssues = useMemo(() => {
    const issues: ValidationIssue[] = [];
    if (draftProfile.v_local_points.length > 0) {
      issues.push(
        ...validateAndNormalizeVoltagePoints(
          "v_local",
          draftProfile.v_local_points,
        ).issues,
      );
    }
    if (draftProfile.v_remote_points.length > 0) {
      issues.push(
        ...validateAndNormalizeVoltagePoints(
          "v_remote",
          draftProfile.v_remote_points,
        ).issues,
      );
    }
    if (draftProfile.current_ch1_points.length > 0) {
      issues.push(
        ...validateAndNormalizeCurrentPoints(
          "current_ch1",
          draftProfile.current_ch1_points,
        ).issues,
      );
    }
    if (draftProfile.current_ch2_points.length > 0) {
      issues.push(
        ...validateAndNormalizeCurrentPoints(
          "current_ch2",
          draftProfile.current_ch2_points,
        ).issues,
      );
    }
    return issues;
  }, [draftProfile]);

  const previewMatchesDraft = useMemo(() => {
    if (!previewProfile) return null;
    return calibrationProfilesPointsEqual(previewProfile, draftProfile);
  }, [previewProfile, draftProfile]);

  const deviceUsingDefaults =
    profileQuery.data?.active.source === "factory-default";

  // Load draft from browser storage (or reset if none) while switching devices/URLs.
  useEffect(() => {
    clearToasts();
    setDraftProfile(makeEmptyDraftProfile());
    setPreviewProfile(null);
    setPreviewAppliedAt(null);
    setImportError(null);
    setImportIssues(null);

    setDraftStorageReady(false);
    const stored = readCalibrationDraftFromStorage(deviceId, baseUrl);
    if (stored) {
      setActiveTab(stored.active_tab);
      setDraftProfile((prev) => ({
        active: prev.active ?? DEFAULT_ACTIVE_PROFILE,
        v_local_points: stored.draft_profile.v_local_points,
        v_remote_points: stored.draft_profile.v_remote_points,
        current_ch1_points: stored.draft_profile.current_ch1_points,
        current_ch2_points: stored.draft_profile.current_ch2_points,
      }));
    } else {
      setActiveTab("voltage");
      setDraftProfile(makeEmptyDraftProfile());
    }
    setDraftStorageReady(true);
  }, [baseUrl, deviceId, clearToasts]);

  // Persist drafts immediately to the browser to prevent accidental loss.
  useEffect(() => {
    if (!draftStorageReady) return;

    if (isDraftEmpty(draftProfile)) {
      writeCalibrationDraftToStorage(deviceId, baseUrl, null);
      return;
    }

    writeCalibrationDraftToStorage(deviceId, baseUrl, {
      version: 3,
      saved_at: new Date().toISOString(),
      device_id: deviceId,
      base_url: baseUrl,
      active_tab: activeTab,
      draft_profile: {
        v_local_points: draftProfile.v_local_points.map((p) => [p.raw, p.mv]),
        v_remote_points: draftProfile.v_remote_points.map((p) => [p.raw, p.mv]),
        current_ch1_points: draftProfile.current_ch1_points.map((p) => [
          [p.raw, p.dac_code],
          p.ma,
        ]),
        current_ch2_points: draftProfile.current_ch2_points.map((p) => [
          [p.raw, p.dac_code],
          p.ma,
        ]),
      },
    });
  }, [draftStorageReady, deviceId, baseUrl, activeTab, draftProfile]);

  // Always attempt to reset mode when leaving the page.
  useEffect(() => {
    return () => {
      postCalibrationMode(baseUrl, { kind: "off" }).catch(console.error);
    };
  }, [baseUrl]);

  // Switch mode when changing tabs. Current tab selection is refined by the
  // active tab selection.
  useEffect(() => {
    postCalibrationMode(baseUrl, {
      kind: activeTab === "voltage" ? "voltage" : activeTab,
    }).catch(console.error);
  }, [activeTab, baseUrl]);

  return (
    <div className="flex flex-col gap-6 max-w-5xl mx-auto">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Calibration</h2>
        <div className="badge badge-neutral gap-2">
          {isOffline ? "OFFLINE / FAULT" : "ONLINE"}
        </div>
      </div>

      <div className="card bg-base-100 shadow-xl border border-base-200">
        <div className="card-body gap-3">
          <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
            <div className="text-sm space-y-1">
              <div>
                <span className="font-bold">Device active:</span>{" "}
                {profileQuery.data ? (
                  <>
                    source=
                    <span className="font-mono">
                      {profileQuery.data.active.source}
                    </span>
                    , fmt=
                    <span className="font-mono">
                      {profileQuery.data.active.fmt_version}
                    </span>
                    , hw=
                    <span className="font-mono">
                      {profileQuery.data.active.hw_rev}
                    </span>
                  </>
                ) : (
                  <span className="text-base-content/60">--</span>
                )}
              </div>
              <div>
                <span className="font-bold">Last read:</span>{" "}
                {profileQuery.dataUpdatedAt ? (
                  <span className="font-mono">
                    {formatLocalTimestamp(profileQuery.dataUpdatedAt)}
                  </span>
                ) : (
                  <span className="text-base-content/60">--</span>
                )}
              </div>
              <div>
                <span className="font-bold">Status:</span>{" "}
                {profileQuery.data ? (
                  draftEmpty && deviceUsingDefaults ? (
                    "No user calibration points / device uses defaults."
                  ) : draftEmpty ? (
                    `No user calibration points in draft / device is ${profileQuery.data.active.source}.`
                  ) : (
                    "Draft not synced to device / sync required."
                  )
                ) : (
                  <span className="text-base-content/60">--</span>
                )}
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-2">
              {draftEmpty ? (
                <div className="badge badge-neutral">Draft: none</div>
              ) : (
                <div className="badge badge-warning">Draft: needs sync</div>
              )}

              {profileQuery.data ? (
                deviceUsingDefaults ? (
                  <div className="badge badge-success">Device: defaults</div>
                ) : (
                  <div className="badge badge-info">
                    Device: user-calibrated
                  </div>
                )
              ) : (
                <div className="badge badge-neutral">Device: --</div>
              )}

              {draftIssues.length > 0 ? (
                <div className="badge badge-error">
                  Draft issues ({draftIssues.length})
                </div>
              ) : !draftEmpty ? (
                <div className="badge badge-success">Draft OK</div>
              ) : null}

              {!previewProfile ? (
                <div className="badge badge-neutral">Preview: device</div>
              ) : previewMatchesDraft ? (
                <div className="badge badge-neutral">Preview up to date</div>
              ) : (
                <div className="badge badge-warning">Preview out of date</div>
              )}

              {previewAppliedAt ? (
                <div className="badge badge-ghost">
                  Preview applied {formatLocalTimestamp(previewAppliedAt)}
                </div>
              ) : null}
            </div>
          </div>

          {importError && (
            <div role="alert" className="alert alert-error text-sm py-2">
              <div className="flex flex-col gap-2">
                <div className="font-bold">{importError}</div>
                {importIssues && importIssues.length > 0 && (
                  <ul className="list-disc pl-5">
                    {importIssues.slice(0, 5).map((issue) => (
                      <li key={`${issue.path}:${issue.message}`}>
                        <span className="font-mono">{issue.path}</span>:{" "}
                        {issue.message}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      <div role="tablist" className="tabs tabs-boxed mt-4">
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === "voltage" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("voltage")}
        >
          电压
        </button>
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === "current_ch1" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("current_ch1")}
        >
          电流通道1
        </button>
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === "current_ch2" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("current_ch2")}
        >
          电流通道2
        </button>
      </div>

      {activeTab === "voltage" ? (
        <VoltageCalibration
          baseUrl={baseUrl}
          status={status}
          deviceProfile={profileQuery.data}
          draftProfile={draftProfile}
          previewProfile={previewProfile}
          onSetDraftProfile={setDraftProfile}
          onSetPreviewProfile={setPreviewProfile}
          onSetPreviewAppliedAt={setPreviewAppliedAt}
          deviceId={deviceId}
          onExportDraft={handleExportDraft}
          onImportDraftFile={handleImportDraftFile}
          onReadDeviceToDraft={requestReadDeviceToDraft}
          readDeviceToDraftPending={readDeviceToDraftPending}
          onAlert={showAlert}
          onEnqueueUndo={enqueueUndo}
          onResetDraftToEmpty={resetDraftToEmpty}
          onRefetchProfile={profileQuery.refetch}
          isOffline={isOffline}
        />
      ) : (
        <CurrentCalibration
          curve={activeTab}
          baseUrl={baseUrl}
          status={status}
          deviceProfile={profileQuery.data}
          draftProfile={draftProfile}
          previewProfile={previewProfile}
          onSetDraftProfile={setDraftProfile}
          onSetPreviewProfile={setPreviewProfile}
          onSetPreviewAppliedAt={setPreviewAppliedAt}
          deviceId={deviceId}
          onExportDraft={handleExportDraft}
          onImportDraftFile={handleImportDraftFile}
          onReadDeviceToDraft={requestReadDeviceToDraft}
          readDeviceToDraftPending={readDeviceToDraftPending}
          onAlert={showAlert}
          onEnqueueUndo={enqueueUndo}
          onResetDraftToEmpty={resetDraftToEmpty}
          onRefetchProfile={profileQuery.refetch}
          isOffline={isOffline}
        />
      )}

      <ConfirmDialog
        open={confirmReadDeviceToDraft}
        title="Read Device Calibration → Draft"
        body="This reads the current calibration profile from the device and overwrites the local web draft."
        details={[
          "Affects: v_local, v_remote, current_ch1, current_ch2 (local draft only).",
          "Writes device: No.",
          "Preview: cleared (returns to device preview).",
          "Irreversible locally: Yes (export draft first if needed).",
        ]}
        confirmLabel="Overwrite Draft"
        destructive
        confirmDisabled={readDeviceToDraftPending || isOffline}
        onCancel={() => setConfirmReadDeviceToDraft(false)}
        onConfirm={() => {
          setConfirmReadDeviceToDraft(false);
          void performReadDeviceToDraft();
        }}
      />

      <AlertDialog
        open={alertDialog !== null}
        title={alertDialog?.title ?? ""}
        body={alertDialog?.body ?? ""}
        details={alertDialog?.details ?? []}
        onClose={() => setAlertDialog(null)}
      />

      {(infoToasts.length > 0 || undoToasts.length > 0) && (
        <div className="toast toast-end toast-bottom z-50">
          {infoToasts.map((toast) => (
            <div key={toast.id} className="alert alert-success text-sm">
              <div className="flex items-center justify-between gap-3 w-full">
                <div className="flex-1">{toast.message}</div>
              </div>
            </div>
          ))}
          {undoToasts.map((toast) => {
            const remaining = Math.max(
              0,
              Math.ceil((toast.expiresAt - undoNow) / 1000),
            );
            return (
              <div key={toast.id} className="alert alert-info text-sm">
                <div className="flex items-center justify-between gap-3 w-full">
                  <div className="flex-1">{toast.message}</div>
                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      className="btn btn-xs btn-outline"
                      onClick={() => undoToast(toast)}
                    >
                      Undo
                    </button>
                    <span className="font-mono text-xs text-base-content/60">
                      {remaining}s
                    </span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function VoltageCalibration({
  baseUrl,
  status,
  deviceProfile,
  draftProfile,
  previewProfile,
  onSetDraftProfile,
  onSetPreviewProfile,
  onSetPreviewAppliedAt,
  deviceId,
  onExportDraft,
  onImportDraftFile,
  onReadDeviceToDraft,
  readDeviceToDraftPending,
  onAlert,
  onEnqueueUndo,
  onResetDraftToEmpty,
  onRefetchProfile,
  isOffline,
}: {
  baseUrl: string;
  status: FastStatusView | null;
  deviceProfile: CalibrationProfile | undefined;
  draftProfile: CalibrationProfile;
  previewProfile: CalibrationProfile | null;
  onSetDraftProfile: React.Dispatch<React.SetStateAction<CalibrationProfile>>;
  onSetPreviewProfile: React.Dispatch<
    React.SetStateAction<CalibrationProfile | null>
  >;
  onSetPreviewAppliedAt: React.Dispatch<React.SetStateAction<number | null>>;
  deviceId: string;
  onExportDraft: () => void;
  onImportDraftFile: (file: File | null) => Promise<void>;
  onReadDeviceToDraft: () => void;
  readDeviceToDraftPending: boolean;
  onAlert: (title: string, body: string, details?: string[]) => void;
  onEnqueueUndo: (action: UndoAction, message: string) => void;
  onResetDraftToEmpty: (message?: string) => void;
  onRefetchProfile: RefetchProfile;
  isOffline: boolean;
}) {
  const [viewTab, setViewTab] = useState<"draft" | "device">("draft");
  const [inputV, setInputV] = useState("12.00");
  const [confirmKind, setConfirmKind] = useState<
    "reset_draft" | "reset_device_voltage" | null
  >(null);

  const effectivePreview = previewProfile ?? deviceProfile ?? null;

  const draftLocalPoints = draftProfile.v_local_points;
  const draftRemotePoints = draftProfile.v_remote_points;

  const previewLocalPoints = effectivePreview?.v_local_points ?? [];
  const previewRemotePoints = effectivePreview?.v_remote_points ?? [];

  const mergedDraft = mergeVoltageCandidatesByMv(
    draftLocalPoints,
    draftRemotePoints,
  );
  const mergedDevice = mergeVoltageCandidatesByMv(
    deviceProfile?.v_local_points ?? [],
    deviceProfile?.v_remote_points ?? [],
  );

  const vLocalDraft = useMemo(
    () => validateAndNormalizeVoltagePoints("v_local", draftLocalPoints),
    [draftLocalPoints],
  );
  const vRemoteDraft = useMemo(
    () => validateAndNormalizeVoltagePoints("v_remote", draftRemotePoints),
    [draftRemotePoints],
  );

  const draftVoltageIssues = useMemo(
    () => [...vLocalDraft.issues, ...vRemoteDraft.issues],
    [vLocalDraft.issues, vRemoteDraft.issues],
  );
  const canWriteToDevice =
    !isOffline &&
    draftLocalPoints.length > 0 &&
    draftRemotePoints.length > 0 &&
    draftVoltageIssues.length === 0;

  const handleCapture = () => {
    const rawLocal = status?.raw.raw_v_nr_100uv;
    const rawRemote = status?.raw.raw_v_rmt_100uv;

    if (rawLocal == null || rawRemote == null) {
      onAlert(
        "Cannot Capture Voltage Point",
        "Raw values not available. Ensure calibration mode is enabled.",
      );
      return;
    }

    const measuredMv = Math.round(Number.parseFloat(inputV) * 1000);
    if (!Number.isFinite(measuredMv) || measuredMv <= 0) {
      onAlert("Cannot Capture Voltage Point", "Invalid voltage input.");
      return;
    }

    const localUnique = new Set(draftLocalPoints.map((p) => p.raw)).size;
    const remoteUnique = new Set(draftRemotePoints.map((p) => p.raw)).size;
    const localHasRaw = draftLocalPoints.some((p) => p.raw === rawLocal);
    const remoteHasRaw = draftRemotePoints.some((p) => p.raw === rawRemote);
    if (!localHasRaw && localUnique >= CALIBRATION_MAX_POINTS) {
      onAlert(
        "Cannot Capture Voltage Point",
        `Too many unique points (max ${CALIBRATION_MAX_POINTS}).`,
      );
      return;
    }
    if (!remoteHasRaw && remoteUnique >= CALIBRATION_MAX_POINTS) {
      onAlert(
        "Cannot Capture Voltage Point",
        `Too many unique points (max ${CALIBRATION_MAX_POINTS}).`,
      );
      return;
    }

    onSetDraftProfile((prev) => {
      const upsert = (
        points: CalibrationPointVoltage[],
        point: CalibrationPointVoltage,
      ): CalibrationPointVoltage[] => {
        const idx = points.findIndex((p) => p.raw === point.raw);
        if (idx < 0) return [...points, point];
        const next = points.slice();
        next[idx] = point;
        return next;
      };
      return {
        ...prev,
        v_local_points: upsert(prev.v_local_points, {
          raw: rawLocal,
          mv: measuredMv,
        }),
        v_remote_points: upsert(prev.v_remote_points, {
          raw: rawRemote,
          mv: measuredMv,
        }),
      };
    });
  };

  const handleDeleteDraftRow = (mv: number) => {
    const local = draftLocalPoints.find((p) => p.mv === mv);
    const remote = draftRemotePoints.find((p) => p.mv === mv);
    if (!local && !remote) return;
    onEnqueueUndo(
      {
        kind: "voltage_points",
        local: local ?? undefined,
        remote: remote ?? undefined,
      },
      `Deleted voltage point (mv=${mv})`,
    );
    onSetDraftProfile((prev) => ({
      ...prev,
      v_local_points: prev.v_local_points.filter((p) => p.mv !== mv),
      v_remote_points: prev.v_remote_points.filter((p) => p.mv !== mv),
    }));
  };

  const previewLocalDataset = previewLocalPoints.map((point) => ({
    x: point.raw,
    y: point.mv,
  }));
  const previewRemoteDataset = previewRemotePoints.map((point) => ({
    x: point.raw,
    y: point.mv,
  }));

  const previewLocalV =
    status?.raw.raw_v_nr_100uv != null && previewLocalDataset.length >= 1
      ? piecewiseLinear(previewLocalDataset, status.raw.raw_v_nr_100uv) / 1000
      : null;

  const previewRemoteV =
    status?.raw.raw_v_rmt_100uv != null && previewRemoteDataset.length >= 1
      ? piecewiseLinear(previewRemoteDataset, status.raw.raw_v_rmt_100uv) / 1000
      : null;

  const readMutation = useMutation({
    mutationFn: async () => onRefetchProfile(),
  });

  const applyToDeviceMutation = useMutation({
    mutationFn: async () => {
      if (draftLocalPoints.length === 0 || draftRemotePoints.length === 0) {
        throw new Error("Draft is empty. Nothing to sync.");
      }

      const local = validateAndNormalizeVoltagePoints(
        "v_local",
        draftLocalPoints,
      );
      const remote = validateAndNormalizeVoltagePoints(
        "v_remote",
        draftRemotePoints,
      );
      const issues = [...local.issues, ...remote.issues];
      if (issues.length > 0) {
        throw new Error(`Draft validation failed: ${issues[0].message}`);
      }

      await postCalibrationApply(baseUrl, {
        kind: "v_local",
        points: local.normalized,
      });
      await new Promise((resolve) => setTimeout(resolve, 200));
      await postCalibrationApply(baseUrl, {
        kind: "v_remote",
        points: remote.normalized,
      });
    },
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const commitToDeviceMutation = useMutation({
    mutationFn: async () => {
      if (draftLocalPoints.length === 0 || draftRemotePoints.length === 0) {
        throw new Error("Draft is empty. Nothing to sync.");
      }

      const local = validateAndNormalizeVoltagePoints(
        "v_local",
        draftLocalPoints,
      );
      const remote = validateAndNormalizeVoltagePoints(
        "v_remote",
        draftRemotePoints,
      );
      const issues = [...local.issues, ...remote.issues];
      if (issues.length > 0) {
        throw new Error(`Draft validation failed: ${issues[0].message}`);
      }

      await postCalibrationCommit(baseUrl, {
        kind: "v_local",
        points: local.normalized,
      });
      await new Promise((resolve) => setTimeout(resolve, 200));
      await postCalibrationCommit(baseUrl, {
        kind: "v_remote",
        points: remote.normalized,
      });
    },
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const resetDeviceVoltageMutation = useMutation({
    mutationFn: async () => {
      // "Reset All" for voltage: only reset v_local + v_remote (not current).
      await postCalibrationReset(baseUrl, { kind: "v_local" });
      await new Promise((resolve) => setTimeout(resolve, 200));
      await postCalibrationReset(baseUrl, { kind: "v_remote" });
    },
    onSuccess: async () => {
      await onRefetchProfile();
      onResetDraftToEmpty("Device reset to defaults. Draft cleared.");
    },
  });

  return (
    <>
      <div role="tablist" className="tabs tabs-boxed">
        <button
          type="button"
          role="tab"
          className={`tab ${viewTab === "draft" ? "tab-active" : ""}`}
          onClick={() => setViewTab("draft")}
        >
          本地草稿
        </button>
        <button
          type="button"
          role="tab"
          className={`tab ${viewTab === "device" ? "tab-active" : ""}`}
          onClick={() => setViewTab("device")}
        >
          设备数据
        </button>
      </div>

      {viewTab === "draft" ? (
        <div className="card bg-base-100 shadow-xl border border-base-200 mt-4">
          <div className="card-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="card-title flex flex-col items-start leading-tight">
                <span>本地草稿</span>
                <span className="text-sm font-normal text-base-content/60">
                  Web
                </span>
              </h3>
            </div>

            <div className="card bg-base-200/40 border border-base-200">
              <div className="card-body py-4 gap-3">
                <div className="flex items-start justify-between gap-3">
                  <h4 className="font-bold text-sm">仅本地（不读写设备）</h4>
                  <div className="badge badge-neutral whitespace-nowrap shrink-0">
                    不读写设备
                  </div>
                </div>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={() => {
                      onSetPreviewProfile(structuredClone(draftProfile));
                      onSetPreviewAppliedAt(Date.now());
                    }}
                    disabled={isDraftEmpty(draftProfile)}
                  >
                    Apply Preview
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={() => setConfirmKind("reset_draft")}
                    disabled={isDraftEmpty(draftProfile)}
                  >
                    Reset Draft
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={onExportDraft}
                    disabled={isDraftEmpty(draftProfile)}
                    title={
                      isDraftEmpty(draftProfile)
                        ? "Export is disabled when the draft is empty."
                        : undefined
                    }
                  >
                    Export
                  </button>
                  <label
                    htmlFor={`calibration-import-${deviceId}-voltage`}
                    className="btn btn-sm btn-outline"
                  >
                    Import
                  </label>
                  <input
                    id={`calibration-import-${deviceId}-voltage`}
                    type="file"
                    accept="application/json"
                    className="hidden"
                    onChange={(event) => {
                      const file = event.currentTarget.files?.[0] ?? null;
                      void onImportDraftFile(file);
                      event.currentTarget.value = "";
                    }}
                  />
                </div>
              </div>
            </div>

            <div className="card bg-base-200/40 border border-base-200">
              <div className="card-body py-4 gap-3">
                <div className="flex items-start justify-between gap-3">
                  <h4 className="font-bold text-sm">硬件 I/O</h4>
                  <div className="flex items-center gap-2">
                    <div className="badge badge-info whitespace-nowrap">
                      读设备
                    </div>
                    <div className="badge badge-warning whitespace-nowrap">
                      写设备
                    </div>
                  </div>
                </div>

                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={onReadDeviceToDraft}
                    disabled={isOffline || readDeviceToDraftPending}
                    title={
                      isOffline
                        ? "Device offline/faulted."
                        : readDeviceToDraftPending
                          ? "Reading device profile..."
                          : "Reads device calibration profile and overwrites the local draft."
                    }
                  >
                    Read Device → Draft
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={() => applyToDeviceMutation.mutate()}
                    disabled={
                      !canWriteToDevice || applyToDeviceMutation.isPending
                    }
                  >
                    Apply
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-secondary"
                    onClick={() => commitToDeviceMutation.mutate()}
                    disabled={
                      !canWriteToDevice || commitToDeviceMutation.isPending
                    }
                  >
                    Commit
                  </button>
                </div>
              </div>
            </div>

            <div className="divider my-0"></div>

            <div className="flex items-end gap-4">
              <label className="form-control w-full max-w-xs">
                <div className="label">
                  <span className="label-text">Measured Voltage (V)</span>
                </div>
                <input
                  type="number"
                  step="0.01"
                  className="input input-bordered"
                  value={inputV}
                  onChange={(event) => setInputV(event.target.value)}
                  disabled={isOffline}
                />
              </label>
              <button
                type="button"
                className="btn btn-primary"
                onClick={handleCapture}
                disabled={isOffline}
              >
                Capture
              </button>
            </div>

            {draftVoltageIssues.length > 0 &&
              (draftLocalPoints.length > 0 || draftRemotePoints.length > 0) && (
                <div role="alert" className="alert alert-warning text-sm py-2">
                  <span>
                    Draft validation:{" "}
                    <span className="font-bold">
                      {draftVoltageIssues[0].message}
                    </span>
                    {draftVoltageIssues.length > 1
                      ? ` (+${draftVoltageIssues.length - 1} more)`
                      : ""}
                  </span>
                </div>
              )}

            <div className="stats shadow">
              <div className="stat">
                <div className="stat-title">Local Voltage (Active)</div>
                <div className="stat-value text-lg">
                  {((status?.raw.v_local_mv ?? 0) / 1000).toFixed(3)} V
                </div>
                <div className="stat-desc">
                  Raw: {status?.raw.raw_v_nr_100uv ?? "--"}
                </div>
              </div>
              <div className="stat">
                <div className="stat-title">Local Preview</div>
                <div className="stat-value text-lg text-primary">
                  {previewLocalV == null
                    ? "--"
                    : `${previewLocalV.toFixed(3)} V`}
                </div>
                <div className="stat-desc">Uses applied preview</div>
              </div>
            </div>

            <div className="stats shadow">
              <div className="stat">
                <div className="stat-title">Remote Voltage (Active)</div>
                <div className="stat-value text-lg">
                  {((status?.raw.v_remote_mv ?? 0) / 1000).toFixed(3)} V
                </div>
                <div className="stat-desc">
                  Raw: {status?.raw.raw_v_rmt_100uv ?? "--"}
                </div>
              </div>
              <div className="stat">
                <div className="stat-title">Remote Preview</div>
                <div className="stat-value text-lg text-primary">
                  {previewRemoteV == null
                    ? "--"
                    : `${previewRemoteV.toFixed(3)} V`}
                </div>
                <div className="stat-desc">Uses applied preview</div>
              </div>
            </div>

            <div className="overflow-x-auto max-h-64">
              <table className="table table-xs table-pin-rows">
                <thead>
                  <tr>
                    <th>Value (mV)</th>
                    <th>Raw Local</th>
                    <th>Raw Remote</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {mergedDraft.map((row) => (
                    <tr
                      key={`${row.mv}-${row.rawLocal ?? "--"}-${row.rawRemote ?? "--"}`}
                    >
                      <td>{row.mv}</td>
                      <td>{row.rawLocal ?? "--"}</td>
                      <td>{row.rawRemote ?? "--"}</td>
                      <td className="text-right">
                        <button
                          type="button"
                          className="btn btn-ghost btn-xs text-error"
                          onClick={() => handleDeleteDraftRow(row.mv)}
                          disabled={isOffline}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                  {mergedDraft.length === 0 && (
                    <tr>
                      <td
                        colSpan={4}
                        className="text-center text-base-content/50"
                      >
                        No draft points.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      ) : (
        <div className="card bg-base-100 shadow-xl border border-base-200 mt-4">
          <div className="card-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="card-title flex flex-col items-start leading-tight">
                <span>设备数据</span>
                <span className="text-sm font-normal text-base-content/60">
                  Hardware
                </span>
              </h3>
              <div className="flex items-center gap-2">
                <div className="badge badge-info whitespace-nowrap">读设备</div>
                <div className="badge badge-warning whitespace-nowrap">
                  写设备
                </div>
              </div>
            </div>

            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                className="btn btn-sm btn-outline"
                onClick={() => readMutation.mutate()}
                disabled={readMutation.isPending}
              >
                Read
              </button>
              <button
                type="button"
                className="btn btn-sm btn-error"
                onClick={() => setConfirmKind("reset_device_voltage")}
                disabled={isOffline || resetDeviceVoltageMutation.isPending}
              >
                Reset
              </button>
            </div>

            <div className="divider my-0"></div>

            <h4 className="font-bold text-sm">
              {deviceProfile?.active.source === "factory-default"
                ? "Device defaults (factory reference, read-only)"
                : "Device profile (read-only)"}
            </h4>
            <div className="overflow-x-auto max-h-64">
              <table className="table table-xs table-pin-rows">
                <thead>
                  <tr>
                    <th>Value (mV)</th>
                    <th>Raw Local</th>
                    <th>Raw Remote</th>
                  </tr>
                </thead>
                <tbody>
                  {mergedDevice.map((row) => (
                    <tr key={row.mv}>
                      <td>{row.mv}</td>
                      <td>{row.rawLocal ?? "--"}</td>
                      <td>{row.rawRemote ?? "--"}</td>
                    </tr>
                  ))}
                  {mergedDevice.length === 0 && (
                    <tr>
                      <td
                        colSpan={3}
                        className="text-center text-base-content/50"
                      >
                        No device profile loaded.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      <ConfirmDialog
        open={confirmKind !== null}
        title={
          confirmKind === "reset_draft"
            ? "Reset Draft (Web only)"
            : "Reset Device Calibration (Voltage)"
        }
        body={
          confirmKind === "reset_draft"
            ? "This clears the local draft (user calibration points). The device is unchanged."
            : "This resets voltage calibration on the device."
        }
        details={
          confirmKind === "reset_draft"
            ? [
                "Affects: v_local, v_remote, current_ch1, current_ch2 (local draft only).",
                "Writes device: No.",
                "This clears all local draft points (export first if needed).",
              ]
            : [
                "Affects: v_local + v_remote.",
                "Writes device: Yes.",
                "Irreversible: Yes (re-calibrate + commit to recover).",
                "Does not affect: current_ch1/current_ch2.",
              ]
        }
        confirmLabel={confirmKind === "reset_draft" ? "Reset Draft" : "Reset"}
        destructive={confirmKind === "reset_device_voltage"}
        confirmDisabled={
          confirmKind === "reset_draft"
            ? isDraftEmpty(draftProfile)
            : resetDeviceVoltageMutation.isPending || isOffline
        }
        onCancel={() => setConfirmKind(null)}
        onConfirm={() => {
          if (confirmKind === "reset_draft") {
            onResetDraftToEmpty();
          } else if (confirmKind === "reset_device_voltage") {
            resetDeviceVoltageMutation.mutate();
          }
          setConfirmKind(null);
        }}
      />
    </>
  );
}

function CurrentCalibration({
  curve,
  baseUrl,
  status,
  deviceProfile,
  draftProfile,
  previewProfile,
  onSetDraftProfile,
  onSetPreviewProfile,
  onSetPreviewAppliedAt,
  deviceId,
  onExportDraft,
  onImportDraftFile,
  onReadDeviceToDraft,
  readDeviceToDraftPending,
  onAlert,
  onEnqueueUndo,
  onResetDraftToEmpty,
  onRefetchProfile,
  isOffline,
}: {
  curve: "current_ch1" | "current_ch2";
  baseUrl: string;
  status: FastStatusView | null;
  deviceProfile: CalibrationProfile | undefined;
  draftProfile: CalibrationProfile;
  previewProfile: CalibrationProfile | null;
  onSetDraftProfile: React.Dispatch<React.SetStateAction<CalibrationProfile>>;
  onSetPreviewProfile: React.Dispatch<
    React.SetStateAction<CalibrationProfile | null>
  >;
  onSetPreviewAppliedAt: React.Dispatch<React.SetStateAction<number | null>>;
  deviceId: string;
  onExportDraft: () => void;
  onImportDraftFile: (file: File | null) => Promise<void>;
  onReadDeviceToDraft: () => void;
  readDeviceToDraftPending: boolean;
  onAlert: (title: string, body: string, details?: string[]) => void;
  onEnqueueUndo: (action: UndoAction, message: string) => void;
  onResetDraftToEmpty: (message?: string) => void;
  onRefetchProfile: RefetchProfile;
  isOffline: boolean;
}) {
  const [viewTab, setViewTab] = useState<"draft" | "device">("draft");
  const [confirmKind, setConfirmKind] = useState<
    "reset_draft" | "reset_device_current" | null
  >(null);

  const channelLabel = curve === "current_ch1" ? "CH1" : "CH2";
  const channelDisplay = curve === "current_ch1" ? "Local" : "Remote";

  const [meterReadingA, setMeterReadingA] = useState("1.000");
  const [targetIMa, setTargetIMa] = useState("1000");

  const effectivePreview = previewProfile ?? deviceProfile ?? null;

  const draftPoints =
    curve === "current_ch1"
      ? draftProfile.current_ch1_points
      : draftProfile.current_ch2_points;

  const previewPoints =
    curve === "current_ch1"
      ? (effectivePreview?.current_ch1_points ?? [])
      : (effectivePreview?.current_ch2_points ?? []);

  const devicePoints =
    curve === "current_ch1"
      ? (deviceProfile?.current_ch1_points ?? [])
      : (deviceProfile?.current_ch2_points ?? []);

  const currentDraft = useMemo(
    () => validateAndNormalizeCurrentPoints(curve, draftPoints),
    [curve, draftPoints],
  );
  const canWriteToDevice =
    !isOffline && draftPoints.length > 0 && currentDraft.issues.length === 0;

  const handleSetOutput = () => {
    const parsed = Number.parseInt(targetIMa, 10);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      onAlert("Cannot Set Output", "Invalid target current.");
      return;
    }
    updateCc(baseUrl, { enable: true, target_i_ma: parsed }).catch(
      console.error,
    );
  };

  const handleCapture = () => {
    const rawCur = status?.raw.raw_cur_100uv;
    const rawDac = status?.raw.raw_dac_code;

    if (rawCur == null || rawDac == null) {
      onAlert(
        "Cannot Capture Current Point",
        "Raw values not available. Ensure calibration mode is enabled.",
      );
      return;
    }

    const measuredMa = Math.round(Number.parseFloat(meterReadingA) * 1000);
    if (!Number.isFinite(measuredMa) || measuredMa <= 0) {
      onAlert("Cannot Capture Current Point", "Invalid current input.");
      return;
    }

    const unique = new Set(draftPoints.map((p) => p.raw)).size;
    const hasRaw = draftPoints.some((p) => p.raw === rawCur);
    if (!hasRaw && unique >= CALIBRATION_MAX_POINTS) {
      onAlert(
        "Cannot Capture Current Point",
        `Too many unique points (max ${CALIBRATION_MAX_POINTS}).`,
      );
      return;
    }

    const point: CalibrationPointCurrent = {
      raw: rawCur,
      ma: measuredMa,
      dac_code: rawDac,
    };
    onSetDraftProfile((prev) => {
      const upsert = (
        points: CalibrationPointCurrent[],
        p: CalibrationPointCurrent,
      ): CalibrationPointCurrent[] => {
        const idx = points.findIndex((entry) => entry.raw === p.raw);
        if (idx < 0) return [...points, p];
        const next = points.slice();
        next[idx] = p;
        return next;
      };
      return {
        ...prev,
        current_ch1_points:
          curve === "current_ch1"
            ? upsert(prev.current_ch1_points, point)
            : prev.current_ch1_points,
        current_ch2_points:
          curve === "current_ch2"
            ? upsert(prev.current_ch2_points, point)
            : prev.current_ch2_points,
      };
    });
  };

  const handleDeleteSample = (index: number) => {
    const removed = draftPoints[index];
    if (!removed) return;
    onEnqueueUndo(
      { kind: "current_point", curve, point: removed },
      `Deleted current sample #${index + 1} (raw=${removed.raw})`,
    );
    onSetDraftProfile((prev) => ({
      ...prev,
      current_ch1_points:
        curve === "current_ch1"
          ? prev.current_ch1_points.filter((_, i) => i !== index)
          : prev.current_ch1_points,
      current_ch2_points:
        curve === "current_ch2"
          ? prev.current_ch2_points.filter((_, i) => i !== index)
          : prev.current_ch2_points,
    }));
  };

  const activeMa =
    curve === "current_ch1" ? status?.raw.i_local_ma : status?.raw.i_remote_ma;
  const previewMa =
    status?.raw.raw_cur_100uv != null && previewPoints.length >= 1
      ? piecewiseLinear(
          previewPoints.map((point) => ({ x: point.raw, y: point.ma })),
          status.raw.raw_cur_100uv,
        )
      : null;

  const readMutation = useMutation({
    mutationFn: async () => onRefetchProfile(),
  });

  const applyToDeviceMutation = useMutation({
    mutationFn: async () => {
      if (draftPoints.length === 0) {
        throw new Error("Draft is empty. Nothing to sync.");
      }
      const validated = validateAndNormalizeCurrentPoints(curve, draftPoints);
      if (validated.issues.length > 0) {
        throw new Error(
          `Draft validation failed: ${validated.issues[0].message}`,
        );
      }
      return postCalibrationApply(baseUrl, {
        kind: curve,
        points: validated.normalized,
      });
    },
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const commitToDeviceMutation = useMutation({
    mutationFn: async () => {
      if (draftPoints.length === 0) {
        throw new Error("Draft is empty. Nothing to sync.");
      }
      const validated = validateAndNormalizeCurrentPoints(curve, draftPoints);
      if (validated.issues.length > 0) {
        throw new Error(
          `Draft validation failed: ${validated.issues[0].message}`,
        );
      }
      return postCalibrationCommit(baseUrl, {
        kind: curve,
        points: validated.normalized,
      });
    },
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const resetDeviceCurrentMutation = useMutation({
    mutationFn: async () => postCalibrationReset(baseUrl, { kind: curve }),
    onSuccess: async () => {
      await onRefetchProfile();
      onResetDraftToEmpty("Device reset to defaults. Draft cleared.");
    },
  });

  return (
    <>
      <div role="tablist" className="tabs tabs-boxed mt-4">
        <button
          type="button"
          role="tab"
          className={`tab ${viewTab === "draft" ? "tab-active" : ""}`}
          onClick={() => setViewTab("draft")}
        >
          本地草稿
        </button>
        <button
          type="button"
          role="tab"
          className={`tab ${viewTab === "device" ? "tab-active" : ""}`}
          onClick={() => setViewTab("device")}
        >
          设备数据
        </button>
      </div>

      {viewTab === "draft" ? (
        <div className="card bg-base-100 shadow-xl border border-base-200 mt-4">
          <div className="card-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="card-title flex flex-col items-start leading-tight">
                <span>本地草稿</span>
                <span className="text-sm font-normal text-base-content/60">
                  Web
                </span>
              </h3>
            </div>

            <div className="card bg-base-200/40 border border-base-200">
              <div className="card-body py-4 gap-3">
                <div className="flex items-start justify-between gap-3">
                  <h4 className="font-bold text-sm">仅本地（不读写设备）</h4>
                  <div className="badge badge-neutral whitespace-nowrap shrink-0">
                    不读写设备
                  </div>
                </div>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={() => {
                      onSetPreviewProfile(structuredClone(draftProfile));
                      onSetPreviewAppliedAt(Date.now());
                    }}
                    disabled={isDraftEmpty(draftProfile)}
                  >
                    Apply Preview
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={() => setConfirmKind("reset_draft")}
                    disabled={isDraftEmpty(draftProfile)}
                  >
                    Reset Draft
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={onExportDraft}
                    disabled={isDraftEmpty(draftProfile)}
                    title={
                      isDraftEmpty(draftProfile)
                        ? "Export is disabled when the draft is empty."
                        : undefined
                    }
                  >
                    Export
                  </button>
                  <label
                    htmlFor={`calibration-import-${deviceId}-current`}
                    className="btn btn-sm btn-outline"
                  >
                    Import
                  </label>
                  <input
                    id={`calibration-import-${deviceId}-current`}
                    type="file"
                    accept="application/json"
                    className="hidden"
                    onChange={(event) => {
                      const file = event.currentTarget.files?.[0] ?? null;
                      void onImportDraftFile(file);
                      event.currentTarget.value = "";
                    }}
                  />
                </div>
              </div>
            </div>

            <div className="card bg-base-200/40 border border-base-200">
              <div className="card-body py-4 gap-3">
                <div className="flex items-start justify-between gap-3">
                  <h4 className="font-bold text-sm">硬件 I/O</h4>
                  <div className="flex items-center gap-2">
                    <div className="badge badge-info whitespace-nowrap">
                      读设备
                    </div>
                    <div className="badge badge-warning whitespace-nowrap">
                      写设备
                    </div>
                  </div>
                </div>

                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={onReadDeviceToDraft}
                    disabled={isOffline || readDeviceToDraftPending}
                    title={
                      isOffline
                        ? "Device offline/faulted."
                        : readDeviceToDraftPending
                          ? "Reading device profile..."
                          : "Reads device calibration profile and overwrites the local draft."
                    }
                  >
                    Read Device → Draft
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={() => applyToDeviceMutation.mutate()}
                    disabled={
                      !canWriteToDevice || applyToDeviceMutation.isPending
                    }
                  >
                    Apply
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-secondary"
                    onClick={() => commitToDeviceMutation.mutate()}
                    disabled={
                      !canWriteToDevice || commitToDeviceMutation.isPending
                    }
                  >
                    Commit
                  </button>
                </div>

                <div className="card bg-base-200/40 border border-base-200">
                  <div className="card-body py-4 gap-3">
                    <h4 className="font-bold text-sm">Output control (CC)</h4>
                    <div className="flex gap-2 flex-wrap">
                      <button
                        type="button"
                        className="btn btn-xs"
                        onClick={() => setTargetIMa("500")}
                      >
                        0.5A
                      </button>
                      <button
                        type="button"
                        className="btn btn-xs"
                        onClick={() => setTargetIMa("1000")}
                      >
                        1A
                      </button>
                      <button
                        type="button"
                        className="btn btn-xs"
                        onClick={() => setTargetIMa("2000")}
                      >
                        2A
                      </button>
                      <button
                        type="button"
                        className="btn btn-xs"
                        onClick={() => setTargetIMa("3000")}
                      >
                        3A
                      </button>
                      <button
                        type="button"
                        className="btn btn-xs"
                        onClick={() => setTargetIMa("4000")}
                      >
                        4A
                      </button>
                      <button
                        type="button"
                        className="btn btn-xs"
                        onClick={() => setTargetIMa("5000")}
                      >
                        5A
                      </button>
                      <input
                        type="number"
                        className="input input-sm input-bordered w-28"
                        value={targetIMa}
                        onChange={(event) => setTargetIMa(event.target.value)}
                        disabled={isOffline}
                      />
                      <button
                        type="button"
                        className="btn btn-sm btn-primary"
                        disabled={isOffline}
                        onClick={handleSetOutput}
                      >
                        Set Output
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="divider my-0"></div>

            <label className="form-control w-full">
              <div className="label">
                <span className="label-text">
                  Meter Reading ({channelDisplay}) (A)
                </span>
              </div>
              <div className="join">
                <input
                  type="number"
                  step="0.001"
                  className="input input-bordered join-item w-full"
                  value={meterReadingA}
                  onChange={(event) => setMeterReadingA(event.target.value)}
                  disabled={isOffline}
                />
                <button
                  type="button"
                  className="btn btn-secondary join-item"
                  onClick={handleCapture}
                  disabled={isOffline}
                >
                  Capture
                </button>
              </div>
            </label>

            {currentDraft.issues.length > 0 && draftPoints.length > 0 && (
              <div role="alert" className="alert alert-warning text-sm py-2">
                <span>
                  Draft validation:{" "}
                  <span className="font-bold">
                    {currentDraft.issues[0].message}
                  </span>
                  {currentDraft.issues.length > 1
                    ? ` (+${currentDraft.issues.length - 1} more)`
                    : ""}
                </span>
              </div>
            )}

            <div className="stats shadow">
              <div className="stat">
                <div className="stat-title">Active Current</div>
                <div className="stat-value text-lg">
                  {(((activeMa ?? 0) / 1000) as number).toFixed(4)} A
                </div>
                <div className="stat-desc">
                  Raw: {status?.raw.raw_cur_100uv ?? "--"}
                </div>
              </div>
              <div className="stat">
                <div className="stat-title">DAC Code</div>
                <div className="stat-value text-lg font-mono">
                  {status?.raw.raw_dac_code ?? "--"}
                </div>
              </div>
              <div className="stat">
                <div className="stat-title">Preview Current</div>
                <div className="stat-value text-lg text-primary">
                  {previewMa == null
                    ? "--"
                    : `${(previewMa / 1000).toFixed(4)} A`}
                </div>
                <div className="stat-desc">Uses applied preview</div>
              </div>
            </div>

            <div className="overflow-x-auto max-h-64">
              <table className="table table-xs table-pin-rows">
                <thead>
                  <tr>
                    <th>Raw</th>
                    <th>DAC</th>
                    <th>Value (mA)</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {draftPoints.map((sample, idx) => (
                    <tr
                      key={`${idx}-${sample.raw}-${sample.ma}-${sample.dac_code}`}
                    >
                      <td>{sample.raw}</td>
                      <td>{sample.dac_code ?? "--"}</td>
                      <td>{sample.ma}</td>
                      <td className="text-right">
                        <button
                          type="button"
                          className="btn btn-ghost btn-xs text-error"
                          onClick={() => handleDeleteSample(idx)}
                          disabled={isOffline}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                  {draftPoints.length === 0 && (
                    <tr>
                      <td
                        colSpan={4}
                        className="text-center text-base-content/50"
                      >
                        No draft points.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      ) : (
        <div className="card bg-base-100 shadow-xl border border-base-200 mt-4">
          <div className="card-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="card-title flex flex-col items-start leading-tight">
                <span>设备数据</span>
                <span className="text-sm font-normal text-base-content/60">
                  Hardware
                </span>
              </h3>
              <div className="flex items-center gap-2">
                <div className="badge badge-info whitespace-nowrap">读设备</div>
                <div className="badge badge-warning whitespace-nowrap">
                  写设备
                </div>
              </div>
            </div>

            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                className="btn btn-sm btn-outline"
                onClick={() => readMutation.mutate()}
                disabled={readMutation.isPending}
              >
                Read
              </button>
              <button
                type="button"
                className="btn btn-sm btn-error"
                onClick={() => setConfirmKind("reset_device_current")}
                disabled={isOffline || resetDeviceCurrentMutation.isPending}
              >
                Reset
              </button>
            </div>

            <div className="divider my-0"></div>

            <h4 className="font-bold text-sm">
              {deviceProfile?.active.source === "factory-default"
                ? "Device defaults (factory reference, read-only)"
                : "Device profile (read-only)"}
            </h4>
            <div className="overflow-x-auto max-h-64">
              <table className="table table-xs table-pin-rows">
                <thead>
                  <tr>
                    <th>Raw</th>
                    <th>DAC</th>
                    <th>Value (mA)</th>
                  </tr>
                </thead>
                <tbody>
                  {devicePoints.map((point, idx) => (
                    <tr key={`${point.raw}-${point.ma}-${idx}`}>
                      <td>{point.raw}</td>
                      <td>{point.dac_code ?? "--"}</td>
                      <td>{point.ma}</td>
                    </tr>
                  ))}
                  {devicePoints.length === 0 && (
                    <tr>
                      <td
                        colSpan={3}
                        className="text-center text-base-content/50"
                      >
                        No device profile loaded.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      <ConfirmDialog
        open={confirmKind !== null}
        title={
          confirmKind === "reset_draft"
            ? "Reset Draft (Web only)"
            : `Reset Device Calibration (Current ${channelLabel})`
        }
        body={
          confirmKind === "reset_draft"
            ? "This clears the local draft (user calibration points). The device is unchanged."
            : "This resets current calibration on the device."
        }
        details={
          confirmKind === "reset_draft"
            ? [
                "Affects: v_local, v_remote, current_ch1, current_ch2 (local draft only).",
                "Writes device: No.",
                "This clears all local draft points (export first if needed).",
              ]
            : [
                `Affects: ${curve}.`,
                "Writes device: Yes.",
                "Irreversible: Yes (re-calibrate + commit to recover).",
              ]
        }
        confirmLabel={confirmKind === "reset_draft" ? "Reset Draft" : "Reset"}
        destructive={confirmKind === "reset_device_current"}
        confirmDisabled={
          confirmKind === "reset_draft"
            ? isDraftEmpty(draftProfile)
            : resetDeviceCurrentMutation.isPending || isOffline
        }
        onCancel={() => setConfirmKind(null)}
        onConfirm={() => {
          if (confirmKind === "reset_draft") {
            onResetDraftToEmpty();
          } else if (confirmKind === "reset_device_current") {
            resetDeviceCurrentMutation.mutate();
          }
          setConfirmKind(null);
        }}
      />
    </>
  );
}

function ConfirmDialog({
  open,
  title,
  body,
  details,
  confirmLabel,
  destructive,
  confirmDisabled,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  body: string;
  details: string[];
  confirmLabel: string;
  destructive: boolean;
  confirmDisabled: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  if (!open) return null;

  return (
    <div className="modal modal-open" role="dialog" aria-modal="true">
      <div className="modal-box">
        <h3 className="font-bold text-lg">{title}</h3>
        <p className="py-3 text-sm">{body}</p>
        <ul className="list-disc pl-5 text-sm space-y-1">
          {details.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>

        <div className="modal-action">
          <button type="button" className="btn" onClick={onCancel}>
            Cancel
          </button>
          <button
            type="button"
            className={`btn ${destructive ? "btn-error" : "btn-primary"}`}
            disabled={confirmDisabled}
            onClick={onConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Close"
        onClick={onCancel}
      />
    </div>
  );
}

function AlertDialog({
  open,
  title,
  body,
  details,
  onClose,
}: {
  open: boolean;
  title: string;
  body: string;
  details: string[];
  onClose: () => void;
}) {
  if (!open) return null;

  return (
    <div className="modal modal-open" role="dialog" aria-modal="true">
      <div className="modal-box">
        <h3 className="font-bold text-lg">{title}</h3>
        <p className="py-3 text-sm">{body}</p>
        {details.length > 0 && (
          <ul className="list-disc pl-5 text-sm space-y-1">
            {details.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        )}

        <div className="modal-action">
          <button type="button" className="btn btn-primary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Close"
        onClick={onClose}
      />
    </div>
  );
}

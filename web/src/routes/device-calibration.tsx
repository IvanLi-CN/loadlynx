import { useQuery } from "@tanstack/react-query";
import { useRouterState } from "@tanstack/react-router";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  getCalibrationProfile,
  getStatus,
  type HttpApiError,
  isMockBaseUrl,
  postCalibrationMode,
  subscribeStatusStream,
} from "../api/client.ts";
import type {
  CalibrationModeRequest,
  CalibrationPointCurrent,
  CalibrationPointVoltage,
  CalibrationProfile,
  FastStatusView,
} from "../api/types.ts";
import {
  calibrationProfilesPointsEqual,
  type ValidationIssue,
  validateAndNormalizeCurrentPoints,
  validateAndNormalizeVoltagePoints,
} from "../calibration/validation.ts";
import { AlertDialog } from "../components/common/alert-dialog.tsx";
import { ConfirmDialog } from "../components/common/confirm-dialog.tsx";
import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import { CurrentCalibrationPanel } from "./device-calibration/current-calibration-panel.tsx";
import {
  type CalibrationTab,
  DEFAULT_ACTIVE_PROFILE,
  formatDeviceCalKind,
  formatLocalTimestamp,
  isDeviceSubroutePath,
  isDraftEmpty,
  makeEmptyDraftProfile,
  type RefetchProfile,
  readCalibrationDraftFromStorage,
  retryDeviceCall,
  type StoredCalibrationDraftV4,
  statusInExpectedCalMode,
  type UndoAction,
  type WithStatusStreamPaused,
  writeCalibrationDraftToStorage,
} from "./device-calibration/shared.ts";
import { VoltageCalibrationPanel } from "./device-calibration/voltage-calibration-panel.tsx";

const CALIBRATION_STATUS_FALLBACK_REFETCH_MS = 500;
const CALIBRATION_STATUS_STALE_TIMEOUT_MS = 2_500;
const CALIBRATION_STATUS_STREAM_STARTUP_TIMEOUT_MS = 1_500;
const calibrationStatusRetryDelay = () => 200 + Math.random() * 300;

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

interface StatusWaiter {
  id: string;
  predicate: (view: FastStatusView) => boolean;
  resolve: (view: FastStatusView) => void;
  reject: (error: Error) => void;
  timeoutId: number;
}

function makeUndoId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `undo-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function expectedCalKindForTab(tab: CalibrationTab): number {
  switch (tab) {
    case "voltage":
      return 1;
    case "current_ch1":
      return 2;
    case "current_ch2":
      return 3;
  }
}

export function DeviceCalibrationRoute() {
  const { deviceId, baseUrl } = useDeviceContext();
  return <DeviceCalibrationPage deviceId={deviceId} baseUrl={baseUrl} />;
}

function DeviceCalibrationPage({
  deviceId,
  baseUrl,
}: {
  deviceId: string;
  baseUrl: string;
}) {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const [activeTab, setActiveTab] = useState<CalibrationTab>(() => {
    if (typeof window === "undefined") {
      return "voltage";
    }
    return (
      readCalibrationDraftFromStorage(deviceId, baseUrl)?.active_tab ??
      "voltage"
    );
  });
  const latestPathnameRef = useRef(pathname);
  latestPathnameRef.current = pathname;
  const [isPageVisible, setIsPageVisible] = useState(() =>
    typeof document === "undefined"
      ? true
      : document.visibilityState === "visible",
  );
  const previousCalibrationBaseUrlRef = useRef<string | null>(null);

  useEffect(() => {
    if (typeof document === "undefined") {
      return undefined;
    }

    const handleVisibility = () => {
      setIsPageVisible(document.visibilityState === "visible");
    };

    document.addEventListener("visibilitychange", handleVisibility);
    return () => {
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, []);

  useEffect(() => {
    const previousBaseUrl = previousCalibrationBaseUrlRef.current;
    previousCalibrationBaseUrlRef.current = baseUrl;
    if (
      !previousBaseUrl ||
      previousBaseUrl === baseUrl ||
      isMockBaseUrl(previousBaseUrl)
    ) {
      return;
    }

    postCalibrationMode(previousBaseUrl, { kind: "off" }).catch(() => {
      // Best-effort cleanup when switching devices.
    });
  }, [baseUrl]);

  useEffect(() => {
    return () => {
      if (isMockBaseUrl(baseUrl)) {
        return;
      }

      const nextPathname = latestPathnameRef.current;
      if (
        isDeviceSubroutePath(nextPathname, deviceId) &&
        !/\/calibration$/.test(nextPathname)
      ) {
        return;
      }

      postCalibrationMode(baseUrl, { kind: "off" }).catch(() => {
        // Best-effort cleanup on route exit.
      });
    };
  }, [baseUrl, deviceId]);

  const [status, setStatus] = useState<FastStatusView | null>(null);
  const [statusStreamPaused, setStatusStreamPaused] = useState(false);
  const [statusStreamConnected, setStatusStreamConnected] = useState(false);
  const [statusFallbackArmed, setStatusFallbackArmed] = useState(false);
  const statusRef = useRef<FastStatusView | null>(status);
  const lastStatusAtRef = useRef<number | null>(null);
  const statusWaitersRef = useRef<StatusWaiter[]>([]);

  const publishStatusSnapshot = useCallback((view: FastStatusView | null) => {
    statusRef.current = view;
    if (view) {
      lastStatusAtRef.current = Date.now();
      const remaining: StatusWaiter[] = [];
      for (const waiter of statusWaitersRef.current) {
        if (waiter.predicate(view)) {
          window.clearTimeout(waiter.timeoutId);
          waiter.resolve(view);
        } else {
          remaining.push(waiter);
        }
      }
      statusWaitersRef.current = remaining;
    }
    setStatus(view);
  }, []);

  const statusPauseDepthRef = useRef(0);
  const withStatusStreamPaused = useCallback<WithStatusStreamPaused>(
    async (op) => {
      if (isMockBaseUrl(baseUrl)) {
        return await op();
      }

      statusPauseDepthRef.current += 1;
      if (statusPauseDepthRef.current === 1) {
        setStatusStreamPaused(true);
        await new Promise((resolve) => window.setTimeout(resolve, 350));
      }
      try {
        return await op();
      } finally {
        statusPauseDepthRef.current -= 1;
        if (statusPauseDepthRef.current === 0) {
          setStatusStreamPaused(false);
        }
      }
    },
    [baseUrl],
  );

  const rejectStatusWaiters = useCallback((error: Error) => {
    for (const waiter of statusWaitersRef.current) {
      window.clearTimeout(waiter.timeoutId);
      waiter.reject(error);
    }
    statusWaitersRef.current = [];
  }, []);

  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  const waitForStatus = useCallback(
    (
      predicate: (view: FastStatusView) => boolean,
      timeoutMs: number,
    ): Promise<FastStatusView> => {
      const current = statusRef.current;
      if (current && predicate(current)) {
        return Promise.resolve(current);
      }
      return new Promise((resolve, reject) => {
        const id = makeUndoId();
        const timeoutId = window.setTimeout(
          () => {
            statusWaitersRef.current = statusWaitersRef.current.filter(
              (waiter) => waiter.id !== id,
            );
            reject(new Error("Timed out waiting for device status"));
          },
          Math.max(0, timeoutMs),
        );
        statusWaitersRef.current.push({
          id,
          predicate,
          resolve,
          reject,
          timeoutId,
        });
      });
    },
    [],
  );

  useEffect(() => {
    lastStatusAtRef.current = null;
    publishStatusSnapshot(null);
    setStatusStreamConnected(false);
    setStatusFallbackArmed(false);
    rejectStatusWaiters(new Error(`Status stream reset for ${baseUrl}`));
  }, [baseUrl, publishStatusSnapshot, rejectStatusWaiters]);

  useEffect(() => {
    if (statusStreamPaused) {
      return undefined;
    }

    const unsubscribe = subscribeStatusStream(
      baseUrl,
      (view) => {
        setStatusStreamConnected(true);
        setStatusFallbackArmed(false);
        publishStatusSnapshot(view);
      },
      () => {
        setStatusStreamConnected(false);
        setStatusFallbackArmed(true);
      },
    );

    return () => {
      unsubscribe();
      setStatusStreamConnected(false);
      rejectStatusWaiters(new Error("Status stream closed"));
    };
  }, [baseUrl, publishStatusSnapshot, rejectStatusWaiters, statusStreamPaused]);

  useEffect(() => {
    if (statusStreamPaused || statusStreamConnected || statusFallbackArmed) {
      return;
    }

    const timeoutId = window.setTimeout(() => {
      if (statusPauseDepthRef.current === 0 && !statusStreamConnected) {
        setStatusFallbackArmed(true);
      }
    }, CALIBRATION_STATUS_STREAM_STARTUP_TIMEOUT_MS);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [statusFallbackArmed, statusStreamConnected, statusStreamPaused]);

  const statusFallbackQuery = useQuery<FastStatusView, HttpApiError>({
    queryKey: ["device", deviceId, baseUrl, "status", "calibration-fallback"],
    queryFn: () => getStatus(baseUrl),
    enabled:
      Boolean(baseUrl) &&
      isPageVisible &&
      statusFallbackArmed &&
      !statusStreamPaused &&
      !statusStreamConnected,
    refetchInterval: isPageVisible
      ? CALIBRATION_STATUS_FALLBACK_REFETCH_MS
      : false,
    refetchIntervalInBackground: false,
    refetchOnWindowFocus: false,
    retry: 2,
    retryDelay: calibrationStatusRetryDelay,
  });

  useEffect(() => {
    if (!statusFallbackQuery.data || statusFallbackQuery.dataUpdatedAt === 0) {
      return;
    }
    publishStatusSnapshot(statusFallbackQuery.data);
  }, [
    publishStatusSnapshot,
    statusFallbackQuery.data,
    statusFallbackQuery.dataUpdatedAt,
  ]);

  useEffect(() => {
    if (
      statusStreamPaused ||
      statusStreamConnected ||
      !statusFallbackQuery.isError ||
      statusFallbackQuery.fetchStatus === "fetching"
    ) {
      return;
    }

    const lastStatusAt = lastStatusAtRef.current;
    if (lastStatusAt === null) {
      publishStatusSnapshot(null);
      return;
    }

    const remainingMs =
      CALIBRATION_STATUS_STALE_TIMEOUT_MS - (Date.now() - lastStatusAt);
    if (remainingMs <= 0) {
      lastStatusAtRef.current = null;
      publishStatusSnapshot(null);
      return;
    }

    const timeoutId = window.setTimeout(() => {
      if (
        statusPauseDepthRef.current === 0 &&
        !statusStreamConnected &&
        statusFallbackQuery.isError &&
        statusFallbackQuery.fetchStatus !== "fetching"
      ) {
        lastStatusAtRef.current = null;
        publishStatusSnapshot(null);
      }
    }, remainingMs);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [
    publishStatusSnapshot,
    statusFallbackQuery.fetchStatus,
    statusFallbackQuery.isError,
    statusStreamConnected,
    statusStreamPaused,
  ]);

  const isOffline =
    status === null ||
    status.analog_state === "offline" ||
    status.analog_state === "faulted";
  const deviceCalKind = status?.raw.cal_kind ?? null;
  const expectedCalKind = expectedCalKindForTab(activeTab);
  const statusMatchesActiveTab = statusInExpectedCalMode(
    status,
    expectedCalKind,
  );

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

  const enqueueInfoToast = useCallback((message: string) => {
    const id = makeUndoId();
    const timeoutId = window.setTimeout(() => {
      setInfoToasts((prev) => prev.filter((toast) => toast.id !== id));
    }, 2_500);
    setInfoToasts((prev) => [...prev, { id, message, timeoutId }]);
  }, []);

  const applyUndoAction = useCallback((action: UndoAction) => {
    if (action.kind === "voltage_points") {
      setDraftProfile((prev) => ({
        ...prev,
        v_local_points: action.local
          ? [...prev.v_local_points, action.local]
          : prev.v_local_points,
        v_remote_points: action.remote
          ? [...prev.v_remote_points, action.remote]
          : prev.v_remote_points,
      }));
      return;
    }

    setDraftProfile((prev) => ({
      ...prev,
      current_ch1_points:
        action.curve === "current_ch1"
          ? [...prev.current_ch1_points, action.point]
          : prev.current_ch1_points,
      current_ch2_points:
        action.curve === "current_ch2"
          ? [...prev.current_ch2_points, action.point]
          : prev.current_ch2_points,
    }));
  }, []);

  const undoToast = useCallback(
    (toast: UndoToastEntry) => {
      window.clearTimeout(toast.timeoutId);
      setUndoToasts((prev) => prev.filter((entry) => entry.id !== toast.id));
      applyUndoAction(toast.action);
    },
    [applyUndoAction],
  );

  const enqueueUndo = useCallback((action: UndoAction, message: string) => {
    const id = makeUndoId();
    const expiresAt = Date.now() + 5_000;
    const timeoutId = window.setTimeout(() => {
      setUndoToasts((prev) => prev.filter((toast) => toast.id !== id));
    }, 5_000);
    setUndoToasts((prev) => [
      ...prev,
      { id, message, action, expiresAt, timeoutId },
    ]);
  }, []);

  const resetDraftToEmpty = useCallback(
    (message = "Draft cleared.") => {
      clearToasts();
      writeCalibrationDraftToStorage(deviceId, baseUrl, null);
      setDraftProfile(makeEmptyDraftProfile(profileQuery.data?.active));
      setPreviewProfile(null);
      setPreviewAppliedAt(null);
      setImportError(null);
      setImportIssues(null);
      enqueueInfoToast(message);
    },
    [
      baseUrl,
      clearToasts,
      deviceId,
      enqueueInfoToast,
      profileQuery.data?.active,
    ],
  );

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

  const handleExportDraft = useCallback(() => {
    if (isDraftEmpty(draftProfile)) {
      return;
    }

    const now = new Date();
    const stamp = now.toISOString().replaceAll(":", "-");
    const payload = {
      schema_version: 3,
      generated_at: now.toISOString(),
      device_id: deviceId,
      active_snapshot: profileQuery.data?.active ?? draftProfile.active,
      curves: {
        v_local_points: draftProfile.v_local_points.map((point) => [
          point.raw,
          point.mv,
        ]),
        v_remote_points: draftProfile.v_remote_points.map((point) => [
          point.raw,
          point.mv,
        ]),
        current_ch1_points: draftProfile.current_ch1_points.map((point) => [
          [point.raw, point.dac_code],
          point.ua,
        ]),
        current_ch2_points: draftProfile.current_ch2_points.map((point) => [
          [point.raw, point.dac_code],
          point.ua,
        ]),
      },
    };

    const blob = new Blob([JSON.stringify(payload, null, 2)], {
      type: "application/json; charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `loadlynx-calibration-draft-${deviceId}-${stamp}.json`;
    anchor.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 250);
  }, [deviceId, draftProfile, profileQuery.data?.active]);

  const handleImportDraftFile = useCallback(
    async (file: File | null) => {
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
        setImportError("Missing curves object in JSON.");
        return;
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

      setDraftProfile(nextProfile);
      setPreviewProfile(null);
      setPreviewAppliedAt(null);
      setImportError(null);
      setImportIssues(null);
      enqueueInfoToast("Imported calibration draft.");
    },
    [
      clearToasts,
      draftProfile.active,
      enqueueInfoToast,
      profileQuery.data?.active,
    ],
  );

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

  const showAlert = useCallback(
    (title: string, body: string, details: string[] = []) => {
      setAlertDialog({ title, body, details });
    },
    [],
  );

  const performReadDeviceToDraft = useCallback(async () => {
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
      const nextProfile = result.data;
      if (!nextProfile) {
        throw new Error("No device profile loaded.");
      }

      setDraftProfile(nextProfile);
      setPreviewProfile(null);
      setPreviewAppliedAt(null);

      enqueueInfoToast(
        isDraftEmpty(nextProfile)
          ? "Device profile is empty; draft cleared."
          : "Loaded device profile into draft.",
      );
    } catch (error) {
      setAlertDialog({
        title: "Failed to Read Device Profile",
        body: String(error),
        details: [],
      });
    } finally {
      setReadDeviceToDraftPending(false);
    }
  }, [
    clearToasts,
    enqueueInfoToast,
    isOffline,
    profileQuery,
    readDeviceToDraftPending,
  ]);

  const requestReadDeviceToDraft = useCallback(() => {
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
  }, [draftEmpty, isOffline, performReadDeviceToDraft]);

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

  useLayoutEffect(() => {
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
  }, [baseUrl, clearToasts, deviceId]);

  useEffect(() => {
    if (!draftStorageReady) return;

    if (isDraftEmpty(draftProfile)) {
      writeCalibrationDraftToStorage(deviceId, baseUrl, null);
      return;
    }

    const nextDraft: StoredCalibrationDraftV4 = {
      version: 4,
      saved_at: new Date().toISOString(),
      device_id: deviceId,
      base_url: baseUrl,
      active_tab: activeTab,
      draft_profile: {
        v_local_points: draftProfile.v_local_points.map((point) => [
          point.raw,
          point.mv,
        ]),
        v_remote_points: draftProfile.v_remote_points.map((point) => [
          point.raw,
          point.mv,
        ]),
        current_ch1_points: draftProfile.current_ch1_points.map((point) => [
          [point.raw, point.dac_code],
          point.ua,
        ]),
        current_ch2_points: draftProfile.current_ch2_points.map((point) => [
          [point.raw, point.dac_code],
          point.ua,
        ]),
      },
    };
    writeCalibrationDraftToStorage(deviceId, baseUrl, nextDraft);
  }, [activeTab, baseUrl, deviceId, draftProfile, draftStorageReady]);

  const modeSyncInFlightRef = useRef<Promise<void> | null>(null);
  const ensureActiveTabCalMode = useCallback(
    async (action: string, opts?: { silent?: boolean }): Promise<boolean> => {
      if (modeSyncInFlightRef.current) {
        try {
          await modeSyncInFlightRef.current;
        } catch {
          // ignore
        }
      }

      if (statusRef.current !== null && isOffline) {
        return false;
      }

      const already = statusRef.current?.raw.cal_kind ?? null;
      if (already === expectedCalKind) {
        return true;
      }

      const kind: CalibrationModeRequest["kind"] =
        activeTab === "voltage" ? "voltage" : activeTab;

      let snapshotAfterCalKind: number | null = null;
      const attempt = (async (): Promise<void> => {
        await withStatusStreamPaused(async () => {
          await retryDeviceCall(() => postCalibrationMode(baseUrl, { kind }), {
            attempts: 4,
            firstDelayMs: 120,
            maxDelayMs: 600,
          });

          try {
            const snapshot = await retryDeviceCall(() => getStatus(baseUrl), {
              attempts: 2,
              firstDelayMs: 80,
              maxDelayMs: 300,
            });
            snapshotAfterCalKind = snapshot.raw.cal_kind ?? null;
            publishStatusSnapshot(snapshot);
          } catch {
            // keep waiting for the stream/fallback path
          }
        });
      })();

      modeSyncInFlightRef.current = attempt;
      try {
        await attempt;
      } catch {
        if (!opts?.silent) {
          showAlert(
            `Cannot ${action}`,
            "Failed to set device calibration mode. Check network/API availability.",
          );
        }
        return false;
      } finally {
        if (modeSyncInFlightRef.current === attempt) {
          modeSyncInFlightRef.current = null;
        }
      }

      if (snapshotAfterCalKind === expectedCalKind) {
        return true;
      }

      try {
        await waitForStatus(
          (view) => (view.raw.cal_kind ?? null) === expectedCalKind,
          1500,
        );
        return true;
      } catch {
        try {
          const snapshot = await retryDeviceCall(() => getStatus(baseUrl), {
            attempts: 2,
            firstDelayMs: 100,
            maxDelayMs: 400,
          });
          publishStatusSnapshot(snapshot);
          if ((snapshot.raw.cal_kind ?? null) === expectedCalKind) {
            return true;
          }
        } catch {
          // ignore final readback failure
        }

        if (!opts?.silent) {
          const seen = statusRef.current?.raw.cal_kind ?? null;
          showAlert(
            "Calibration mode mismatch",
            "Device did not switch to the expected calibration mode.",
            [
              `expected=${formatDeviceCalKind(expectedCalKind)}`,
              `device=${formatDeviceCalKind(seen)}`,
            ],
          );
        }
        return false;
      }
    },
    [
      activeTab,
      baseUrl,
      expectedCalKind,
      isOffline,
      publishStatusSnapshot,
      showAlert,
      waitForStatus,
      withStatusStreamPaused,
    ],
  );

  useEffect(() => {
    void ensureActiveTabCalMode("Sync", { silent: true });
  }, [ensureActiveTabCalMode]);

  return (
    <PageContainer variant="full" className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Calibration</h2>
        <div className="ll-badge ll-badge-neutral gap-2">
          {isOffline ? "OFFLINE / FAULT" : "ONLINE"}
        </div>
      </div>

      <div className="ll-panel bg-base-100 shadow-xl border border-base-200">
        <div className="ll-panel-body gap-3">
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
              <div
                className={`ll-badge ${statusMatchesActiveTab ? "ll-badge-success" : "ll-badge-warning"}`}
                title={`device=${formatDeviceCalKind(deviceCalKind)} expected=${formatDeviceCalKind(expectedCalKind)}`}
              >
                cal_mode: {formatDeviceCalKind(deviceCalKind)}
              </div>
              {deviceCalKind !== expectedCalKind && !isOffline ? (
                <button
                  type="button"
                  className="ll-button ll-button-xs ll-button-ghost"
                  onClick={() => {
                    void ensureActiveTabCalMode("Sync");
                  }}
                >
                  Sync
                </button>
              ) : null}

              {draftEmpty ? (
                <div className="ll-badge ll-badge-neutral">Draft: none</div>
              ) : (
                <div className="ll-badge ll-badge-warning">
                  Draft: needs sync
                </div>
              )}

              {profileQuery.data ? (
                deviceUsingDefaults ? (
                  <div className="ll-badge ll-badge-success">
                    Device: defaults
                  </div>
                ) : (
                  <div className="ll-badge ll-badge-info">
                    Device: user-calibrated
                  </div>
                )
              ) : (
                <div className="ll-badge ll-badge-neutral">Device: --</div>
              )}

              {draftIssues.length > 0 ? (
                <div className="ll-badge ll-badge-error">
                  Draft issues ({draftIssues.length})
                </div>
              ) : !draftEmpty ? (
                <div className="ll-badge ll-badge-success">Draft OK</div>
              ) : null}

              {!previewProfile ? (
                <div className="ll-badge ll-badge-neutral">Preview: device</div>
              ) : previewMatchesDraft ? (
                <div className="ll-badge ll-badge-neutral">
                  Preview up to date
                </div>
              ) : (
                <div className="ll-badge ll-badge-warning">
                  Preview out of date
                </div>
              )}

              {previewAppliedAt ? (
                <div className="ll-badge ll-badge-ghost">
                  Preview applied {formatLocalTimestamp(previewAppliedAt)}
                </div>
              ) : null}
            </div>
          </div>

          {importError ? (
            <div role="alert" className="ll-alert ll-alert-error text-sm py-2">
              <div className="flex flex-col gap-2">
                <div className="font-bold">{importError}</div>
                {importIssues && importIssues.length > 0 ? (
                  <ul className="list-disc pl-5">
                    {importIssues.slice(0, 5).map((issue) => (
                      <li key={`${issue.path}:${issue.message}`}>
                        <span className="font-mono">{issue.path}</span>:{" "}
                        {issue.message}
                      </li>
                    ))}
                  </ul>
                ) : null}
              </div>
            </div>
          ) : null}

          {!isOffline && !statusMatchesActiveTab ? (
            <output className="ll-alert ll-alert-info text-sm py-2">
              <span>
                正在同步校准模式：等待设备切换到{" "}
                <span className="font-mono">
                  {formatDeviceCalKind(expectedCalKind)}
                </span>
                。在同步完成前，RAW / DAC
                会保持占位，避免把旧模式的数据误显示到当前页签。
              </span>
            </output>
          ) : null}
        </div>
      </div>

      <div role="tablist" className="ll-tabs mt-4">
        <button
          type="button"
          role="tab"
          className={`ll-tab ${activeTab === "voltage" ? "ll-tab-active" : ""}`}
          onClick={() => setActiveTab("voltage")}
        >
          电压
        </button>
        <button
          type="button"
          role="tab"
          className={`ll-tab ${activeTab === "current_ch1" ? "ll-tab-active" : ""}`}
          onClick={() => setActiveTab("current_ch1")}
        >
          电流通道1
        </button>
        <button
          type="button"
          role="tab"
          className={`ll-tab ${activeTab === "current_ch2" ? "ll-tab-active" : ""}`}
          onClick={() => setActiveTab("current_ch2")}
        >
          电流通道2
        </button>
      </div>

      {activeTab === "voltage" ? (
        <VoltageCalibrationPanel
          baseUrl={baseUrl}
          status={statusMatchesActiveTab ? status : null}
          latestStatusRef={statusRef}
          ensureMode={ensureActiveTabCalMode}
          withStatusStreamPaused={withStatusStreamPaused}
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
          onInfoToast={enqueueInfoToast}
          onEnqueueUndo={enqueueUndo}
          onResetDraftToEmpty={resetDraftToEmpty}
          onRefetchProfile={profileQuery.refetch as RefetchProfile}
          isOffline={isOffline}
        />
      ) : (
        <CurrentCalibrationPanel
          curve={activeTab}
          baseUrl={baseUrl}
          status={statusMatchesActiveTab ? status : null}
          latestStatusRef={statusRef}
          ensureMode={ensureActiveTabCalMode}
          withStatusStreamPaused={withStatusStreamPaused}
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
          onInfoToast={enqueueInfoToast}
          onEnqueueUndo={enqueueUndo}
          onResetDraftToEmpty={resetDraftToEmpty}
          onRefetchProfile={profileQuery.refetch as RefetchProfile}
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

      {infoToasts.length > 0 || undoToasts.length > 0 ? (
        <div className="toast toast-end toast-bottom z-50">
          {infoToasts.map((toast) => (
            <div key={toast.id} className="ll-alert ll-alert-success text-sm">
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
              <div key={toast.id} className="ll-alert ll-alert-info text-sm">
                <div className="flex items-center justify-between gap-3 w-full">
                  <div className="flex-1">{toast.message}</div>
                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      className="ll-button ll-button-xs ll-button-outline"
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
      ) : null}
    </PageContainer>
  );
}

export default DeviceCalibrationRoute;

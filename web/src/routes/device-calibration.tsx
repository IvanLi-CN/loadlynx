import type { QueryObserverResult } from "@tanstack/react-query";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
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
  calibrationProfilesPointsEqual,
  type ValidationIssue,
  validateAndNormalizeCurrentPoints,
  validateAndNormalizeVoltagePoints,
} from "../calibration/validation.ts";
import { useDevicesQuery } from "../devices/hooks.ts";

type RefetchProfile = () => Promise<
  QueryObserverResult<CalibrationProfile, HttpApiError>
>;

type UndoAction =
  | {
      kind: "voltage";
      mv: number;
      local?: { index: number; point: CalibrationPointVoltage };
      remote?: { index: number; point: CalibrationPointVoltage };
    }
  | {
      kind: "current";
      curve: "current_ch1" | "current_ch2";
      index: number;
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
  const [activeTab, setActiveTab] = useState<"voltage" | "current">("voltage");

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
  const [previewProfile, setPreviewProfile] = useState<CalibrationProfile | null>(
    null,
  );
  const [previewAppliedAt, setPreviewAppliedAt] = useState<number | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const [importIssues, setImportIssues] = useState<ValidationIssue[] | null>(null);
  const [undoToasts, setUndoToasts] = useState<UndoToastEntry[]>([]);
  const [infoToasts, setInfoToasts] = useState<InfoToastEntry[]>([]);
  const [undoNow, setUndoNow] = useState(() => Date.now());

  const clearToasts = () => {
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
  };

  const enqueueInfoToast = (message: string) => {
    const id = makeUndoId();
    const timeoutId = window.setTimeout(() => {
      setInfoToasts((prev) => prev.filter((t) => t.id !== id));
    }, 2_500);
    setInfoToasts((prev) => [...prev, { id, message, timeoutId }]);
  };

  const applyUndoAction = (action: UndoAction) => {
    setDraftProfile((prev) => {
      const next = structuredClone(prev);

      if (action.kind === "voltage") {
        if (action.local) {
          const idx = Math.max(
            0,
            Math.min(action.local.index, next.v_local_points.length),
          );
          next.v_local_points.splice(idx, 0, action.local.point);
        }
        if (action.remote) {
          const idx = Math.max(
            0,
            Math.min(action.remote.index, next.v_remote_points.length),
          );
          next.v_remote_points.splice(idx, 0, action.remote.point);
        }
        return next;
      }

      const idx = Math.max(0, Math.min(action.index, 5));
      if (action.curve === "current_ch1") {
        next.current_ch1_points.splice(idx, 0, action.point);
      } else {
        next.current_ch2_points.splice(idx, 0, action.point);
      }
      return next;
    });
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
    setDraftProfile(makeEmptyDraftProfile(profileQuery.data?.active));
    setPreviewProfile(null);
    setPreviewAppliedAt(null);
    setImportError(null);
    setImportIssues(null);
    enqueueInfoToast(message);
  };

  useEffect(() => {
    if (undoToasts.length === 0) return;
    const id = window.setInterval(() => setUndoNow(Date.now()), 250);
    return () => window.clearInterval(id);
  }, [undoToasts.length]);

  const handleExportDraft = () => {
    if (isDraftEmpty(draftProfile)) {
      alert("Draft is empty. Export is disabled.");
      return;
    }

    const now = new Date();
    const stamp = now.toISOString().replaceAll(":", "-");
    const payload = {
      schema_version: 1,
      generated_at: now.toISOString(),
      device_id: deviceId,
      active_snapshot: profileQuery.data?.active ?? draftProfile.active,
      curves: {
        v_local_points: draftProfile.v_local_points,
        v_remote_points: draftProfile.v_remote_points,
        current_ch1_points: draftProfile.current_ch1_points,
        current_ch2_points: draftProfile.current_ch2_points,
      },
    };

    downloadJson(`loadlynx-calibration-draft-${deviceId}-${stamp}.json`, payload);
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
      if (typeof value !== "object" || value === null) {
        issues.push({ path, message: "point must be an object" });
        return null;
      }
      const obj = value as Record<string, unknown>;
      const raw = readNumber(obj.raw ?? obj.raw_100uv);
      const mv = readNumber(obj.mv ?? obj.meas_mv);
      if (raw == null) issues.push({ path: `${path}.raw`, message: "raw must be a number" });
      if (mv == null) issues.push({ path: `${path}.mv`, message: "mv must be a number" });
      if (raw == null || mv == null) return null;
      return { raw, mv };
    };

    const parseCurrentPoint = (
      value: unknown,
      path: string,
    ): CalibrationPointCurrent | null => {
      if (typeof value !== "object" || value === null) {
        issues.push({ path, message: "point must be an object" });
        return null;
      }
      const obj = value as Record<string, unknown>;
      const raw = readNumber(obj.raw ?? obj.raw_100uv);
      const ma = readNumber(obj.ma ?? obj.meas_ma);
      const dac = readNumber(obj.dac_code ?? obj.raw_dac_code);
      if (raw == null) issues.push({ path: `${path}.raw`, message: "raw must be a number" });
      if (ma == null) issues.push({ path: `${path}.ma`, message: "ma must be a number" });
      if (dac == null)
        issues.push({ path: `${path}.dac_code`, message: "dac_code must be a number" });
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
      v_local_points: parseVoltagePoints(curves.v_local_points, "v_local_points"),
      v_remote_points: parseVoltagePoints(curves.v_remote_points, "v_remote_points"),
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
        ? validateAndNormalizeVoltagePoints("v_local", nextProfile.v_local_points)
        : { normalized: [], issues: [] };
    const vRemote =
      nextProfile.v_remote_points.length > 0
        ? validateAndNormalizeVoltagePoints("v_remote", nextProfile.v_remote_points)
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

  const draftIssues = useMemo(() => {
    const issues: ValidationIssue[] = [];
    if (draftProfile.v_local_points.length > 0) {
      issues.push(
        ...validateAndNormalizeVoltagePoints("v_local", draftProfile.v_local_points)
          .issues,
      );
    }
    if (draftProfile.v_remote_points.length > 0) {
      issues.push(
        ...validateAndNormalizeVoltagePoints("v_remote", draftProfile.v_remote_points)
          .issues,
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

  const deviceUsingDefaults = profileQuery.data?.active.source === "factory-default";

  // Reset local state while switching devices/URLs.
  useEffect(() => {
    clearToasts();
    setDraftProfile(makeEmptyDraftProfile());
    setPreviewProfile(null);
    setPreviewAppliedAt(null);
    setImportError(null);
    setImportIssues(null);
  }, [baseUrl, deviceId]);

  // Always attempt to reset mode when leaving the page.
  useEffect(() => {
    return () => {
      postCalibrationMode(baseUrl, { kind: "off" }).catch(console.error);
    };
  }, [baseUrl]);

  // Switch mode when changing tabs. Current tab selection is refined by the
  // CurrentCalibration component (CH1/CH2) on mount.
  useEffect(() => {
    if (activeTab === "voltage") {
      postCalibrationMode(baseUrl, { kind: "voltage" }).catch(console.error);
    }
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
                    source=<span className="font-mono">{profileQuery.data.active.source}</span>,{" "}
                    fmt=<span className="font-mono">{profileQuery.data.active.fmt_version}</span>,{" "}
                    hw=<span className="font-mono">{profileQuery.data.active.hw_rev}</span>
                  </>
                ) : (
                  <span className="text-base-content/60">--</span>
                )}
              </div>
              <div>
                <span className="font-bold">Last read:</span>{" "}
                {profileQuery.dataUpdatedAt ? (
                  <span className="font-mono">{formatLocalTimestamp(profileQuery.dataUpdatedAt)}</span>
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
                  <div className="badge badge-info">Device: user-calibrated</div>
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

      <div role="tablist" className="tabs tabs-boxed">
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === "voltage" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("voltage")}
        >
          Voltage
        </button>
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === "current" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("current")}
        >
          Current
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
          onEnqueueUndo={enqueueUndo}
          onResetDraftToEmpty={resetDraftToEmpty}
          onRefetchProfile={profileQuery.refetch}
          isOffline={isOffline}
        />
      ) : (
        <CurrentCalibration
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
          onEnqueueUndo={enqueueUndo}
          onResetDraftToEmpty={resetDraftToEmpty}
          onRefetchProfile={profileQuery.refetch}
          isOffline={isOffline}
        />
      )}

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
  onSetDraftProfile: React.Dispatch<
    React.SetStateAction<CalibrationProfile>
  >;
  onSetPreviewProfile: React.Dispatch<
    React.SetStateAction<CalibrationProfile | null>
  >;
  onSetPreviewAppliedAt: React.Dispatch<React.SetStateAction<number | null>>;
  deviceId: string;
  onExportDraft: () => void;
  onImportDraftFile: (file: File | null) => Promise<void>;
  onEnqueueUndo: (action: UndoAction, message: string) => void;
  onResetDraftToEmpty: (message?: string) => void;
  onRefetchProfile: RefetchProfile;
  isOffline: boolean;
}) {
  const [inputV, setInputV] = useState("12.00");
  const [confirmKind, setConfirmKind] = useState<
    "reset_draft" | "reset_device_voltage" | null
  >(null);

  const effectivePreview = previewProfile ?? deviceProfile ?? null;

  const draftLocalPoints = draftProfile.v_local_points;
  const draftRemotePoints = draftProfile.v_remote_points;

  const previewLocalPoints = effectivePreview?.v_local_points ?? [];
  const previewRemotePoints = effectivePreview?.v_remote_points ?? [];

  const mergedDraft = mergeVoltageCandidatesByMv(draftLocalPoints, draftRemotePoints);
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
      alert("Raw values not available. Ensure calibration mode is enabled.");
      return;
    }

    const measuredMv = Math.round(Number.parseFloat(inputV) * 1000);
    if (!Number.isFinite(measuredMv) || measuredMv <= 0) {
      alert("Invalid voltage input.");
      return;
    }

    if (draftLocalPoints.length >= 5 || draftRemotePoints.length >= 5) {
      alert("Too many points (max 5).");
      return;
    }

    onSetDraftProfile((prev) => {
      const next = structuredClone(prev);
      next.v_local_points = validateAndNormalizeVoltagePoints("v_local", [
        ...prev.v_local_points,
        { raw: rawLocal, mv: measuredMv },
      ]).normalized;

      next.v_remote_points = validateAndNormalizeVoltagePoints("v_remote", [
        ...prev.v_remote_points,
        { raw: rawRemote, mv: measuredMv },
      ]).normalized;

      return next;
    });
  };

  const handleDeleteByMv = (mv: number) => {
    const localIndex = draftProfile.v_local_points.findIndex((p) => p.mv === mv);
    const remoteIndex = draftProfile.v_remote_points.findIndex((p) => p.mv === mv);

    const local =
      localIndex !== -1
        ? { index: localIndex, point: draftProfile.v_local_points[localIndex] }
        : undefined;
    const remote =
      remoteIndex !== -1
        ? { index: remoteIndex, point: draftProfile.v_remote_points[remoteIndex] }
        : undefined;

    if (local || remote) {
      onEnqueueUndo(
        { kind: "voltage", mv, local, remote },
        `Point deleted (${mv} mV)`,
      );
    }

    onSetDraftProfile((prev) => {
      const next = structuredClone(prev);
      const nextLocalIndex = next.v_local_points.findIndex((p) => p.mv === mv);
      const nextRemoteIndex = next.v_remote_points.findIndex((p) => p.mv === mv);

      if (nextLocalIndex !== -1) next.v_local_points.splice(nextLocalIndex, 1);
      if (nextRemoteIndex !== -1) next.v_remote_points.splice(nextRemoteIndex, 1);

      return next;
    });
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

      await postCalibrationApply(baseUrl, { kind: "v_local", points: local.normalized });
      await new Promise((resolve) => setTimeout(resolve, 200));
      await postCalibrationApply(baseUrl, { kind: "v_remote", points: remote.normalized });
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

      await postCalibrationCommit(baseUrl, { kind: "v_local", points: local.normalized });
      await new Promise((resolve) => setTimeout(resolve, 200));
      await postCalibrationCommit(baseUrl, { kind: "v_remote", points: remote.normalized });
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
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div className="card bg-base-100 shadow-xl border border-base-200">
        <div className="card-body gap-4">
          <div className="flex items-start justify-between gap-3">
            <h3 className="card-title flex flex-col items-start leading-tight">
              <span>Local Draft</span>
              <span className="text-sm font-normal text-base-content/60">Web</span>
            </h3>
            <div className="badge badge-neutral">No device writes</div>
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
              disabled={isOffline || draftLocalPoints.length >= 5}
            >
              Capture
            </button>
          </div>

          {draftVoltageIssues.length > 0 &&
            (draftLocalPoints.length > 0 || draftRemotePoints.length > 0) && (
            <div role="alert" className="alert alert-warning text-sm py-2">
              <span>
                Draft validation:{" "}
                <span className="font-bold">{draftVoltageIssues[0].message}</span>
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
                {previewLocalV == null ? "--" : `${previewLocalV.toFixed(3)} V`}
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
                {previewRemoteV == null ? "--" : `${previewRemoteV.toFixed(3)} V`}
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
                  <tr key={row.mv}>
                    <td>{row.mv}</td>
                    <td>{row.rawLocal ?? "--"}</td>
                    <td>{row.rawRemote ?? "--"}</td>
                    <td className="text-right">
                      <button
                        type="button"
                        className="btn btn-ghost btn-xs text-error"
                        onClick={() => handleDeleteByMv(row.mv)}
                        disabled={
                          isOffline
                        }
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
                {mergedDraft.length === 0 && (
                  <tr>
                    <td colSpan={4} className="text-center text-base-content/50">
                      No draft points.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <div className="card bg-base-100 shadow-xl border border-base-200">
        <div className="card-body gap-4">
          <div className="flex items-start justify-between gap-3">
            <h3 className="card-title flex flex-col items-start leading-tight">
              <span>Device Sync</span>
              <span className="text-sm font-normal text-base-content/60">Hardware</span>
            </h3>
            <div className="badge badge-warning whitespace-nowrap shrink-0">
              Writes device
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
              className="btn btn-sm btn-outline"
              onClick={() => applyToDeviceMutation.mutate()}
              disabled={!canWriteToDevice || applyToDeviceMutation.isPending}
            >
              Sync calibration to device (Apply)
            </button>
            <button
              type="button"
              className="btn btn-sm btn-secondary"
              onClick={() => commitToDeviceMutation.mutate()}
              disabled={!canWriteToDevice || commitToDeviceMutation.isPending}
            >
              Sync calibration to device (Commit)
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
                    <td colSpan={3} className="text-center text-base-content/50">
                      No device profile loaded.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      </div>

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
  onSetDraftProfile: React.Dispatch<
    React.SetStateAction<CalibrationProfile>
  >;
  onSetPreviewProfile: React.Dispatch<
    React.SetStateAction<CalibrationProfile | null>
  >;
  onSetPreviewAppliedAt: React.Dispatch<React.SetStateAction<number | null>>;
  deviceId: string;
  onExportDraft: () => void;
  onImportDraftFile: (file: File | null) => Promise<void>;
  onEnqueueUndo: (action: UndoAction, message: string) => void;
  onResetDraftToEmpty: (message?: string) => void;
  onRefetchProfile: RefetchProfile;
  isOffline: boolean;
}) {
  const [channel, setChannel] = useState<"ch1" | "ch2">("ch1");
  const [confirmKind, setConfirmKind] = useState<
    "reset_draft" | "reset_device_current" | null
  >(null);

  useEffect(() => {
    const kind = channel === "ch1" ? "current_ch1" : "current_ch2";
    postCalibrationMode(baseUrl, { kind }).catch(console.error);
  }, [baseUrl, channel]);

  const [meterReadingA, setMeterReadingA] = useState("1.000");
  const [targetIMa, setTargetIMa] = useState("1000");

  const effectivePreview = previewProfile ?? deviceProfile ?? null;

  const kind = channel === "ch1" ? "current_ch1" : "current_ch2";
  const draftPoints =
    channel === "ch1"
      ? draftProfile.current_ch1_points
      : draftProfile.current_ch2_points;

  const previewPoints =
    channel === "ch1"
      ? effectivePreview?.current_ch1_points ?? []
      : effectivePreview?.current_ch2_points ?? [];

  const devicePoints =
    channel === "ch1"
      ? deviceProfile?.current_ch1_points ?? []
      : deviceProfile?.current_ch2_points ?? [];

  const currentDraft = useMemo(
    () => validateAndNormalizeCurrentPoints(kind, draftPoints),
    [kind, draftPoints],
  );
  const canWriteToDevice =
    !isOffline && draftPoints.length > 0 && currentDraft.issues.length === 0;

  const handleSetOutput = () => {
    const parsed = Number.parseInt(targetIMa, 10);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      alert("Invalid target current.");
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
      alert("Raw values not available. Ensure calibration mode is enabled.");
      return;
    }

    const measuredMa = Math.round(Number.parseFloat(meterReadingA) * 1000);
    if (!Number.isFinite(measuredMa) || measuredMa <= 0) {
      alert("Invalid current input.");
      return;
    }

    if (draftPoints.length >= 5) {
      alert("Too many points (max 5).");
      return;
    }

    onSetDraftProfile((prev) => {
      const next = structuredClone(prev);
      const existingPoints =
        channel === "ch1" ? prev.current_ch1_points : prev.current_ch2_points;
      const nextPoints = validateAndNormalizeCurrentPoints(kind, [
        ...existingPoints,
        { raw: rawCur, ma: measuredMa, dac_code: rawDac },
      ]).normalized;

      if (channel === "ch1") {
        next.current_ch1_points = nextPoints;
      } else {
        next.current_ch2_points = nextPoints;
      }
      return next;
    });
  };

  const handleDeleteCandidate = (index: number) => {
    const removed = draftPoints[index];
    if (removed) {
      onEnqueueUndo(
        {
          kind: "current",
          curve: kind,
          index,
          point: removed,
        },
        `Point deleted (${removed.ma} mA)`,
      );
    }

    onSetDraftProfile((prev) => {
      const next = structuredClone(prev);
      const existingPoints =
        channel === "ch1" ? prev.current_ch1_points : prev.current_ch2_points;
      const updated = existingPoints.slice();
      updated.splice(index, 1);
      if (channel === "ch1") {
        next.current_ch1_points = updated;
      } else {
        next.current_ch2_points = updated;
      }
      return next;
    });
  };

  const activeMa =
    channel === "ch1" ? status?.raw.i_local_ma : status?.raw.i_remote_ma;
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
      const points =
        channel === "ch1"
          ? draftProfile.current_ch1_points
          : draftProfile.current_ch2_points;
      const validated = validateAndNormalizeCurrentPoints(kind, points);
      if (validated.issues.length > 0) {
        throw new Error(`Draft validation failed: ${validated.issues[0].message}`);
      }
      return postCalibrationApply(baseUrl, { kind, points: validated.normalized });
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
      const points =
        channel === "ch1"
          ? draftProfile.current_ch1_points
          : draftProfile.current_ch2_points;
      const validated = validateAndNormalizeCurrentPoints(kind, points);
      if (validated.issues.length > 0) {
        throw new Error(`Draft validation failed: ${validated.issues[0].message}`);
      }
      return postCalibrationCommit(baseUrl, { kind, points: validated.normalized });
    },
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const resetDeviceCurrentMutation = useMutation({
    mutationFn: async () => postCalibrationReset(baseUrl, { kind }),
    onSuccess: async () => {
      await onRefetchProfile();
      onResetDraftToEmpty("Device reset to defaults. Draft cleared.");
    },
  });

  return (
    <>
      <div className="flex flex-col gap-6">
      <div className="flex justify-center">
        <div className="join">
          <input
            className="join-item btn"
            type="radio"
            name="calibration-current-channel"
            aria-label="Channel 1 (Low Range)"
            checked={channel === "ch1"}
            onChange={() => setChannel("ch1")}
          />
          <input
            className="join-item btn"
            type="radio"
            name="calibration-current-channel"
            aria-label="Channel 2 (High Range)"
            checked={channel === "ch2"}
            onChange={() => setChannel("ch2")}
          />
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="card bg-base-100 shadow-xl border border-base-200">
          <div className="card-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="card-title flex flex-col items-start leading-tight">
                <span>Local Draft</span>
                <span className="text-sm font-normal text-base-content/60">
                  Web
                </span>
              </h3>
              <div className="badge badge-neutral">No device writes</div>
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

            <div className="divider my-0"></div>

            <label className="form-control w-full">
              <div className="label">
                <span className="label-text">
                  Meter Reading ({channel === "ch1" ? "Local" : "Remote"}) (A)
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
                  disabled={isOffline || draftPoints.length >= 5}
                >
                  Capture
                </button>
              </div>
            </label>

            {currentDraft.issues.length > 0 && draftPoints.length > 0 && (
              <div role="alert" className="alert alert-warning text-sm py-2">
                <span>
                  Draft validation:{" "}
                  <span className="font-bold">{currentDraft.issues[0].message}</span>
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
                  {previewMa == null ? "--" : `${(previewMa / 1000).toFixed(4)} A`}
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
                  {draftPoints.map((point, idx) => (
                    <tr key={`${point.raw}-${point.ma}-${idx}`}>
                      <td>{point.raw}</td>
                      <td>{point.dac_code ?? "--"}</td>
                      <td>{point.ma}</td>
                      <td className="text-right">
                        <button
                          type="button"
                          className="btn btn-ghost btn-xs text-error"
                          onClick={() => handleDeleteCandidate(idx)}
                          disabled={isOffline}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                  {draftPoints.length === 0 && (
                    <tr>
                      <td colSpan={4} className="text-center text-base-content/50">
                        No draft points.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <div className="card bg-base-100 shadow-xl border border-base-200">
          <div className="card-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="card-title flex flex-col items-start leading-tight">
                <span>Device Sync</span>
                <span className="text-sm font-normal text-base-content/60">
                  Hardware
                </span>
              </h3>
              <div className="badge badge-warning whitespace-nowrap shrink-0">
                Writes device
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
                className="btn btn-sm btn-outline"
                onClick={() => applyToDeviceMutation.mutate()}
                disabled={!canWriteToDevice || applyToDeviceMutation.isPending}
              >
                Sync calibration to device (Apply)
              </button>
              <button
                type="button"
                className="btn btn-sm btn-secondary"
                onClick={() => commitToDeviceMutation.mutate()}
                disabled={!canWriteToDevice || commitToDeviceMutation.isPending}
              >
                Sync calibration to device (Commit)
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
                    onClick={() => setTargetIMa("3000")}
                  >
                    3A
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
                      <td colSpan={3} className="text-center text-base-content/50">
                        No device profile loaded.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
      </div>

      <ConfirmDialog
        open={confirmKind !== null}
        title={
          confirmKind === "reset_draft"
            ? "Reset Draft (Web only)"
            : `Reset Device Calibration (Current ${channel})`
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
                `Affects: ${kind}.`,
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
    <div
      className="modal modal-open"
      role="dialog"
      aria-modal="true"
      onClick={onCancel}
    >
      <div className="modal-box" onClick={(event) => event.stopPropagation()}>
        <h3 className="font-bold text-lg">{title}</h3>
        <p className="py-3 text-sm">{body}</p>
        <ul className="list-disc pl-5 text-sm space-y-1">
          {details.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>

        <div className="modal-action">
          <button type="button" className="btn" autoFocus onClick={onCancel}>
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
    </div>
  );
}

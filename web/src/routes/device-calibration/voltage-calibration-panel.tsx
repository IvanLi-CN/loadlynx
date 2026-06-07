import { useMutation } from "@tanstack/react-query";
import {
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
  useMemo,
  useState,
} from "react";
import {
  postCalibrationApply,
  postCalibrationCommit,
  postCalibrationReset,
} from "../../api/client.ts";
import type { CalibrationProfile, FastStatusView } from "../../api/types.ts";
import { piecewiseLinearDecimal } from "../../calibration/piecewise.ts";
import {
  validateAndNormalizeCurrentPoints,
  validateAndNormalizeVoltagePoints,
} from "../../calibration/validation.ts";
import { ConfirmDialog } from "../../components/common/confirm-dialog.tsx";
import {
  formatMvAsV,
  isDraftEmpty,
  mergeVoltageCandidatesByIndex,
  mergeVoltageCandidatesByMv,
  parseVoltageInputToMv,
  type RefetchProfile,
  retryDeviceCall,
  sleep,
  type UndoAction,
  type VoltageInputUnit,
  type WithStatusStreamPaused,
} from "./shared.ts";

export interface VoltageCalibrationPanelProps {
  baseUrl: string;
  status: FastStatusView | null;
  latestStatusRef: MutableRefObject<FastStatusView | null>;
  ensureMode: (action: string, opts?: { silent?: boolean }) => Promise<boolean>;
  withStatusStreamPaused: WithStatusStreamPaused;
  deviceProfile: CalibrationProfile | undefined;
  draftProfile: CalibrationProfile;
  previewProfile: CalibrationProfile | null;
  onSetDraftProfile: Dispatch<SetStateAction<CalibrationProfile>>;
  onSetPreviewProfile: Dispatch<SetStateAction<CalibrationProfile | null>>;
  onSetPreviewAppliedAt: Dispatch<SetStateAction<number | null>>;
  deviceId: string;
  onExportDraft: () => void;
  onImportDraftFile: (file: File | null) => Promise<void>;
  onReadDeviceToDraft: () => void;
  readDeviceToDraftPending: boolean;
  onAlert: (title: string, body: string, details?: string[]) => void;
  onInfoToast: (message: string) => void;
  onEnqueueUndo: (action: UndoAction, message: string) => void;
  onResetDraftToEmpty: (message?: string) => void;
  onRefetchProfile: RefetchProfile;
  isOffline: boolean;
}

export function VoltageCalibrationPanel({
  baseUrl,
  status,
  latestStatusRef,
  ensureMode,
  withStatusStreamPaused,
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
  onInfoToast,
  onEnqueueUndo,
  onResetDraftToEmpty,
  onRefetchProfile,
  isOffline,
}: VoltageCalibrationPanelProps) {
  const [viewTab, setViewTab] = useState<"draft" | "device">("draft");
  const [inputV, setInputV] = useState("12.00");
  const inputVUnit: VoltageInputUnit = "V";
  const [confirmKind, setConfirmKind] = useState<
    "reset_draft" | "reset_device_voltage" | null
  >(null);

  const effectivePreview = previewProfile ?? deviceProfile ?? null;

  const draftLocalPoints = draftProfile.v_local_points;
  const draftRemotePoints = draftProfile.v_remote_points;
  const previewLocalPoints = effectivePreview?.v_local_points ?? [];
  const previewRemotePoints = effectivePreview?.v_remote_points ?? [];

  const mergedDraft = mergeVoltageCandidatesByIndex(
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
    !isOffline && (draftLocalPoints.length > 0 || draftRemotePoints.length > 0);

  const handleCapture = async () => {
    const ok = await ensureMode("Capture");
    if (!ok) return;

    const rawLocal = latestStatusRef.current?.raw.raw_v_nr_100uv;
    const rawRemote = latestStatusRef.current?.raw.raw_v_rmt_100uv;
    if (rawLocal == null || rawRemote == null) {
      onAlert(
        "Cannot Capture Voltage Point",
        "Raw values not available. Ensure calibration mode is enabled.",
      );
      return;
    }

    const measuredMv = parseVoltageInputToMv(inputV, inputVUnit);
    if (measuredMv == null || measuredMv <= 0) {
      onAlert("Cannot Capture Voltage Point", "Invalid voltage input.");
      return;
    }

    const existingIndex = (() => {
      const count = Math.min(draftLocalPoints.length, draftRemotePoints.length);
      for (let index = 0; index < count; index += 1) {
        const local = draftLocalPoints[index];
        const remote = draftRemotePoints[index];
        if (
          local &&
          remote &&
          local.raw === rawLocal &&
          local.mv === measuredMv &&
          remote.raw === rawRemote &&
          remote.mv === measuredMv
        ) {
          return index;
        }
      }
      return null;
    })();

    onSetDraftProfile((prev) => {
      const localPoint = { raw: rawLocal, mv: measuredMv };
      const remotePoint = { raw: rawRemote, mv: measuredMv };
      const count = Math.min(
        prev.v_local_points.length,
        prev.v_remote_points.length,
      );
      let duplicateIndex: number | null = null;
      for (let index = 0; index < count; index += 1) {
        const local = prev.v_local_points[index];
        const remote = prev.v_remote_points[index];
        if (
          local &&
          remote &&
          local.raw === localPoint.raw &&
          local.mv === localPoint.mv &&
          remote.raw === remotePoint.raw &&
          remote.mv === remotePoint.mv
        ) {
          duplicateIndex = index;
          break;
        }
      }

      return {
        ...prev,
        v_local_points:
          duplicateIndex == null
            ? [...prev.v_local_points, localPoint]
            : [
                ...prev.v_local_points.filter(
                  (_, index) => index !== duplicateIndex,
                ),
                localPoint,
              ],
        v_remote_points:
          duplicateIndex == null
            ? [...prev.v_remote_points, remotePoint]
            : [
                ...prev.v_remote_points.filter(
                  (_, index) => index !== duplicateIndex,
                ),
                remotePoint,
              ],
      };
    });

    if (existingIndex != null) {
      onInfoToast(
        `Duplicate voltage sample replaced (${formatMvAsV(measuredMv)} V).`,
      );
    }
  };

  const handleDeleteDraftRow = (index: number) => {
    const local = draftLocalPoints[index];
    const remote = draftRemotePoints[index];
    if (!local && !remote) return;
    const mv = local?.mv ?? remote?.mv ?? null;
    onEnqueueUndo(
      {
        kind: "voltage_points",
        local: local ?? undefined,
        remote: remote ?? undefined,
      },
      `Deleted voltage sample #${index + 1} (mv=${mv ?? "--"})`,
    );
    onSetDraftProfile((prev) => ({
      ...prev,
      v_local_points: prev.v_local_points.filter((_, row) => row !== index),
      v_remote_points: prev.v_remote_points.filter((_, row) => row !== index),
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

  const previewLocalMv =
    status?.raw.raw_v_nr_100uv != null && previewLocalDataset.length >= 1
      ? piecewiseLinearDecimal(previewLocalDataset, status.raw.raw_v_nr_100uv)
      : null;
  const previewRemoteMv =
    status?.raw.raw_v_rmt_100uv != null && previewRemoteDataset.length >= 1
      ? piecewiseLinearDecimal(previewRemoteDataset, status.raw.raw_v_rmt_100uv)
      : null;

  const readMutation = useMutation({
    mutationFn: async () => onRefetchProfile(),
  });

  const applyToDeviceMutation = useMutation({
    mutationFn: async () => {
      await withStatusStreamPaused(async () => {
        if (draftLocalPoints.length === 0 && draftRemotePoints.length === 0) {
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
          onAlert(
            "Calibration data cleanup (Apply)",
            "Draft contains duplicate/conflicting samples. Apply will use a cleaned curve and may drop/merge points.",
            issues.map((issue) => `${issue.path}: ${issue.message}`),
          );
        }

        if (local.normalized.length === 0 && remote.normalized.length === 0) {
          throw new Error("No valid points after cleanup. Nothing to apply.");
        }

        if (local.normalized.length > 0) {
          await retryDeviceCall(() =>
            postCalibrationApply(baseUrl, {
              kind: "v_local",
              points: local.normalized,
            }),
          );
          await sleep(200);
        }
        if (remote.normalized.length > 0) {
          await retryDeviceCall(() =>
            postCalibrationApply(baseUrl, {
              kind: "v_remote",
              points: remote.normalized,
            }),
          );
        }
      });
    },
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const commitToDeviceMutation = useMutation({
    mutationFn: async () => {
      await withStatusStreamPaused(async () => {
        if (draftLocalPoints.length === 0 && draftRemotePoints.length === 0) {
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
          onAlert(
            "Calibration data cleanup (Commit)",
            "Draft contains duplicate/conflicting samples. Commit will use a cleaned curve and may drop/merge points.",
            issues.map((issue) => `${issue.path}: ${issue.message}`),
          );
        }

        if (local.normalized.length === 0 && remote.normalized.length === 0) {
          throw new Error("No valid points after cleanup. Nothing to commit.");
        }

        if (local.normalized.length > 0) {
          await retryDeviceCall(() =>
            postCalibrationCommit(baseUrl, {
              kind: "v_local",
              points: local.normalized,
            }),
          );
          await sleep(200);
        }
        if (remote.normalized.length > 0) {
          await retryDeviceCall(() =>
            postCalibrationCommit(baseUrl, {
              kind: "v_remote",
              points: remote.normalized,
            }),
          );
        }
      });
    },
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const resetDeviceVoltageMutation = useMutation({
    mutationFn: async () => {
      await withStatusStreamPaused(async () => {
        await retryDeviceCall(() =>
          postCalibrationReset(baseUrl, { kind: "v_local" }),
        );
        await sleep(200);
        await retryDeviceCall(() =>
          postCalibrationReset(baseUrl, { kind: "v_remote" }),
        );
      });
    },
    onSuccess: async () => {
      await onRefetchProfile();
      onResetDraftToEmpty("Device reset to defaults. Draft cleared.");
    },
  });

  return (
    <>
      <div role="tablist" className="ll-tabs">
        <button
          type="button"
          role="tab"
          className={`ll-tab ${viewTab === "draft" ? "ll-tab-active" : ""}`}
          onClick={() => setViewTab("draft")}
        >
          本地草稿
        </button>
        <button
          type="button"
          role="tab"
          className={`ll-tab ${viewTab === "device" ? "ll-tab-active" : ""}`}
          onClick={() => setViewTab("device")}
        >
          设备数据
        </button>
      </div>

      {viewTab === "draft" ? (
        <div className="ll-panel bg-base-100 shadow-xl border border-base-200 mt-4">
          <div className="ll-panel-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="ll-panel-title flex flex-col items-start leading-tight">
                <span>本地草稿</span>
                <span className="text-sm font-normal text-base-content/60">
                  Web
                </span>
              </h3>
            </div>

            <div className="ll-panel bg-base-200/40 border border-base-200">
              <div className="ll-panel-body py-4 gap-3">
                <div className="flex items-start justify-between gap-3">
                  <h4 className="font-bold text-sm">仅本地（不读写设备）</h4>
                  <div className="ll-badge ll-badge-neutral whitespace-nowrap shrink-0">
                    不读写设备
                  </div>
                </div>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="ll-button ll-button-sm ll-button-outline"
                    onClick={() => {
                      const vLocal = validateAndNormalizeVoltagePoints(
                        "v_local",
                        draftProfile.v_local_points,
                      );
                      const vRemote = validateAndNormalizeVoltagePoints(
                        "v_remote",
                        draftProfile.v_remote_points,
                      );
                      const c1 = validateAndNormalizeCurrentPoints(
                        "current_ch1",
                        draftProfile.current_ch1_points,
                      );
                      const c2 = validateAndNormalizeCurrentPoints(
                        "current_ch2",
                        draftProfile.current_ch2_points,
                      );
                      const issues = [
                        ...vLocal.issues,
                        ...vRemote.issues,
                        ...c1.issues,
                        ...c2.issues,
                      ];
                      if (issues.length > 0) {
                        onAlert(
                          "Calibration data cleanup (Preview)",
                          "Draft contains duplicate/conflicting samples. Preview will use a cleaned curve and may drop/merge points.",
                          issues.map(
                            (issue) => `${issue.path}: ${issue.message}`,
                          ),
                        );
                      }
                      onSetPreviewProfile({
                        active: draftProfile.active,
                        v_local_points: vLocal.normalized,
                        v_remote_points: vRemote.normalized,
                        current_ch1_points: c1.normalized,
                        current_ch2_points: c2.normalized,
                      });
                      onSetPreviewAppliedAt(Date.now());
                    }}
                    disabled={isDraftEmpty(draftProfile)}
                  >
                    Apply Preview
                  </button>
                  <button
                    type="button"
                    className="ll-button ll-button-sm ll-button-outline"
                    onClick={() => setConfirmKind("reset_draft")}
                    disabled={isDraftEmpty(draftProfile)}
                  >
                    Reset Draft
                  </button>
                  <button
                    type="button"
                    className="ll-button ll-button-sm ll-button-outline"
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
                    className="ll-button ll-button-sm ll-button-outline"
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

            <div className="ll-panel bg-base-200/40 border border-base-200">
              <div className="ll-panel-body py-4 gap-3">
                <div className="flex items-start justify-between gap-3">
                  <h4 className="font-bold text-sm">硬件 I/O</h4>
                  <div className="flex items-center gap-2">
                    <div className="ll-badge ll-badge-info whitespace-nowrap">
                      读设备
                    </div>
                    <div className="ll-badge ll-badge-warning whitespace-nowrap">
                      写设备
                    </div>
                  </div>
                </div>

                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="ll-button ll-button-sm ll-button-outline"
                    onClick={onReadDeviceToDraft}
                    disabled={isOffline || readDeviceToDraftPending}
                  >
                    Read Device → Draft
                  </button>
                  <button
                    type="button"
                    className="ll-button ll-button-sm ll-button-outline"
                    onClick={() => applyToDeviceMutation.mutate()}
                    disabled={
                      !canWriteToDevice || applyToDeviceMutation.isPending
                    }
                  >
                    Apply
                  </button>
                  <button
                    type="button"
                    className="ll-button ll-button-sm ll-button-secondary"
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
              <label className="ll-form-control w-full max-w-xs">
                <div className="ll-label-row">
                  <span className="ll-label-text">Measured Voltage (V)</span>
                </div>
                <input
                  type="number"
                  step="0.000001"
                  className="ll-input"
                  value={inputV}
                  onChange={(event) => setInputV(event.target.value)}
                  disabled={isOffline}
                />
              </label>
              <button
                type="button"
                className="ll-button ll-button-primary"
                onClick={handleCapture}
                disabled={isOffline}
              >
                Capture
              </button>
            </div>

            {draftVoltageIssues.length > 0 &&
            (draftLocalPoints.length > 0 || draftRemotePoints.length > 0) ? (
              <div
                role="alert"
                className="ll-alert ll-alert-warning text-sm py-2"
              >
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
            ) : null}

            <div className="ll-stats shadow">
              <div className="ll-stat">
                <div className="ll-stat-title">Local Voltage (Active)</div>
                <div className="ll-stat-value text-lg">
                  {formatMvAsV(status?.raw.v_local_mv ?? 0)} V
                </div>
                <div className="ll-stat-desc">
                  Raw: {status?.raw.raw_v_nr_100uv ?? "--"}
                </div>
              </div>
              <div className="ll-stat">
                <div className="ll-stat-title">Local Preview</div>
                <div className="ll-stat-value text-lg text-primary">
                  {previewLocalMv == null
                    ? "--"
                    : `${previewLocalMv.div(1000).toFixed(3)} V`}
                </div>
                <div className="ll-stat-desc">Uses applied preview</div>
              </div>
            </div>

            <div className="ll-stats shadow">
              <div className="ll-stat">
                <div className="ll-stat-title">Remote Voltage (Active)</div>
                <div className="ll-stat-value text-lg">
                  {formatMvAsV(status?.raw.v_remote_mv ?? 0)} V
                </div>
                <div className="ll-stat-desc">
                  Raw: {status?.raw.raw_v_rmt_100uv ?? "--"}
                </div>
              </div>
              <div className="ll-stat">
                <div className="ll-stat-title">Remote Preview</div>
                <div className="ll-stat-value text-lg text-primary">
                  {previewRemoteMv == null
                    ? "--"
                    : `${previewRemoteMv.div(1000).toFixed(3)} V`}
                </div>
                <div className="ll-stat-desc">Uses applied preview</div>
              </div>
            </div>

            <div className="overflow-x-auto max-h-64">
              <table className="ll-table ll-table-xs">
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
                    <tr key={row.index}>
                      <td>
                        {row.mv ?? "--"}
                        {(row.mvLocal != null &&
                          row.mvRemote != null &&
                          row.mvLocal !== row.mvRemote) ||
                        (row.mvLocal == null && row.mvRemote != null) ||
                        (row.mvRemote == null && row.mvLocal != null)
                          ? " *"
                          : ""}
                      </td>
                      <td>{row.rawLocal ?? "--"}</td>
                      <td>{row.rawRemote ?? "--"}</td>
                      <td className="text-right">
                        <button
                          type="button"
                          className="ll-button ll-button-ghost ll-button-xs text-error"
                          onClick={() => handleDeleteDraftRow(row.index)}
                          disabled={isOffline}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                  {mergedDraft.length === 0 ? (
                    <tr>
                      <td
                        colSpan={4}
                        className="text-center text-base-content/50"
                      >
                        No draft points.
                      </td>
                    </tr>
                  ) : null}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      ) : (
        <div className="ll-panel bg-base-100 shadow-xl border border-base-200 mt-4">
          <div className="ll-panel-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="ll-panel-title flex flex-col items-start leading-tight">
                <span>设备数据</span>
                <span className="text-sm font-normal text-base-content/60">
                  Hardware
                </span>
              </h3>
              <div className="flex items-center gap-2">
                <div className="ll-badge ll-badge-info whitespace-nowrap">
                  读设备
                </div>
                <div className="ll-badge ll-badge-warning whitespace-nowrap">
                  写设备
                </div>
              </div>
            </div>

            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                className="ll-button ll-button-sm ll-button-outline"
                onClick={() => readMutation.mutate()}
                disabled={readMutation.isPending}
              >
                Read
              </button>
              <button
                type="button"
                className="ll-button ll-button-sm ll-button-danger"
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
              <table className="ll-table ll-table-xs">
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
                  {mergedDevice.length === 0 ? (
                    <tr>
                      <td
                        colSpan={3}
                        className="text-center text-base-content/50"
                      >
                        No device profile loaded.
                      </td>
                    </tr>
                  ) : null}
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

import { useMutation } from "@tanstack/react-query";
import {
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  postCalibrationApply,
  postCalibrationCommit,
  postCalibrationReset,
} from "../../api/client.ts";
import type { CalibrationProfile, FastStatusView } from "../../api/types.ts";
import { piecewiseLinearDecimal } from "../../calibration/piecewise.ts";
import { validateAndNormalizeVoltagePoints } from "../../calibration/validation.ts";
import { ConfirmDialog } from "../../components/common/confirm-dialog.tsx";
import {
  CalibrationDeviceWriteButtons,
  CalibrationDraftActionsPanel,
  CalibrationHardwareIoPanel,
} from "./calibration-action-panels.tsx";
import {
  getResetDraftConfirmConfig,
  getResetVoltageDeviceConfirmConfig,
} from "./calibration-confirm.ts";
import { CalibrationDeviceViewPanel } from "./calibration-device-view-panel.tsx";
import { CalibrationViewTabs } from "./calibration-view-tabs.tsx";
import {
  runCalibrationReset,
  runVoltageCalibrationWrite,
} from "./device-write.ts";
import { applyCalibrationPreview } from "./preview-profile.ts";
import {
  formatMvAsV,
  formatUvToUnit,
  isDraftEmpty,
  mergeVoltageCandidatesByIndex,
  mergeVoltageCandidatesByMv,
  parseVoltageInputToMv,
  parseVoltageInputToUv,
  type RefetchProfile,
  type UndoAction,
  type VoltageInputUnit,
  type WithStatusStreamPaused,
} from "./shared.ts";
import { useCalibrationStore } from "./store-context.tsx";

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
  const { t } = useTranslation();
  const calibrationStore = useCalibrationStore();
  const [viewTab, setViewTab] = useState<"draft" | "device">("draft");
  const [inputV, setInputV] = useState("12.000000");
  const inputVUnit: VoltageInputUnit = "V";
  const [voltageOptionsLoaded, setVoltageOptionsLoaded] = useState(false);
  const [confirmKind, setConfirmKind] = useState<
    "reset_draft" | "reset_device_voltage" | null
  >(null);

  useEffect(() => {
    if (typeof window === "undefined") return;
    setVoltageOptionsLoaded(false);

    try {
      const stored = calibrationStore.getVoltageOptions(deviceId, baseUrl);
      const unit = stored.unit ?? "V";
      const inputUv = stored.inputUv ?? 12_000_000;
      setInputV(formatUvToUnit(inputUv, unit));
    } catch {
      // ignore
    } finally {
      setVoltageOptionsLoaded(true);
    }
  }, [baseUrl, calibrationStore, deviceId]);

  useEffect(() => {
    if (typeof window === "undefined" || !voltageOptionsLoaded) return;
    const inputUv = parseVoltageInputToUv(inputV, inputVUnit);
    if (inputUv == null) return;

    try {
      calibrationStore.setVoltageOptions(deviceId, baseUrl, {
        inputUv,
        unit: inputVUnit,
      });
    } catch {
      // ignore
    }
  }, [baseUrl, calibrationStore, deviceId, inputV, voltageOptionsLoaded]);

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
    mutationFn: async () =>
      runVoltageCalibrationWrite({
        action: "Apply",
        baseUrl,
        draftLocalPoints,
        draftRemotePoints,
        onAlert,
        postPoints: async ({ baseUrl, kind, points }) =>
          postCalibrationApply(baseUrl, {
            kind,
            points,
          }),
        withStatusStreamPaused,
      }),
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const commitToDeviceMutation = useMutation({
    mutationFn: async () =>
      runVoltageCalibrationWrite({
        action: "Commit",
        baseUrl,
        draftLocalPoints,
        draftRemotePoints,
        onAlert,
        postPoints: async ({ baseUrl, kind, points }) =>
          postCalibrationCommit(baseUrl, {
            kind,
            points,
          }),
        withStatusStreamPaused,
      }),
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const resetDeviceVoltageMutation = useMutation({
    mutationFn: async () =>
      runCalibrationReset({
        baseUrl,
        kinds: ["v_local", "v_remote"],
        resetKind: async ({ baseUrl, kind }) =>
          postCalibrationReset(baseUrl, { kind }),
        withStatusStreamPaused,
      }),
    onSuccess: async () => {
      await onRefetchProfile();
      onResetDraftToEmpty("Device reset to defaults. Draft cleared.");
    },
  });

  const resetDraftConfirm = getResetDraftConfirmConfig(
    isDraftEmpty(draftProfile),
  );
  const resetDeviceConfirm = getResetVoltageDeviceConfirmConfig();

  return (
    <>
      <CalibrationViewTabs activeView={viewTab} onSelectView={setViewTab} />

      {viewTab === "draft" ? (
        <div className="ll-panel bg-base-100 shadow-xl border border-base-200 mt-4">
          <div className="ll-panel-body gap-4">
            <div className="flex items-start justify-between gap-3">
              <h3 className="ll-panel-title flex flex-col items-start leading-tight">
                <span>{t("calibration.localDraft")}</span>
                <span className="text-sm font-normal text-base-content/60">
                  Web
                </span>
              </h3>
            </div>

            <CalibrationDraftActionsPanel
              disableApplyPreview={isDraftEmpty(draftProfile)}
              disableExport={isDraftEmpty(draftProfile)}
              disableResetDraft={isDraftEmpty(draftProfile)}
              exportTitle={
                isDraftEmpty(draftProfile)
                  ? "Export is disabled when the draft is empty."
                  : undefined
              }
              importInputId={`calibration-import-${deviceId}-voltage`}
              onApplyPreview={() =>
                applyCalibrationPreview({
                  draftProfile,
                  onAlert,
                  onSetPreviewAppliedAt: (value) =>
                    onSetPreviewAppliedAt(value),
                  onSetPreviewProfile: (value) => onSetPreviewProfile(value),
                })
              }
              onExportDraft={onExportDraft}
              onImportDraftFile={onImportDraftFile}
              onResetDraft={() => setConfirmKind("reset_draft")}
            />

            <CalibrationHardwareIoPanel
              actionButtons={
                <CalibrationDeviceWriteButtons
                  applyPending={applyToDeviceMutation.isPending}
                  commitPending={commitToDeviceMutation.isPending}
                  disableApply={!canWriteToDevice}
                  disableCommit={!canWriteToDevice}
                  onApply={() => applyToDeviceMutation.mutate()}
                  onCommit={() => commitToDeviceMutation.mutate()}
                />
              }
              disableReadDeviceToDraft={isOffline || readDeviceToDraftPending}
              onReadDeviceToDraft={onReadDeviceToDraft}
            />

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
        <CalibrationDeviceViewPanel
          deviceProfileSource={deviceProfile?.active.source}
          onReadDeviceProfile={() => readMutation.mutate()}
          onRequestReset={() => setConfirmKind("reset_device_voltage")}
          readPending={readMutation.isPending}
          resetDisabled={isOffline || resetDeviceVoltageMutation.isPending}
        >
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
        </CalibrationDeviceViewPanel>
      )}

      <ConfirmDialog
        open={confirmKind !== null}
        {...(confirmKind === "reset_draft"
          ? resetDraftConfirm
          : resetDeviceConfirm)}
        confirmDisabled={
          confirmKind === "reset_draft"
            ? resetDraftConfirm.confirmDisabled
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

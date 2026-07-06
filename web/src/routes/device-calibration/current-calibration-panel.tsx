import { useMutation } from "@tanstack/react-query";
import {
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  postCalibrationApply,
  postCalibrationCommit,
  postCalibrationReset,
  updateCc,
} from "../../api/client.ts";
import type {
  CalibrationPointCurrent,
  CalibrationProfile,
  FastStatusView,
} from "../../api/types.ts";
import { piecewiseLinearDecimal } from "../../calibration/piecewise.ts";
import { validateAndNormalizeCurrentPoints } from "../../calibration/validation.ts";
import { ConfirmDialog } from "../../components/common/confirm-dialog.tsx";
import {
  CalibrationDeviceWriteButtons,
  CalibrationDraftActionsPanel,
  CalibrationHardwareIoPanel,
} from "./calibration-action-panels.tsx";
import {
  getCopyCh1ToCh2ConfirmConfig,
  getResetCurrentDeviceConfirmConfig,
  getResetDraftConfirmConfig,
} from "./calibration-confirm.ts";
import { CalibrationDeviceViewPanel } from "./calibration-device-view-panel.tsx";
import { CalibrationViewTabs } from "./calibration-view-tabs.tsx";
import {
  runCalibrationReset,
  runCurrentCalibrationWrite,
} from "./device-write.ts";
import { applyCalibrationPreview } from "./preview-profile.ts";
import {
  type CurrentInputUnit,
  formatMaAsA,
  formatUaAsA,
  formatUaToUnit,
  isDraftEmpty,
  parseCurrentInputToUa,
  type RefetchProfile,
  type UndoAction,
  type WithStatusStreamPaused,
} from "./shared.ts";
import { useCalibrationStore } from "./store-context.tsx";

export interface CurrentCalibrationPanelProps {
  curve: "current_ch1" | "current_ch2";
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

export function CurrentCalibrationPanel({
  curve,
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
}: CurrentCalibrationPanelProps) {
  const calibrationStore = useCalibrationStore();
  const [viewTab, setViewTab] = useState<"draft" | "device">("draft");
  const [confirmKind, setConfirmKind] = useState<
    "reset_draft" | "reset_device_current" | "copy_ch1_to_ch2" | null
  >(null);
  const channelLabel = curve === "current_ch1" ? "CH1" : "CH2";
  const channelDisplay = curve === "current_ch1" ? "Local" : "Remote";

  const [inputUnit, setInputUnit] = useState<CurrentInputUnit>("A");
  const [meterReading, setMeterReading] = useState("1.000000");
  const [baselineInputByCurve, setBaselineInputByCurve] = useState<{
    current_ch1: string;
    current_ch2: string;
  }>({ current_ch1: "0.000000", current_ch2: "0.000000" });
  const [baselineUaByCurve, setBaselineUaByCurve] = useState<{
    current_ch1: number;
    current_ch2: number;
  }>({ current_ch1: 0, current_ch2: 0 });
  const [currentOptionsLoaded, setCurrentOptionsLoaded] = useState(false);
  const [targetIMa, setTargetIMa] = useState("1000");

  const baselineInput = baselineInputByCurve[curve];
  const baselineUa = baselineUaByCurve[curve];

  const setBaselineInputForCurve = useCallback(
    (value: string) => {
      setBaselineInputByCurve((prev) => ({ ...prev, [curve]: value }));
      const parsed = parseCurrentInputToUa(value, inputUnit);
      if (parsed != null) {
        setBaselineUaByCurve((prev) => ({ ...prev, [curve]: parsed }));
      }
    },
    [curve, inputUnit],
  );

  const meterUa = useMemo(
    () => parseCurrentInputToUa(meterReading, inputUnit),
    [inputUnit, meterReading],
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    setCurrentOptionsLoaded(false);

    try {
      const ch1 = calibrationStore.getCurrentOptions(
        deviceId,
        baseUrl,
        "current_ch1",
      );
      const ch2 = calibrationStore.getCurrentOptions(
        deviceId,
        baseUrl,
        "current_ch2",
      );
      const unit = ch1.unit ?? ch2.unit ?? "A";
      const baselineUaCh1 = ch1.baselineUa ?? 0;
      const baselineUaCh2 = ch2.baselineUa ?? 0;

      setInputUnit(unit);
      setBaselineUaByCurve({
        current_ch1: baselineUaCh1,
        current_ch2: baselineUaCh2,
      });
      setBaselineInputByCurve({
        current_ch1: formatUaToUnit(baselineUaCh1, unit),
        current_ch2: formatUaToUnit(baselineUaCh2, unit),
      });
      setMeterReading(formatUaToUnit(1_000_000, unit));
    } catch {
      // ignore
    } finally {
      setCurrentOptionsLoaded(true);
    }
  }, [baseUrl, calibrationStore, deviceId]);

  useEffect(() => {
    if (typeof window === "undefined" || !currentOptionsLoaded) return;
    try {
      for (const curveKey of ["current_ch1", "current_ch2"] as const) {
        calibrationStore.setCurrentOptions(deviceId, baseUrl, curveKey, {
          baselineUa: baselineUaByCurve[curveKey],
          unit: inputUnit,
        });
      }
    } catch {
      // ignore
    }
  }, [
    baselineUaByCurve,
    currentOptionsLoaded,
    baseUrl,
    calibrationStore,
    deviceId,
    inputUnit,
  ]);

  const inputUnitStep = inputUnit === "A" ? "0.000001" : "0.001";
  const meterAdjustedUa =
    meterUa == null ? null : Math.max(0, meterUa - baselineUa);
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
  const canWriteToDevice = !isOffline && draftPoints.length > 0;

  const copyCh1DevicePoints = deviceProfile?.current_ch1_points ?? [];
  const copyCh1SourcePoints =
    draftProfile.current_ch1_points.length > 0
      ? draftProfile.current_ch1_points
      : copyCh1DevicePoints;
  const copyCh1SourceLabel =
    draftProfile.current_ch1_points.length > 0 ? "Draft" : "Device";

  const performCopyCh1ToCh2 = () => {
    if (copyCh1SourcePoints.length === 0) {
      onAlert(
        "Cannot Copy",
        "CH1 has no calibration points to copy (draft and device).",
      );
      return;
    }
    onSetDraftProfile((prev) => {
      const source =
        prev.current_ch1_points.length > 0
          ? prev.current_ch1_points
          : copyCh1DevicePoints;
      return {
        ...prev,
        current_ch2_points: source.map((point) => ({ ...point })),
      };
    });
  };

  const handleCopyCh1ToCh2 = () => {
    if (copyCh1SourcePoints.length === 0) {
      onAlert(
        "Cannot Copy",
        "CH1 has no calibration points to copy (draft and device).",
      );
      return;
    }
    if (draftProfile.current_ch2_points.length > 0) {
      setConfirmKind("copy_ch1_to_ch2");
      return;
    }
    performCopyCh1ToCh2();
  };

  const handleUnitChange = (next: CurrentInputUnit) => {
    if (next === inputUnit) return;
    const currentMeterUa = parseCurrentInputToUa(meterReading, inputUnit) ?? 0;
    setInputUnit(next);
    setMeterReading(formatUaToUnit(currentMeterUa, next));
    setBaselineInputByCurve({
      current_ch1: formatUaToUnit(baselineUaByCurve.current_ch1, next),
      current_ch2: formatUaToUnit(baselineUaByCurve.current_ch2, next),
    });
  };

  const handleSetOutput = async () => {
    const parsed = Number.parseInt(targetIMa, 10);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      onAlert("Cannot Set Output", "Invalid target current.");
      return;
    }

    const ok = await ensureMode("Set Output");
    if (!ok) return;

    try {
      await updateCc(baseUrl, { enable: true, target_i_ma: parsed });
    } catch {
      onAlert(
        "Cannot Set Output",
        "Device rejected or failed to apply CC setpoint.",
      );
    }
  };

  const handleCapture = async () => {
    const ok = await ensureMode("Capture");
    if (!ok) return;

    const latest = latestStatusRef.current;
    const rawCur = latest?.raw.raw_cur_100uv;
    const rawDac = latest?.raw.raw_dac_code;
    if (rawCur == null || rawDac == null) {
      onAlert(
        "Cannot Capture Current Point",
        "Raw values not available. Ensure calibration mode is enabled.",
      );
      return;
    }

    const meterUaParsed = parseCurrentInputToUa(meterReading, inputUnit);
    if (meterUaParsed == null) {
      onAlert("Cannot Capture Current Point", "Invalid current input.");
      return;
    }
    const baselineUaParsed = parseCurrentInputToUa(baselineInput, inputUnit);
    if (baselineUaParsed == null) {
      onAlert("Cannot Capture Current Point", "Invalid baseline current.");
      return;
    }
    const measuredUa = meterUaParsed - baselineUaParsed;
    if (measuredUa < 0) {
      onAlert(
        "Cannot Capture Current Point",
        "Baseline current is larger than the meter reading.",
      );
      return;
    }

    const point: CalibrationPointCurrent = {
      raw: rawCur,
      ua: measuredUa,
      dac_code: rawDac,
    };

    const existingIndex = draftPoints.findIndex(
      (existing) =>
        existing.raw === point.raw &&
        existing.ua === point.ua &&
        existing.dac_code === point.dac_code,
    );
    onSetDraftProfile((prev) => {
      const existingPoints =
        curve === "current_ch1"
          ? prev.current_ch1_points
          : prev.current_ch2_points;
      let removed = false;
      const nextPoints: CalibrationPointCurrent[] = [];
      for (const existing of existingPoints) {
        if (
          !removed &&
          existing.raw === point.raw &&
          existing.ua === point.ua &&
          existing.dac_code === point.dac_code
        ) {
          removed = true;
          continue;
        }
        nextPoints.push(existing);
      }
      nextPoints.push(point);

      return {
        ...prev,
        current_ch1_points:
          curve === "current_ch1" ? nextPoints : prev.current_ch1_points,
        current_ch2_points:
          curve === "current_ch2" ? nextPoints : prev.current_ch2_points,
      };
    });

    if (existingIndex !== -1) {
      onInfoToast(
        `Duplicate ${channelLabel} current sample replaced (raw=${point.raw}, dac=${point.dac_code}, value=${formatUaToUnit(point.ua, inputUnit)} ${inputUnit}).`,
      );
    }
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
          ? prev.current_ch1_points.filter((_, row) => row !== index)
          : prev.current_ch1_points,
      current_ch2_points:
        curve === "current_ch2"
          ? prev.current_ch2_points.filter((_, row) => row !== index)
          : prev.current_ch2_points,
    }));
  };

  const activeMa =
    curve === "current_ch1" ? status?.raw.i_local_ma : status?.raw.i_remote_ma;
  const previewUa =
    status?.raw.raw_cur_100uv != null && previewPoints.length >= 1
      ? piecewiseLinearDecimal(
          previewPoints.map((point) => ({ x: point.raw, y: point.ua })),
          status.raw.raw_cur_100uv,
        )
      : null;

  const readMutation = useMutation({
    mutationFn: async () => onRefetchProfile(),
  });

  const applyToDeviceMutation = useMutation({
    mutationFn: async () =>
      runCurrentCalibrationWrite({
        action: "Apply",
        baseUrl,
        curve,
        draftPoints,
        onAlert,
        postPoints: async ({ baseUrl, curve, points }) =>
          postCalibrationApply(baseUrl, {
            kind: curve,
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
      runCurrentCalibrationWrite({
        action: "Commit",
        baseUrl,
        curve,
        draftPoints,
        onAlert,
        postPoints: async ({ baseUrl, curve, points }) =>
          postCalibrationCommit(baseUrl, {
            kind: curve,
            points,
          }),
        withStatusStreamPaused,
      }),
    onSuccess: async () => {
      await onRefetchProfile();
    },
  });

  const resetDeviceCurrentMutation = useMutation({
    mutationFn: async () =>
      runCalibrationReset({
        baseUrl,
        kinds: [curve],
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
  const copyCh1ToCh2Confirm = getCopyCh1ToCh2ConfirmConfig(copyCh1SourceLabel);
  const resetDeviceConfirm = getResetCurrentDeviceConfirmConfig(
    curve,
    channelLabel,
  );

  return (
    <>
      <CalibrationViewTabs
        activeView={viewTab}
        className="ll-tabs mt-4"
        onSelectView={setViewTab}
      />

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

            <CalibrationDraftActionsPanel
              disableApplyPreview={isDraftEmpty(draftProfile)}
              disableExport={isDraftEmpty(draftProfile)}
              disableResetDraft={isDraftEmpty(draftProfile)}
              extraActions={
                curve === "current_ch2" ? (
                  <button
                    type="button"
                    className="ll-button ll-button-sm ll-button-outline"
                    onClick={handleCopyCh1ToCh2}
                    disabled={copyCh1SourcePoints.length === 0}
                  >
                    Copy CH1 → CH2
                  </button>
                ) : null
              }
              importInputId={`calibration-import-${deviceId}-current`}
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
            >
              <div className="ll-panel bg-base-200/40 border border-base-200">
                <div className="ll-panel-body py-4 gap-3">
                  <h4 className="font-bold text-sm">Output control (CC)</h4>
                  <div className="flex gap-2 flex-wrap">
                    {["500", "1000", "2000", "3000", "4000", "5000"].map(
                      (value) => (
                        <button
                          key={value}
                          type="button"
                          className="ll-button ll-button-xs"
                          onClick={() => setTargetIMa(value)}
                        >
                          {value === "500"
                            ? "0.5A"
                            : `${Number(value) / 1000}A`}
                        </button>
                      ),
                    )}
                    <input
                      type="number"
                      className="ll-input ll-input-sm w-28"
                      value={targetIMa}
                      onChange={(event) => setTargetIMa(event.target.value)}
                      disabled={isOffline}
                    />
                    <button
                      type="button"
                      className="ll-button ll-button-sm ll-button-primary"
                      disabled={isOffline}
                      onClick={handleSetOutput}
                    >
                      Set Output
                    </button>
                  </div>
                </div>
              </div>
            </CalibrationHardwareIoPanel>

            <div className="divider my-0"></div>

            <div className="flex items-center justify-between gap-3 flex-wrap">
              <div className="text-sm font-bold whitespace-nowrap shrink-0">
                电流单位
              </div>
              <div className="ll-join">
                <button
                  type="button"
                  className={`ll-button ll-button-sm ll-join-item ${inputUnit === "A" ? "ll-button-active" : ""}`}
                  onClick={() => handleUnitChange("A")}
                  disabled={isOffline}
                >
                  A
                </button>
                <button
                  type="button"
                  className={`ll-button ll-button-sm ll-join-item ${inputUnit === "mA" ? "ll-button-active" : ""}`}
                  onClick={() => handleUnitChange("mA")}
                  disabled={isOffline}
                >
                  mA
                </button>
              </div>
            </div>

            <details className="ll-disclosure bg-base-200/40 border border-base-200">
              <summary className="ll-disclosure-title text-sm font-bold">
                高级选项
              </summary>
              <div className="ll-disclosure-content">
                <label className="ll-form-control w-full max-w-lg">
                  <div className="ll-label-row">
                    <span className="ll-label-text">
                      基础电流扣除 ({channelDisplay}) ({inputUnit})
                    </span>
                  </div>
                  <div className="ll-join w-full">
                    <input
                      type="number"
                      step={inputUnitStep}
                      min="0"
                      className="ll-input ll-input-sm ll-join-item w-full"
                      value={baselineInput}
                      onChange={(event) =>
                        setBaselineInputForCurve(event.target.value)
                      }
                      onBlur={() =>
                        setBaselineInputForCurve(
                          formatUaToUnit(baselineUa, inputUnit),
                        )
                      }
                      disabled={isOffline}
                    />
                    <button
                      type="button"
                      className="ll-button ll-button-sm ll-button-outline ll-join-item"
                      onClick={() => {
                        setBaselineUaByCurve((prev) => ({
                          ...prev,
                          [curve]: 0,
                        }));
                        setBaselineInputForCurve(formatUaToUnit(0, inputUnit));
                      }}
                      disabled={isOffline}
                    >
                      Clear
                    </button>
                  </div>
                </label>
              </div>
            </details>

            <label className="ll-form-control w-full">
              <div className="ll-label-row">
                <span className="ll-label-text">
                  Meter Reading ({channelDisplay}) ({inputUnit})
                </span>
              </div>
              <div className="ll-join">
                <input
                  type="number"
                  step={inputUnitStep}
                  className="ll-input ll-join-item w-full"
                  value={meterReading}
                  onChange={(event) => setMeterReading(event.target.value)}
                  onBlur={() => {
                    const parsed = parseCurrentInputToUa(
                      meterReading,
                      inputUnit,
                    );
                    setMeterReading(formatUaToUnit(parsed ?? 0, inputUnit));
                  }}
                  disabled={isOffline}
                />
                <button
                  type="button"
                  className="ll-button ll-button-secondary ll-join-item"
                  onClick={handleCapture}
                  disabled={isOffline}
                >
                  Capture
                </button>
              </div>
              {meterAdjustedUa != null && baselineUa > 0 ? (
                <div className="ll-label-row">
                  <span className="ll-label-text-alt text-base-content/70">
                    Adjusted: {formatUaToUnit(meterAdjustedUa, inputUnit)}{" "}
                    {inputUnit}
                  </span>
                </div>
              ) : null}
            </label>

            {currentDraft.issues.length > 0 && draftPoints.length > 0 ? (
              <div
                role="alert"
                className="ll-alert ll-alert-warning text-sm py-2"
              >
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
            ) : null}

            <div className="ll-stats shadow">
              <div className="ll-stat">
                <div className="ll-stat-title">Active Current</div>
                <div className="ll-stat-value text-lg">
                  {formatMaAsA(activeMa ?? 0)} A
                </div>
                <div className="ll-stat-desc">
                  Raw: {status?.raw.raw_cur_100uv ?? "--"}
                </div>
              </div>
              <div className="ll-stat">
                <div className="ll-stat-title">DAC Code</div>
                <div className="ll-stat-value text-lg font-mono">
                  {status?.raw.raw_dac_code ?? "--"}
                </div>
              </div>
              <div className="ll-stat">
                <div className="ll-stat-title">Preview Current</div>
                <div className="ll-stat-value text-lg text-primary">
                  {previewUa == null ? "--" : `${formatUaAsA(previewUa)} A`}
                </div>
                <div className="ll-stat-desc">Uses applied preview</div>
              </div>
            </div>

            <div className="overflow-x-auto max-h-64">
              <table className="ll-table ll-table-xs">
                <thead>
                  <tr>
                    <th>Raw</th>
                    <th>DAC</th>
                    <th>Value ({inputUnit})</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {draftPoints.map((sample, index) => (
                    <tr key={`${sample.raw}-${sample.ua}-${sample.dac_code}`}>
                      <td>{sample.raw}</td>
                      <td>{sample.dac_code ?? "--"}</td>
                      <td>{formatUaToUnit(sample.ua, inputUnit)}</td>
                      <td className="text-right">
                        <button
                          type="button"
                          className="ll-button ll-button-ghost ll-button-xs text-error"
                          onClick={() => handleDeleteSample(index)}
                          disabled={isOffline}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                  {draftPoints.length === 0 ? (
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
          onRequestReset={() => setConfirmKind("reset_device_current")}
          readPending={readMutation.isPending}
          resetDisabled={isOffline || resetDeviceCurrentMutation.isPending}
        >
          <div className="overflow-x-auto max-h-64">
            <table className="ll-table ll-table-xs">
              <thead>
                <tr>
                  <th>Raw</th>
                  <th>DAC</th>
                  <th>Value ({inputUnit})</th>
                </tr>
              </thead>
              <tbody>
                {devicePoints.map((point) => (
                  <tr key={`${point.raw}-${point.ua}-${point.dac_code}`}>
                    <td>{point.raw}</td>
                    <td>{point.dac_code ?? "--"}</td>
                    <td>{formatUaToUnit(point.ua, inputUnit)}</td>
                  </tr>
                ))}
                {devicePoints.length === 0 ? (
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
          : confirmKind === "copy_ch1_to_ch2"
            ? copyCh1ToCh2Confirm
            : resetDeviceConfirm)}
        confirmDisabled={
          confirmKind === "reset_draft"
            ? resetDraftConfirm.confirmDisabled
            : confirmKind === "copy_ch1_to_ch2"
              ? copyCh1SourcePoints.length === 0
              : resetDeviceCurrentMutation.isPending || isOffline
        }
        onCancel={() => setConfirmKind(null)}
        onConfirm={() => {
          if (confirmKind === "reset_draft") {
            onResetDraftToEmpty();
          } else if (confirmKind === "copy_ch1_to_ch2") {
            performCopyCh1ToCh2();
          } else if (confirmKind === "reset_device_current") {
            resetDeviceCurrentMutation.mutate();
          }
          setConfirmKind(null);
        }}
      />
    </>
  );
}

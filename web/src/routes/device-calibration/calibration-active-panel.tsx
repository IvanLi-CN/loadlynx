import type { Dispatch, RefObject, SetStateAction } from "react";
import type { CalibrationProfile, FastStatusView } from "../../api/types.ts";
import { CurrentCalibrationPanel } from "./current-calibration-panel.tsx";
import type { CalibrationTab, RefetchProfile, UndoAction } from "./shared.ts";
import { VoltageCalibrationPanel } from "./voltage-calibration-panel.tsx";

export function CalibrationActivePanel(input: {
  activeTab: CalibrationTab;
  baseUrl: string;
  deviceId: string;
  deviceProfile?: CalibrationProfile;
  draftProfile: CalibrationProfile;
  ensureMode: (action: string, opts?: { silent?: boolean }) => Promise<boolean>;
  isOffline: boolean;
  latestStatusRef: RefObject<FastStatusView | null>;
  onAlert: (title: string, body: string, details?: string[]) => void;
  onEnqueueUndo: (action: UndoAction, message: string) => void;
  onExportDraft: () => void;
  onImportDraftFile: (file: File | null) => Promise<void>;
  onInfoToast: (message: string) => void;
  onReadDeviceToDraft: () => void;
  onRefetchProfile: RefetchProfile;
  onResetDraftToEmpty: (message?: string) => void;
  onSetDraftProfile: Dispatch<SetStateAction<CalibrationProfile>>;
  onSetPreviewAppliedAt: Dispatch<SetStateAction<number | null>>;
  onSetPreviewProfile: Dispatch<SetStateAction<CalibrationProfile | null>>;
  previewProfile: CalibrationProfile | null;
  readDeviceToDraftPending: boolean;
  status: FastStatusView | null;
  withStatusStreamPaused: <T>(op: () => Promise<T>) => Promise<T>;
}) {
  const {
    activeTab,
    baseUrl,
    deviceId,
    deviceProfile,
    draftProfile,
    ensureMode,
    isOffline,
    latestStatusRef,
    onAlert,
    onEnqueueUndo,
    onExportDraft,
    onImportDraftFile,
    onInfoToast,
    onReadDeviceToDraft,
    onRefetchProfile,
    onResetDraftToEmpty,
    onSetDraftProfile,
    onSetPreviewAppliedAt,
    onSetPreviewProfile,
    previewProfile,
    readDeviceToDraftPending,
    status,
    withStatusStreamPaused,
  } = input;

  if (activeTab === "voltage") {
    return (
      <VoltageCalibrationPanel
        baseUrl={baseUrl}
        status={status}
        latestStatusRef={latestStatusRef}
        ensureMode={ensureMode}
        withStatusStreamPaused={withStatusStreamPaused}
        deviceProfile={deviceProfile}
        draftProfile={draftProfile}
        previewProfile={previewProfile}
        onSetDraftProfile={onSetDraftProfile}
        onSetPreviewProfile={onSetPreviewProfile}
        onSetPreviewAppliedAt={onSetPreviewAppliedAt}
        deviceId={deviceId}
        onExportDraft={onExportDraft}
        onImportDraftFile={onImportDraftFile}
        onReadDeviceToDraft={onReadDeviceToDraft}
        readDeviceToDraftPending={readDeviceToDraftPending}
        onAlert={onAlert}
        onInfoToast={onInfoToast}
        onEnqueueUndo={onEnqueueUndo}
        onResetDraftToEmpty={onResetDraftToEmpty}
        onRefetchProfile={onRefetchProfile}
        isOffline={isOffline}
      />
    );
  }

  return (
    <CurrentCalibrationPanel
      curve={activeTab}
      baseUrl={baseUrl}
      status={status}
      latestStatusRef={latestStatusRef}
      ensureMode={ensureMode}
      withStatusStreamPaused={withStatusStreamPaused}
      deviceProfile={deviceProfile}
      draftProfile={draftProfile}
      previewProfile={previewProfile}
      onSetDraftProfile={onSetDraftProfile}
      onSetPreviewProfile={onSetPreviewProfile}
      onSetPreviewAppliedAt={onSetPreviewAppliedAt}
      deviceId={deviceId}
      onExportDraft={onExportDraft}
      onImportDraftFile={onImportDraftFile}
      onReadDeviceToDraft={onReadDeviceToDraft}
      readDeviceToDraftPending={readDeviceToDraftPending}
      onAlert={onAlert}
      onInfoToast={onInfoToast}
      onEnqueueUndo={onEnqueueUndo}
      onResetDraftToEmpty={onResetDraftToEmpty}
      onRefetchProfile={onRefetchProfile}
      isOffline={isOffline}
    />
  );
}

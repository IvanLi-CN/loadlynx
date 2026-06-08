import { useQuery } from "@tanstack/react-query";
import { useRouterState } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  getCalibrationProfile,
  type HttpApiError,
  isMockBaseUrl,
  postCalibrationMode,
} from "../api/client.ts";
import type { CalibrationProfile } from "../api/types.ts";
import { calibrationProfilesPointsEqual } from "../calibration/validation.ts";
import { PageContainer } from "../components/layout/page-container.tsx";
import {
  DEVICE_QUERY_PARTS,
  makeDeviceQueryKey,
} from "../devices/device-query-key.ts";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import { usePageVisibility } from "../lib/page-visibility.ts";
import { CalibrationActivePanel } from "./device-calibration/calibration-active-panel.tsx";
import { CalibrationDialogs } from "./device-calibration/calibration-dialogs.tsx";
import { CalibrationOverviewPanel } from "./device-calibration/calibration-overview-panel.tsx";
import { CalibrationTabList } from "./device-calibration/calibration-tab-list.tsx";
import { CalibrationToastStack } from "./device-calibration/calibration-toast-stack.tsx";
import { collectCalibrationDraftIssues } from "./device-calibration/draft-issues.ts";
import {
  type CalibrationTab,
  expectedCalKindForTab,
  isDeviceSubroutePath,
  type RefetchProfile,
  statusInExpectedCalMode,
} from "./device-calibration/shared.ts";
import { useCalibrationStore } from "./device-calibration/store-context.tsx";
import { useCalibrationAlert } from "./device-calibration/use-calibration-alert.ts";
import { useCalibrationDraft } from "./device-calibration/use-calibration-draft.ts";
import { useCalibrationModeSync } from "./device-calibration/use-calibration-mode-sync.ts";
import { useCalibrationStatus } from "./device-calibration/use-calibration-status.ts";
import { useCalibrationToasts } from "./device-calibration/use-calibration-toasts.ts";

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
  const calibrationStore = useCalibrationStore();
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const [activeTab, setActiveTab] = useState<CalibrationTab>(() => {
    if (typeof window === "undefined") {
      return "voltage";
    }
    return (
      calibrationStore.getDraft(deviceId, baseUrl)?.active_tab ?? "voltage"
    );
  });
  const latestPathnameRef = useRef(pathname);
  latestPathnameRef.current = pathname;
  const isPageVisible = usePageVisibility();
  const previousCalibrationBaseUrlRef = useRef<string | null>(null);

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

  const {
    getLatestCalKind,
    getLatestStatus,
    latestStatusRef: statusRef,
    publishStatusSnapshot,
    status,
    waitForStatus,
    withStatusStreamPaused,
  } = useCalibrationStatus({
    deviceId,
    baseUrl,
    isPageVisible,
  });

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
    queryKey: makeDeviceQueryKey(
      deviceId,
      baseUrl,
      ...DEVICE_QUERY_PARTS.calibrationProfile,
    ),
    queryFn: () => getCalibrationProfile(baseUrl),
    enabled: Boolean(baseUrl),
    staleTime: Number.POSITIVE_INFINITY,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });

  const {
    clearToasts,
    enqueueInfoToast,
    enqueueUndo,
    infoToasts,
    undoNow,
    undoToast,
    undoToasts,
  } = useCalibrationToasts();
  const { alertDialog, clearAlert, showAlert } = useCalibrationAlert();

  const {
    applyUndoAction,
    confirmReadDeviceToDraft,
    draftEmpty,
    draftProfile,
    handleExportDraft,
    handleImportDraftFile,
    importError,
    importIssues,
    performReadDeviceToDraft,
    previewAppliedAt,
    previewProfile,
    readDeviceToDraftPending,
    requestReadDeviceToDraft,
    resetDraftToEmpty,
    setConfirmReadDeviceToDraft,
    setDraftProfile,
    setPreviewAppliedAt,
    setPreviewProfile,
  } = useCalibrationDraft({
    activeTab,
    baseUrl,
    calibrationStore,
    clearToasts,
    deviceActiveProfile: profileQuery.data?.active,
    deviceId,
    enqueueInfoToast,
    isOffline,
    onAlert: showAlert,
    refetchProfile: profileQuery.refetch as RefetchProfile,
    setActiveTab,
  });

  const draftIssues = useMemo(
    () => collectCalibrationDraftIssues(draftProfile),
    [draftProfile],
  );

  const previewMatchesDraft = useMemo(() => {
    if (!previewProfile) return null;
    return calibrationProfilesPointsEqual(previewProfile, draftProfile);
  }, [previewProfile, draftProfile]);

  const ensureActiveTabCalMode = useCalibrationModeSync({
    activeTab,
    baseUrl,
    expectedCalKind,
    getLatestCalKind,
    getLatestStatus,
    isOffline,
    onAlert: showAlert,
    publishStatusSnapshot,
    waitForStatus,
    withStatusStreamPaused,
  });

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

      <CalibrationOverviewPanel
        deviceProfile={profileQuery.data}
        deviceCalKind={deviceCalKind}
        draftEmpty={draftEmpty}
        draftIssueCount={draftIssues.length}
        expectedCalKind={expectedCalKind}
        importError={importError}
        importIssues={importIssues}
        isOffline={isOffline}
        onSyncMode={() => {
          void ensureActiveTabCalMode("Sync");
        }}
        previewAppliedAt={previewAppliedAt}
        previewMatchesDraft={previewMatchesDraft}
        previewProfile={previewProfile}
        profileUpdatedAt={profileQuery.dataUpdatedAt}
        statusMatchesActiveTab={statusMatchesActiveTab}
      />

      <CalibrationTabList activeTab={activeTab} onSelectTab={setActiveTab} />

      <CalibrationActivePanel
        activeTab={activeTab}
        baseUrl={baseUrl}
        deviceId={deviceId}
        deviceProfile={profileQuery.data}
        draftProfile={draftProfile}
        ensureMode={ensureActiveTabCalMode}
        isOffline={isOffline}
        latestStatusRef={statusRef}
        onAlert={showAlert}
        onEnqueueUndo={enqueueUndo}
        onExportDraft={handleExportDraft}
        onImportDraftFile={handleImportDraftFile}
        onInfoToast={enqueueInfoToast}
        onReadDeviceToDraft={requestReadDeviceToDraft}
        onRefetchProfile={profileQuery.refetch as RefetchProfile}
        onResetDraftToEmpty={resetDraftToEmpty}
        onSetDraftProfile={setDraftProfile}
        onSetPreviewAppliedAt={setPreviewAppliedAt}
        onSetPreviewProfile={setPreviewProfile}
        previewProfile={previewProfile}
        readDeviceToDraftPending={readDeviceToDraftPending}
        status={statusMatchesActiveTab ? status : null}
        withStatusStreamPaused={withStatusStreamPaused}
      />

      <CalibrationDialogs
        alertDialog={alertDialog}
        confirmReadDeviceToDraft={confirmReadDeviceToDraft}
        isOffline={isOffline}
        onCloseAlert={clearAlert}
        onConfirmReadDeviceToDraft={() => {
          setConfirmReadDeviceToDraft(false);
          void performReadDeviceToDraft();
        }}
        onDismissReadDeviceToDraft={() => setConfirmReadDeviceToDraft(false)}
        readDeviceToDraftPending={readDeviceToDraftPending}
      />

      <CalibrationToastStack
        applyUndoAction={applyUndoAction}
        infoToasts={infoToasts}
        undoNow={undoNow}
        undoToast={undoToast}
        undoToasts={undoToasts}
      />
    </PageContainer>
  );
}

export default DeviceCalibrationRoute;

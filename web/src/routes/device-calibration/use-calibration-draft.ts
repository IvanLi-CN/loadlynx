import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
} from "react";
import type {
  CalibrationActiveProfile,
  CalibrationProfile,
} from "../../api/types.ts";
import type { ValidationIssue } from "../../calibration/validation.ts";
import { downloadJsonFile } from "../../lib/download.ts";
import {
  applyCalibrationUndoAction,
  createCalibrationDraftExportPayload,
  createStoredCalibrationDraft,
  restoreCalibrationDraftProfile,
} from "./draft-codec.ts";
import { parseCalibrationDraftImport } from "./import-draft.ts";
import {
  type CalibrationTab,
  DEFAULT_ACTIVE_PROFILE,
  isDraftEmpty,
  makeEmptyDraftProfile,
  type RefetchProfile,
  type UndoAction,
} from "./shared.ts";
import type { CalibrationStore } from "./store.ts";

export function useCalibrationDraft(input: {
  activeTab: CalibrationTab;
  baseUrl: string;
  calibrationStore: CalibrationStore;
  clearToasts: () => void;
  deviceActiveProfile?: CalibrationActiveProfile;
  deviceId: string;
  enqueueInfoToast: (message: string) => void;
  isOffline: boolean;
  onAlert: (title: string, body: string, details?: string[]) => void;
  refetchProfile: RefetchProfile;
  setActiveTab: (tab: CalibrationTab) => void;
}) {
  const {
    activeTab,
    baseUrl,
    calibrationStore,
    clearToasts,
    deviceActiveProfile,
    deviceId,
    enqueueInfoToast,
    isOffline,
    onAlert,
    refetchProfile,
    setActiveTab,
  } = input;

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
  const [draftStorageReady, setDraftStorageReady] = useState(false);
  const [confirmReadDeviceToDraft, setConfirmReadDeviceToDraft] =
    useState(false);
  const [readDeviceToDraftPending, setReadDeviceToDraftPending] =
    useState(false);

  const applyUndoAction = useCallback((action: UndoAction) => {
    setDraftProfile((prev) => applyCalibrationUndoAction(prev, action));
  }, []);

  const resetDraftToEmpty = useCallback(
    (message = "Draft cleared.") => {
      clearToasts();
      calibrationStore.setDraft(deviceId, baseUrl, null);
      setDraftProfile(makeEmptyDraftProfile(deviceActiveProfile));
      setPreviewProfile(null);
      setPreviewAppliedAt(null);
      setImportError(null);
      setImportIssues(null);
      enqueueInfoToast(message);
    },
    [
      baseUrl,
      calibrationStore,
      clearToasts,
      deviceActiveProfile,
      deviceId,
      enqueueInfoToast,
    ],
  );

  useEffect(() => {
    setDraftProfile((prev) => ({
      ...prev,
      active: deviceActiveProfile ?? prev.active ?? DEFAULT_ACTIVE_PROFILE,
    }));
  }, [deviceActiveProfile]);

  const handleExportDraft = useCallback(() => {
    if (isDraftEmpty(draftProfile)) {
      return;
    }

    const now = new Date();
    const stamp = now.toISOString().replaceAll(":", "-");
    downloadJsonFile(
      `loadlynx-calibration-draft-${deviceId}-${stamp}.json`,
      createCalibrationDraftExportPayload({
        activeSnapshot: deviceActiveProfile ?? draftProfile.active,
        deviceId,
        generatedAt: now,
        profile: draftProfile,
      }),
    );
  }, [deviceActiveProfile, deviceId, draftProfile]);

  const handleImportDraftFile = useCallback(
    async (file: File | null) => {
      if (!file) {
        return;
      }

      setImportError(null);
      setImportIssues(null);
      clearToasts();

      const activeFallback =
        deviceActiveProfile ?? draftProfile.active ?? DEFAULT_ACTIVE_PROFILE;
      const result = parseCalibrationDraftImport(
        await file.text(),
        activeFallback,
      );
      if (!result.ok) {
        setImportError(result.error);
        setImportIssues(result.issues);
        return;
      }

      setDraftProfile(result.profile);
      setPreviewProfile(null);
      setPreviewAppliedAt(null);
      setImportError(null);
      setImportIssues(null);
      enqueueInfoToast("Imported calibration draft.");
    },
    [clearToasts, deviceActiveProfile, draftProfile.active, enqueueInfoToast],
  );

  const draftEmpty = useMemo(() => isDraftEmpty(draftProfile), [draftProfile]);

  const performReadDeviceToDraft = useCallback(async () => {
    if (readDeviceToDraftPending) {
      return;
    }
    if (isOffline) {
      onAlert(
        "Cannot Read Device Profile",
        "Device is offline/faulted; cannot read calibration profile.",
      );
      return;
    }

    clearToasts();
    setImportError(null);
    setImportIssues(null);
    setReadDeviceToDraftPending(true);

    try {
      const result = await refetchProfile();
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
      onAlert("Failed to Read Device Profile", String(error));
    } finally {
      setReadDeviceToDraftPending(false);
    }
  }, [
    clearToasts,
    enqueueInfoToast,
    isOffline,
    onAlert,
    readDeviceToDraftPending,
    refetchProfile,
  ]);

  const requestReadDeviceToDraft = useCallback(() => {
    if (isOffline) {
      onAlert(
        "Cannot Read Device Profile",
        "Device is offline/faulted; cannot read calibration profile.",
      );
      return;
    }
    if (!draftEmpty) {
      setConfirmReadDeviceToDraft(true);
      return;
    }
    void performReadDeviceToDraft();
  }, [draftEmpty, isOffline, onAlert, performReadDeviceToDraft]);

  useLayoutEffect(() => {
    clearToasts();
    setDraftProfile(makeEmptyDraftProfile());
    setPreviewProfile(null);
    setPreviewAppliedAt(null);
    setImportError(null);
    setImportIssues(null);

    setDraftStorageReady(false);
    const stored = calibrationStore.getDraft(deviceId, baseUrl);
    if (stored) {
      setActiveTab(stored.active_tab);
      setDraftProfile(restoreCalibrationDraftProfile(stored));
    } else {
      setActiveTab("voltage");
      setDraftProfile(makeEmptyDraftProfile());
    }
    setDraftStorageReady(true);
  }, [baseUrl, calibrationStore, clearToasts, deviceId, setActiveTab]);

  useEffect(() => {
    if (!draftStorageReady) {
      return;
    }

    if (isDraftEmpty(draftProfile)) {
      calibrationStore.setDraft(deviceId, baseUrl, null);
      return;
    }

    calibrationStore.setDraft(
      deviceId,
      baseUrl,
      createStoredCalibrationDraft({
        activeTab,
        baseUrl,
        deviceId,
        profile: draftProfile,
        savedAt: new Date(),
      }),
    );
  }, [
    activeTab,
    baseUrl,
    calibrationStore,
    deviceId,
    draftProfile,
    draftStorageReady,
  ]);

  return {
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
  };
}

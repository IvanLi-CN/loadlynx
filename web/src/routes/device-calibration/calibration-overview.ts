export interface CalibrationImportIssuePreview {
  path: string;
  message: string;
}

export interface CalibrationOverviewToneLabel {
  tone: "success" | "warning" | "info" | "error" | "neutral" | "ghost";
  label: string;
}

export function describeCalibrationDraftStatus(input: {
  activeSource?: string;
  draftEmpty: boolean;
  deviceUsingDefaults: boolean;
  hasDeviceProfile: boolean;
}): string | null {
  const { activeSource, draftEmpty, deviceUsingDefaults, hasDeviceProfile } =
    input;

  if (!hasDeviceProfile) {
    return null;
  }
  if (draftEmpty && deviceUsingDefaults) {
    return "No user calibration points / device uses defaults.";
  }
  if (draftEmpty) {
    return `No user calibration points in draft / device is ${activeSource}.`;
  }
  return "Draft not synced to device / sync required.";
}

export function describeCalibrationDraftBadge(input: {
  draftEmpty: boolean;
  draftIssueCount: number;
}): CalibrationOverviewToneLabel | null {
  const { draftEmpty, draftIssueCount } = input;

  if (draftIssueCount > 0) {
    return {
      tone: "error",
      label: `Draft issues (${draftIssueCount})`,
    };
  }
  if (!draftEmpty) {
    return {
      tone: "success",
      label: "Draft OK",
    };
  }
  return null;
}

export function describeCalibrationDeviceBadge(input: {
  deviceUsingDefaults: boolean;
  hasDeviceProfile: boolean;
}): CalibrationOverviewToneLabel {
  const { deviceUsingDefaults, hasDeviceProfile } = input;

  if (!hasDeviceProfile) {
    return {
      tone: "neutral",
      label: "Device: --",
    };
  }
  if (deviceUsingDefaults) {
    return {
      tone: "success",
      label: "Device: defaults",
    };
  }
  return {
    tone: "info",
    label: "Device: user-calibrated",
  };
}

export function describeCalibrationPreviewBadge(input: {
  hasPreviewProfile: boolean;
  previewMatchesDraft: boolean | null;
}): CalibrationOverviewToneLabel {
  const { hasPreviewProfile, previewMatchesDraft } = input;

  if (!hasPreviewProfile) {
    return {
      tone: "neutral",
      label: "Preview: device",
    };
  }
  if (previewMatchesDraft) {
    return {
      tone: "neutral",
      label: "Preview up to date",
    };
  }
  return {
    tone: "warning",
    label: "Preview out of date",
  };
}

export function getCalibrationImportIssuePreview(
  issues: CalibrationImportIssuePreview[] | null,
  limit = 5,
): CalibrationImportIssuePreview[] {
  if (!issues || issues.length === 0) {
    return [];
  }
  return issues.slice(0, Math.max(0, limit));
}

import type { CalibrationProfile } from "../../api/types.ts";
import {
  validateAndNormalizeCurrentPoints,
  validateAndNormalizeVoltagePoints,
} from "../../calibration/validation.ts";

export interface CalibrationPreviewProfileResult {
  issueDetails: string[];
  previewProfile: CalibrationProfile;
}

export interface ApplyCalibrationPreviewInput {
  draftProfile: CalibrationProfile;
  now?: () => number;
  onAlert: (title: string, body: string, details?: string[]) => void;
  onSetPreviewAppliedAt: (value: number) => void;
  onSetPreviewProfile: (value: CalibrationProfile) => void;
}

export function createCalibrationPreviewProfile(
  draftProfile: CalibrationProfile,
): CalibrationPreviewProfileResult {
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

  return {
    issueDetails: issues.map((issue) => `${issue.path}: ${issue.message}`),
    previewProfile: {
      active: draftProfile.active,
      v_local_points: vLocal.normalized,
      v_remote_points: vRemote.normalized,
      current_ch1_points: c1.normalized,
      current_ch2_points: c2.normalized,
    },
  };
}

export function applyCalibrationPreview(input: ApplyCalibrationPreviewInput) {
  const {
    draftProfile,
    now = Date.now,
    onAlert,
    onSetPreviewAppliedAt,
    onSetPreviewProfile,
  } = input;
  const { issueDetails, previewProfile } =
    createCalibrationPreviewProfile(draftProfile);

  if (issueDetails.length > 0) {
    onAlert(
      "Calibration data cleanup (Preview)",
      "Draft contains duplicate/conflicting samples. Preview will use a cleaned curve and may drop/merge points.",
      issueDetails,
    );
  }

  onSetPreviewProfile(previewProfile);
  onSetPreviewAppliedAt(now());
}

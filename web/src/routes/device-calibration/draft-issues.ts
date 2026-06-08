import type { CalibrationProfile } from "../../api/types.ts";
import {
  type ValidationIssue,
  validateAndNormalizeCurrentPoints,
  validateAndNormalizeVoltagePoints,
} from "../../calibration/validation.ts";

export function collectCalibrationDraftIssues(
  profile: CalibrationProfile,
): ValidationIssue[] {
  const issues: ValidationIssue[] = [];

  if (profile.v_local_points.length > 0) {
    issues.push(
      ...validateAndNormalizeVoltagePoints("v_local", profile.v_local_points)
        .issues,
    );
  }
  if (profile.v_remote_points.length > 0) {
    issues.push(
      ...validateAndNormalizeVoltagePoints("v_remote", profile.v_remote_points)
        .issues,
    );
  }
  if (profile.current_ch1_points.length > 0) {
    issues.push(
      ...validateAndNormalizeCurrentPoints(
        "current_ch1",
        profile.current_ch1_points,
      ).issues,
    );
  }
  if (profile.current_ch2_points.length > 0) {
    issues.push(
      ...validateAndNormalizeCurrentPoints(
        "current_ch2",
        profile.current_ch2_points,
      ).issues,
    );
  }

  return issues;
}

import type {
  CalibrationActiveProfile,
  CalibrationProfile,
} from "../../api/types.ts";
import {
  type CalibrationTab,
  DEFAULT_ACTIVE_PROFILE,
  makeEmptyDraftProfile,
  type ParsedCalibrationDraft,
  type StoredCalibrationDraftV4,
  type UndoAction,
} from "./shared.ts";

export interface CalibrationDraftExportPayload {
  schema_version: 3;
  generated_at: string;
  device_id: string;
  active_snapshot: CalibrationActiveProfile;
  curves: StoredCalibrationDraftV4["draft_profile"];
}

export function serializeCalibrationDraftProfile(
  profile: CalibrationProfile,
): StoredCalibrationDraftV4["draft_profile"] {
  return {
    v_local_points: profile.v_local_points.map((point) => [
      point.raw,
      point.mv,
    ]),
    v_remote_points: profile.v_remote_points.map((point) => [
      point.raw,
      point.mv,
    ]),
    current_ch1_points: profile.current_ch1_points.map((point) => [
      [point.raw, point.dac_code],
      point.ua,
    ]),
    current_ch2_points: profile.current_ch2_points.map((point) => [
      [point.raw, point.dac_code],
      point.ua,
    ]),
  };
}

export function restoreCalibrationDraftProfile(
  stored: ParsedCalibrationDraft | null,
  active?: CalibrationActiveProfile,
): CalibrationProfile {
  if (!stored) {
    return makeEmptyDraftProfile(active);
  }

  return {
    active: active ?? DEFAULT_ACTIVE_PROFILE,
    v_local_points: stored.draft_profile.v_local_points,
    v_remote_points: stored.draft_profile.v_remote_points,
    current_ch1_points: stored.draft_profile.current_ch1_points,
    current_ch2_points: stored.draft_profile.current_ch2_points,
  };
}

export function createCalibrationDraftExportPayload(input: {
  activeSnapshot: CalibrationActiveProfile;
  deviceId: string;
  generatedAt: Date;
  profile: CalibrationProfile;
}): CalibrationDraftExportPayload {
  const { activeSnapshot, deviceId, generatedAt, profile } = input;
  return {
    schema_version: 3,
    generated_at: generatedAt.toISOString(),
    device_id: deviceId,
    active_snapshot: activeSnapshot,
    curves: serializeCalibrationDraftProfile(profile),
  };
}

export function createStoredCalibrationDraft(input: {
  activeTab: CalibrationTab;
  baseUrl: string;
  deviceId: string;
  profile: CalibrationProfile;
  savedAt: Date;
}): StoredCalibrationDraftV4 {
  const { activeTab, baseUrl, deviceId, profile, savedAt } = input;
  return {
    version: 4,
    saved_at: savedAt.toISOString(),
    device_id: deviceId,
    base_url: baseUrl,
    active_tab: activeTab,
    draft_profile: serializeCalibrationDraftProfile(profile),
  };
}

export function applyCalibrationUndoAction(
  profile: CalibrationProfile,
  action: UndoAction,
): CalibrationProfile {
  if (action.kind === "voltage_points") {
    return {
      ...profile,
      v_local_points: action.local
        ? [...profile.v_local_points, action.local]
        : profile.v_local_points,
      v_remote_points: action.remote
        ? [...profile.v_remote_points, action.remote]
        : profile.v_remote_points,
    };
  }

  return {
    ...profile,
    current_ch1_points:
      action.curve === "current_ch1"
        ? [...profile.current_ch1_points, action.point]
        : profile.current_ch1_points,
    current_ch2_points:
      action.curve === "current_ch2"
        ? [...profile.current_ch2_points, action.point]
        : profile.current_ch2_points,
  };
}

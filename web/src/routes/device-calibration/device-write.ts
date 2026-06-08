import type {
  CalibrationCurveKind,
  CalibrationPointCurrent,
  CalibrationPointVoltage,
} from "../../api/types.ts";
import {
  validateAndNormalizeCurrentPoints,
  validateAndNormalizeVoltagePoints,
} from "../../calibration/validation.ts";
import { retryDeviceCall, type WithStatusStreamPaused } from "./shared.ts";

type AlertFn = (title: string, body: string, details?: string[]) => void;

function getCleanupAlertBody(action: "Apply" | "Commit"): string {
  return `Draft contains duplicate/conflicting samples. ${action} will use a cleaned curve and may drop/merge points.`;
}

export async function runCurrentCalibrationWrite(input: {
  action: "Apply" | "Commit";
  baseUrl: string;
  curve: "current_ch1" | "current_ch2";
  draftPoints: CalibrationPointCurrent[];
  onAlert: AlertFn;
  postPoints: (input: {
    baseUrl: string;
    curve: "current_ch1" | "current_ch2";
    points: CalibrationPointCurrent[];
  }) => Promise<void>;
  withStatusStreamPaused: WithStatusStreamPaused;
}) {
  const {
    action,
    baseUrl,
    curve,
    draftPoints,
    onAlert,
    postPoints,
    withStatusStreamPaused,
  } = input;

  await withStatusStreamPaused(async () => {
    if (draftPoints.length === 0) {
      throw new Error("Draft is empty. Nothing to sync.");
    }

    const validated = validateAndNormalizeCurrentPoints(curve, draftPoints);
    if (validated.issues.length > 0) {
      onAlert(
        `Calibration data cleanup (${action})`,
        getCleanupAlertBody(action),
        validated.issues.map((issue) => `${issue.path}: ${issue.message}`),
      );
    }
    if (validated.normalized.length === 0) {
      throw new Error(
        `No valid points after cleanup. Nothing to ${action.toLowerCase()}.`,
      );
    }

    await retryDeviceCall(() =>
      postPoints({
        baseUrl,
        curve,
        points: validated.normalized,
      }),
    );
  });
}

export async function runVoltageCalibrationWrite(input: {
  action: "Apply" | "Commit";
  baseUrl: string;
  draftLocalPoints: CalibrationPointVoltage[];
  draftRemotePoints: CalibrationPointVoltage[];
  onAlert: AlertFn;
  postPoints: (input: {
    baseUrl: string;
    kind: "v_local" | "v_remote";
    points: CalibrationPointVoltage[];
  }) => Promise<void>;
  withStatusStreamPaused: WithStatusStreamPaused;
}) {
  const {
    action,
    baseUrl,
    draftLocalPoints,
    draftRemotePoints,
    onAlert,
    postPoints,
    withStatusStreamPaused,
  } = input;

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
        `Calibration data cleanup (${action})`,
        getCleanupAlertBody(action),
        issues.map((issue) => `${issue.path}: ${issue.message}`),
      );
    }

    if (local.normalized.length === 0 && remote.normalized.length === 0) {
      throw new Error(
        `No valid points after cleanup. Nothing to ${action.toLowerCase()}.`,
      );
    }

    if (local.normalized.length > 0) {
      await retryDeviceCall(() =>
        postPoints({
          baseUrl,
          kind: "v_local",
          points: local.normalized,
        }),
      );
    }
    if (remote.normalized.length > 0) {
      await retryDeviceCall(() =>
        postPoints({
          baseUrl,
          kind: "v_remote",
          points: remote.normalized,
        }),
      );
    }
  });
}

export async function runCalibrationReset(input: {
  baseUrl: string;
  kinds: CalibrationCurveKind[];
  resetKind: (input: {
    baseUrl: string;
    kind: CalibrationCurveKind;
  }) => Promise<void>;
  withStatusStreamPaused: WithStatusStreamPaused;
}) {
  const { baseUrl, kinds, resetKind, withStatusStreamPaused } = input;

  await withStatusStreamPaused(async () => {
    for (const kind of kinds) {
      await retryDeviceCall(() => resetKind({ baseUrl, kind }));
    }
  });
}

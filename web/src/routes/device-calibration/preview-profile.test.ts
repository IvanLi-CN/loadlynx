import { expect, test } from "vitest";

import type { CalibrationProfile } from "../../api/types.ts";
import {
  applyCalibrationPreview,
  createCalibrationPreviewProfile,
} from "./preview-profile.ts";

test("createCalibrationPreviewProfile normalizes all curves and returns issue details", () => {
  const result = createCalibrationPreviewProfile({
    active: {
      source: "user",
      fmt_version: 3,
      hw_rev: 2,
    },
    v_local_points: [
      { raw: 200, mv: 3000 },
      { raw: 100, mv: 3000 },
    ],
    v_remote_points: [],
    current_ch1_points: [
      { raw: 20, dac_code: 2, ua: 1000 },
      { raw: 10, dac_code: 1, ua: 1000 },
    ],
    current_ch2_points: [],
  });

  expect(result.previewProfile).toEqual({
    active: {
      source: "user",
      fmt_version: 3,
      hw_rev: 2,
    },
    v_local_points: [{ raw: 100, mv: 3000 }],
    v_remote_points: [],
    current_ch1_points: [{ raw: 10, dac_code: 1, ua: 1000 }],
    current_ch2_points: [],
  });
  expect(result.issueDetails).toEqual([
    "v_local_points: duplicate mv=3000 (2 samples): using median",
    "current_ch1_points: duplicate ua=1000 (2 samples): raw uses median, dac uses median",
  ]);
});

test("applyCalibrationPreview alerts with cleanup details and updates preview state", () => {
  const alerts: Array<{
    title: string;
    body: string;
    details?: string[];
  }> = [];
  let appliedAt = 0;
  let previewProfile: CalibrationProfile | null = null;

  applyCalibrationPreview({
    draftProfile: {
      active: {
        source: "user",
        fmt_version: 3,
        hw_rev: 2,
      },
      v_local_points: [
        { raw: 200, mv: 3000 },
        { raw: 100, mv: 3000 },
      ],
      v_remote_points: [],
      current_ch1_points: [],
      current_ch2_points: [],
    },
    now: () => 42,
    onAlert: (title, body, details) => {
      alerts.push({ title, body, details });
    },
    onSetPreviewAppliedAt: (value) => {
      appliedAt = value;
    },
    onSetPreviewProfile: (value) => {
      previewProfile = value;
    },
  });

  expect(alerts).toEqual([
    {
      title: "Calibration data cleanup (Preview)",
      body: "Draft contains duplicate/conflicting samples. Preview will use a cleaned curve and may drop/merge points.",
      details: ["v_local_points: duplicate mv=3000 (2 samples): using median"],
    },
  ]);
  expect(appliedAt).toBe(42);
  expect(previewProfile).toEqual({
    active: {
      source: "user",
      fmt_version: 3,
      hw_rev: 2,
    },
    v_local_points: [{ raw: 100, mv: 3000 }],
    v_remote_points: [],
    current_ch1_points: [],
    current_ch2_points: [],
  });
});

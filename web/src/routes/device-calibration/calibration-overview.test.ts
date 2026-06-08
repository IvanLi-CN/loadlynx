import { expect, test } from "vitest";

import {
  describeCalibrationDeviceBadge,
  describeCalibrationDraftBadge,
  describeCalibrationDraftStatus,
  describeCalibrationPreviewBadge,
  getCalibrationImportIssuePreview,
} from "./calibration-overview.ts";

test("describeCalibrationDraftStatus covers defaults, user profile, and dirty draft", () => {
  expect(
    describeCalibrationDraftStatus({
      activeSource: "factory-default",
      draftEmpty: true,
      deviceUsingDefaults: true,
      hasDeviceProfile: true,
    }),
  ).toBe("No user calibration points / device uses defaults.");

  expect(
    describeCalibrationDraftStatus({
      activeSource: "user",
      draftEmpty: true,
      deviceUsingDefaults: false,
      hasDeviceProfile: true,
    }),
  ).toBe("No user calibration points in draft / device is user.");

  expect(
    describeCalibrationDraftStatus({
      activeSource: "user",
      draftEmpty: false,
      deviceUsingDefaults: false,
      hasDeviceProfile: true,
    }),
  ).toBe("Draft not synced to device / sync required.");

  expect(
    describeCalibrationDraftStatus({
      activeSource: "user",
      draftEmpty: true,
      deviceUsingDefaults: false,
      hasDeviceProfile: false,
    }),
  ).toBeNull();
});

test("describeCalibrationDraftBadge reports error, success, or no badge", () => {
  expect(
    describeCalibrationDraftBadge({
      draftEmpty: false,
      draftIssueCount: 2,
    }),
  ).toEqual({
    tone: "error",
    label: "Draft issues (2)",
  });

  expect(
    describeCalibrationDraftBadge({
      draftEmpty: false,
      draftIssueCount: 0,
    }),
  ).toEqual({
    tone: "success",
    label: "Draft OK",
  });

  expect(
    describeCalibrationDraftBadge({
      draftEmpty: true,
      draftIssueCount: 0,
    }),
  ).toBeNull();
});

test("describeCalibrationDeviceBadge and preview badge cover empty/default/stale states", () => {
  expect(
    describeCalibrationDeviceBadge({
      deviceUsingDefaults: false,
      hasDeviceProfile: false,
    }),
  ).toEqual({
    tone: "neutral",
    label: "Device: --",
  });

  expect(
    describeCalibrationDeviceBadge({
      deviceUsingDefaults: true,
      hasDeviceProfile: true,
    }),
  ).toEqual({
    tone: "success",
    label: "Device: defaults",
  });

  expect(
    describeCalibrationPreviewBadge({
      hasPreviewProfile: false,
      previewMatchesDraft: null,
    }),
  ).toEqual({
    tone: "neutral",
    label: "Preview: device",
  });

  expect(
    describeCalibrationPreviewBadge({
      hasPreviewProfile: true,
      previewMatchesDraft: true,
    }),
  ).toEqual({
    tone: "neutral",
    label: "Preview up to date",
  });

  expect(
    describeCalibrationPreviewBadge({
      hasPreviewProfile: true,
      previewMatchesDraft: false,
    }),
  ).toEqual({
    tone: "warning",
    label: "Preview out of date",
  });
});

test("getCalibrationImportIssuePreview truncates and tolerates missing issues", () => {
  expect(getCalibrationImportIssuePreview(null)).toEqual([]);
  expect(
    getCalibrationImportIssuePreview(
      [
        { path: "a", message: "1" },
        { path: "b", message: "2" },
        { path: "c", message: "3" },
      ],
      2,
    ),
  ).toEqual([
    { path: "a", message: "1" },
    { path: "b", message: "2" },
  ]);
});

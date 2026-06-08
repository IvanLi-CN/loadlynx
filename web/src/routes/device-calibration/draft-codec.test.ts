import { expect, test } from "vitest";

import {
  applyCalibrationUndoAction,
  createCalibrationDraftExportPayload,
  createStoredCalibrationDraft,
  restoreCalibrationDraftProfile,
  serializeCalibrationDraftProfile,
} from "./draft-codec.ts";
import type { ParsedCalibrationDraft } from "./shared.ts";

test("serializeCalibrationDraftProfile converts points to storage wire shape", () => {
  expect(
    serializeCalibrationDraftProfile({
      active: {
        source: "user",
        fmt_version: 3,
        hw_rev: 2,
      },
      v_local_points: [{ raw: 11, mv: 22 }],
      v_remote_points: [{ raw: 33, mv: 44 }],
      current_ch1_points: [{ raw: 55, dac_code: 66, ua: 77 }],
      current_ch2_points: [{ raw: 88, dac_code: 99, ua: 111 }],
    }),
  ).toEqual({
    v_local_points: [[11, 22]],
    v_remote_points: [[33, 44]],
    current_ch1_points: [[[55, 66], 77]],
    current_ch2_points: [[[88, 99], 111]],
  });
});

test("restoreCalibrationDraftProfile restores points and active snapshot", () => {
  const stored: ParsedCalibrationDraft = {
    version: 4,
    saved_at: "2026-06-08T01:02:03.000Z",
    device_id: "device-001",
    base_url: "http://device",
    active_tab: "current_ch2",
    draft_profile: {
      v_local_points: [{ raw: 11, mv: 22 }],
      v_remote_points: [{ raw: 33, mv: 44 }],
      current_ch1_points: [{ raw: 55, dac_code: 66, ua: 77 }],
      current_ch2_points: [{ raw: 88, dac_code: 99, ua: 111 }],
    },
  };

  expect(
    restoreCalibrationDraftProfile(stored, {
      source: "user",
      fmt_version: 7,
      hw_rev: 9,
    }),
  ).toEqual({
    active: {
      source: "user",
      fmt_version: 7,
      hw_rev: 9,
    },
    v_local_points: [{ raw: 11, mv: 22 }],
    v_remote_points: [{ raw: 33, mv: 44 }],
    current_ch1_points: [{ raw: 55, dac_code: 66, ua: 77 }],
    current_ch2_points: [{ raw: 88, dac_code: 99, ua: 111 }],
  });
});

test("createCalibrationDraftExportPayload builds stable export schema", () => {
  const generatedAt = new Date("2026-06-08T01:02:03.000Z");
  expect(
    createCalibrationDraftExportPayload({
      activeSnapshot: {
        source: "user",
        fmt_version: 3,
        hw_rev: 2,
      },
      deviceId: "device-001",
      generatedAt,
      profile: {
        active: {
          source: "factory-default",
          fmt_version: 1,
          hw_rev: 1,
        },
        v_local_points: [{ raw: 11, mv: 22 }],
        v_remote_points: [],
        current_ch1_points: [],
        current_ch2_points: [{ raw: 88, dac_code: 99, ua: 111 }],
      },
    }),
  ).toEqual({
    schema_version: 3,
    generated_at: "2026-06-08T01:02:03.000Z",
    device_id: "device-001",
    active_snapshot: {
      source: "user",
      fmt_version: 3,
      hw_rev: 2,
    },
    curves: {
      v_local_points: [[11, 22]],
      v_remote_points: [],
      current_ch1_points: [],
      current_ch2_points: [[[88, 99], 111]],
    },
  });
});

test("createStoredCalibrationDraft builds v4 storage payload", () => {
  expect(
    createStoredCalibrationDraft({
      activeTab: "current_ch1",
      baseUrl: "http://device",
      deviceId: "device-001",
      profile: {
        active: {
          source: "user",
          fmt_version: 3,
          hw_rev: 2,
        },
        v_local_points: [],
        v_remote_points: [{ raw: 33, mv: 44 }],
        current_ch1_points: [{ raw: 55, dac_code: 66, ua: 77 }],
        current_ch2_points: [],
      },
      savedAt: new Date("2026-06-08T01:02:03.000Z"),
    }),
  ).toEqual({
    version: 4,
    saved_at: "2026-06-08T01:02:03.000Z",
    device_id: "device-001",
    base_url: "http://device",
    active_tab: "current_ch1",
    draft_profile: {
      v_local_points: [],
      v_remote_points: [[33, 44]],
      current_ch1_points: [[[55, 66], 77]],
      current_ch2_points: [],
    },
  });
});

test("applyCalibrationUndoAction appends restored voltage and current points", () => {
  const profile = {
    active: {
      source: "user" as const,
      fmt_version: 3,
      hw_rev: 2,
    },
    v_local_points: [{ raw: 1, mv: 2 }],
    v_remote_points: [],
    current_ch1_points: [],
    current_ch2_points: [{ raw: 3, dac_code: 4, ua: 5 }],
  };

  expect(
    applyCalibrationUndoAction(profile, {
      kind: "voltage_points",
      local: { raw: 6, mv: 7 },
      remote: { raw: 8, mv: 9 },
    }),
  ).toEqual({
    ...profile,
    v_local_points: [
      { raw: 1, mv: 2 },
      { raw: 6, mv: 7 },
    ],
    v_remote_points: [{ raw: 8, mv: 9 }],
  });

  expect(
    applyCalibrationUndoAction(profile, {
      kind: "current_point",
      curve: "current_ch1",
      point: { raw: 10, dac_code: 11, ua: 12 },
    }),
  ).toEqual({
    ...profile,
    current_ch1_points: [{ raw: 10, dac_code: 11, ua: 12 }],
    current_ch2_points: [{ raw: 3, dac_code: 4, ua: 5 }],
  });
});

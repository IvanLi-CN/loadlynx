import { expect, test } from "vitest";

import {
  mapCalibrationProfileUiToWire,
  mapCalibrationProfileWireToUi,
} from "./client-calibration.ts";
import {
  mockGetCalibrationProfile,
  mockPostCalibrationApply,
  mockPostCalibrationCommit,
} from "./client-mock-calibration.ts";

test("preserves calibration persistence status across profile mapping", () => {
  const wire = {
    active: { source: "user-calibrated", fmt_version: 3, hw_rev: 1 },
    persistence: { status: "commit-verified" },
    v_local_points: [{ raw_100uv: 1, meas_mv: 2 }],
    v_remote_points: [],
    current_ch1_points: [{ raw_100uv: 3, raw_dac_code: 4, meas_ma: 5 }],
    current_ch2_points: [],
  };

  const ui = mapCalibrationProfileWireToUi(wire);
  expect(ui.persistence).toEqual({ status: "commit-verified" });
  expect(mapCalibrationProfileUiToWire(ui).persistence).toEqual({
    status: "commit-verified",
  });
});

test("mock calibration preserves persistence through Apply and Commit", async () => {
  const baseUrl = "mock://calibration-persistence-regression";
  const initial = await mockGetCalibrationProfile(baseUrl);
  expect(initial.persistence).toEqual({ status: "factory-default" });

  const request = {
    kind: "v_local" as const,
    points: [
      { raw: 100, mv: 3000 },
      { raw: 200, mv: 5000 },
    ],
  };
  await mockPostCalibrationApply(baseUrl, request);
  expect((await mockGetCalibrationProfile(baseUrl)).persistence).toEqual({
    status: "ram-only",
  });

  await mockPostCalibrationCommit(baseUrl, request);
  expect((await mockGetCalibrationProfile(baseUrl)).persistence).toEqual({
    status: "commit-verified",
  });
});

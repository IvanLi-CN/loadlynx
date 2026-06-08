import { expect, test } from "vitest";

import { collectCalibrationDraftIssues } from "./draft-issues.ts";

test("collectCalibrationDraftIssues merges validation issues across curves", () => {
  const issues = collectCalibrationDraftIssues({
    active: {
      source: "user",
      fmt_version: 3,
      hw_rev: 2,
    },
    v_local_points: [{ raw: 1.5, mv: 1000 }],
    v_remote_points: [],
    current_ch1_points: [],
    current_ch2_points: [{ raw: 10, dac_code: 12.5, ua: 2000 }],
  });

  expect(issues).toEqual([
    {
      path: "v_local_points[0].raw",
      message: "raw_100uv must be an integer",
    },
    {
      path: "v_local_points",
      message: "points must contain 1..24 items",
    },
    {
      path: "current_ch2_points[0].dac_code",
      message: "raw_dac_code must be an integer",
    },
    {
      path: "current_ch2_points",
      message: "points must contain 1..24 items",
    },
  ]);
});

test("collectCalibrationDraftIssues skips empty curves without synthetic errors", () => {
  expect(
    collectCalibrationDraftIssues({
      active: {
        source: "factory-default",
        fmt_version: 1,
        hw_rev: 1,
      },
      v_local_points: [],
      v_remote_points: [],
      current_ch1_points: [],
      current_ch2_points: [],
    }),
  ).toEqual([]);
});

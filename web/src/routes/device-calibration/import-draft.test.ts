import { expect, test } from "vitest";

import type { CalibrationActiveProfile } from "../../api/types.ts";
import { parseCalibrationDraftImport } from "./import-draft.ts";

const ACTIVE_FALLBACK: CalibrationActiveProfile = {
  source: "factory-default",
  fmt_version: 1,
  hw_rev: 1,
};

test("parseCalibrationDraftImport accepts valid schema v3 exports", () => {
  const result = parseCalibrationDraftImport(
    JSON.stringify({
      schema_version: 3,
      curves: {
        v_local_points: [[100, 2500]],
        v_remote_points: [[120, 2600]],
        current_ch1_points: [[[10, 20], 3000]],
        current_ch2_points: [[[11, 21], 4000]],
      },
    }),
    ACTIVE_FALLBACK,
  );

  expect(result.ok).toBe(true);
  if (!result.ok) {
    return;
  }
  expect(result.profile.v_local_points).toEqual([{ raw: 100, mv: 2500 }]);
  expect(result.profile.current_ch1_points).toEqual([
    { raw: 10, dac_code: 20, ua: 3000 },
  ]);
});

test("parseCalibrationDraftImport converts legacy milliamp current drafts to microamps", () => {
  const result = parseCalibrationDraftImport(
    JSON.stringify({
      version: 2,
      profile: {
        v_local_points: [[100, 2500]],
        v_remote_points: [[120, 2600]],
        current_ch1_points: [[[10, 20], 3]],
        current_ch2_points: [[[11, 21], 4]],
      },
    }),
    ACTIVE_FALLBACK,
  );

  expect(result.ok).toBe(true);
  if (!result.ok) {
    return;
  }
  expect(result.profile.current_ch1_points[0]?.ua).toBe(3000);
  expect(result.profile.current_ch2_points[0]?.ua).toBe(4000);
});

test("parseCalibrationDraftImport rejects out-of-range calibration values", () => {
  const result = parseCalibrationDraftImport(
    JSON.stringify({
      schema_version: 3,
      curves: {
        v_local_points: [[50000, 2500]],
        v_remote_points: [[120, 2600]],
        current_ch1_points: [[[10, 20], 3000]],
        current_ch2_points: [[[11, 21], 4000]],
      },
    }),
    ACTIVE_FALLBACK,
  );

  expect(result.ok).toBe(false);
  if (result.ok) {
    return;
  }
  expect(result.error).toBe("Import validation failed.");
  expect(
    result.issues?.some((issue) => issue.path === "v_local_points[0].raw"),
  ).toBe(true);
});

test("parseCalibrationDraftImport rejects invalid JSON", () => {
  const result = parseCalibrationDraftImport("{not-json", ACTIVE_FALLBACK);

  expect(result).toEqual({
    ok: false,
    error: "Invalid JSON file.",
    issues: null,
  });
});

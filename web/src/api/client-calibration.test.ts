import { expect, test } from "vitest";

import {
  mapCalibrationProfileUiToWire,
  mapCalibrationProfileWireToUi,
} from "./client-calibration.ts";

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

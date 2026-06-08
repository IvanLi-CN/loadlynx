import { expect, test } from "vitest";

import {
  getCopyCh1ToCh2ConfirmConfig,
  getResetCurrentDeviceConfirmConfig,
  getResetDraftConfirmConfig,
  getResetVoltageDeviceConfirmConfig,
} from "./calibration-confirm.ts";

test("getResetDraftConfirmConfig returns shared reset-draft semantics", () => {
  expect(getResetDraftConfirmConfig(false)).toEqual({
    title: "Reset Draft (Web only)",
    body: "This clears the local draft (user calibration points). The device is unchanged.",
    details: [
      "Affects: v_local, v_remote, current_ch1, current_ch2 (local draft only).",
      "Writes device: No.",
      "This clears all local draft points (export first if needed).",
    ],
    confirmLabel: "Reset Draft",
    destructive: false,
    confirmDisabled: false,
  });

  expect(getResetDraftConfirmConfig(true).confirmDisabled).toBe(true);
});

test("device reset confirm configs stay aligned across panels", () => {
  expect(getResetCurrentDeviceConfirmConfig("current_ch2", "CH2")).toEqual({
    title: "Reset Device Calibration (Current CH2)",
    body: "This resets current calibration on the device.",
    details: [
      "Affects: current_ch2.",
      "Writes device: Yes.",
      "Irreversible: Yes (re-calibrate + commit to recover).",
    ],
    confirmLabel: "Reset",
    destructive: true,
  });

  expect(getResetVoltageDeviceConfirmConfig()).toEqual({
    title: "Reset Device Calibration (Voltage)",
    body: "This resets voltage calibration on the device.",
    details: [
      "Affects: v_local + v_remote.",
      "Writes device: Yes.",
      "Irreversible: Yes (re-calibrate + commit to recover).",
      "Does not affect: current_ch1/current_ch2.",
    ],
    confirmLabel: "Reset",
    destructive: true,
  });
});

test("copy current-channel confirm config includes source label", () => {
  expect(getCopyCh1ToCh2ConfirmConfig("Draft")).toEqual({
    title: "Copy CH1 → CH2 (Draft)",
    body: "This overwrites CH2 draft points with CH1 calibration points. The device is unchanged.",
    details: [
      "Affects: current_ch2 (local draft only).",
      "Source: current_ch1 (Draft).",
      "Writes device: No.",
      "Irreversible locally: Yes (export draft first if needed).",
    ],
    confirmLabel: "Copy",
    destructive: false,
  });
});

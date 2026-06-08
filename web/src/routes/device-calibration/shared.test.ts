import { expect, test } from "vitest";

import {
  readCalibrationCurrentOptionsFromStorage,
  readCalibrationDraftFromStorage,
  readCalibrationVoltageOptionsFromStorage,
  writeCalibrationCurrentOptionsToStorage,
  writeCalibrationDraftToStorage,
  writeCalibrationVoltageOptionsToStorage,
} from "./shared.ts";

function makeStorage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem(key: string) {
      return values.get(key) ?? null;
    },
    setItem(key: string, value: string) {
      values.set(key, value);
    },
    removeItem(key: string) {
      values.delete(key);
    },
  };
}

test("readCalibrationCurrentOptionsFromStorage reads v2 values", () => {
  const storage = makeStorage({
    "loadlynx:calibration-current-options:v2:device-001:http%3A%2F%2Fdevice:current_ch1":
      JSON.stringify({
        baseline_ua: 123456,
        unit: "mA",
      }),
  });

  expect(
    readCalibrationCurrentOptionsFromStorage(
      storage,
      "device-001",
      "http://device",
      "current_ch1",
    ),
  ).toEqual({
    baselineUa: 123456,
    unit: "mA",
  });
});

test("readCalibrationCurrentOptionsFromStorage falls back to v1 baseline", () => {
  const storage = makeStorage({
    "loadlynx:calibration-current-options:v1:device-001:http%3A%2F%2Fdevice:current_ch2":
      JSON.stringify({
        baseline_a: "0.250000",
      }),
  });

  expect(
    readCalibrationCurrentOptionsFromStorage(
      storage,
      "device-001",
      "http://device",
      "current_ch2",
    ),
  ).toEqual({
    baselineUa: 250000,
    unit: null,
  });
});

test("readCalibrationCurrentOptionsFromStorage tolerates malformed JSON", () => {
  const storage = makeStorage({
    "loadlynx:calibration-current-options:v2:device-001:http%3A%2F%2Fdevice:current_ch1":
      "{bad-json",
  });

  expect(
    readCalibrationCurrentOptionsFromStorage(
      storage,
      "device-001",
      "http://device",
      "current_ch1",
    ),
  ).toEqual({
    baselineUa: null,
    unit: null,
  });
});

test("writeCalibrationCurrentOptionsToStorage writes v2 and removes v1", () => {
  const storage = makeStorage({
    "loadlynx:calibration-current-options:v1:device-001:http%3A%2F%2Fdevice:current_ch1":
      JSON.stringify({
        baseline_a: "1.000000",
      }),
  });

  writeCalibrationCurrentOptionsToStorage(
    storage,
    "device-001",
    "http://device",
    "current_ch1",
    {
      baselineUa: 500000,
      unit: "A",
    },
  );

  expect(
    storage.getItem(
      "loadlynx:calibration-current-options:v2:device-001:http%3A%2F%2Fdevice:current_ch1",
    ),
  ).toBe(JSON.stringify({ baseline_ua: 500000, unit: "A" }));
  expect(
    storage.getItem(
      "loadlynx:calibration-current-options:v1:device-001:http%3A%2F%2Fdevice:current_ch1",
    ),
  ).toBeNull();
});

test("readCalibrationVoltageOptionsFromStorage reads v2 values", () => {
  const storage = makeStorage({
    "loadlynx:calibration-voltage-options:v2:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({
        input_uv: 12_345_678,
        unit: "V",
      }),
  });

  expect(
    readCalibrationVoltageOptionsFromStorage(
      storage,
      "device-001",
      "http://device",
    ),
  ).toEqual({
    inputUv: 12_345_678,
    unit: "V",
  });
});

test("readCalibrationVoltageOptionsFromStorage falls back to v1 input", () => {
  const storage = makeStorage({
    "loadlynx:calibration-voltage-options:v1:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({
        input_v: "4.250000",
      }),
  });

  expect(
    readCalibrationVoltageOptionsFromStorage(
      storage,
      "device-001",
      "http://device",
    ),
  ).toEqual({
    inputUv: 4_250_000,
    unit: "V",
  });
});

test("writeCalibrationVoltageOptionsToStorage writes v2 and removes v1", () => {
  const storage = makeStorage({
    "loadlynx:calibration-voltage-options:v1:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({
        input_v: "1.000000",
      }),
  });

  writeCalibrationVoltageOptionsToStorage(
    storage,
    "device-001",
    "http://device",
    {
      inputUv: 500_000,
      unit: "V",
    },
  );

  expect(
    storage.getItem(
      "loadlynx:calibration-voltage-options:v2:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBe(JSON.stringify({ input_uv: 500_000, unit: "V" }));
  expect(
    storage.getItem(
      "loadlynx:calibration-voltage-options:v1:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBeNull();
});

test("readCalibrationDraftFromStorage reads v4 draft payload", () => {
  const storage = makeStorage({
    "loadlynx:calibration-draft:v4:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({
        version: 4,
        saved_at: "2026-06-08T00:00:00.000Z",
        device_id: "device-001",
        base_url: "http://device",
        active_tab: "current_ch2",
        draft_profile: {
          v_local_points: [[123, 4567]],
          v_remote_points: [{ raw: 124, mv: 4568 }],
          current_ch1_points: [[[11, 22], 333444]],
          current_ch2_points: [{ raw: 55, dac_code: 66, ua: 777888 }],
        },
      }),
  });

  expect(
    readCalibrationDraftFromStorage(storage, "device-001", "http://device"),
  ).toEqual({
    version: 4,
    saved_at: "2026-06-08T00:00:00.000Z",
    device_id: "device-001",
    base_url: "http://device",
    active_tab: "current_ch2",
    draft_profile: {
      v_local_points: [{ raw: 123, mv: 4567 }],
      v_remote_points: [{ raw: 124, mv: 4568 }],
      current_ch1_points: [{ raw: 11, dac_code: 22, ua: 333444 }],
      current_ch2_points: [{ raw: 55, dac_code: 66, ua: 777888 }],
    },
  });
});

test("readCalibrationDraftFromStorage upgrades legacy current tab and v3 current units", () => {
  const storage = makeStorage({
    "loadlynx:calibration-draft:v3:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({
        version: 3,
        saved_at: "2026-06-08T00:00:00.000Z",
        device_id: "device-001",
        base_url: "http://device",
        active_tab: "current",
        draft_profile: {
          v_local_points: [],
          v_remote_points: [],
          current_ch1_points: [[[10, 20], 123]],
          current_ch2_points: [{ raw: 30, raw_dac_code: 40, meas_ma: 456 }],
        },
      }),
  });

  expect(
    readCalibrationDraftFromStorage(storage, "device-001", "http://device"),
  ).toEqual({
    version: 4,
    saved_at: "2026-06-08T00:00:00.000Z",
    device_id: "device-001",
    base_url: "http://device",
    active_tab: "current_ch1",
    draft_profile: {
      v_local_points: [],
      v_remote_points: [],
      current_ch1_points: [{ raw: 10, dac_code: 20, ua: 123000 }],
      current_ch2_points: [{ raw: 30, dac_code: 40, ua: 456000 }],
    },
  });
});

test("writeCalibrationDraftToStorage writes v4 and removes legacy versions", () => {
  const storage = makeStorage({
    "loadlynx:calibration-draft:v2:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({ version: 2 }),
    "loadlynx:calibration-draft:v3:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({ version: 3 }),
  });

  writeCalibrationDraftToStorage(storage, "device-001", "http://device", {
    version: 4,
    saved_at: "2026-06-08T00:00:00.000Z",
    device_id: "device-001",
    base_url: "http://device",
    active_tab: "voltage",
    draft_profile: {
      v_local_points: [[1, 2]],
      v_remote_points: [[3, 4]],
      current_ch1_points: [[[5, 6], 7]],
      current_ch2_points: [[[8, 9], 10]],
    },
  });

  expect(
    storage.getItem(
      "loadlynx:calibration-draft:v4:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBe(
    JSON.stringify({
      version: 4,
      saved_at: "2026-06-08T00:00:00.000Z",
      device_id: "device-001",
      base_url: "http://device",
      active_tab: "voltage",
      draft_profile: {
        v_local_points: [[1, 2]],
        v_remote_points: [[3, 4]],
        current_ch1_points: [[[5, 6], 7]],
        current_ch2_points: [[[8, 9], 10]],
      },
    }),
  );
  expect(
    storage.getItem(
      "loadlynx:calibration-draft:v2:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBeNull();
  expect(
    storage.getItem(
      "loadlynx:calibration-draft:v3:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBeNull();
});

test("writeCalibrationDraftToStorage clears all versions when draft is null", () => {
  const storage = makeStorage({
    "loadlynx:calibration-draft:v2:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({ version: 2 }),
    "loadlynx:calibration-draft:v3:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({ version: 3 }),
    "loadlynx:calibration-draft:v4:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({ version: 4 }),
  });

  writeCalibrationDraftToStorage(storage, "device-001", "http://device", null);

  expect(
    storage.getItem(
      "loadlynx:calibration-draft:v2:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBeNull();
  expect(
    storage.getItem(
      "loadlynx:calibration-draft:v3:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBeNull();
  expect(
    storage.getItem(
      "loadlynx:calibration-draft:v4:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBeNull();
});

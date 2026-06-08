import { expect, test } from "vitest";

import {
  LocalStorageCalibrationStore,
  MemoryCalibrationStore,
} from "./store.ts";

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

test("MemoryCalibrationStore stores and restores draft and options", () => {
  const store = new MemoryCalibrationStore();

  store.setDraft("device-001", "http://device", {
    version: 4,
    saved_at: "2026-06-08T00:00:00.000Z",
    device_id: "device-001",
    base_url: "http://device",
    active_tab: "current_ch2",
    draft_profile: {
      v_local_points: [[1, 2]],
      v_remote_points: [[3, 4]],
      current_ch1_points: [[[5, 6], 7]],
      current_ch2_points: [[[8, 9], 10]],
    },
  });
  store.setCurrentOptions("device-001", "http://device", "current_ch1", {
    baselineUa: 123456,
    unit: "A",
  });
  store.setVoltageOptions("device-001", "http://device", {
    inputUv: 12_345_678,
    unit: "V",
  });

  expect(store.getDraft("device-001", "http://device")).toEqual({
    version: 4,
    saved_at: "2026-06-08T00:00:00.000Z",
    device_id: "device-001",
    base_url: "http://device",
    active_tab: "current_ch2",
    draft_profile: {
      v_local_points: [{ raw: 1, mv: 2 }],
      v_remote_points: [{ raw: 3, mv: 4 }],
      current_ch1_points: [{ raw: 5, dac_code: 6, ua: 7 }],
      current_ch2_points: [{ raw: 8, dac_code: 9, ua: 10 }],
    },
  });
  expect(
    store.getCurrentOptions("device-001", "http://device", "current_ch1"),
  ).toEqual({
    baselineUa: 123456,
    unit: "A",
  });
  expect(store.getVoltageOptions("device-001", "http://device")).toEqual({
    inputUv: 12_345_678,
    unit: "V",
  });
});

test("LocalStorageCalibrationStore delegates to supplied storage", () => {
  const storage = makeStorage({
    "loadlynx:calibration-draft:v4:device-001:http%3A%2F%2Fdevice":
      JSON.stringify({
        version: 4,
        saved_at: "2026-06-08T00:00:00.000Z",
        device_id: "device-001",
        base_url: "http://device",
        active_tab: "voltage",
        draft_profile: {
          v_local_points: [[1, 2]],
          v_remote_points: [],
          current_ch1_points: [],
          current_ch2_points: [],
        },
      }),
  });
  const store = new LocalStorageCalibrationStore(storage as Storage);

  expect(store.getDraft("device-001", "http://device")?.active_tab).toBe(
    "voltage",
  );

  store.setVoltageOptions("device-001", "http://device", {
    inputUv: 500_000,
    unit: "V",
  });

  expect(
    storage.getItem(
      "loadlynx:calibration-voltage-options:v2:device-001:http%3A%2F%2Fdevice",
    ),
  ).toBe(JSON.stringify({ input_uv: 500_000, unit: "V" }));
});

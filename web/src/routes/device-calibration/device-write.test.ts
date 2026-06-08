import { expect, test } from "vitest";

import {
  runCalibrationReset,
  runCurrentCalibrationWrite,
  runVoltageCalibrationWrite,
} from "./device-write.ts";

test("runCurrentCalibrationWrite normalizes points and alerts on cleanup", async () => {
  const alerts: Array<{
    title: string;
    body: string;
    details?: string[];
  }> = [];
  const writes: Array<{
    baseUrl: string;
    curve: "current_ch1" | "current_ch2";
    points: Array<{ raw: number; ua: number; dac_code: number }>;
  }> = [];
  let pauseCalls = 0;

  await runCurrentCalibrationWrite({
    action: "Apply",
    baseUrl: "http://device",
    curve: "current_ch1",
    draftPoints: [
      { raw: 20, ua: 1000, dac_code: 2 },
      { raw: 10, ua: 1000, dac_code: 1 },
    ],
    onAlert: (title, body, details) => {
      alerts.push({ title, body, details });
    },
    postPoints: async (input) => {
      writes.push(input);
    },
    withStatusStreamPaused: async (op) => {
      pauseCalls += 1;
      return op();
    },
  });

  expect(pauseCalls).toBe(1);
  expect(alerts).toEqual([
    {
      title: "Calibration data cleanup (Apply)",
      body: "Draft contains duplicate/conflicting samples. Apply will use a cleaned curve and may drop/merge points.",
      details: [
        "current_ch1_points: duplicate ua=1000 (2 samples): raw uses median, dac uses median",
      ],
    },
  ]);
  expect(writes).toEqual([
    {
      baseUrl: "http://device",
      curve: "current_ch1",
      points: [{ raw: 10, ua: 1000, dac_code: 1 }],
    },
  ]);
});

test("runCurrentCalibrationWrite rejects empty drafts before writing", async () => {
  const writes: unknown[] = [];

  await expect(
    runCurrentCalibrationWrite({
      action: "Commit",
      baseUrl: "http://device",
      curve: "current_ch2",
      draftPoints: [],
      onAlert: () => {},
      postPoints: async (input) => {
        writes.push(input);
      },
      withStatusStreamPaused: async (op) => op(),
    }),
  ).rejects.toThrow("Draft is empty. Nothing to sync.");

  expect(writes).toHaveLength(0);
});

test("runVoltageCalibrationWrite writes only normalized non-empty curves", async () => {
  const alerts: Array<{
    title: string;
    body: string;
    details?: string[];
  }> = [];
  const writes: Array<{
    baseUrl: string;
    kind: "v_local" | "v_remote";
    points: Array<{ raw: number; mv: number }>;
  }> = [];

  await runVoltageCalibrationWrite({
    action: "Commit",
    baseUrl: "http://device",
    draftLocalPoints: [
      { raw: 200, mv: 3000 },
      { raw: 100, mv: 3000 },
    ],
    draftRemotePoints: [],
    onAlert: (title, body, details) => {
      alerts.push({ title, body, details });
    },
    postPoints: async (input) => {
      writes.push(input);
    },
    withStatusStreamPaused: async (op) => op(),
  });

  expect(alerts).toEqual([
    {
      title: "Calibration data cleanup (Commit)",
      body: "Draft contains duplicate/conflicting samples. Commit will use a cleaned curve and may drop/merge points.",
      details: ["v_local_points: duplicate mv=3000 (2 samples): using median"],
    },
  ]);
  expect(writes).toEqual([
    {
      baseUrl: "http://device",
      kind: "v_local",
      points: [{ raw: 100, mv: 3000 }],
    },
  ]);
});

test("runCalibrationReset resets each requested kind under one pause window", async () => {
  const resets: Array<{ baseUrl: string; kind: string }> = [];
  let pauseCalls = 0;

  await runCalibrationReset({
    baseUrl: "http://device",
    kinds: ["v_local", "v_remote"],
    resetKind: async (input) => {
      resets.push(input);
    },
    withStatusStreamPaused: async (op) => {
      pauseCalls += 1;
      return op();
    },
  });

  expect(pauseCalls).toBe(1);
  expect(resets).toEqual([
    { baseUrl: "http://device", kind: "v_local" },
    { baseUrl: "http://device", kind: "v_remote" },
  ]);
});

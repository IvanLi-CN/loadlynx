import { describe, expect, it } from "vitest";
import type { Preset } from "../../api/types.ts";
import {
  makePresetInputMemoryKey,
  parsePresetInputValue,
  reconcilePresetInputMemory,
} from "./preset-input-memory.ts";

describe("preset-input-memory", () => {
  it("parses current input with unit into internal mA value and keeps formatted text", () => {
    expect(parsePresetInputValue(" 2.3A ", "current")).toEqual({
      ok: true,
      value: 2300,
      displayText: "2.3 A",
    });
  });

  it("parses plain integer input as raw internal units", () => {
    expect(parsePresetInputValue("23000", "current")).toEqual({
      ok: true,
      value: 23000,
      displayText: "23000",
    });
  });

  it("drops remembered display text once hardware preset value changes", () => {
    const preset: Preset = {
      preset_id: 1,
      mode: "cc",
      target_i_ma: 2500,
      target_v_mv: 12000,
      target_p_mw: 15000,
      min_v_mv: 0,
      max_i_ma_total: 10000,
      max_p_mw: 150000,
    };

    const key = makePresetInputMemoryKey({
      deviceId: "mock-001",
      baseUrl: "http://127.0.0.1:22848",
      presetId: 1,
      field: "target_i_ma",
    });

    const next = reconcilePresetInputMemory({
      store: {
        [key]: {
          value: 2300,
          text: "2.3 A",
        },
      },
      deviceId: "mock-001",
      baseUrl: "http://127.0.0.1:22848",
      presets: [preset],
    });

    expect(next).toEqual({});
  });

  it("keeps remembered display text while hardware preset value still matches", () => {
    const preset: Preset = {
      preset_id: 1,
      mode: "cc",
      target_i_ma: 2300,
      target_v_mv: 12000,
      target_p_mw: 15000,
      min_v_mv: 0,
      max_i_ma_total: 10000,
      max_p_mw: 150000,
    };

    const key = makePresetInputMemoryKey({
      deviceId: "mock-001",
      baseUrl: "http://127.0.0.1:22848",
      presetId: 1,
      field: "target_i_ma",
    });

    const next = reconcilePresetInputMemory({
      store: {
        [key]: {
          value: 2300,
          text: "2.3 A",
        },
      },
      deviceId: "mock-001",
      baseUrl: "http://127.0.0.1:22848",
      presets: [preset],
    });

    expect(next).toEqual({
      [key]: {
        value: 2300,
        text: "2.3 A",
      },
    });
  });
});

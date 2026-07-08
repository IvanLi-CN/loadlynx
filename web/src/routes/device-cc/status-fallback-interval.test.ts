import { describe, expect, it } from "vitest";
import {
  DEVD_FAST_STATUS_TARGET_PERIOD_MS,
  getManualStatusPollDelayMs,
  getFastStatusRefetchIntervalMs,
  HTTP_FAST_STATUS_REFETCH_MS,
  usesManualDevdStatusPolling,
} from "./status-fallback-interval.ts";

describe("status-fallback-interval", () => {
  it("keeps direct HTTP fallback polling at 400ms", () => {
    expect(HTTP_FAST_STATUS_REFETCH_MS).toBe(400);
    expect(getFastStatusRefetchIntervalMs("http://192.0.2.15")).toBe(400);
  });

  it("uses the tighter devd fallback cadence for USB compat URLs", () => {
    expect(DEVD_FAST_STATUS_TARGET_PERIOD_MS).toBe(333);
    expect(
      usesManualDevdStatusPolling(
        "http://127.0.0.1:30180/?device_id=loadlynx-abc123&lease_id=lease-1",
      ),
    ).toBe(true);
    expect(
      getFastStatusRefetchIntervalMs(
        "http://127.0.0.1:30180/?device_id=loadlynx-abc123&lease_id=lease-1",
      ),
    ).toBe(false);
  });

  it("defaults to the HTTP cadence when no baseUrl is available", () => {
    expect(getFastStatusRefetchIntervalMs(undefined)).toBe(400);
  });

  it("compensates the next devd poll delay by the elapsed request time", () => {
    expect(getManualStatusPollDelayMs(333, 1_000, 1_100)).toBe(233);
    expect(getManualStatusPollDelayMs(333, 1_000, 1_400)).toBe(0);
  });
});

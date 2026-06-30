import { describe, expect, it } from "vitest";
import {
  getStatusRenderDelay,
  STREAM_UI_INTERVAL_MS,
  shouldCommitStatusImmediately,
} from "./status-stream-gate.ts";

describe("status-stream-gate", () => {
  it("commits immediately when no prior render exists", () => {
    expect(shouldCommitStatusImmediately(null, 100)).toBe(true);
    expect(getStatusRenderDelay(null, 100)).toBe(0);
  });

  it("defers updates until the minimum interval elapses", () => {
    expect(
      shouldCommitStatusImmediately(1_000, 1_000 + STREAM_UI_INTERVAL_MS - 1),
    ).toBe(false);
    expect(getStatusRenderDelay(1_000, 1_000 + STREAM_UI_INTERVAL_MS - 1)).toBe(
      1,
    );
  });

  it("allows immediate commits once the interval has elapsed", () => {
    expect(
      shouldCommitStatusImmediately(1_000, 1_000 + STREAM_UI_INTERVAL_MS),
    ).toBe(true);
    expect(getStatusRenderDelay(1_000, 1_000 + STREAM_UI_INTERVAL_MS)).toBe(0);
  });
});

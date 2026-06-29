import { afterEach, describe, expect, it, vi } from "vitest";
import { subscribeMockStatusStream } from "./mock-status-stream.ts";
import type { FastStatusView } from "./types.ts";

function makeStatus(sequence: number): FastStatusView {
  return {
    raw: {
      uptime_ms: sequence * 500,
      mode: 1,
      state_flags: 0,
      enable: true,
      target_value: 0,
      i_local_ma: 0,
      i_remote_ma: 0,
      v_local_mv: 12_000,
      v_remote_mv: 11_950,
      calc_p_mw: 0,
      dac_headroom_mv: 300,
      loop_error: 0,
      sink_core_temp_mc: 30_000,
      sink_exhaust_temp_mc: 29_000,
      mcu_temp_mc: 31_000,
      fault_flags: 0,
    },
    link_up: true,
    hello_seen: true,
    analog_state: "ready",
    fault_flags_decoded: [],
    state_flags_decoded: [],
  };
}

describe("subscribeMockStatusStream", () => {
  afterEach(() => {
    vi.useRealTimers();
    globalThis.__LOADLYNX_MOCK_STATUS_STREAMS__?.clear();
  });

  it("shares a single polling loop for the same baseUrl", async () => {
    vi.useFakeTimers();
    let sequence = 0;
    const readStatus = vi.fn(async () => makeStatus(++sequence));
    const firstListener = vi.fn();
    const secondListener = vi.fn();

    const disposeFirst = subscribeMockStatusStream({
      baseUrl: "mock://demo-1",
      onMessage: firstListener,
      readStatus,
      intervalMs: 500,
    });
    const disposeSecond = subscribeMockStatusStream({
      baseUrl: "mock://demo-1",
      onMessage: secondListener,
      readStatus,
      intervalMs: 500,
    });

    await vi.advanceTimersByTimeAsync(500);
    expect(readStatus).toHaveBeenCalledTimes(1);
    expect(firstListener).toHaveBeenCalledTimes(1);
    expect(secondListener).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(500);
    expect(readStatus).toHaveBeenCalledTimes(2);
    expect(firstListener).toHaveBeenCalledTimes(2);
    expect(secondListener).toHaveBeenCalledTimes(2);

    disposeFirst();
    disposeSecond();
    await vi.advanceTimersByTimeAsync(1_000);
    expect(readStatus).toHaveBeenCalledTimes(2);
  });

  it("does not overlap reads when a prior tick is still in flight", async () => {
    vi.useFakeTimers();

    let resolveRead: ((value: FastStatusView) => void) | null = null;
    const readStatus = vi.fn(
      () =>
        new Promise<FastStatusView>((resolve) => {
          resolveRead = resolve;
        }),
    );
    const listener = vi.fn();

    const dispose = subscribeMockStatusStream({
      baseUrl: "mock://demo-1",
      onMessage: listener,
      readStatus,
      intervalMs: 500,
    });

    await vi.advanceTimersByTimeAsync(500);
    expect(readStatus).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(2_000);
    expect(readStatus).toHaveBeenCalledTimes(1);

    resolveRead?.(makeStatus(1));
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(500);
    expect(readStatus).toHaveBeenCalledTimes(2);
    expect(listener).toHaveBeenCalledTimes(1);

    dispose();
  });
});

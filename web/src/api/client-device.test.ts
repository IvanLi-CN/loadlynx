import { afterEach, expect, test, vi } from "vitest";

import { __testClearDeviceQueues, getStatus } from "./client.ts";

afterEach(() => {
  vi.restoreAllMocks();
  __testClearDeviceQueues();
});

test("getStatus reads devd compat status through the cache-backed endpoint", async () => {
  let requestedUrl = "";
  vi.spyOn(globalThis, "fetch").mockImplementation(
    async (input: RequestInfo | URL) => {
      requestedUrl = input.toString();
      return new Response(
        JSON.stringify({
          analog_state: "ready",
          link_up: true,
          hello_seen: true,
          status: {
            uptime_ms: 1_234,
            state_flags: 2,
            enable: false,
            i_local_ma: 0,
            i_remote_ma: 0,
            v_local_mv: 61,
            v_remote_mv: 24,
            calc_p_mw: 0,
            fault_flags: 0,
          },
          control: {
            mode: "cc",
            output_enabled: false,
            target_i_ma: 0,
            target_v_mv: 12_000,
            target_p_mw: 0,
            min_v_mv: 0,
          },
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      );
    },
  );

  const status = await getStatus(
    "http://127.0.0.1:24567/api/compat?device_id=ll-001&lease_id=lease-001",
    { cache: true },
  );

  expect(requestedUrl).toContain("/api/v1/status?cache=true");
  expect(requestedUrl).toContain("device_id=ll-001");
  expect(requestedUrl).toContain("lease_id=lease-001");
  expect(status.link_up).toBe(true);
  expect(status.analog_state).toBe("ready");
  expect(status.raw.v_local_mv).toBe(61);
});

import { expect, test } from "@playwright/test";

// Simulate an SSE-capable device using a mocked EventSource and mocked HTTP
// responses. Verifies that the CC page consumes stream updates and avoids
// falling back to polling once the stream is active.
test("status SSE stream drives CC page without extra polling", async ({
  page,
}) => {
  // Device registry seed so the CC route is reachable.
  await page.addInitScript(() => {
    const devices = [
      {
        id: "dev-sse",
        name: "Mock SSE Device",
        baseUrl: "http://fake-sse",
      },
    ];
    localStorage.setItem("loadlynx.devices", JSON.stringify(devices));
  });

  // Mock HTTP + EventSource before app code runs.
  await page.addInitScript(() => {
    // Counters for assertions.
    const globalWithCounters = window as unknown as {
      __statusFetchCount: number;
      __statusMessages: number;
    };
    globalWithCounters.__statusFetchCount = 0;
    globalWithCounters.__statusMessages = 0;

    const mockIdentity = {
      device_id: "dev-sse",
      digital_fw_version: "digital 0.1.0 (mock sse)",
      analog_fw_version: "analog 0.1.0 (mock sse)",
      protocol_version: 1,
      uptime_ms: 123_000,
      network: {
        ip: "192.168.0.123",
        mac: "00:11:22:33:44:55",
        hostname: "mock-sse",
      },
      capabilities: {
        cc_supported: true,
        cv_supported: false,
        cp_supported: false,
        api_version: "1.0.0",
      },
    };

    let streamTick = 0;
    const nextStatus = () => {
      streamTick += 1;
      return {
        raw: {
          uptime_ms: 1000 + streamTick * 200,
          mode: 0,
          state_flags: 0,
          enable: true,
          target_value: 1500,
          i_local_ma: 800 + streamTick,
          i_remote_ma: 200,
          v_local_mv: 12000,
          v_remote_mv: 11950,
          calc_p_mw: 15000,
          dac_headroom_mv: 500,
          loop_error: 0,
          sink_core_temp_mc: 40000,
          sink_exhaust_temp_mc: 38000,
          mcu_temp_mc: 39000,
          fault_flags: 0,
        },
        link_up: true,
        hello_seen: true,
        analog_state: "ready",
        fault_flags_decoded: [],
      };
    };

    const mockCc = {
      enable: true,
      target_i_ma: 1500,
      limit_profile: {
        max_i_ma: 5000,
        max_p_mw: 60000,
        ovp_mv: 40000,
        temp_trip_mc: 80000,
        thermal_derate_pct: 100,
      },
      protection: {
        voltage_mode: "protect",
        power_mode: "protect",
      },
      i_total_ma: 1800,
      v_main_mv: 12000,
      p_main_mw: 216000,
    };

    const origFetch = window.fetch.bind(window);
    window.fetch = (input: RequestInfo | URL, init?: RequestInit) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.includes("fake-sse")) {
        if (url.includes("/api/v1/identity")) {
          return Promise.resolve(
            new Response(JSON.stringify(mockIdentity), {
              status: 200,
              headers: { "Content-Type": "application/json" },
            }),
          );
        }
        if (url.includes("/api/v1/cc")) {
          return Promise.resolve(
            new Response(JSON.stringify(mockCc), {
              status: 200,
              headers: { "Content-Type": "application/json" },
            }),
          );
        }
        if (url.includes("/api/v1/status")) {
          globalWithCounters.__statusFetchCount += 1;
          const payload = nextStatus();
          return Promise.resolve(
            new Response(
              JSON.stringify({
                status: payload.raw,
                link_up: payload.link_up,
                hello_seen: payload.hello_seen,
                analog_state: payload.analog_state,
                fault_flags_decoded: payload.fault_flags_decoded,
              }),
              {
                status: 200,
                headers: { "Content-Type": "application/json" },
              },
            ),
          );
        }
      }
      return origFetch(input, init);
    };

    class MockEventSource {
      url: string;
      readyState: number;
      onmessage: ((ev: MessageEvent) => void) | null = null;
      onerror: ((ev: Event) => void) | null = null;
      private listeners: Record<string, Array<(ev: MessageEvent) => void>> = {};
      private timer: number | null = null;

      constructor(url: string) {
        this.url = url;
        this.readyState = 1; // OPEN
        const tick = () => {
          const payload = nextStatus();
          const ev = { data: JSON.stringify(payload) } as MessageEvent;
          this.emit("message", ev);
          this.emit("status", ev);
          globalWithCounters.__statusMessages += 1;
          this.timer = window.setTimeout(tick, 150);
        };
        tick();
      }

      addEventListener(type: string, handler: (ev: MessageEvent) => void) {
        this.listeners[type] ||= [];
        this.listeners[type].push(handler);
      }

      removeEventListener(type: string, handler: (ev: MessageEvent) => void) {
        this.listeners[type] = (this.listeners[type] || []).filter(
          (h) => h !== handler,
        );
      }

      emit(type: string, ev: MessageEvent) {
        const list = this.listeners[type] || [];
        for (const h of list) {
          h(ev);
        }
        if (type === "message" && this.onmessage) {
          this.onmessage(ev);
        }
        if (type === "error" && this.onerror) {
          this.onerror(ev as unknown as Event);
        }
      }

      close() {
        if (this.timer) {
          clearTimeout(this.timer);
        }
        this.readyState = 2; // CLOSED
      }
    }

    // Replace global EventSource so subscribeStatusStream uses the mock.
    (window as unknown as { EventSource: typeof EventSource }).EventSource =
      MockEventSource as unknown as typeof EventSource;
  });

  await page.goto("/dev-sse/cc");

  await expect(page.getByText(/LINK UP/i)).toBeVisible();
  await expect(page.getByText(/Online/i)).toBeVisible();

  await page.waitForFunction(
    () =>
      (window as unknown as { __statusMessages: number }).__statusMessages >= 3,
    undefined,
    { timeout: 2000 },
  );

  // Ensure polling isn't hammering /status once the stream is active.
  await page.waitForTimeout(600);
  const fetchCount = await page.evaluate(
    () =>
      (window as unknown as { __statusFetchCount: number }).__statusFetchCount,
  );
  expect(fetchCount).toBeLessThanOrEqual(1);
});

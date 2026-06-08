import { expect, test } from "vitest";

import {
  __testParseDiscoveredIdentity,
  __testScanSingleHost,
  __testScanSubnet,
} from "./scan-hooks.ts";

test("parseDiscoveredIdentity accepts a complete LoadLynx identity payload", () => {
  const identity = __testParseDiscoveredIdentity(
    JSON.stringify({
      device_id: "llx-d68638",
      digital_fw_version: "digital 0.1.0",
      analog_fw_version: "analog 0.1.0",
      protocol_version: 1,
      uptime_ms: 1234,
      network: {
        ip: "192.168.1.23",
        mac: "aa:bb:cc:dd:ee:ff",
        hostname: "loadlynx-d68638.local",
      },
      capabilities: {
        api_version: "2.0.0",
      },
    }),
  );

  expect(identity?.device_id).toBe("llx-d68638");
  expect(identity?.network.ip).toBe("192.168.1.23");
});

test("parseDiscoveredIdentity rejects partial objects that happen to be valid JSON", () => {
  expect(
    __testParseDiscoveredIdentity(
      JSON.stringify({
        device_id: "llx-bad",
        capabilities: { api_version: "2.0.0" },
      }),
    ),
  ).toBeNull();
});

test("parseDiscoveredIdentity rejects non-object JSON payloads", () => {
  expect(__testParseDiscoveredIdentity('"not-an-object"')).toBeNull();
  expect(__testParseDiscoveredIdentity("123")).toBeNull();
});

test("scanSingleHost stops immediately when parent signal is already aborted", async () => {
  const controller = new AbortController();
  controller.abort();

  const originalFetch = globalThis.fetch;
  let fetchCalled = false;
  globalThis.fetch = async () => {
    fetchCalled = true;
    return new Response("{}");
  };

  try {
    const result = await __testScanSingleHost(
      "192.168.1.23",
      400,
      controller.signal,
    );
    expect(result).toBeNull();
    expect(fetchCalled).toBe(false);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("scanSubnet stops dispatching hosts once the shared signal aborts", async () => {
  const controller = new AbortController();
  const originalFetch = globalThis.fetch;
  const seenIps: string[] = [];

  globalThis.fetch = async (input) => {
    const url = String(input);
    seenIps.push(url);
    controller.abort();
    return new Response(
      JSON.stringify({
        device_id: "llx-d68638",
        digital_fw_version: "digital 0.1.0",
        analog_fw_version: "analog 0.1.0",
        protocol_version: 1,
        uptime_ms: 1234,
        network: {
          ip: "192.168.1.23",
          mac: "aa:bb:cc:dd:ee:ff",
          hostname: "loadlynx-d68638.local",
        },
        capabilities: {
          api_version: "2.0.0",
        },
      }),
    );
  };

  try {
    await __testScanSubnet(
      {
        seedIp: "192.168.1.23",
        maxConcurrency: 1,
        perHostTimeoutMs: 400,
        signal: controller.signal,
      },
      undefined,
    );

    expect(seenIps.length).toBe(1);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

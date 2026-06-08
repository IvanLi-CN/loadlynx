import { expect, test } from "vitest";

import {
  __testDevdJson,
  __testMockDevd,
  buildDevdCompatBaseUrl,
  DevdApiError,
} from "./client.ts";

test("mock devd rejects unsupported routes instead of returning fake success", () => {
  expect(() => __testMockDevd("/api/v1/unknown")).toThrow(DevdApiError);
  expect(() => __testMockDevd("/api/v1/unknown")).toThrow(
    "Unsupported mock devd route: /api/v1/unknown",
  );
});

test("mock devd keeps supported scan route available", () => {
  const response = __testMockDevd<{ devices: Array<{ id: string }> }>(
    "/api/v1/devices/scan",
  );

  expect(response.devices.length).toBeGreaterThan(0);
  expect(response.devices[0]?.id).toBe("mock-loadlynx-devd");
});

test("mock devd maps invalid request bodies into DevdApiError", () => {
  expect(() =>
    __testMockDevd("/api/v1/serial/lease", {
      method: "POST",
      body: "{",
    }),
  ).toThrow(DevdApiError);
  expect(() =>
    __testMockDevd("/api/v1/serial/lease", {
      method: "POST",
      body: "{",
    }),
  ).toThrow("Invalid mock JSON body for /api/v1/serial/lease");
});

test("mock devd rejects non-object JSON bodies", () => {
  expect(() =>
    __testMockDevd("/api/v1/serial/lease", {
      method: "POST",
      body: "123",
    }),
  ).toThrow(DevdApiError);
  expect(() =>
    __testMockDevd("/api/v1/serial/lease", {
      method: "POST",
      body: "123",
    }),
  ).toThrow(
    "Mock devd request body for /api/v1/serial/lease must be a JSON object",
  );
});

test("buildDevdCompatBaseUrl maps invalid base URLs into DevdApiError", () => {
  expect(() =>
    buildDevdCompatBaseUrl({
      baseUrl: "::not-a-url::",
      deviceId: "digital-aabbcc",
    }),
  ).toThrow(DevdApiError);
  expect(() =>
    buildDevdCompatBaseUrl({
      baseUrl: "::not-a-url::",
      deviceId: "digital-aabbcc",
    }),
  ).toThrow("Invalid devd base URL: ::not-a-url::");
});

test("devd client maps invalid JSON into DevdApiError", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () =>
    new Response("not-json", {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });

  try {
    await expect(
      __testDevdJson("http://127.0.0.1:30180", "/api/v1/devices/scan"),
    ).rejects.toMatchObject({
      name: "DevdApiError",
      status: 200,
      code: "INVALID_JSON",
      message: "Invalid JSON from /api/v1/devices/scan",
    });
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("devd client maps fetch failures into NETWORK_ERROR", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () => {
    throw new TypeError("fetch failed");
  };

  try {
    await expect(
      __testDevdJson("http://127.0.0.1:30180", "/api/v1/devices/scan"),
    ).rejects.toMatchObject({
      name: "DevdApiError",
      status: 0,
      code: "NETWORK_ERROR",
      message: "fetch failed",
    });
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("devd client maps error envelopes into DevdApiError", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () =>
    new Response(
      JSON.stringify({
        error: {
          code: "UNSUPPORTED_OPERATION",
          message: "devd route disabled",
        },
      }),
      {
        status: 404,
        headers: { "Content-Type": "application/json" },
      },
    );

  try {
    await expect(
      __testDevdJson("http://127.0.0.1:30180", "/api/v1/devices/scan"),
    ).rejects.toMatchObject({
      name: "DevdApiError",
      status: 404,
      code: "UNSUPPORTED_OPERATION",
      message: "devd route disabled",
    });
  } finally {
    globalThis.fetch = originalFetch;
  }
});

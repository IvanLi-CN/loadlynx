import { afterEach, expect, test, vi } from "vitest";

import {
  __testClearDeviceQueues,
  __testHttpJsonQueued,
  HttpApiError,
  isDevdCompatBaseUrl,
  supportsBackupWifiCredentials,
} from "./client-core.ts";

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  __testClearDeviceQueues();
});

test("detects devd compatibility base URLs", () => {
  const baseUrl =
    "http://127.0.0.1:24567/api/compat?device_id=ll-001&lease_id=lease-001";

  expect(isDevdCompatBaseUrl(baseUrl)).toBe(true);
  expect(isDevdCompatBaseUrl("http://192.0.2.55")).toBe(false);
  expect(isDevdCompatBaseUrl("mock://demo-1")).toBe(false);
});

test("devd and direct HTTP URLs support WiFi credential backup reads", () => {
  expect(supportsBackupWifiCredentials("http://192.0.2.55")).toBe(true);
  expect(
    supportsBackupWifiCredentials(
      "http://127.0.0.1:24567/api/compat?device_id=ll-001&lease_id=lease-001",
    ),
  ).toBe(true);
  expect(supportsBackupWifiCredentials("")).toBe(false);
});

test("device HTTP requests time out instead of staying pending forever", async () => {
  vi.useFakeTimers();
  vi.spyOn(globalThis, "fetch").mockImplementation(
    (_input: RequestInfo | URL, init?: RequestInit) =>
      new Promise<Response>((_resolve, reject) => {
        init?.signal?.addEventListener("abort", () => {
          reject(new DOMException("aborted", "AbortError"));
        });
      }),
  );

  const request = __testHttpJsonQueued(
    "http://device.local",
    "/api/v1/identity",
  ).catch((error: unknown) => error);

  await vi.advanceTimersByTimeAsync(15_000);

  const error = await request;
  expect(error).toBeInstanceOf(HttpApiError);
  expect(error).toMatchObject({
    code: "REQUEST_TIMEOUT",
    retryable: true,
  });
});

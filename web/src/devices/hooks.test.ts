import { expect, test } from "vitest";

import { HttpApiError } from "../api/client.ts";
import {
  getDeviceControlQueryOptions,
  getDeviceIdentityQueryOptions,
  getDevicePdQueryOptions,
  getDevicePresetsQueryOptions,
  getDeviceStatusQueryOptions,
  getDeviceWifiQueryOptions,
} from "./hooks.ts";

test("getDeviceIdentityQueryOptions disables retries for missing base URLs", async () => {
  const options = getDeviceIdentityQueryOptions("device-001", undefined);

  await expect(options.queryFn()).rejects.toMatchObject({
    code: "NO_BASE_URL",
  });
  expect(options.enabled).toBe(false);
  expect(
    options.retry(
      0,
      new HttpApiError({
        status: 0,
        code: "NO_BASE_URL",
        message: "Device base URL is not available",
        retryable: false,
      }),
    ),
  ).toBe(false);
});

test("getDeviceIdentityQueryOptions retries real devices twice", () => {
  const options = getDeviceIdentityQueryOptions(
    "device-001",
    "http://device.local",
  );

  expect(
    options.retry(
      0,
      new HttpApiError({
        status: 0,
        code: "NETWORK_ERROR",
        message: "network",
        retryable: true,
      }),
    ),
  ).toBe(true);
  expect(
    options.retry(
      2,
      new HttpApiError({
        status: 0,
        code: "NETWORK_ERROR",
        message: "network",
        retryable: true,
      }),
    ),
  ).toBe(false);
});

test("getDeviceIdentityQueryOptions only retries mock devices once", () => {
  const options = getDeviceIdentityQueryOptions("mock-001", "mock://demo-1");

  expect(
    options.retry(
      0,
      new HttpApiError({
        status: 0,
        code: "NETWORK_ERROR",
        message: "network",
        retryable: true,
      }),
    ),
  ).toBe(true);
  expect(
    options.retry(
      1,
      new HttpApiError({
        status: 0,
        code: "NETWORK_ERROR",
        message: "network",
        retryable: true,
      }),
    ),
  ).toBe(false);
});

test("getDevicePdQueryOptions builds the PD key and respects enabled/refetch settings", () => {
  const options = getDevicePdQueryOptions({
    deviceId: "device-001",
    baseUrl: "http://device.local",
    enabled: true,
    refetchInterval: 1500,
    retryDelay: 500,
  });

  expect(options.queryKey).toEqual([
    "device",
    "device-001",
    "http://device.local",
    "pd",
  ]);
  expect(options.enabled).toBe(true);
  expect(options.refetchInterval).toBe(1500);
  expect(options.refetchIntervalInBackground).toBe(false);
});

test("getDevicePdQueryOptions supports custom parts and retry overrides", () => {
  const options = getDevicePdQueryOptions({
    deviceId: "device-001",
    baseUrl: "http://device.local",
    enabled: true,
    refetchInterval: false,
    parts: ["backup-pd"],
    retry: false,
    retryDelay: 500,
  });

  expect(options.queryKey).toEqual([
    "device",
    "device-001",
    "http://device.local",
    "backup-pd",
  ]);
  expect(options.retry).toBe(false);
});

test("getDeviceStatusQueryOptions keeps shared retry policy and status key", () => {
  const options = getDeviceStatusQueryOptions({
    deviceId: "device-001",
    baseUrl: "http://device.local",
    enabled: true,
    refetchInterval: 1000,
  });

  expect(options.queryKey).toEqual([
    "device",
    "device-001",
    "http://device.local",
    "status",
  ]);
  expect(options.retry).toBe(2);
  expect(options.refetchIntervalInBackground).toBe(false);
});

test("getDeviceStatusQueryOptions supports fallback-specific overrides", () => {
  const options = getDeviceStatusQueryOptions({
    deviceId: "device-001",
    baseUrl: "http://device.local",
    enabled: true,
    parts: ["status", "calibration-fallback"],
    refetchInterval: 500,
    refetchOnWindowFocus: false,
    retry: 2,
    retryDelay: 250,
  });

  expect(options.queryKey).toEqual([
    "device",
    "device-001",
    "http://device.local",
    "status",
    "calibration-fallback",
  ]);
  expect(options.refetchOnWindowFocus).toBe(false);
  expect(options.retryDelay).toBe(250);
});

test("getDeviceControlQueryOptions reuses control key and retry delay", () => {
  const options = getDeviceControlQueryOptions({
    deviceId: "device-001",
    baseUrl: "http://device.local",
    enabled: true,
    retryDelay: 500,
  });

  expect(options.queryKey).toEqual([
    "device",
    "device-001",
    "http://device.local",
    "control",
  ]);
  expect(options.enabled).toBe(true);
  expect(options.retryDelay).toBe(500);
});

test("getDevicePresetsQueryOptions reuses presets key and retry delay", () => {
  const options = getDevicePresetsQueryOptions({
    deviceId: "device-001",
    baseUrl: "http://device.local",
    enabled: true,
    retryDelay: 500,
  });

  expect(options.queryKey).toEqual([
    "device",
    "device-001",
    "http://device.local",
    "presets",
  ]);
  expect(options.enabled).toBe(true);
  expect(options.retryDelay).toBe(500);
});

test("getDeviceWifiQueryOptions reuses wifi key", () => {
  const options = getDeviceWifiQueryOptions({
    deviceId: "device-001",
    baseUrl: "http://device.local",
    enabled: true,
  });

  expect(options.queryKey).toEqual([
    "device",
    "device-001",
    "http://device.local",
    "wifi",
  ]);
  expect(options.enabled).toBe(true);
});

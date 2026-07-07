import { expect, test } from "vitest";

import { HttpApiError } from "../api/client.ts";
import {
  getDeviceControlQueryOptions,
  getDeviceIdentityQueryOptions,
  getDevicePdQueryOptions,
  getDevicePresetsQueryOptions,
  getDeviceQueryRetry,
  getDeviceStatusQueryOptions,
  getDeviceWifiQueryOptions,
  upsertRealDevice,
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

test("getDeviceQueryRetry stops retrying saturated USB devd serial paths", () => {
  const retry = getDeviceQueryRetry(
    "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
  );

  expect(
    retry(
      0,
      new HttpApiError({
        status: 409,
        code: "device_busy",
        message: "USB serial operation queue is full",
        retryable: false,
      }),
    ),
  ).toBe(false);
  expect(
    retry(
      0,
      new HttpApiError({
        status: 0,
        code: "NETWORK_ERROR",
        message: "network",
        retryable: true,
      }),
    ),
  ).toBe(true);
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
  expect(typeof options.retry).toBe("function");
  expect(
    (options.retry as (failureCount: number, error: Error) => boolean)(
      2,
      new HttpApiError({
        status: 0,
        code: "NETWORK_ERROR",
        message: "network",
        retryable: true,
      }),
    ),
  ).toBe(false);
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

test("getDeviceWifiQueryOptions reuses wifi key and supports live refresh controls", () => {
  const options = getDeviceWifiQueryOptions({
    deviceId: "device-001",
    baseUrl: "http://device.local",
    enabled: true,
    refetchInterval: 2000,
    refetchOnWindowFocus: true,
    retry: 2,
  });

  expect(options.queryKey).toEqual([
    "device",
    "device-001",
    "http://device.local",
    "wifi",
  ]);
  expect(options.enabled).toBe(true);
  expect(options.refetchInterval).toBe(2000);
  expect(options.refetchIntervalInBackground).toBe(false);
  expect(options.refetchOnWindowFocus).toBe(true);
  expect(options.retry).toBe(2);
});

test("upsertRealDevice merges USB and LAN entries with the same hardware identity", () => {
  const next = upsertRealDevice(
    [
      {
        id: "device-001",
        name: "ESP32-S3 USB CDC",
        baseUrl:
          "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
        identityDeviceId: "loadlynx-d68638",
        connectionMarks: ["usb"],
        devd: {
          baseUrl: "http://127.0.0.1:19390",
          deviceId: "digital-2bdf",
          leaseId: "lease-1",
        },
      },
    ],
    {
      name: "LoadLynx d68638 WiFi",
      baseUrl: "http://192.168.31.216",
      identityDeviceId: "loadlynx-d68638",
      connectionMarks: ["lan"],
    },
  );

  expect(next).toHaveLength(1);
  expect(next[0]).toEqual({
    id: "device-001",
    name: "LoadLynx d68638 WiFi",
    baseUrl: "http://192.168.31.216",
    identityDeviceId: "loadlynx-d68638",
    connectionMarks: ["lan", "usb"],
    lan: {
      baseUrl: "http://192.168.31.216",
    },
    devd: {
      baseUrl: "http://127.0.0.1:19390",
      deviceId: "digital-2bdf",
      leaseId: "lease-1",
    },
  });
});

test("upsertRealDevice auto-binds LAN from connected WiFi status discovered through USB", () => {
  const next = upsertRealDevice(
    [
      {
        id: "device-001",
        name: "ESP32-S3 USB CDC",
        baseUrl:
          "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
        identityDeviceId: "loadlynx-d68638",
        connectionMarks: ["usb"],
        devd: {
          baseUrl: "http://127.0.0.1:19390",
          deviceId: "digital-2bdf",
          leaseId: "lease-1",
        },
      },
    ],
    {
      name: "ESP32-S3 USB CDC",
      baseUrl:
        "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
      identityDeviceId: "loadlynx-d68638",
      connectionMarks: ["usb"],
      devd: {
        baseUrl: "http://127.0.0.1:19390",
        deviceId: "digital-2bdf",
        leaseId: "lease-1",
      },
      wifi: {
        ssid: "BenchNet",
        source: "user",
        state: "connected",
        ip: "192.168.31.216",
        last_error: null,
      },
    },
  );

  expect(next).toHaveLength(1);
  expect(next[0]).toMatchObject({
    id: "device-001",
    identityDeviceId: "loadlynx-d68638",
    connectionMarks: ["lan", "usb"],
    lan: {
      baseUrl: "http://192.168.31.216",
    },
  });
});

test("upsertRealDevice prefers mDNS hostname over IP when both are available", () => {
  const next = upsertRealDevice([], {
    name: "LoadLynx d68638",
    baseUrl: "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
    identityDeviceId: "loadlynx-d68638",
    connectionMarks: ["usb"],
    identity: {
      device_id: "loadlynx-d68638",
      digital_fw_version: "digital 0.1.0",
      analog_fw_version: "analog 0.1.0",
      protocol_version: 1,
      uptime_ms: 1,
      network: {
        ip: "192.168.31.216",
        mac: "00:11:22:33:44:55",
        hostname: "loadlynx-d68638.local",
      },
      hostname: "loadlynx-d68638.local",
      capabilities: {
        api_version: "2.0.0",
        cc_supported: true,
        cv_supported: true,
        cp_supported: true,
      },
    },
  });

  expect(next[0]).toMatchObject({
    connectionMarks: ["lan", "usb"],
    lan: {
      baseUrl: "http://loadlynx-d68638.local",
    },
  });
});

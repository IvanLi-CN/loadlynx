import { expect, test } from "vitest";

import {
  DEVICE_QUERY_PARTS,
  makeDeviceQueryKey,
  UNBOUND_DEVICE_BASE_URL,
} from "./device-query-key.ts";

test("makeDeviceQueryKey includes baseUrl in the query identity", () => {
  expect(
    makeDeviceQueryKey(
      "device-001",
      "http://127.0.0.1:30180?lease_id=a",
      ...DEVICE_QUERY_PARTS.pd,
    ),
  ).toEqual([
    "device",
    "device-001",
    "http://127.0.0.1:30180?lease_id=a",
    "pd",
  ]);

  expect(
    makeDeviceQueryKey(
      "device-001",
      "http://127.0.0.1:30180?lease_id=b",
      ...DEVICE_QUERY_PARTS.pd,
    ),
  ).not.toEqual(
    makeDeviceQueryKey(
      "device-001",
      "http://127.0.0.1:30180?lease_id=a",
      ...DEVICE_QUERY_PARTS.pd,
    ),
  );
});

test("makeDeviceQueryKey falls back to the unbound baseUrl marker", () => {
  expect(
    makeDeviceQueryKey("device-001", null, ...DEVICE_QUERY_PARTS.identity),
  ).toEqual(["device", "device-001", UNBOUND_DEVICE_BASE_URL, "identity"]);
});

test("makeDeviceQueryKey supports nested calibration subpaths", () => {
  expect(
    makeDeviceQueryKey(
      "device-001",
      "http://127.0.0.1:30180?lease_id=a",
      ...DEVICE_QUERY_PARTS.calibrationStatusFallback,
    ),
  ).toEqual([
    "device",
    "device-001",
    "http://127.0.0.1:30180?lease_id=a",
    "status",
    "calibration-fallback",
  ]);
});

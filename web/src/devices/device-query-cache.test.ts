import { QueryClient } from "@tanstack/react-query";
import { expect, test } from "vitest";

import {
  invalidateDeviceQuery,
  setDeviceQueryData,
} from "./device-query-cache.ts";
import { DEVICE_QUERY_PARTS, makeDeviceQueryKey } from "./device-query-key.ts";

test("setDeviceQueryData updates only the targeted device/baseUrl cache entry", () => {
  const queryClient = new QueryClient();
  const leaseA = "http://127.0.0.1:30180?lease_id=a";
  const leaseB = "http://127.0.0.1:30180?lease_id=b";

  setDeviceQueryData(queryClient, "device-001", leaseA, DEVICE_QUERY_PARTS.pd, {
    attached: true,
  });
  setDeviceQueryData(queryClient, "device-001", leaseB, DEVICE_QUERY_PARTS.pd, {
    attached: false,
  });

  expect(
    queryClient.getQueryData(
      makeDeviceQueryKey("device-001", leaseA, ...DEVICE_QUERY_PARTS.pd),
    ),
  ).toEqual({ attached: true });
  expect(
    queryClient.getQueryData(
      makeDeviceQueryKey("device-001", leaseB, ...DEVICE_QUERY_PARTS.pd),
    ),
  ).toEqual({ attached: false });
});

test("invalidateDeviceQuery invalidates only the targeted device/baseUrl cache entry", async () => {
  const queryClient = new QueryClient();
  const leaseA = "http://127.0.0.1:30180?lease_id=a";
  const leaseB = "http://127.0.0.1:30180?lease_id=b";
  const keyA = makeDeviceQueryKey(
    "device-001",
    leaseA,
    ...DEVICE_QUERY_PARTS.status,
  );
  const keyB = makeDeviceQueryKey(
    "device-001",
    leaseB,
    ...DEVICE_QUERY_PARTS.status,
  );

  queryClient.setQueryData(keyA, { ok: true });
  queryClient.setQueryData(keyB, { ok: true });

  await invalidateDeviceQuery(
    queryClient,
    "device-001",
    leaseA,
    ...DEVICE_QUERY_PARTS.status,
  );

  expect(queryClient.getQueryState(keyA)?.isInvalidated).toBe(true);
  expect(queryClient.getQueryState(keyB)?.isInvalidated).toBe(false);
});

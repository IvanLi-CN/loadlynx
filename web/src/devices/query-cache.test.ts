import { QueryClient } from "@tanstack/react-query";
import { expect, test } from "vitest";

import type { StoredDevice } from "./device-store.ts";
import {
  invalidateDevicesQueryCache,
  syncDevicesQueryCache,
} from "./query-cache.ts";

test("syncDevicesQueryCache updates all devices query variants", () => {
  const queryClient = new QueryClient();
  const nextDevices: StoredDevice[] = [
    {
      id: "device-001",
      name: "LoadLynx A",
      baseUrl: "http://loadlynx-a.local",
    },
  ];

  queryClient.setQueryData(["devices", "real"], []);
  queryClient.setQueryData(["devices", "demo"], []);
  queryClient.setQueryData(["unrelated"], ["leave-me-alone"]);

  syncDevicesQueryCache(queryClient, nextDevices);

  expect(queryClient.getQueryData(["devices", "real"])).toEqual(nextDevices);
  expect(queryClient.getQueryData(["devices", "demo"])).toEqual(nextDevices);
  expect(queryClient.getQueryData(["unrelated"])).toEqual(["leave-me-alone"]);
});

test("syncDevicesQueryCache can seed the primary devices query when missing", () => {
  const queryClient = new QueryClient();
  const nextDevices: StoredDevice[] = [
    {
      id: "device-002",
      name: "LoadLynx B",
      baseUrl: "http://loadlynx-b.local",
    },
  ];

  syncDevicesQueryCache(queryClient, nextDevices, ["devices", "real"]);

  expect(queryClient.getQueryData(["devices", "real"])).toEqual(nextDevices);
});

test("invalidateDevicesQueryCache invalidates only devices queries", async () => {
  const queryClient = new QueryClient();

  queryClient.setQueryData(["devices", "real"], []);
  queryClient.setQueryData(["devices", "demo"], []);
  queryClient.setQueryData(["device", "device-001", "http://x", "status"], {
    ok: true,
  });

  await invalidateDevicesQueryCache(queryClient);

  expect(queryClient.getQueryState(["devices", "real"])?.isInvalidated).toBe(
    true,
  );
  expect(queryClient.getQueryState(["devices", "demo"])?.isInvalidated).toBe(
    true,
  );
  expect(
    queryClient.getQueryState(["device", "device-001", "http://x", "status"])
      ?.isInvalidated,
  ).toBe(false);
});

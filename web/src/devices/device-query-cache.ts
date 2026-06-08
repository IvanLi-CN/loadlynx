import type { QueryClient } from "@tanstack/react-query";

import type { DeviceQueryParts } from "./device-query-key.ts";
import { makeDeviceQueryKey } from "./device-query-key.ts";

type DeviceQueryUpdater<T> = T | ((prev: T | undefined) => T | undefined);

export function setDeviceQueryData<T>(
  queryClient: QueryClient,
  deviceId: string | null | undefined,
  baseUrl: string | null | undefined,
  parts: DeviceQueryParts,
  value: DeviceQueryUpdater<T>,
): void {
  queryClient.setQueryData<T>(
    makeDeviceQueryKey(deviceId, baseUrl, ...parts),
    value as DeviceQueryUpdater<T>,
  );
}

export function invalidateDeviceQuery(
  queryClient: QueryClient,
  deviceId: string | null | undefined,
  baseUrl: string | null | undefined,
  ...parts: DeviceQueryParts
) {
  return queryClient.invalidateQueries({
    queryKey: makeDeviceQueryKey(deviceId, baseUrl, ...parts),
  });
}

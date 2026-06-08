import type { QueryClient } from "@tanstack/react-query";

import type { StoredDevice } from "./device-store.ts";

export type DevicesQueryKey = readonly ["devices", "demo" | "real"];

export function syncDevicesQueryCache(
  queryClient: QueryClient,
  devices: StoredDevice[],
  primaryKey?: DevicesQueryKey,
): void {
  if (primaryKey) {
    queryClient.setQueryData<StoredDevice[]>(primaryKey, devices);
  }
  queryClient.setQueriesData<StoredDevice[]>(
    { queryKey: ["devices"] },
    devices,
  );
}

export function invalidateDevicesQueryCache(queryClient: QueryClient) {
  return queryClient.invalidateQueries({
    queryKey: ["devices"],
  });
}

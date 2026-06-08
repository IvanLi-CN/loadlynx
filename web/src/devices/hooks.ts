import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ENABLE_MOCK,
  getControl,
  getIdentity,
  getPd,
  getPresets,
  getStatus,
  getWifiStatus,
  HttpApiError,
  isMockBaseUrl,
} from "../api/client.ts";
import type { Identity } from "../api/types.ts";
import { resolveDemoMode } from "../lib/demo-mode.ts";
import {
  DEVICE_QUERY_PARTS,
  type DeviceQueryParts,
  makeDeviceQueryKey,
} from "./device-query-key.ts";
import type { StoredDevice } from "./device-store.ts";
import { syncDevicesQueryCache } from "./query-cache.ts";
import { useDeviceStore } from "./store-context.tsx";

function getActiveDemoMode(): boolean {
  return typeof window !== "undefined"
    ? resolveDemoMode(window.location, window.localStorage)
    : false;
}

function getDevicesQueryKey(isDemoMode: boolean) {
  return ["devices", isDemoMode ? "demo" : "real"] as const;
}

const deviceIdentityRetryDelay = () => 200 + Math.random() * 300;
const deviceStatusRetryDelay = () => 200 + Math.random() * 300;

export function getDeviceIdentityQueryOptions(
  deviceId: string | undefined,
  baseUrl: string | undefined,
) {
  const queryKey = makeDeviceQueryKey(
    deviceId,
    baseUrl,
    ...DEVICE_QUERY_PARTS.identity,
  );

  return {
    queryKey,
    enabled: Boolean(baseUrl),
    queryFn: async () => {
      if (!baseUrl) {
        throw new HttpApiError({
          status: 0,
          code: "NO_BASE_URL",
          message: "Device base URL is not available",
          retryable: false,
        });
      }
      return getIdentity(baseUrl);
    },
    retry: (failureCount: number, error: HttpApiError) => {
      if (error instanceof HttpApiError && error.code === "NO_BASE_URL") {
        return false;
      }
      const isRealDevice =
        Boolean(baseUrl) && baseUrl !== undefined && !isMockBaseUrl(baseUrl);
      return isRealDevice ? failureCount < 2 : failureCount < 1;
    },
    retryDelay: deviceIdentityRetryDelay,
  } as const;
}

export function useDevicesQuery() {
  const store = useDeviceStore();
  const isDemoMode = getActiveDemoMode();
  return useQuery({
    queryKey: getDevicesQueryKey(isDemoMode),
    queryFn: async () => {
      return store.getDevices();
    },
  });
}

export function useDeviceIdentityByBaseUrl(
  deviceId: string | undefined,
  baseUrl: string | undefined,
) {
  return useQuery<Identity, HttpApiError>(
    getDeviceIdentityQueryOptions(deviceId, baseUrl),
  );
}

export function useDeviceIdentity(device: StoredDevice | null | undefined) {
  return useDeviceIdentityByBaseUrl(device?.id, device?.baseUrl);
}

export function getDevicePdQueryOptions(input: {
  deviceId: string | undefined;
  baseUrl: string | undefined;
  enabled: boolean;
  refetchInterval: number | false;
  parts?: DeviceQueryParts;
  retry?: boolean | number;
  retryDelay: number | ((attemptIndex: number) => number);
}) {
  const {
    deviceId,
    baseUrl,
    enabled,
    refetchInterval,
    parts = DEVICE_QUERY_PARTS.pd,
    retry,
    retryDelay,
  } = input;
  return {
    queryKey: makeDeviceQueryKey(deviceId, baseUrl, ...parts),
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getPd(baseUrl);
    },
    enabled,
    refetchInterval,
    refetchIntervalInBackground: false,
    ...(retry === undefined ? {} : { retry }),
    retryDelay,
  } as const;
}

export function getDeviceStatusQueryOptions(input: {
  deviceId: string | undefined;
  baseUrl: string | undefined;
  enabled: boolean;
  parts?: DeviceQueryParts;
  refetchInterval: number | false;
  refetchOnWindowFocus?: boolean;
  retry?: boolean | number;
  retryDelay?: number | ((attemptIndex: number) => number);
}) {
  const {
    deviceId,
    baseUrl,
    enabled,
    parts = DEVICE_QUERY_PARTS.status,
    refetchInterval,
    refetchOnWindowFocus,
    retry = 2,
    retryDelay = deviceStatusRetryDelay,
  } = input;
  return {
    queryKey: makeDeviceQueryKey(deviceId, baseUrl, ...parts),
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getStatus(baseUrl);
    },
    enabled,
    refetchInterval,
    refetchIntervalInBackground: false,
    ...(refetchOnWindowFocus === undefined ? {} : { refetchOnWindowFocus }),
    retry,
    retryDelay,
  } as const;
}

export function getDeviceControlQueryOptions(input: {
  deviceId: string | undefined;
  baseUrl: string | undefined;
  enabled: boolean;
  retryDelay: number | ((attemptIndex: number) => number);
}) {
  const { deviceId, baseUrl, enabled, retryDelay } = input;
  return {
    queryKey: makeDeviceQueryKey(
      deviceId,
      baseUrl,
      ...DEVICE_QUERY_PARTS.control,
    ),
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getControl(baseUrl);
    },
    enabled,
    retryDelay,
  } as const;
}

export function getDevicePresetsQueryOptions(input: {
  deviceId: string | undefined;
  baseUrl: string | undefined;
  enabled: boolean;
  retryDelay: number | ((attemptIndex: number) => number);
}) {
  const { deviceId, baseUrl, enabled, retryDelay } = input;
  return {
    queryKey: makeDeviceQueryKey(
      deviceId,
      baseUrl,
      ...DEVICE_QUERY_PARTS.presets,
    ),
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getPresets(baseUrl);
    },
    enabled,
    retryDelay,
  } as const;
}

export function getDeviceWifiQueryOptions(input: {
  deviceId: string | undefined;
  baseUrl: string | undefined;
  enabled: boolean;
}) {
  const { deviceId, baseUrl, enabled } = input;
  return {
    queryKey: makeDeviceQueryKey(deviceId, baseUrl, ...DEVICE_QUERY_PARTS.wifi),
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getWifiStatus(baseUrl);
    },
    enabled,
  } as const;
}

export function useAddDeviceMutation() {
  // Adds a demo device backed by the in-memory mock backend. This is only
  // available when ENABLE_MOCK is true.
  const store = useDeviceStore();
  const queryClient = useQueryClient();
  const isDemoMode = getActiveDemoMode();
  const queryKey = getDevicesQueryKey(isDemoMode);

  return useMutation({
    mutationFn: async () => {
      if (!ENABLE_MOCK) {
        throw new Error("Mock backend is disabled");
      }
      const current = store.getDevices();
      const demoCount = current.filter((device) =>
        device.baseUrl.startsWith("mock://"),
      ).length;
      const index = demoCount + 1;
      const nextDevice: StoredDevice = {
        id: `mock-${String(index).padStart(3, "0")}`,
        name: `Demo Device #${index}`,
        baseUrl: `mock://demo-${index}`,
      };
      const next = [...current, nextDevice];
      store.setDevices(next);
      return next;
    },
    onSuccess: (next) => {
      syncDevicesQueryCache(queryClient, next, queryKey);
    },
  });
}

export interface AddRealDeviceInput {
  name: string;
  baseUrl: string;
  connectionMarks?: StoredDevice["connectionMarks"];
  devd?: StoredDevice["devd"];
}

export function useAddRealDeviceMutation() {
  const store = useDeviceStore();
  const queryClient = useQueryClient();
  const isDemoMode = getActiveDemoMode();
  const queryKey = getDevicesQueryKey(isDemoMode);

  return useMutation({
    mutationFn: async (input: AddRealDeviceInput) => {
      const current = store.getDevices();
      const index = current.length + 1;
      const nextDevice: StoredDevice = {
        id: `device-${String(index).padStart(3, "0")}`,
        name: input.name,
        baseUrl: input.baseUrl,
        connectionMarks: input.connectionMarks,
        devd: input.devd,
      };
      const next = [...current, nextDevice];
      store.setDevices(next);
      return next;
    },
    onSuccess: (next) => {
      syncDevicesQueryCache(queryClient, next, queryKey);
    },
  });
}

export * from "./scan-hooks.ts";

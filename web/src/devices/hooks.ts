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
  isDevdCompatBaseUrl,
  isMockBaseUrl,
} from "../api/client.ts";
import type { Identity, WifiStatus } from "../api/types.ts";
import { resolveDemoMode } from "../lib/demo-mode.ts";
import { isUsbSerialUnavailableError } from "../lib/http-error.ts";
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

type DeviceQueryRetry =
  | boolean
  | number
  | ((failureCount: number, error: Error) => boolean);

export function getDeviceQueryRetry(
  baseUrl: string | undefined,
  maxRealDeviceRetries = 2,
  maxMockDeviceRetries = 1,
): (failureCount: number, error: Error) => boolean {
  return (failureCount, error) => {
    if (error instanceof HttpApiError) {
      if (error.code === "NO_BASE_URL") {
        return false;
      }
      if (
        baseUrl &&
        isDevdCompatBaseUrl(baseUrl) &&
        isUsbSerialUnavailableError(error)
      ) {
        return false;
      }
    }
    const isRealDevice =
      Boolean(baseUrl) && baseUrl !== undefined && !isMockBaseUrl(baseUrl);
    return (
      failureCount <
      (isRealDevice ? maxRealDeviceRetries : maxMockDeviceRetries)
    );
  };
}

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
    retry: getDeviceQueryRetry(baseUrl),
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
  readCache?: boolean;
  refetchInterval: number | false;
  refetchOnWindowFocus?: boolean;
  retry?: DeviceQueryRetry;
  retryDelay?: number | ((attemptIndex: number) => number);
}) {
  const {
    deviceId,
    baseUrl,
    enabled,
    parts = DEVICE_QUERY_PARTS.status,
    readCache = false,
    refetchInterval,
    refetchOnWindowFocus,
    retry = getDeviceQueryRetry(baseUrl),
    retryDelay = deviceStatusRetryDelay,
  } = input;
  return {
    queryKey: makeDeviceQueryKey(deviceId, baseUrl, ...parts),
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getStatus(baseUrl, { cache: readCache });
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
  refetchInterval?: number | false;
  refetchOnWindowFocus?: boolean;
  retry?: DeviceQueryRetry;
  retryDelay?: number | ((attemptIndex: number) => number);
}) {
  const {
    deviceId,
    baseUrl,
    enabled,
    refetchInterval,
    refetchOnWindowFocus,
    retry,
    retryDelay,
  } = input;
  return {
    queryKey: makeDeviceQueryKey(deviceId, baseUrl, ...DEVICE_QUERY_PARTS.wifi),
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getWifiStatus(baseUrl);
    },
    enabled,
    ...(refetchInterval === undefined ? {} : { refetchInterval }),
    refetchIntervalInBackground: false,
    ...(refetchOnWindowFocus === undefined ? {} : { refetchOnWindowFocus }),
    ...(retry === undefined ? {} : { retry }),
    ...(retryDelay === undefined ? {} : { retryDelay }),
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
  identityDeviceId?: string;
  connectionMarks?: StoredDevice["connectionMarks"];
  lan?: StoredDevice["lan"];
  devd?: StoredDevice["devd"];
  webSerial?: StoredDevice["webSerial"];
  identity?: Identity | null;
  wifi?: WifiStatus | null;
  lanBaseUrlHints?: string[];
}

const CONNECTION_MARK_ORDER: NonNullable<StoredDevice["connectionMarks"]> = [
  "lan",
  "usb",
  "digital_flash",
  "analog_flash",
];

function normalizeInputIdentityDeviceId(
  identityDeviceId: string | undefined,
): string | undefined {
  const trimmed = identityDeviceId?.trim();
  return trimmed || undefined;
}

function getStoredHardwareIdentity(device: StoredDevice): string | undefined {
  return (
    normalizeInputIdentityDeviceId(device.identityDeviceId) ??
    normalizeInputIdentityDeviceId(device.webSerial?.identityDeviceId)
  );
}

function mergeConnectionMarks(
  first: StoredDevice["connectionMarks"],
  second: StoredDevice["connectionMarks"],
): StoredDevice["connectionMarks"] {
  const marks = new Set([...(first ?? []), ...(second ?? [])]);
  const ordered = CONNECTION_MARK_ORDER.filter((mark) => marks.has(mark));
  return ordered.length > 0 ? ordered : undefined;
}

function isLanBaseUrl(baseUrl: string): boolean {
  try {
    const url = new URL(baseUrl);
    const hostname = url.hostname.toLowerCase();
    return (
      hostname !== "localhost" && hostname !== "127.0.0.1" && hostname !== "::1"
    );
  } catch {
    return false;
  }
}

function normalizeHttpBaseUrl(baseUrl: string | undefined): string | undefined {
  const trimmed = baseUrl?.trim();
  if (!trimmed) {
    return undefined;
  }

  try {
    const url = new URL(trimmed);
    if (url.protocol !== "http:" && url.protocol !== "https:") {
      return undefined;
    }
    url.pathname = "/";
    url.search = "";
    url.hash = "";
    const normalized = url.toString();
    return normalized.endsWith("/") ? normalized.slice(0, -1) : normalized;
  } catch {
    return undefined;
  }
}

function normalizeHttpHost(
  host: string | null | undefined,
): string | undefined {
  const trimmed = host?.trim();
  if (!trimmed) {
    return undefined;
  }
  return trimmed.replace(/^https?:\/\//i, "").replace(/\/.*$/, "");
}

function isUsableLanHost(host: string | undefined): host is string {
  if (!host) {
    return false;
  }
  const normalized = host.toLowerCase();
  const looksLikeIpAddress = /^\d{1,3}(?:\.\d{1,3}){3}$/.test(normalized);
  const looksLikeDnsName = normalized.includes(".");
  return (
    (looksLikeIpAddress || looksLikeDnsName) &&
    normalized !== "localhost" &&
    normalized !== "127.0.0.1" &&
    normalized !== "::1" &&
    normalized !== "0.0.0.0" &&
    normalized !== "unknown"
  );
}

function makeHttpBaseUrlFromHost(
  host: string | null | undefined,
): string | undefined {
  const normalizedHost = normalizeHttpHost(host);
  if (!isUsableLanHost(normalizedHost)) {
    return undefined;
  }
  return normalizeHttpBaseUrl(`http://${normalizedHost}`);
}

function deriveLanEndpoint(input: AddRealDeviceInput): StoredDevice["lan"] {
  const candidates = [
    normalizeHttpBaseUrl(input.lan?.baseUrl),
    normalizeHttpBaseUrl(input.baseUrl),
    ...(input.lanBaseUrlHints ?? []).map(normalizeHttpBaseUrl),
    makeHttpBaseUrlFromHost(input.identity?.hostname),
    makeHttpBaseUrlFromHost(input.identity?.network.hostname),
    makeHttpBaseUrlFromHost(input.wifi?.ip),
    makeHttpBaseUrlFromHost(input.identity?.network.ip),
  ];

  const baseUrl = candidates.find(
    (candidate) => candidate && isLanBaseUrl(candidate),
  );
  return baseUrl ? { baseUrl } : undefined;
}

function getInputLanEndpoint(input: AddRealDeviceInput): StoredDevice["lan"] {
  return deriveLanEndpoint(input);
}

export function upsertRealDevice(
  current: StoredDevice[],
  input: AddRealDeviceInput,
): StoredDevice[] {
  const identityDeviceId = normalizeInputIdentityDeviceId(
    input.identityDeviceId,
  );
  const lan = getInputLanEndpoint(input);
  const existingIndex = identityDeviceId
    ? current.findIndex(
        (device) => getStoredHardwareIdentity(device) === identityDeviceId,
      )
    : -1;

  if (existingIndex >= 0) {
    const existing = current[existingIndex];
    if (!existing) {
      return current;
    }

    const next = [...current];
    next[existingIndex] = {
      ...existing,
      name: input.name,
      baseUrl: input.baseUrl,
      identityDeviceId,
      connectionMarks: mergeConnectionMarks(
        existing.connectionMarks,
        mergeConnectionMarks(input.connectionMarks, lan ? ["lan"] : undefined),
      ),
      lan: lan ?? existing.lan,
      devd: input.devd ?? existing.devd,
      webSerial: input.webSerial ?? existing.webSerial,
    };
    return next;
  }

  const nextDevice: StoredDevice = {
    id: `device-${String(current.length + 1).padStart(3, "0")}`,
    name: input.name,
    baseUrl: input.baseUrl,
    identityDeviceId,
    connectionMarks: mergeConnectionMarks(
      input.connectionMarks,
      lan ? ["lan"] : undefined,
    ),
    lan,
    devd: input.devd,
    webSerial: input.webSerial,
  };

  return [...current, nextDevice];
}

export function useAddRealDeviceMutation() {
  const store = useDeviceStore();
  const queryClient = useQueryClient();
  const isDemoMode = getActiveDemoMode();
  const queryKey = getDevicesQueryKey(isDemoMode);

  return useMutation({
    mutationFn: async (input: AddRealDeviceInput) => {
      const current = store.getDevices();
      const next = upsertRealDevice(current, input);
      store.setDevices(next);
      return store.getDevices();
    },
    onSuccess: (next) => {
      syncDevicesQueryCache(queryClient, next, queryKey);
    },
  });
}

export * from "./scan-hooks.ts";

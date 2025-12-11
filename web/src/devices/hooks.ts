import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ENABLE_MOCK,
  getIdentity,
  HttpApiError,
  isMockBaseUrl,
} from "../api/client.ts";
import type { Identity } from "../api/types.ts";
import { loadDevices, type StoredDevice, saveDevices } from "./device-store.ts";

export function useDevicesQuery() {
  return useQuery({
    queryKey: ["devices"],
    queryFn: async () => loadDevices(),
  });
}

export function useDeviceIdentity(device: StoredDevice | null | undefined) {
  const baseUrl = device?.baseUrl;
  const queryKey = ["device", device?.id ?? "unknown", "identity"] as const;
  const jitterRetryDelay = () => 200 + Math.random() * 300;

  return useQuery<Identity, HttpApiError>({
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
    retry: (failureCount, error) => {
      if (error instanceof HttpApiError && error.code === "NO_BASE_URL") {
        return false;
      }
      const isRealDevice =
        Boolean(baseUrl) && baseUrl !== undefined && !isMockBaseUrl(baseUrl);
      return isRealDevice ? failureCount < 2 : failureCount < 1;
    },
    retryDelay: jitterRetryDelay,
  });
}

export function useAddDeviceMutation() {
  // Adds a demo device backed by the in-memory mock backend. This is only
  // available when ENABLE_MOCK is true.
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      if (!ENABLE_MOCK) {
        throw new Error("Mock backend is disabled");
      }
      const current = loadDevices();
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
      saveDevices(next);
      return next;
    },
    onSuccess: (next) => {
      queryClient.setQueryData<StoredDevice[]>(["devices"], next);
    },
  });
}

export interface AddRealDeviceInput {
  name: string;
  baseUrl: string;
}

export function useAddRealDeviceMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: AddRealDeviceInput) => {
      const current = loadDevices();
      const index = current.length + 1;
      const nextDevice: StoredDevice = {
        id: `device-${String(index).padStart(3, "0")}`,
        name: input.name,
        baseUrl: input.baseUrl,
      };
      const next = [...current, nextDevice];
      saveDevices(next);
      return next;
    },
    onSuccess: (next) => {
      queryClient.setQueryData<StoredDevice[]>(["devices"], next);
    },
  });
}

export * from "./scan-hooks.ts";

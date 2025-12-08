import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getIdentity, HttpApiError } from "../api/client.ts";
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
    retry: 1,
  });
}

export function useAddDeviceMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      const current = loadDevices();
      const index = current.length + 1;
      const nextDevice: StoredDevice = {
        id: `mock-${String(index).padStart(3, "0")}`,
        name: `Mock Device ${index}`,
        baseUrl: "http://localhost:25219",
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

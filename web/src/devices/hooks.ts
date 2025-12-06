import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { loadDevices, type StoredDevice, saveDevices } from "./device-store.ts";

export function useDevicesQuery() {
  return useQuery({
    queryKey: ["devices"],
    queryFn: async () => loadDevices(),
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

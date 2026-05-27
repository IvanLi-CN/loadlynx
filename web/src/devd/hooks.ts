import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import type { StoredDevice } from "../devices/device-store.ts";
import { useDeviceStore } from "../devices/store-context.tsx";
import {
  buildDevdCompatBaseUrl,
  createDevdLease,
  DEFAULT_DEVD_BASE_URL,
  DevdApiError,
  flashDevdDevice,
  getDevdSession,
  heartbeatDevdLease,
  scanDevdDevices,
  selectDevdArtifact,
} from "./client.ts";
import type { DevdTargetKind } from "./types.ts";

export function useDevdScan(baseUrl: string = DEFAULT_DEVD_BASE_URL) {
  return useMutation({
    mutationFn: () => scanDevdDevices(baseUrl),
  });
}

export function useCreateDevdLease(baseUrl: string = DEFAULT_DEVD_BASE_URL) {
  return useMutation({
    mutationFn: (deviceId: string) => createDevdLease(deviceId, baseUrl),
  });
}

export function useDevdLeaseHeartbeats(devices: StoredDevice[] | undefined) {
  const store = useDeviceStore();
  const queryClient = useQueryClient();

  useEffect(() => {
    const leases = (devices ?? [])
      .map((device) => ({ storedDeviceId: device.id, devd: device.devd }))
      .filter(
        (
          item,
        ): item is {
          storedDeviceId: string;
          devd: { baseUrl: string; deviceId: string; leaseId: string };
        } =>
          Boolean(
            item.devd?.baseUrl && item.devd.deviceId && item.devd.leaseId,
          ),
      );
    if (leases.length === 0) {
      return;
    }

    let cancelled = false;
    const heartbeat = () => {
      for (const { storedDeviceId, devd } of leases) {
        heartbeatDevdLease(devd.leaseId, devd.baseUrl).catch((error) => {
          if (
            cancelled ||
            !(error instanceof DevdApiError) ||
            error.code !== "web_session_expired"
          ) {
            return;
          }

          createDevdLease(devd.deviceId, devd.baseUrl)
            .then((lease) => {
              if (cancelled) {
                return;
              }
              const current = store.getDevices();
              const next = current.map((device) => {
                if (
                  device.id !== storedDeviceId ||
                  device.devd?.leaseId !== devd.leaseId
                ) {
                  return device;
                }

                const nextDevd = {
                  ...device.devd,
                  leaseId: lease.lease_id,
                };
                return {
                  ...device,
                  baseUrl: buildDevdCompatBaseUrl(nextDevd),
                  devd: nextDevd,
                };
              });
              store.setDevices(next);
              queryClient.setQueryData<StoredDevice[]>(["devices"], next);
            })
            .catch(() => {
              // Best-effort renewal; the device row remains visible for manual reconnect.
            });
        });
      }
    };
    heartbeat();
    const interval = window.setInterval(() => {
      if (!cancelled) {
        heartbeat();
      }
    }, 2_000);

    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [devices, queryClient, store]);
}

export function useDevdSession(
  deviceId: string | null,
  leaseId: string | null,
  baseUrl: string = DEFAULT_DEVD_BASE_URL,
) {
  return useQuery({
    queryKey: ["devd", "session", baseUrl, deviceId, leaseId],
    enabled: Boolean(deviceId && leaseId),
    queryFn: () => getDevdSession(deviceId ?? "", leaseId ?? "", baseUrl),
  });
}

export function useDevdFlash(baseUrl: string = DEFAULT_DEVD_BASE_URL) {
  return useMutation({
    mutationFn: (input: {
      deviceId: string;
      leaseId?: string;
      target: DevdTargetKind;
      artifactId?: string;
      dryRun: boolean;
    }) => flashDevdDevice({ baseUrl, ...input }),
  });
}

export function useDevdArtifactSelect(baseUrl: string = DEFAULT_DEVD_BASE_URL) {
  return useMutation({
    mutationFn: (input: {
      deviceId: string;
      manifestPath: string;
      artifactId?: string;
    }) => selectDevdArtifact({ baseUrl, ...input }),
  });
}

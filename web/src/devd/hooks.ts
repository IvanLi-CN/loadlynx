import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import type { StoredDevice } from "../devices/device-store.ts";
import {
  createDevdLease,
  DEFAULT_DEVD_BASE_URL,
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
  useEffect(() => {
    const leases = (devices ?? [])
      .map((device) => device.devd)
      .filter(
        (
          devd,
        ): devd is { baseUrl: string; deviceId: string; leaseId: string } =>
          Boolean(devd?.baseUrl && devd.deviceId && devd.leaseId),
      );
    if (leases.length === 0) {
      return;
    }

    let cancelled = false;
    const heartbeat = () => {
      for (const lease of leases) {
        heartbeatDevdLease(lease.leaseId, lease.baseUrl).catch(() => {
          // Best-effort heartbeat; session views surface expired leases explicitly.
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
  }, [devices]);
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

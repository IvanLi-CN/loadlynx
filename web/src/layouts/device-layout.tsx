import {
  Link,
  Outlet,
  useParams,
  useRouterState,
} from "@tanstack/react-router";
import { createContext, useContext, useEffect, useMemo, useRef } from "react";
import { postCalibrationMode } from "../api/client.ts";
import type { StoredDevice } from "../devices/device-store.ts";
import { useDevicesQuery } from "../devices/hooks.ts";

type DeviceContextValue = {
  deviceId: string;
  device: StoredDevice;
  baseUrl: string;
};

const DeviceContext = createContext<DeviceContextValue | null>(null);

export function useDeviceContext() {
  const value = useContext(DeviceContext);
  if (!value) {
    throw new Error("useDeviceContext must be used within <DeviceLayout />");
  }
  return value;
}

export function DeviceLayout() {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const isCalibrationPage = /\/calibration$/.test(pathname);
  const { deviceId } = useParams({ strict: false }) as {
    deviceId?: string;
  };

  const devicesQuery = useDevicesQuery();
  const device = useMemo(
    () =>
      deviceId && devicesQuery.data
        ? devicesQuery.data.find((entry) => entry.id === deviceId)
        : undefined,
    [devicesQuery.data, deviceId],
  );

  const lastKnownBaseUrlRef = useRef<string | null>(null);
  useEffect(() => {
    if (device?.baseUrl) {
      lastKnownBaseUrlRef.current = device.baseUrl;
    }
  }, [device?.baseUrl]);

  useEffect(() => {
    void pathname;
    if (isCalibrationPage) return;

    const baseUrl = device?.baseUrl ?? lastKnownBaseUrlRef.current;
    if (!baseUrl) return;

    postCalibrationMode(baseUrl, { kind: "off" }).catch(() => {
      // Best-effort; do not block rendering or show UI errors here.
    });
  }, [device?.baseUrl, isCalibrationPage, pathname]);

  if (devicesQuery.isLoading) {
    return <p className="text-sm text-base-content/60">Loading device...</p>;
  }

  if (!deviceId || !device) {
    return (
      <div className="flex flex-col gap-4 max-w-xl">
        <h2 className="text-xl font-bold">Device not found</h2>
        <p className="text-sm text-base-content/70">
          The requested device ID{" "}
          <code className="font-mono bg-base-200 px-1 rounded">
            {deviceId ?? "unknown"}
          </code>{" "}
          does not exist in the local registry. Please return to the device list
          and add or select a device.
        </p>
        <div>
          <Link to="/devices" className="btn btn-sm btn-outline">
            Back to devices
          </Link>
        </div>
      </div>
    );
  }

  const contextValue: DeviceContextValue = {
    deviceId,
    device,
    baseUrl: device.baseUrl,
  };

  return (
    <DeviceContext.Provider value={contextValue}>
      <Outlet />
    </DeviceContext.Provider>
  );
}

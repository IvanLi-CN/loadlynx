import {
  Link,
  Outlet,
  useParams,
  useRouterState,
} from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { postCalibrationMode } from "../api/client.ts";
import { useDevicesQuery } from "../devices/hooks.ts";

export function ConsoleLayout() {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const isCalibrationPage = /\/calibration$/.test(pathname);
  const { deviceId } = useParams({ strict: false }) as {
    deviceId?: string;
  };

  const { data: devices } = useDevicesQuery();
  const currentDevice =
    deviceId && devices
      ? devices.find((device) => device.id === deviceId)
      : undefined;

  const lastDeviceBaseUrlRef = useRef<string | null>(null);
  useEffect(() => {
    if (currentDevice?.baseUrl) {
      lastDeviceBaseUrlRef.current = currentDevice.baseUrl;
    }
  }, [currentDevice?.baseUrl]);

  // Enforce calibration mode based on the current page:
  // - /calibration manages its own mode (voltage/current tabs)
  // - all other pages should keep mode off
  //
  // This makes page entry resilient even if the previous page failed to clean up
  // (e.g., refresh, navigation glitches, tab close).
  useEffect(() => {
    void pathname;
    if (isCalibrationPage) return;

    const baseUrl = currentDevice?.baseUrl ?? lastDeviceBaseUrlRef.current;
    if (!baseUrl) return;

    postCalibrationMode(baseUrl, { kind: "off" }).catch(() => {
      // Best-effort; do not block navigation or show UI errors here.
    });
  }, [currentDevice?.baseUrl, isCalibrationPage, pathname]);

  return (
    <>
      <header className="navbar bg-base-200 border-b border-base-300 px-6 py-2">
        <div className="navbar-start">
          <div className="flex flex-col items-start">
            <h1 className="text-xl font-semibold px-2">LoadLynx Web Console</h1>
            <span className="text-xs text-base-content/70 px-2">
              Network device manager & CC control
            </span>
          </div>
        </div>

        <div className="navbar-end flex gap-4">
          <label className="form-control w-full max-w-[250px]">
            <div className="label pt-0 pb-1">
              <span className="label-text-alt text-base-content/70">
                Current device
              </span>
            </div>
            <div className="join">
              <select
                disabled
                className="select select-bordered select-sm w-full join-item text-xs"
              >
                <option>
                  {currentDevice
                    ? `${currentDevice.name} (${currentDevice.id})`
                    : "No device selected (device selector)"}
                </option>
              </select>
              <button
                type="button"
                className="btn btn-sm join-item btn-square"
                disabled
              >
                ▼
              </button>
            </div>
          </label>

          <div className="flex items-end pb-1">
            <Link
              to="/devices"
              activeProps={{ className: "btn-active" }}
              className="btn btn-sm btn-outline rounded-full"
            >
              Add device
            </Link>
          </div>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <aside className="w-64 bg-base-200/50 border-r border-base-300 overflow-y-auto">
          <ul className="menu p-4 w-full gap-1">
            <li className="menu-title uppercase tracking-wider opacity-70 text-xs">
              Navigation
            </li>
            <li>
              <Link
                to="/devices"
                activeProps={{ className: "active" }}
                className="rounded-box"
              >
                Devices
              </Link>
            </li>

            {deviceId ? (
              <>
                <li>
                  <Link
                    to="/$deviceId/cc"
                    params={{ deviceId }}
                    activeProps={{ className: "active" }}
                    className="rounded-box"
                  >
                    CC Control
                  </Link>
                </li>
                <li>
                  <Link
                    to="/$deviceId/status"
                    params={{ deviceId }}
                    activeProps={{ className: "active" }}
                    className="rounded-box"
                  >
                    Status
                  </Link>
                </li>
                <li>
                  <Link
                    to="/$deviceId/settings"
                    params={{ deviceId }}
                    activeProps={{ className: "active" }}
                    className="rounded-box"
                  >
                    Settings
                  </Link>
                </li>
                <li>
                  <Link
                    to="/$deviceId/calibration"
                    params={{ deviceId }}
                    activeProps={{ className: "active" }}
                    className="rounded-box"
                  >
                    Calibration
                  </Link>
                </li>
              </>
            ) : (
              <>
                <li>
                  <button
                    type="button"
                    disabled
                    className="disabled:bg-transparent disabled:text-base-content/30 cursor-not-allowed"
                  >
                    CC Control
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    disabled
                    className="disabled:bg-transparent disabled:text-base-content/30 cursor-not-allowed"
                  >
                    Status
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    disabled
                    className="disabled:bg-transparent disabled:text-base-content/30 cursor-not-allowed"
                  >
                    Settings
                  </button>
                </li>
              </>
            )}

            <li className="menu-title mt-6 uppercase tracking-wider opacity-70 text-xs">
              Other functions
            </li>
          </ul>
        </aside>

        <main className="flex-1 p-6 overflow-y-auto bg-base-100">
          <Outlet />
        </main>
      </div>
    </>
  );
}

export default ConsoleLayout;

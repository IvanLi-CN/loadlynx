import {
  Link,
  Outlet,
  useNavigate,
  useParams,
  useRouterState,
} from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { AppIcon } from "../components/icons/app-icon.tsx";
import {
  NAV_ICON_CALIBRATION,
  NAV_ICON_CC,
  NAV_ICON_COLLAPSE,
  NAV_ICON_DEVICES,
  NAV_ICON_EXPAND,
  NAV_ICON_MENU,
  NAV_ICON_SETTINGS,
  NAV_ICON_STATUS,
} from "../components/icons/nav-icons.ts";
import { useDevicesQuery } from "../devices/hooks.ts";

type DeviceTab = "cc" | "status" | "settings" | "calibration";

function isDeviceTab(value: string): value is DeviceTab {
  return (
    value === "cc" ||
    value === "status" ||
    value === "settings" ||
    value === "calibration"
  );
}

export function ConsoleLayout() {
  const navigate = useNavigate();

  const { deviceId } = useParams({ strict: false }) as {
    deviceId?: string;
  };
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });

  const { data: devices } = useDevicesQuery();
  const currentDevice =
    deviceId && devices
      ? devices.find((device) => device.id === deviceId)
      : undefined;

  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isMediumSidebarExpanded, setIsMediumSidebarExpanded] = useState(false);

  const closeDrawer = () => setIsDrawerOpen(false);

  useEffect(() => {
    if (!isDrawerOpen) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setIsDrawerOpen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = previousOverflow;
    };
  }, [isDrawerOpen]);

  const currentDeviceTab: DeviceTab | null = useMemo(() => {
    const segments = pathname.split("/").filter(Boolean);
    if (segments.length < 2) return null;
    if (!deviceId) return null;
    if (segments[0] !== deviceId) return null;
    return isDeviceTab(segments[1]) ? segments[1] : null;
  }, [deviceId, pathname]);

  const handleDeviceSwitch = (nextDeviceId: string) => {
    if (!nextDeviceId) return;
    if (nextDeviceId === deviceId) {
      closeDrawer();
      return;
    }

    const targetTab: DeviceTab = currentDeviceTab ?? "cc";

    closeDrawer();

    switch (targetTab) {
      case "cc":
        navigate({ to: "/$deviceId/cc", params: { deviceId: nextDeviceId } });
        return;
      case "status":
        navigate({
          to: "/$deviceId/status",
          params: { deviceId: nextDeviceId },
        });
        return;
      case "settings":
        navigate({
          to: "/$deviceId/settings",
          params: { deviceId: nextDeviceId },
        });
        return;
      case "calibration":
        navigate({
          to: "/$deviceId/calibration",
          params: { deviceId: nextDeviceId },
        });
        return;
    }
  };

  const SidebarNav = ({ variant }: { variant: "drawer" | "sidebar" }) => {
    const isDrawer = variant === "drawer";
    const isSidebarRail = !isDrawer && !isMediumSidebarExpanded;

    const labelVisibilityClass = isDrawer
      ? ""
      : isMediumSidebarExpanded
        ? "md:inline"
        : "md:hidden lg:inline";

    const itemLayoutClass = isDrawer
      ? "justify-start"
      : isMediumSidebarExpanded
        ? "md:justify-start"
        : "md:justify-center lg:justify-start";

    const itemPaddingClass = isDrawer
      ? "px-3 py-2"
      : isMediumSidebarExpanded
        ? "md:px-3 md:py-2"
        : "md:px-2 md:py-2 lg:px-3 lg:py-2";

    const linkClassName = [
      "w-full rounded-box flex items-center gap-3",
      itemLayoutClass,
      itemPaddingClass,
    ].join(" ");

    const disabledButtonClassName = [
      "w-full rounded-box flex items-center gap-3 text-left",
      itemLayoutClass,
      itemPaddingClass,
      "disabled:bg-transparent disabled:text-base-content/30 cursor-not-allowed",
    ].join(" ");

    const navIconSize = isDrawer ? 18 : 20;

    return (
      <ul
        className={isDrawer ? "menu p-4 w-full gap-1" : "menu p-3 w-full gap-1"}
      >
        <li
          className={[
            "menu-title uppercase tracking-wider opacity-70 text-xs",
            isSidebarRail ? "md:hidden lg:block" : "",
          ].join(" ")}
        >
          Navigation
        </li>

        <li>
          <Link
            to="/devices"
            activeProps={{ className: `${linkClassName} active` }}
            className={linkClassName}
            aria-label="Devices"
            title="Devices"
            onClick={isDrawer ? closeDrawer : undefined}
          >
            <AppIcon icon={NAV_ICON_DEVICES} size={navIconSize} />
            <span className={labelVisibilityClass}>Devices</span>
          </Link>
        </li>

        {deviceId ? (
          <>
            <li>
              <Link
                to="/$deviceId/cc"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label="CC Control"
                title="CC Control"
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_CC} size={navIconSize} />
                <span className={labelVisibilityClass}>CC Control</span>
              </Link>
            </li>
            <li>
              <Link
                to="/$deviceId/status"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label="Status"
                title="Status"
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_STATUS} size={navIconSize} />
                <span className={labelVisibilityClass}>Status</span>
              </Link>
            </li>
            <li>
              <Link
                to="/$deviceId/settings"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label="Settings"
                title="Settings"
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_SETTINGS} size={navIconSize} />
                <span className={labelVisibilityClass}>Settings</span>
              </Link>
            </li>
            <li>
              <Link
                to="/$deviceId/calibration"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label="Calibration"
                title="Calibration"
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_CALIBRATION} size={navIconSize} />
                <span className={labelVisibilityClass}>Calibration</span>
              </Link>
            </li>
          </>
        ) : (
          <>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_CC} size={navIconSize} />
                <span className={labelVisibilityClass}>CC Control</span>
              </button>
            </li>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_STATUS} size={navIconSize} />
                <span className={labelVisibilityClass}>Status</span>
              </button>
            </li>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_SETTINGS} size={navIconSize} />
                <span className={labelVisibilityClass}>Settings</span>
              </button>
            </li>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_CALIBRATION} size={navIconSize} />
                <span className={labelVisibilityClass}>Calibration</span>
              </button>
            </li>
          </>
        )}
      </ul>
    );
  };

  return (
    <>
      <header className="navbar bg-base-200 border-b border-base-300 px-3 sm:px-4 md:px-6 py-2">
        <div className="navbar-start">
          <button
            type="button"
            className="btn btn-ghost btn-square md:hidden"
            aria-label="Open navigation drawer"
            onClick={() => setIsDrawerOpen(true)}
          >
            <AppIcon icon={NAV_ICON_MENU} />
          </button>

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
        <aside
          className={[
            "hidden md:flex flex-col bg-base-200/50 border-r border-base-300 overflow-y-auto",
            isMediumSidebarExpanded ? "md:w-64" : "md:w-20",
            "lg:w-64",
          ].join(" ")}
        >
          <div
            className={[
              "px-3 py-2 w-full flex items-center lg:hidden",
              isMediumSidebarExpanded ? "justify-end" : "justify-center",
            ].join(" ")}
          >
            <button
              type="button"
              className="btn btn-ghost btn-sm btn-square"
              aria-label={
                isMediumSidebarExpanded ? "Collapse sidebar" : "Expand sidebar"
              }
              title={isMediumSidebarExpanded ? "Collapse sidebar" : "Expand sidebar"}
              onClick={() => setIsMediumSidebarExpanded((value) => !value)}
            >
              <AppIcon
                icon={isMediumSidebarExpanded ? NAV_ICON_COLLAPSE : NAV_ICON_EXPAND}
                size={20}
              />
            </button>
          </div>

          <SidebarNav variant="sidebar" />
        </aside>

        <main className="flex-1 p-3 sm:p-4 md:p-6 overflow-y-auto bg-base-100">
          <Outlet />
        </main>
      </div>

      {isDrawerOpen ? (
        <div className="fixed inset-0 z-50">
          <div
            className="absolute inset-0 bg-black/40"
            aria-hidden="true"
            onClick={closeDrawer}
          />
          <div
            className="absolute inset-y-0 left-0 w-80 max-w-[85vw] bg-base-100 shadow-xl border-r border-base-300"
            role="dialog"
            aria-modal="true"
            aria-label="Navigation drawer"
          >
            <div className="flex h-full flex-col">
              <div className="flex items-center justify-between px-4 py-3 border-b border-base-300 bg-base-200">
                <div className="font-semibold">Navigation</div>
                <button
                  type="button"
                  className="btn btn-ghost btn-sm btn-square"
                  aria-label="Close navigation drawer"
                  onClick={closeDrawer}
                >
                  ✕
                </button>
              </div>

              <div className="flex-1 overflow-y-auto">
                <SidebarNav variant="drawer" />
              </div>

              <div className="border-t border-base-300 bg-base-200">
                <div className="px-4 py-3">
                  <div className="text-xs uppercase tracking-wider opacity-70 mb-2">
                    Device switcher
                  </div>
                  <div className="join w-full">
                    <select
                      className="select select-bordered select-sm w-full join-item text-xs"
                      value={deviceId ?? ""}
                      onChange={(event) => {
                        handleDeviceSwitch(event.target.value);
                      }}
                      aria-label="Switch device"
                    >
                      <option value="" disabled>
                        {devices?.length
                          ? "Select a device…"
                          : "No devices available"}
                      </option>
                      {(devices ?? []).map((device) => (
                        <option key={device.id} value={device.id}>
                          {device.name} ({device.id})
                        </option>
                      ))}
                    </select>
                    <button
                      type="button"
                      className="btn btn-sm join-item btn-square"
                      aria-label="Close drawer"
                      title="Close drawer"
                      onClick={closeDrawer}
                    >
                      ✕
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}

export default ConsoleLayout;

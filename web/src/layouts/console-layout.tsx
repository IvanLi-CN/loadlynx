import {
  Link,
  Outlet,
  useNavigate,
  useParams,
  useRouterState,
} from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AppIcon } from "../components/icons/app-icon.tsx";
import {
  NAV_ICON_CALIBRATION,
  NAV_ICON_CC,
  NAV_ICON_COLLAPSE,
  NAV_ICON_DEVICES,
  NAV_ICON_EXPAND,
  NAV_ICON_FIRMWARE,
  NAV_ICON_MENU,
  NAV_ICON_PD,
  NAV_ICON_SETTINGS,
  NAV_ICON_STATUS,
} from "../components/icons/nav-icons.ts";
import { AppVersionLink } from "../components/layout/app-version-link.tsx";
import { LanguageSwitcher } from "../components/layout/language-switcher.tsx";
import { Select } from "../components/ui/field.tsx";
import { useDevdLeaseHeartbeats } from "../devd/hooks.ts";
import { useDevicesQuery } from "../devices/hooks.ts";

type DeviceTab =
  | "cc"
  | "status"
  | "pd"
  | "settings"
  | "calibration"
  | "firmware";

function isDeviceTab(value: string): value is DeviceTab {
  return (
    value === "cc" ||
    value === "status" ||
    value === "pd" ||
    value === "settings" ||
    value === "calibration" ||
    value === "firmware"
  );
}

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export function ConsoleLayout() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const storybookRuntime = isStorybookRuntime();

  const { deviceId } = useParams({ strict: false }) as {
    deviceId?: string;
  };
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });

  const { data: devices } = useDevicesQuery();
  useDevdLeaseHeartbeats(devices);
  const currentDevice =
    deviceId && devices
      ? devices.find((device) => device.id === deviceId)
      : undefined;

  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isMediumSidebarExpanded, setIsMediumSidebarExpanded] = useState(false);

  const closeDrawer = () => setIsDrawerOpen(false);

  const appVersion = import.meta.env.VITE_APP_VERSION?.trim() || null;
  const appGitSha = import.meta.env.VITE_APP_GIT_SHA?.trim() || null;
  const appGitTag = import.meta.env.VITE_APP_GIT_TAG?.trim() || null;
  const githubRepo =
    import.meta.env.VITE_GITHUB_REPO?.trim() || "IvanLi-CN/loadlynx";

  const shouldShowVersionLink = !storybookRuntime && appVersion != null;
  const sidebarVersionVisibilityClass = isMediumSidebarExpanded
    ? ""
    : "md:hidden lg:block";

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
      case "pd":
        navigate({
          to: "/$deviceId/pd",
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
      case "firmware":
        navigate({
          to: "/$deviceId/firmware",
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
      "ll-nav-link",
      itemLayoutClass,
      itemPaddingClass,
    ].join(" ");

    const disabledButtonClassName = [
      "ll-nav-link text-left",
      itemLayoutClass,
      itemPaddingClass,
      "disabled:bg-transparent disabled:text-base-content/30 cursor-not-allowed",
    ].join(" ");

    const navIconSize = isDrawer ? 18 : 20;

    return (
      <ul
        className={isDrawer ? "p-4 w-full space-y-1" : "p-3 w-full space-y-1"}
      >
        <li
          className={[
            "px-3 pb-2 font-mono uppercase tracking-wider opacity-70 text-xs",
            isSidebarRail ? "md:hidden lg:block" : "",
          ].join(" ")}
        >
          {t("nav.navigation")}
        </li>

        <li>
          <Link
            to="/devices"
            activeProps={{ className: `${linkClassName} active` }}
            className={linkClassName}
            aria-label={t("nav.devices")}
            title={t("nav.devices")}
            onClick={isDrawer ? closeDrawer : undefined}
          >
            <AppIcon icon={NAV_ICON_DEVICES} size={navIconSize} />
            <span className={labelVisibilityClass}>{t("nav.devices")}</span>
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
                aria-label={t("nav.cc")}
                title={t("nav.cc")}
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_CC} size={navIconSize} />
                <span className={labelVisibilityClass}>{t("nav.cc")}</span>
              </Link>
            </li>
            <li>
              <Link
                to="/$deviceId/status"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label={t("nav.status")}
                title={t("nav.status")}
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_STATUS} size={navIconSize} />
                <span className={labelVisibilityClass}>{t("nav.status")}</span>
              </Link>
            </li>
            <li>
              <Link
                to="/$deviceId/pd"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label={t("nav.pd")}
                title={t("nav.pd")}
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_PD} size={navIconSize} />
                <span className={labelVisibilityClass}>{t("nav.pd")}</span>
              </Link>
            </li>
            <li>
              <Link
                to="/$deviceId/settings"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label={t("nav.settings")}
                title={t("nav.settings")}
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_SETTINGS} size={navIconSize} />
                <span className={labelVisibilityClass}>
                  {t("nav.settings")}
                </span>
              </Link>
            </li>
            <li>
              <Link
                to="/$deviceId/firmware"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label={t("nav.firmware")}
                title={t("nav.firmware")}
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_FIRMWARE} size={navIconSize} />
                <span className={labelVisibilityClass}>
                  {t("nav.firmware")}
                </span>
              </Link>
            </li>
            <li>
              <Link
                to="/$deviceId/calibration"
                params={{ deviceId }}
                activeProps={{ className: `${linkClassName} active` }}
                className={linkClassName}
                aria-label={t("nav.calibration")}
                title={t("nav.calibration")}
                onClick={isDrawer ? closeDrawer : undefined}
              >
                <AppIcon icon={NAV_ICON_CALIBRATION} size={navIconSize} />
                <span className={labelVisibilityClass}>
                  {t("nav.calibration")}
                </span>
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
                <span className={labelVisibilityClass}>{t("nav.cc")}</span>
              </button>
            </li>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_STATUS} size={navIconSize} />
                <span className={labelVisibilityClass}>{t("nav.status")}</span>
              </button>
            </li>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_PD} size={navIconSize} />
                <span className={labelVisibilityClass}>{t("nav.pd")}</span>
              </button>
            </li>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_SETTINGS} size={navIconSize} />
                <span className={labelVisibilityClass}>
                  {t("nav.settings")}
                </span>
              </button>
            </li>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_FIRMWARE} size={navIconSize} />
                <span className={labelVisibilityClass}>
                  {t("nav.firmware")}
                </span>
              </button>
            </li>
            <li>
              <button
                type="button"
                disabled
                className={disabledButtonClassName}
              >
                <AppIcon icon={NAV_ICON_CALIBRATION} size={navIconSize} />
                <span className={labelVisibilityClass}>
                  {t("nav.calibration")}
                </span>
              </button>
            </li>
          </>
        )}
      </ul>
    );
  };

  return (
    <>
      <header className="ll-topbar flex flex-wrap items-center justify-between gap-3 px-3 py-3 sm:px-4 md:px-6">
        <div className="flex min-w-0 items-center gap-2">
          <button
            type="button"
            className="ll-button ll-button-ghost ll-button-square md:hidden"
            aria-label={t("shell.openNavigation")}
            onClick={() => setIsDrawerOpen(true)}
          >
            <AppIcon icon={NAV_ICON_MENU} />
          </button>

          <div className="flex flex-col items-start">
            <h1 className="px-2 text-lg font-bold sm:text-xl">
              {t("app.title")}
            </h1>
            <span className="px-2 text-xs text-base-content/70">
              {t("app.subtitle")}
            </span>
          </div>
        </div>

        <div className="flex flex-1 flex-wrap items-end justify-end gap-3">
          <LanguageSwitcher />
          <label
            className="ll-form-control w-full max-w-[250px]"
            htmlFor="current-device-selector"
          >
            <span className="ll-label-text-alt">
              {t("shell.currentDevice")}
            </span>
            <Select
              id="current-device-selector"
              name="current_device"
              disabled
              className="ll-select-sm text-xs"
            >
              <option>
                {currentDevice
                  ? `${currentDevice.name} (${currentDevice.id})`
                  : t("shell.noDeviceSelected")}
              </option>
            </Select>
          </label>

          <div className="flex items-end pb-1">
            <Link
              to="/devices"
              activeProps={{ className: "ll-button-active" }}
              className="ll-button ll-button-sm ll-button-outline ll-button-circle"
            >
              {t("shell.addDevice")}
            </Link>
          </div>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        <aside
          className={[
            "hidden md:flex flex-col bg-base-200/50 border-r border-base-300 overflow-hidden",
            "ll-sidebar",
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
              className="ll-button ll-button-ghost ll-button-sm ll-button-square"
              aria-label={
                isMediumSidebarExpanded
                  ? t("shell.collapseSidebar")
                  : t("shell.expandSidebar")
              }
              title={
                isMediumSidebarExpanded
                  ? t("shell.collapseSidebar")
                  : t("shell.expandSidebar")
              }
              onClick={() => setIsMediumSidebarExpanded((value) => !value)}
            >
              <AppIcon
                icon={
                  isMediumSidebarExpanded ? NAV_ICON_COLLAPSE : NAV_ICON_EXPAND
                }
                size={20}
              />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto">
            <SidebarNav variant="sidebar" />
          </div>

          {shouldShowVersionLink ? (
            <div
              className={[
                "border-t border-base-300 bg-base-200/50",
                sidebarVersionVisibilityClass,
              ].join(" ")}
            >
              <div className="px-3 py-3 flex items-center">
                <AppVersionLink
                  version={appVersion}
                  repo={githubRepo}
                  sha={appGitSha}
                  tag={appGitTag}
                />
              </div>
            </div>
          ) : null}
        </aside>

        <main className="flex-1 overflow-y-auto bg-base-100/55 p-3 sm:p-4 md:p-6">
          <Outlet />
        </main>
      </div>

      {isDrawerOpen ? (
        <div className="fixed inset-0 z-50">
          <div
            className="ll-drawer-backdrop absolute inset-0"
            aria-hidden="true"
            onClick={closeDrawer}
          />
          <div
            className="ll-drawer-panel absolute inset-y-0 left-0 w-80 max-w-[85vw]"
            role="dialog"
            aria-modal="true"
            aria-label={t("nav.navigation")}
          >
            <div className="flex h-full flex-col">
              <div className="flex items-center justify-between border-b border-base-300 bg-base-200/70 px-4 py-3">
                <div className="font-semibold">{t("nav.navigation")}</div>
                <button
                  type="button"
                  className="ll-button ll-button-ghost ll-button-sm ll-button-square"
                  aria-label={t("shell.closeNavigation")}
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
                    {t("shell.deviceSwitcher")}
                  </div>
                  <div className="ll-join w-full">
                    <select
                      id="drawer-device-selector"
                      name="drawer_device"
                      className="ll-select ll-select-sm ll-join-item text-xs"
                      value={deviceId ?? ""}
                      onChange={(event) => {
                        handleDeviceSwitch(event.target.value);
                      }}
                      aria-label={t("shell.deviceSwitcher")}
                    >
                      <option value="" disabled>
                        {devices?.length
                          ? t("shell.selectDevice")
                          : t("shell.noDevicesAvailable")}
                      </option>
                      {(devices ?? []).map((device) => (
                        <option key={device.id} value={device.id}>
                          {device.name} ({device.id})
                        </option>
                      ))}
                    </select>
                    <button
                      type="button"
                      className="ll-button ll-button-sm ll-button-square ll-join-item"
                      aria-label={t("shell.closeNavigation")}
                      title={t("shell.closeNavigation")}
                      onClick={closeDrawer}
                    >
                      ✕
                    </button>
                  </div>
                </div>

                {shouldShowVersionLink ? (
                  <div className="border-t border-base-300 px-4 py-2 flex items-center justify-end">
                    <AppVersionLink
                      version={appVersion}
                      repo={githubRepo}
                      sha={appGitSha}
                      tag={appGitTag}
                    />
                  </div>
                ) : null}
              </div>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}

export default ConsoleLayout;

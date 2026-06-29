import { useQuery } from "@tanstack/react-query";
import {
  Link,
  Outlet,
  useNavigate,
  useParams,
  useRouterState,
} from "@tanstack/react-router";
import { Check, ChevronRight, MonitorCog } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HttpApiError } from "../api/client.ts";
import type { FastStatusView } from "../api/types.ts";
import { AppIcon } from "../components/icons/app-icon.tsx";
import {
  NAV_ICON_CC,
  NAV_ICON_DEVICES,
  NAV_ICON_SETTINGS,
} from "../components/icons/nav-icons.ts";
import { AppVersionLink } from "../components/layout/app-version-link.tsx";
import { LanguageSwitcher } from "../components/layout/language-switcher.tsx";
import { useDevdLeaseHeartbeats } from "../devd/hooks.ts";
import type { StoredDevice } from "../devices/device-store.ts";
import {
  getDeviceStatusQueryOptions,
  useDevicesQuery,
} from "../devices/hooks.ts";
import { useDeviceStore } from "../devices/store-context.tsx";
import {
  getConnectionLabels,
  getDeviceRouteIntentFromHref,
  getPrimarySection,
  type PrimarySection,
} from "./console-navigation.ts";

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

function useIsCompactViewport() {
  const [isCompact, setIsCompact] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth < 1024 : false,
  );

  useEffect(() => {
    if (typeof window === "undefined") return undefined;

    const media = window.matchMedia("(max-width: 1023px)");
    const update = () => setIsCompact(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  return isCompact;
}

type DevicePresenceState = {
  tone: "connected" | "connecting" | "error" | "disconnected";
  label: string;
};

function getDevicePresenceState(
  t: ReturnType<typeof useTranslation>["t"],
  device: StoredDevice | undefined,
  status: FastStatusView | undefined,
  isInitialLoading: boolean,
  error: HttpApiError | null,
): DevicePresenceState {
  if (!device?.baseUrl) {
    return {
      tone: "disconnected",
      label: t("shell.deviceStatusDisconnected"),
    };
  }

  if (
    status?.fault_flags_decoded.length ||
    status?.analog_state === "faulted"
  ) {
    return {
      tone: "error",
      label: t("shell.deviceStatusError"),
    };
  }

  if (status?.link_up === false || status?.analog_state === "offline") {
    return {
      tone: "connecting",
      label: t("shell.deviceStatusConnecting"),
    };
  }

  if (status) {
    return {
      tone: "connected",
      label: t("shell.deviceStatusConnected"),
    };
  }

  if (isInitialLoading) {
    return {
      tone: "connecting",
      label: t("shell.deviceStatusConnecting"),
    };
  }

  if (error) {
    return {
      tone: "error",
      label: t("shell.deviceStatusError"),
    };
  }

  return {
    tone: "disconnected",
    label: t("shell.deviceStatusDisconnected"),
  };
}

function DeviceSheet(props: {
  open: boolean;
  onClose: () => void;
  currentDeviceId?: string;
  currentHref: string;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const devicesQuery = useDevicesQuery();
  const devices = devicesQuery.data ?? [];

  useEffect(() => {
    if (!props.open) return undefined;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        props.onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = previousOverflow;
    };
  }, [props.open, props.onClose]);

  if (!props.open) {
    return null;
  }

  const intent = getDeviceRouteIntentFromHref(props.currentHref);

  return (
    <div className="fixed inset-0 z-50">
      <button
        type="button"
        aria-label={t("shell.closeDeviceSheet")}
        className="absolute inset-0 bg-slate-950/65 backdrop-blur-[2px]"
        onClick={props.onClose}
      />
      <aside
        role="dialog"
        aria-modal="true"
        aria-label={t("shell.deviceSwitcher")}
        className="absolute inset-y-0 right-0 flex w-full max-w-[420px] flex-col border-l border-cyan-400/25 bg-[linear-gradient(180deg,oklch(0.16_0.04_262/.98),oklch(0.09_0.03_262/.99))] shadow-[-18px_0_48px_oklch(0.82_0.17_210/.12)]"
      >
        <div className="border-b border-base-300/80 px-5 py-4">
          <div className="flex items-start justify-between gap-4">
            <div>
              <div className="text-xs font-mono uppercase tracking-[0.16em] text-base-content/50">
                {t("shell.deviceSwitcher")}
              </div>
              <h2 className="mt-2 text-lg font-bold">
                {t("shell.chooseDevice")}
              </h2>
            </div>
            <button
              type="button"
              className="ll-button ll-button-ghost ll-button-square ll-button-square-lg ll-drawer-close-button"
              onClick={props.onClose}
              aria-label={t("shell.closeDeviceSheet")}
            >
              <span aria-hidden="true" className="ll-drawer-close-button__icon">
                ×
              </span>
            </button>
          </div>
          <p className="mt-2 text-sm text-base-content/60">
            {t("shell.deviceSheetHint")}
          </p>
        </div>

        <div className="flex-1 overflow-y-auto px-4 py-4">
          <div className="space-y-3">
            {devices.map((device) => {
              const active = device.id === props.currentDeviceId;
              const connectionLabels = getConnectionLabels(device);

              return (
                <button
                  key={device.id}
                  type="button"
                  className={[
                    "group w-full rounded-[20px] border px-4 py-4 text-left transition",
                    active
                      ? "border-cyan-300/55 bg-cyan-400/10 shadow-[0_0_28px_oklch(0.82_0.17_210/.12)]"
                      : "border-base-300/70 bg-base-200/30 hover:border-cyan-300/40 hover:bg-base-200/45",
                  ].join(" ")}
                  onClick={() => {
                    props.onClose();
                    if (intent.route === "status") {
                      navigate({
                        to: "/$deviceId/status",
                        params: { deviceId: device.id },
                      });
                      return;
                    }
                    if (intent.route === "settings") {
                      navigate({
                        to: "/$deviceId/settings",
                        params: { deviceId: device.id },
                      });
                      return;
                    }
                    if (intent.route === "calibration") {
                      navigate({
                        to: "/$deviceId/calibration",
                        params: { deviceId: device.id },
                      });
                      return;
                    }
                    if (intent.route === "firmware") {
                      navigate({
                        to: "/$deviceId/firmware",
                        params: { deviceId: device.id },
                      });
                      return;
                    }
                    if (intent.route === "about") {
                      navigate({
                        to: "/$deviceId/about",
                        params: { deviceId: device.id },
                      });
                      return;
                    }
                    navigate({
                      to: "/$deviceId/cc",
                      params: { deviceId: device.id },
                      search: intent.panel ? { panel: "pd" } : {},
                    });
                  }}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="truncate text-sm font-semibold">
                        {device.name}
                      </div>
                      <div className="mt-1 font-mono text-[11px] text-base-content/55">
                        {device.id}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {active ? (
                        <span className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-cyan-300/45 bg-cyan-300/14 text-cyan-100">
                          <Check size={15} />
                        </span>
                      ) : null}
                      <ChevronRight
                        size={16}
                        className="text-base-content/45 transition group-hover:text-cyan-100"
                        aria-hidden="true"
                      />
                    </div>
                  </div>
                  <div className="mt-3 flex flex-wrap gap-2">
                    {connectionLabels.map((label) => (
                      <span
                        key={label}
                        className="rounded-full border border-base-300/75 bg-base-200/35 px-2.5 py-1 text-[10px] font-mono uppercase tracking-[0.12em] text-base-content/70"
                      >
                        {label}
                      </span>
                    ))}
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      </aside>
    </div>
  );
}

export function ConsoleLayout() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const storybookRuntime = isStorybookRuntime();
  const isCompactViewport = useIsCompactViewport();
  const { deviceId } = useParams({ strict: false }) as { deviceId?: string };
  const deviceStore = useDeviceStore();
  const location = useRouterState({ select: (state) => state.location });
  const pathname = location.pathname;
  const href = location.href;
  const currentSection = getPrimarySection(pathname, deviceId);

  const { data: devices } = useDevicesQuery();
  useDevdLeaseHeartbeats(devices);

  const routeDevice =
    deviceId && devices
      ? devices.find((device) => device.id === deviceId)
      : undefined;
  const [lastActiveDeviceId, setLastActiveDeviceId] = useState<string | null>(
    () => deviceStore.getLastActiveDeviceId(),
  );

  useEffect(() => {
    if (!deviceId) {
      return;
    }
    setLastActiveDeviceId(deviceId);
    deviceStore.setLastActiveDeviceId(deviceId);
  }, [deviceId, deviceStore]);

  const rememberedDevice =
    !routeDevice && lastActiveDeviceId && devices
      ? devices.find((device) => device.id === lastActiveDeviceId)
      : undefined;
  const currentDevice = routeDevice ?? rememberedDevice;
  const currentDeviceMeta = currentDevice?.id ?? null;

  const currentDeviceStatusQuery = useQuery<FastStatusView, HttpApiError>({
    ...getDeviceStatusQueryOptions({
      deviceId: currentDevice?.id,
      baseUrl: currentDevice?.baseUrl,
      enabled: Boolean(currentDevice?.baseUrl),
      refetchInterval: 3000,
      refetchOnWindowFocus: false,
      retry: 1,
      retryDelay: 250,
    }),
  });

  const currentDevicePresence = getDevicePresenceState(
    t,
    currentDevice,
    currentDeviceStatusQuery.data,
    currentDeviceStatusQuery.isLoading,
    currentDeviceStatusQuery.data ? null : currentDeviceStatusQuery.error,
  );
  const currentDevicePresenceClassName = {
    connected: "bg-emerald-400 shadow-[0_0_14px_rgba(52,211,153,0.55)]",
    connecting:
      "bg-amber-400 shadow-[0_0_14px_rgba(251,191,36,0.55)] motion-safe:animate-pulse",
    error: "bg-red-400 shadow-[0_0_14px_rgba(248,113,113,0.52)]",
    disconnected: "bg-slate-500 shadow-[0_0_10px_rgba(100,116,139,0.35)]",
  }[currentDevicePresence.tone];
  const deviceSwitcherAriaLabel = currentDevice
    ? `${t("shell.deviceSwitcher")}：${currentDevice.name}，${currentDevicePresence.label}`
    : `${t("shell.deviceSwitcher")}：${currentDevicePresence.label}`;

  const [isDeviceSheetOpen, setIsDeviceSheetOpen] = useState(false);
  const lastHrefRef = useRef(href);

  useEffect(() => {
    if (lastHrefRef.current === href) {
      return;
    }
    lastHrefRef.current = href;
    setIsDeviceSheetOpen(false);
  }, [href]);

  const primaryNav = useMemo(
    () => [
      {
        key: "overview",
        label: t("nav.overview"),
        icon: NAV_ICON_DEVICES,
        to: "/devices" as const,
      },
      {
        key: "dashboard",
        label: t("nav.dashboard"),
        icon: NAV_ICON_CC,
        to: currentDevice ? ("/$deviceId/cc" as const) : null,
      },
      {
        key: "system",
        label: t("nav.system"),
        icon: NAV_ICON_SETTINGS,
        to: currentDevice ? ("/$deviceId/settings" as const) : null,
      },
    ],
    [currentDevice, t],
  );

  const appVersion = import.meta.env.VITE_APP_VERSION?.trim() || null;
  const appGitSha = import.meta.env.VITE_APP_GIT_SHA?.trim() || null;
  const appGitTag = import.meta.env.VITE_APP_GIT_TAG?.trim() || null;
  const githubRepo =
    import.meta.env.VITE_GITHUB_REPO?.trim() || "IvanLi-CN/loadlynx";

  const handlePrimaryNavigate = (section: PrimarySection) => {
    if (section === "overview") {
      navigate({ to: "/devices" });
      return;
    }

    if (!currentDevice) {
      navigate({ to: "/devices" });
      return;
    }

    if (section === "dashboard") {
      navigate({ to: "/$deviceId/cc", params: { deviceId: currentDevice.id } });
      return;
    }

    navigate({
      to: "/$deviceId/settings",
      params: { deviceId: currentDevice.id },
    });
  };

  const openDeviceSelector = () => {
    if (isCompactViewport) {
      const returnTo = href || "/devices";
      navigate({ to: "/devices", search: { returnTo } });
      return;
    }
    setIsDeviceSheetOpen(true);
  };

  return (
    <>
      <div className="flex min-h-dvh flex-col">
        <header className="sticky top-0 z-30 border-b border-cyan-400/15 bg-[linear-gradient(180deg,oklch(0.13_0.04_262/.96),oklch(0.11_0.035_262/.9))] backdrop-blur-xl">
          <div className="mx-auto flex w-full max-w-[var(--ll-page-max-workspace)] flex-col gap-4 px-3 py-3 sm:px-4 md:px-6">
            <div className="flex flex-wrap items-start justify-between gap-4 lg:grid lg:grid-cols-[minmax(0,260px)_1fr_minmax(0,360px)] lg:items-center">
              <div className="min-w-0">
                <div className="flex items-center gap-3">
                  <span className="inline-flex h-11 w-11 items-center justify-center rounded-2xl border border-cyan-300/30 bg-cyan-400/10 text-cyan-100 shadow-[0_0_24px_oklch(0.82_0.17_210/.12)]">
                    <MonitorCog size={20} aria-hidden="true" />
                  </span>
                  <div className="min-w-0">
                    <h1 className="truncate text-lg font-bold sm:text-xl">
                      {t("app.title")}
                    </h1>
                    <p className="text-xs text-base-content/60 sm:text-sm">
                      {t("app.subtitle")}
                    </p>
                  </div>
                </div>
              </div>

              <nav
                aria-label={t("nav.primary")}
                className="order-3 overflow-x-auto lg:order-none"
              >
                <div className="flex min-w-max gap-2">
                  {primaryNav.map((item) => {
                    const active = item.key === currentSection;
                    const commonClass = [
                      "ll-button ll-button-sm border",
                      active
                        ? "ll-button-active border-cyan-300/40 bg-cyan-400/12"
                        : "ll-button-ghost border-base-300/60",
                    ].join(" ");

                    if (item.key === "overview") {
                      return (
                        <Link
                          key={item.key}
                          to="/devices"
                          className={commonClass}
                        >
                          <AppIcon icon={item.icon} size={16} />
                          {item.label}
                        </Link>
                      );
                    }

                    if (!currentDevice || item.to == null) {
                      return (
                        <button
                          key={item.key}
                          type="button"
                          className={commonClass}
                          disabled
                        >
                          <AppIcon icon={item.icon} size={16} />
                          {item.label}
                        </button>
                      );
                    }

                    return (
                      <button
                        key={item.key}
                        type="button"
                        className={commonClass}
                        onClick={() =>
                          handlePrimaryNavigate(item.key as PrimarySection)
                        }
                      >
                        <AppIcon icon={item.icon} size={16} />
                        {item.label}
                      </button>
                    );
                  })}
                </div>
              </nav>

              <div className="flex min-w-0 items-center justify-end gap-3 lg:justify-self-end">
                <LanguageSwitcher />
                <button
                  type="button"
                  className="group flex min-w-0 max-w-[320px] items-center gap-2 rounded-2xl border border-cyan-400/22 bg-base-200/35 px-3 py-2 text-left transition hover:border-cyan-300/45 hover:bg-base-200/50"
                  onClick={openDeviceSelector}
                  aria-label={deviceSwitcherAriaLabel}
                  title={currentDevicePresence.label}
                >
                  <span className="flex min-w-0 flex-1 items-center gap-2 overflow-hidden whitespace-nowrap">
                    <span
                      className={`inline-flex h-2.5 w-2.5 shrink-0 rounded-full border border-base-100/75 ${currentDevicePresenceClassName}`}
                      aria-hidden="true"
                    />
                    <span className="sr-only">
                      {currentDevicePresence.label}
                    </span>
                    <span className="truncate text-sm font-semibold">
                      {currentDevice
                        ? currentDevice.name
                        : t("shell.noDeviceSelected")}
                    </span>
                    {currentDeviceMeta ? (
                      <span className="hidden min-w-0 items-center gap-2 overflow-hidden 2xl:flex">
                        <span
                          className="text-base-content/28"
                          aria-hidden="true"
                        >
                          /
                        </span>
                        <span className="truncate font-mono text-[11px] text-base-content/55">
                          {currentDeviceMeta}
                        </span>
                      </span>
                    ) : null}
                  </span>
                  <ChevronRight
                    size={16}
                    className="shrink-0 text-base-content/45 transition group-hover:text-cyan-100"
                    aria-hidden="true"
                  />
                </button>
              </div>
            </div>
          </div>
        </header>

        <main
          className={[
            "mx-auto flex w-full max-w-[var(--ll-page-max-workspace)] flex-1 flex-col px-3 sm:px-4 md:px-6",
            currentSection === "dashboard" ? "py-0" : "py-4 md:py-6",
          ].join(" ")}
        >
          <Outlet />
        </main>

        {!storybookRuntime && appVersion ? (
          <footer className="border-t border-base-300/45 px-3 py-4 sm:px-4 md:px-6">
            <div className="mx-auto flex w-full max-w-[var(--ll-page-max-workspace)] justify-end">
              <AppVersionLink
                version={appVersion}
                repo={githubRepo}
                sha={appGitSha}
                tag={appGitTag}
              />
            </div>
          </footer>
        ) : null}
      </div>

      <DeviceSheet
        open={isDeviceSheetOpen}
        onClose={() => setIsDeviceSheetOpen(false)}
        currentDeviceId={currentDevice?.id}
        currentHref={href}
      />
    </>
  );
}

export default ConsoleLayout;

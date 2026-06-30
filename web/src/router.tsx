import type { QueryClient } from "@tanstack/react-query";
import {
  createBrowserHistory,
  createRootRouteWithContext,
  createRoute,
  createRouter,
  lazyRouteComponent,
  type RouterHistory,
  redirect,
} from "@tanstack/react-router";
import { ConsoleLayout } from "./layouts/console-layout.tsx";
import { DeviceLayout } from "./layouts/device-layout.tsx";
import { RootLayout } from "./layouts/root-layout.tsx";
import { SystemLayout } from "./layouts/system-layout.tsx";

export interface RouterContext {
  queryClient: QueryClient;
}

export type CalibrationSection = "voltage" | "current_ch1" | "current_ch2";

const rootRoute = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
});

const consoleRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "console",
  component: ConsoleLayout,
});

const indexRoute = createRoute({
  getParentRoute: () => consoleRoute,
  path: "/",
  component: lazyRouteComponent(
    () => import("./routes/devices.tsx"),
    "DevicesRoute",
  ),
});

const devicesRoute = createRoute({
  getParentRoute: () => consoleRoute,
  path: "devices",
  component: lazyRouteComponent(
    () => import("./routes/devices.tsx"),
    "DevicesRoute",
  ),
});

const deviceRoute = createRoute({
  getParentRoute: () => consoleRoute,
  path: "$deviceId",
  component: DeviceLayout,
});

const deviceCcRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "cc",
  component: lazyRouteComponent(
    () => import("./routes/device-cc.tsx"),
    "DeviceCcRoute",
  ),
});

const devicePdRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "pd",
  beforeLoad: ({ params }) => {
    throw redirect({
      to: "/$deviceId/cc",
      params: { deviceId: params.deviceId },
      search: { panel: "pd" },
      replace: true,
    });
  },
});

const systemRoute = createRoute({
  getParentRoute: () => deviceRoute,
  id: "system",
  component: SystemLayout,
});

const deviceStatusRoute = createRoute({
  getParentRoute: () => systemRoute,
  path: "status",
  component: lazyRouteComponent(
    () => import("./routes/device-status.tsx"),
    "DeviceStatusRoute",
  ),
});

const deviceSettingsRoute = createRoute({
  getParentRoute: () => systemRoute,
  path: "settings",
  component: lazyRouteComponent(
    () => import("./routes/device-settings.tsx"),
    "DeviceSettingsRoute",
  ),
});

const deviceCalibrationRoute = createRoute({
  getParentRoute: () => systemRoute,
  path: "calibration",
  validateSearch: (
    search: Record<string, unknown>,
  ): { section?: CalibrationSection } => {
    const section = search.section;
    if (
      section === "voltage" ||
      section === "current_ch1" ||
      section === "current_ch2"
    ) {
      return { section };
    }
    return {};
  },
  component: lazyRouteComponent(
    () => import("./routes/device-calibration.tsx"),
    "DeviceCalibrationRoute",
  ),
});

const deviceFirmwareRoute = createRoute({
  getParentRoute: () => systemRoute,
  path: "firmware",
  component: lazyRouteComponent(
    () => import("./routes/device-firmware.tsx"),
    "DeviceFirmwareRoute",
  ),
});

const deviceAboutRoute = createRoute({
  getParentRoute: () => systemRoute,
  path: "about",
  component: lazyRouteComponent(
    () => import("./routes/device-about.tsx"),
    "DeviceAboutRoute",
  ),
});

const systemRouteTree = systemRoute.addChildren([
  deviceStatusRoute,
  deviceSettingsRoute,
  deviceCalibrationRoute,
  deviceFirmwareRoute,
  deviceAboutRoute,
]);

const deviceRouteTree = deviceRoute.addChildren([
  deviceCcRoute,
  devicePdRoute,
  systemRouteTree,
]);

const consoleRouteTree = consoleRoute.addChildren([
  indexRoute,
  devicesRoute,
  deviceRouteTree,
]);

const routeTree = rootRoute.addChildren([consoleRouteTree]);

export function createAppRouter(
  queryClient: QueryClient,
  history?: RouterHistory,
) {
  return createRouter({
    routeTree,
    context: { queryClient },
    history: history ?? createBrowserHistory(),
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}

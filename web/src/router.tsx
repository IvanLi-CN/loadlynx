import type { QueryClient } from "@tanstack/react-query";
import {
  createBrowserHistory,
  createRootRouteWithContext,
  createRoute,
  createRouter,
  lazyRouteComponent,
  type RouterHistory,
} from "@tanstack/react-router";
import { ConsoleLayout } from "./layouts/console-layout.tsx";
import { DeviceLayout } from "./layouts/device-layout.tsx";
import { RootLayout } from "./layouts/root-layout.tsx";

export interface RouterContext {
  queryClient: QueryClient;
}

const rootRoute = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
});

const consoleRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "console",
  component: ConsoleLayout,
});

// Index route: for now just show the devices view.
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

const deviceStatusRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "status",
  component: lazyRouteComponent(
    () => import("./routes/device-status.tsx"),
    "DeviceStatusRoute",
  ),
});

const devicePdRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "pd",
  component: lazyRouteComponent(
    () => import("./routes/device-pd.tsx"),
    "DevicePdRoute",
  ),
});

const deviceSettingsRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "settings",
  component: lazyRouteComponent(
    () => import("./routes/device-settings.tsx"),
    "DeviceSettingsRoute",
  ),
});

const deviceCalibrationRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "calibration",
  component: lazyRouteComponent(
    () => import("./routes/device-calibration.tsx"),
    "DeviceCalibrationRoute",
  ),
});

const deviceFirmwareRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "firmware",
  component: lazyRouteComponent(
    () => import("./routes/device-firmware.tsx"),
    "DeviceFirmwareRoute",
  ),
});

const deviceRouteTree = deviceRoute.addChildren([
  deviceCcRoute,
  deviceStatusRoute,
  devicePdRoute,
  deviceSettingsRoute,
  deviceCalibrationRoute,
  deviceFirmwareRoute,
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
    // This infers the Router instance type across the project.
    router: ReturnType<typeof createAppRouter>;
  }
}

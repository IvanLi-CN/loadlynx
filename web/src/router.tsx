import type { QueryClient } from "@tanstack/react-query";
import {
  createBrowserHistory,
  createRootRouteWithContext,
  createRoute,
  createRouter,
  type RouterHistory,
} from "@tanstack/react-router";
import { ConsoleLayout } from "./layouts/console-layout.tsx";
import { DeviceLayout } from "./layouts/device-layout.tsx";
import { RootLayout } from "./layouts/root-layout.tsx";
import { DeviceCalibrationRoute } from "./routes/device-calibration.tsx";
import { DeviceCcRoute } from "./routes/device-cc.tsx";
import { DeviceSettingsRoute } from "./routes/device-settings.tsx";
import { DeviceStatusRoute } from "./routes/device-status.tsx";
import { DevicesRoute } from "./routes/devices.tsx";

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
  component: DevicesRoute,
});

const devicesRoute = createRoute({
  getParentRoute: () => consoleRoute,
  path: "devices",
  component: DevicesRoute,
});

const deviceRoute = createRoute({
  getParentRoute: () => consoleRoute,
  path: "$deviceId",
  component: DeviceLayout,
});

const deviceCcRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "cc",
  component: DeviceCcRoute,
});

const deviceStatusRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "status",
  component: DeviceStatusRoute,
});

const deviceSettingsRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "settings",
  component: DeviceSettingsRoute,
});

const deviceCalibrationRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "calibration",
  component: DeviceCalibrationRoute,
  staticData: { layout: "tool" },
});

const deviceRouteTree = deviceRoute.addChildren([
  deviceCcRoute,
  deviceStatusRoute,
  deviceSettingsRoute,
  deviceCalibrationRoute,
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

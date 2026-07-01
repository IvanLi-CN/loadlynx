import type { QueryClient } from "@tanstack/react-query";
import {
  createBrowserHistory,
  createRootRouteWithContext,
  createRoute,
  createRouter,
  lazyRouteComponent,
  type RouterHistory,
} from "@tanstack/react-router";
import { RoutePendingView } from "./components/layout/route-pending-view.tsx";
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

function pendingRoute(title: string, description?: string) {
  return {
    pendingComponent: () => (
      <RoutePendingView title={title} description={description} />
    ),
  };
}

// Index route: for now just show the devices view.
const indexRoute = createRoute({
  getParentRoute: () => consoleRoute,
  path: "/",
  ...pendingRoute("正在打开设备列表", "正在准备设备与 devd 状态"),
  component: lazyRouteComponent(
    () => import("./routes/devices.tsx"),
    "DevicesRoute",
  ),
});

const devicesRoute = createRoute({
  getParentRoute: () => consoleRoute,
  path: "devices",
  ...pendingRoute("正在打开设备列表", "正在准备设备与 devd 状态"),
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
  ...pendingRoute("正在打开 CC 控制", "正在加载控制面板"),
  component: lazyRouteComponent(
    () => import("./routes/device-cc.tsx"),
    "DeviceCcRoute",
  ),
});

const deviceStatusRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "status",
  ...pendingRoute("正在打开状态页", "正在加载实时状态面板"),
  component: lazyRouteComponent(
    () => import("./routes/device-status.tsx"),
    "DeviceStatusRoute",
  ),
});

const devicePdRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "pd",
  ...pendingRoute("正在打开 USB-PD", "正在加载 USB-PD 面板"),
  component: lazyRouteComponent(
    () => import("./routes/device-pd.tsx"),
    "DevicePdRoute",
  ),
});

const deviceSettingsRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "settings",
  ...pendingRoute("正在打开设置", "正在加载设备设置"),
  component: lazyRouteComponent(
    () => import("./routes/device-settings.tsx"),
    "DeviceSettingsRoute",
  ),
});

const deviceCalibrationRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "calibration",
  ...pendingRoute("正在打开校准", "正在加载校准工作区"),
  component: lazyRouteComponent(
    () => import("./routes/device-calibration.tsx"),
    "DeviceCalibrationRoute",
  ),
});

const deviceFirmwareRoute = createRoute({
  getParentRoute: () => deviceRoute,
  path: "firmware",
  ...pendingRoute("正在打开 Firmware", "正在加载固件操作面板"),
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
    defaultPreload: "intent",
    defaultPendingComponent: () => <RoutePendingView />,
    defaultPendingMs: 0,
    defaultPendingMinMs: 180,
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    // This infers the Router instance type across the project.
    router: ReturnType<typeof createAppRouter>;
  }
}

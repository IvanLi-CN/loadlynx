import type { QueryClient } from "@tanstack/react-query";
import {
  createRootRouteWithContext,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { AppLayout } from "./routes/app-layout.tsx";
import { DeviceCcRoute } from "./routes/device-cc.tsx";
import { DeviceSettingsRoute } from "./routes/device-settings.tsx";
import { DeviceStatusRoute } from "./routes/device-status.tsx";
import { DevicesRoute } from "./routes/devices.tsx";

export interface RouterContext {
  queryClient: QueryClient;
}

const rootRoute = createRootRouteWithContext<RouterContext>()({
  component: AppLayout,
});

// Index route: for now just show the devices view.
const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: DevicesRoute,
});

const devicesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "devices",
  component: DevicesRoute,
});

// Baseline pattern: /:deviceId/:functionPath*
// For now we materialize a few concrete children under it.
const deviceCcRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "$deviceId/cc",
  component: DeviceCcRoute,
});

const deviceStatusRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "$deviceId/status",
  component: DeviceStatusRoute,
});

const deviceSettingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "$deviceId/settings",
  component: DeviceSettingsRoute,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  devicesRoute,
  deviceCcRoute,
  deviceStatusRoute,
  deviceSettingsRoute,
]);

export function createAppRouter(queryClient: QueryClient) {
  return createRouter({
    routeTree,
    context: { queryClient },
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    // This infers the Router instance type across the project.
    router: ReturnType<typeof createAppRouter>;
  }
}

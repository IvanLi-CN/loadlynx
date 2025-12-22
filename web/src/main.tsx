import "./index.css";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { LocalStorageDeviceStore } from "./devices/device-store.ts";
import { DeviceStoreProvider } from "./devices/store-context.tsx";
import { createAppRouter } from "./router.tsx";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error('Root element with id "root" not found');
}

const queryClient = new QueryClient();
const router = createAppRouter(queryClient);
const deviceStore = new LocalStorageDeviceStore(window.localStorage);

createRoot(rootElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <DeviceStoreProvider store={deviceStore}>
        <RouterProvider router={router} />
      </DeviceStoreProvider>
    </QueryClientProvider>
  </StrictMode>,
);

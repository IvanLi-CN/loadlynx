import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { useState } from "react";
import type { StoredDevice } from "../../devices/device-store.ts";
import { MemoryDeviceStore } from "../../devices/device-store.ts";
import { DeviceStoreProvider } from "../../devices/store-context.tsx";
import { createAppRouter } from "../../router.tsx";
import { MemoryCalibrationStore } from "../../routes/device-calibration/store.ts";
import { CalibrationStoreProvider } from "../../routes/device-calibration/store-context.tsx";

export const DEFAULT_MOCK_DEVICES: StoredDevice[] = [
  {
    id: "mock-001",
    name: "Demo Device #1",
    baseUrl: "mock://demo-1",
  },
  {
    id: "mock-002",
    name: "Demo Device #2",
    baseUrl: "mock://demo-2",
  },
];

export function RouteStoryHarness(props: {
  initialPath: string;
  devices?: StoredDevice[];
  beforeMount?: (stores: {
    deviceStore: MemoryDeviceStore;
    calibrationStore: MemoryCalibrationStore;
  }) => void;
}) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            retry: false,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
          },
          mutations: { retry: false },
        },
      }),
  );

  const [deviceStore] = useState(
    () => new MemoryDeviceStore(props.devices ?? DEFAULT_MOCK_DEVICES),
  );
  const [calibrationStore] = useState(() => new MemoryCalibrationStore());

  useState(() => {
    props.beforeMount?.({ deviceStore, calibrationStore });
    return null;
  });

  const [router] = useState(() => {
    const history = createMemoryHistory({
      initialEntries: [props.initialPath],
    });
    return createAppRouter(queryClient, history);
  });

  return (
    <QueryClientProvider client={queryClient}>
      <DeviceStoreProvider store={deviceStore}>
        <CalibrationStoreProvider store={calibrationStore}>
          <RouterProvider router={router} />
        </CalibrationStoreProvider>
      </DeviceStoreProvider>
    </QueryClientProvider>
  );
}

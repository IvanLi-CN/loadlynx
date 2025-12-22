import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createMemoryHistory } from "@tanstack/react-router";
import { useState } from "react";
import type { StoredDevice } from "../../devices/device-store.ts";
import { MemoryDeviceStore } from "../../devices/device-store.ts";
import { DeviceStoreProvider } from "../../devices/store-context.tsx";
import { createAppRouter } from "../../router.tsx";

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

  const [router] = useState(() => {
    const history = createMemoryHistory({ initialEntries: [props.initialPath] });
    return createAppRouter(queryClient, history);
  });

  return (
    <QueryClientProvider client={queryClient}>
      <DeviceStoreProvider store={deviceStore}>
        <RouterProvider router={router} />
      </DeviceStoreProvider>
    </QueryClientProvider>
  );
}

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";
import { DevicesPanel } from "./devices-panel.tsx";
import { MemoryDeviceStore } from "./device-store.ts";
import { DeviceStoreProvider } from "./store-context.tsx";

function StoryRoot() {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { retry: false },
          mutations: { retry: false },
        },
      }),
  );

  const [deviceStore] = useState(
    () =>
      new MemoryDeviceStore([
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
      ]),
  );

  return (
    <QueryClientProvider client={queryClient}>
      <DeviceStoreProvider store={deviceStore}>
        <DevicesPanel />
      </DeviceStoreProvider>
    </QueryClientProvider>
  );
}

const meta = {
  title: "Devices/Panel (No side effects)",
  component: StoryRoot,
} satisfies Meta<typeof StoryRoot>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};


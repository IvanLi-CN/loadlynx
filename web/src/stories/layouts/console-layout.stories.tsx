import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "storybook/test";
import type { StoredDevice } from "../../devices/device-store.ts";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function ConsoleLayoutStory(props: { initialPath: string }) {
  return <RouteStoryHarness initialPath={props.initialPath} />;
}

const meta = {
  title: "Layouts/ConsoleLayout",
  component: ConsoleLayoutStory,
  args: {
    initialPath: "/mock-001/cc",
  },
} satisfies Meta<typeof ConsoleLayoutStory>;

export default meta;

type Story = StoryObj<typeof meta>;

const MULTI_TRANSPORT_DEVICES: StoredDevice[] = [
  {
    id: "device-001",
    name: "LoadLynx d68638",
    baseUrl: "http://192.168.31.216",
    identityDeviceId: "loadlynx-d68638",
    connectionMarks: ["lan", "usb"],
    lan: {
      baseUrl: "http://192.168.31.216",
    },
    devd: {
      baseUrl: "http://127.0.0.1:30180",
      deviceId: "digital-2bdf",
      leaseId: "lease-1",
    },
  },
];

const USB_ACTIVE_WITH_STALE_LAN_DEVICE: StoredDevice[] = [
  {
    id: "device-002",
    name: "ESP32-S3 USB CDC (/dev/cu.usbmodem212101)",
    baseUrl:
      "http://127.0.0.1:19390/?device_id=loadlynx-d68638&lease_id=lease-1",
    identityDeviceId: "loadlynx-d68638",
    connectionMarks: ["lan", "usb"],
    lan: {
      baseUrl: "http://192.168.31.216",
    },
    devd: {
      baseUrl: "http://127.0.0.1:19390",
      deviceId: "loadlynx-d68638",
      leaseId: "lease-1",
    },
  },
];

export const Large: Story = {
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(
      () => {
        canvas.getByText("LoadLynx Web Console");
        canvas.getByRole("navigation", { name: "主导航" });
        canvas.getByRole("link", { name: "总览" });
        canvas.getByRole("button", { name: "仪表盘" });
        canvas.getByRole("button", { name: "系统" });
      },
      { timeout: 5_000 },
    );

    const asides = canvasElement.querySelectorAll("aside");
    if (asides.length > 1) {
      throw new Error(
        "Expected the header shell to render without legacy side navigation",
      );
    }
  },
};

export const Medium: Story = {
  globals: {
    viewport: { value: "loadlynxMedium", isRotated: false },
  },
  play: async ({ canvas }) => {
    await waitFor(
      () => {
        canvas.getByText("LoadLynx Web Console");
        canvas.getByRole("navigation", { name: "主导航" });
        canvas.getByRole("button", { name: /当前设备：/ });
      },
      { timeout: 5_000 },
    );
  },
};

export const SmallSelectionFlow: Story = {
  globals: {
    viewport: { value: "loadlynxSmall", isRotated: false },
  },
  play: async ({ canvas, userEvent }) => {
    await canvas.findByText("LoadLynx Web Console");
    const deviceButton = await canvas.findByRole("button", {
      name: /当前设备：/,
    });
    await userEvent.click(deviceButton);

    await waitFor(() => {
      canvas.getByRole("heading", { name: "选择设备" });
    });
    await waitFor(() => {
      if (canvas.getAllByRole("link", { name: "使用此设备" }).length < 2) {
        throw new Error("Expected mobile selection CTAs for overview devices");
      }
    });
  },
};

export const NoDeviceSelected: Story = {
  args: {
    initialPath: "/devices",
  },
};

export const OverviewKeepsLastActiveDevice: Story = {
  render: () => (
    <RouteStoryHarness initialPath="/devices" lastActiveDeviceId="mock-001" />
  ),
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  play: async ({ canvas }) => {
    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "总览" });
        const deviceButton = canvas.getByRole("button", {
          name: /当前设备：/,
        });
        if (!deviceButton.textContent?.includes("Demo Device #1")) {
          throw new Error(
            "Expected header device switcher to keep Demo Device #1",
          );
        }
        if (!deviceButton.textContent?.includes("mock-001")) {
          throw new Error("Expected header device switcher to keep mock-001");
        }
      },
      { timeout: 5_000 },
    );
  },
};

export const ConnectionSwitcherSheet: Story = {
  render: () => (
    <RouteStoryHarness
      initialPath="/device-001/settings"
      devices={MULTI_TRANSPORT_DEVICES}
    />
  ),
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  play: async ({ canvas, userEvent }) => {
    const deviceButton = await canvas.findByRole("button", {
      name: /当前设备：/,
    });
    await userEvent.click(deviceButton);

    const dialog = await canvas.findByRole("dialog", { name: "当前设备" });
    const drawer = within(dialog);
    await waitFor(() => {
      drawer.getByRole("heading", { name: "选择设备" });
      drawer.getByRole("group", { name: "连接方式" });
      drawer.getByRole("button", { name: "WiFi" });
      drawer.getByRole("button", { name: "USB" });
      drawer.getByRole("button", { name: "Serial" });
    });
  },
};

export const ConnectionSwitcherRejectsStaleWifi: Story = {
  render: () => (
    <RouteStoryHarness
      initialPath="/device-002/settings"
      devices={USB_ACTIVE_WITH_STALE_LAN_DEVICE}
    />
  ),
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  play: async ({ canvas, userEvent }) => {
    const deviceButton = await canvas.findByRole("button", {
      name: /当前设备：/,
    });
    await userEvent.click(deviceButton);

    const dialog = await canvas.findByRole("dialog", { name: "当前设备" });
    const drawer = within(dialog);
    const wifiButton = await drawer.findByRole("button", { name: "WiFi" });
    await userEvent.click(wifiButton);

    await waitFor(
      () => {
        drawer.getByRole("heading", { name: "选择设备" });
        drawer.getByText("无法切换到 WiFi");
        drawer.getByText(/这个 WiFi 通道当前不可达/);
      },
      { timeout: 5_000 },
    );

    await waitFor(() => {
      const currentPath = drawer.getByText(/当前：USB/);
      if (!currentPath) {
        throw new Error("Expected current management path to remain USB");
      }
    });
  },
};

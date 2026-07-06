import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "storybook/test";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

const DEVD_DEVICE = {
  id: "devd-bound",
  name: "Mock LoadLynx devd device",
  baseUrl: "mock://devd-lan",
  connectionMarks: ["usb", "lan"] as Array<"usb" | "lan">,
  devd: {
    baseUrl: "http://127.0.0.1:30180",
    deviceId: "mock-loadlynx-devd",
    leaseId: "mock-lease-1",
  },
};

function FirmwareRouteStory() {
  return (
    <RouteStoryHarness
      initialPath="/devd-bound/firmware"
      devices={[DEVD_DEVICE]}
    />
  );
}

function FirmwareRouteNoLeaseStory() {
  return (
    <RouteStoryHarness
      initialPath="/devd-bound/firmware"
      devices={[
        {
          ...DEVD_DEVICE,
          devd: {
            baseUrl: "http://127.0.0.1:30180",
            deviceId: "mock-loadlynx-devd",
          },
        },
      ]}
    />
  );
}

const meta = {
  title: "Routes/Firmware",
  component: FirmwareRouteStory,
  parameters: {
    viewport: { defaultViewport: "loadlynxLarge" },
  },
} satisfies Meta<typeof FirmwareRouteStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DevdBound: Story = {};

export const DryRunEvidence: Story = {
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "Firmware" });
      },
      { timeout: 5000 },
    );
    await userEvent.type(
      canvas.getByPlaceholderText("Select or type a staged artifact id"),
      "digital-release-aabbcc",
    );
    await userEvent.type(
      canvas.getByPlaceholderText("/path/to/firmware-catalog.json"),
      "/tmp/loadlynx-firmware-catalog.json",
    );
    await userEvent.click(
      canvas.getByRole("button", { name: "Check firmware" }),
    );

    await waitFor(() => {
      canvas.getByText(/mock-loadlynx-devd/);
      canvas.getByText(/"device_id": "mock-loadlynx-devd"/, {
        selector: "code",
      });
      canvas.getByText(/"target": "mock"/, { selector: "code" });
    });
  },
};

export const MissingLease: Story = {
  render: () => <FirmwareRouteNoLeaseStory />,
};

export const WebSerialGate: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "Firmware" });
        canvas.getByText("Web Serial flash");
        canvas.getByPlaceholderText("yes");
        canvas.getByText("Acknowledge non-project firmware risk");
      },
      { timeout: 5000 },
    );
  },
};

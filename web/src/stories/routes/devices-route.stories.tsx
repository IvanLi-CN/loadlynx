import type { Meta, StoryObj } from "@storybook/react";
import { waitFor } from "storybook/test";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function DevicesRouteStory() {
  return <RouteStoryHarness initialPath="/devices" />;
}

const meta = {
  title: "Routes/Devices",
  component: DevicesRouteStory,
} satisfies Meta<typeof DevicesRouteStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const DevdDiscovery: Story = {
  parameters: {
    viewport: { defaultViewport: "loadlynxLarge" },
  },
  play: async ({ canvas, userEvent }) => {
    const scanDevdButton = await canvas.findByRole(
      "button",
      { name: "Scan devd" },
      { timeout: 5_000 },
    );
    await userEvent.click(scanDevdButton);

    await waitFor(
      () => {
        canvas.getByText("Mock LoadLynx devd device");
        canvas.getByRole("button", { name: "Create USB lease" });
      },
      { timeout: 5_000 },
    );
  },
};

export const DevdLeaseCreated: Story = {
  parameters: {
    viewport: { defaultViewport: "loadlynxLarge" },
  },
  play: async ({ canvas, userEvent }) => {
    const scanDevdButton = await canvas.findByRole(
      "button",
      { name: "Scan devd" },
      { timeout: 5_000 },
    );
    await userEvent.click(scanDevdButton);
    await waitFor(
      () => {
        canvas.getByRole("button", { name: "Create USB lease" });
      },
      { timeout: 5_000 },
    );
    await userEvent.click(
      canvas.getByRole("button", { name: "Create USB lease" }),
    );

    await waitFor(
      () => {
        if (canvas.getAllByText("Mock LoadLynx devd device").length < 2) {
          throw new Error("Expected devd candidate and registry row");
        }
        if (canvas.getAllByRole("link", { name: "Firmware" }).length < 3) {
          throw new Error("Expected firmware links for registered devices");
        }
      },
      { timeout: 5_000 },
    );
  },
};

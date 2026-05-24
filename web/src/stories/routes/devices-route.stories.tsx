import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
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
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Scan devd" }));

    await waitFor(() => {
      canvas.getByText("Mock LoadLynx devd device");
      canvas.getByRole("button", { name: "Create USB lease" });
    });
  },
};

export const DevdLeaseCreated: Story = {
  parameters: {
    viewport: { defaultViewport: "loadlynxLarge" },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Scan devd" }));
    await waitFor(() => {
      canvas.getByRole("button", { name: "Create USB lease" });
    });
    await userEvent.click(
      canvas.getByRole("button", { name: "Create USB lease" }),
    );

    await waitFor(() => {
      if (canvas.getAllByText("Mock LoadLynx devd device").length < 2) {
        throw new Error("Expected devd candidate and registry row");
      }
      if (canvas.getAllByRole("link", { name: "Firmware" }).length < 3) {
        throw new Error("Expected firmware links for registered devices");
      }
    });
  },
};

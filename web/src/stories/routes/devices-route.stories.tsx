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

export const Default: Story = {
  play: async ({ canvas }) => {
    await waitFor(
      () => {
        canvas.getByText("LoadLynx Web Console");
        canvas.getByRole("heading", { name: "总览" });
      },
      { timeout: 5_000 },
    );
    await canvas.findByText("Demo Device #1");
    await waitFor(() => {
      if (canvas.getAllByRole("link", { name: "打开仪表盘" }).length < 2) {
        throw new Error("Expected dashboard entry links for overview cards");
      }
      if (canvas.getAllByRole("link", { name: "打开系统" }).length < 2) {
        throw new Error("Expected system entry links for overview cards");
      }
    });
  },
};

export const UltraWideDesktop: Story = {
  parameters: {
    viewport: { defaultViewport: "loadlynxDesktopUltra" },
  },
  play: async ({ canvas, canvasElement }) => {
    await canvas.findByRole("heading", { name: "总览" });
    await canvas.findByText("Demo Device #1");

    const pageContainer = canvasElement.querySelector(
      "[data-ll-page-container='workspace']",
    );
    if (!(pageContainer instanceof HTMLElement)) {
      throw new Error("Expected workspace page container on overview");
    }

    const viewportWidth = canvasElement.getBoundingClientRect().width;
    const containerRect = pageContainer.getBoundingClientRect();
    const leftGap = containerRect.left;
    const rightGap = viewportWidth - containerRect.right;

    if (Math.abs(leftGap - rightGap) > 4) {
      throw new Error(
        `Expected centered workspace container, got left=${leftGap} right=${rightGap}`,
      );
    }
  },
};

export const MobileSelectionMode: Story = {
  render: () => (
    <RouteStoryHarness initialPath="/devices?returnTo=%2Fmock-001%2Fcc%3Fpanel%3Dpd" />
  ),
  play: async ({ canvas }) => {
    await canvas.findByRole("heading", { name: "选择设备" });
    await waitFor(() => {
      if (canvas.getAllByRole("link", { name: "使用此设备" }).length < 2) {
        throw new Error("Expected selection CTAs for each overview device");
      }
    });
  },
};

export const DevdDiscovery: Story = {
  parameters: {
    viewport: { defaultViewport: "loadlynxLarge" },
  },
  play: async ({ canvas, userEvent }) => {
    const scanDevdButton = await canvas.findByRole(
      "button",
      { name: "Refresh" },
      { timeout: 5_000 },
    );
    await userEvent.click(scanDevdButton);

    await waitFor(
      () => {
        canvas.getByText("Mock LoadLynx devd device");
        canvas.getByRole("button", { name: "Add from devd" });
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
      { name: "Refresh" },
      { timeout: 5_000 },
    );
    await userEvent.click(scanDevdButton);
    await waitFor(
      () => {
        canvas.getByRole("button", { name: "Add from devd" });
      },
      { timeout: 5_000 },
    );
    await userEvent.click(
      canvas.getByRole("button", { name: "Add from devd" }),
    );

    await waitFor(
      () => {
        if (canvas.getAllByText("Mock LoadLynx devd device").length < 2) {
          throw new Error("Expected devd candidate and registry row");
        }
        if (canvas.getAllByRole("link", { name: "打开系统" }).length < 2) {
          throw new Error(
            "Expected overview system links for registered devices",
          );
        }
      },
      { timeout: 5_000 },
    );
  },
};

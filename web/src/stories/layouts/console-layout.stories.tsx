import type { Meta, StoryObj } from "@storybook/react";
import { within } from "@testing-library/dom";
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

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const asides = canvasElement.querySelectorAll("aside");
    if (asides.length !== 1) {
      throw new Error(
        "Expected sidebar <aside> to be visible in default layout",
      );
    }
    await canvas.findByRole("link", { name: "Status" });
  },
};

export const ToolMode: Story = {
  args: {
    initialPath: "/mock-001/calibration",
  },
  play: async ({ canvasElement }) => {
    await within(canvasElement).findByRole("heading", { name: "Calibration" });
    const asides = canvasElement.querySelectorAll("aside");
    if (asides.length !== 0) {
      throw new Error("Expected sidebar <aside> to be hidden in tool layout");
    }
  },
};

export const NoDeviceSelected: Story = {
  args: {
    initialPath: "/devices",
  },
};

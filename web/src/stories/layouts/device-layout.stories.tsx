import type { Meta, StoryObj } from "@storybook/react";
import { within } from "@testing-library/dom";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function DeviceLayoutStory(props: { initialPath: string }) {
  return <RouteStoryHarness initialPath={props.initialPath} />;
}

const meta = {
  title: "Layouts/DeviceLayout",
  component: DeviceLayoutStory,
} satisfies Meta<typeof DeviceLayoutStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const ResolvedDevice: Story = {
  args: {
    initialPath: "/mock-001/status",
  },
};

export const NotFound: Story = {
  args: {
    initialPath: "/unknown/status",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByRole("heading", { name: "Device not found" });
    await canvas.findByRole("link", { name: "Back to devices" });
  },
};

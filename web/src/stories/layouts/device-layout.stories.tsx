import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "storybook/test";
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
    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "Device not found" });
        canvas.getByRole("link", { name: "Back to Overview" });
      },
      { timeout: 5_000 },
    );
  },
};

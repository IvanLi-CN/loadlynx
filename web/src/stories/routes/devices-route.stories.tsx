import type { Meta, StoryObj } from "@storybook/react";
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


import type { Meta, StoryObj } from "@storybook/react";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function StatusRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/status" />;
}

const meta = {
  title: "Routes/Status",
  component: StatusRouteStory,
} satisfies Meta<typeof StatusRouteStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

import type { Meta, StoryObj } from "@storybook/react";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function AboutRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/about" />;
}

const meta = {
  title: "Routes/About",
  component: AboutRouteStory,
} satisfies Meta<typeof AboutRouteStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

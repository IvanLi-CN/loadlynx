import type { Meta, StoryObj } from "@storybook/react";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function PdRouteDefaultStory() {
  return <RouteStoryHarness initialPath="/mock-001/pd" />;
}

function PdRouteUnsupportedStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/pd"
      devices={[
        {
          id: "mock-001",
          name: "PD Unsupported",
          baseUrl: "mock://demo-no-pd",
        },
      ]}
    />
  );
}

function PdRouteLinkDownStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/pd"
      devices={[
        {
          id: "mock-001",
          name: "PD Link Down",
          baseUrl: "mock://demo-link-down",
        },
      ]}
    />
  );
}

const meta = {
  title: "Routes/USB‑PD",
  component: PdRouteDefaultStory,
} satisfies Meta<typeof PdRouteDefaultStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Unsupported: Story = {
  render: () => <PdRouteUnsupportedStory />,
};

export const LinkDown: Story = {
  render: () => <PdRouteLinkDownStory />,
};

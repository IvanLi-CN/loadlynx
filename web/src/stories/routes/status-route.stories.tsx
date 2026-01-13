import type { Meta, StoryObj } from "@storybook/react";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function StatusRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/status" />;
}

function StatusRoutePdUnsupportedStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/status"
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

function StatusRoutePdLinkDownStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/status"
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

function StatusRoutePdDetachedStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/status"
      devices={[
        {
          id: "mock-001",
          name: "PD Detached",
          baseUrl: "mock://demo-not-attached",
        },
      ]}
    />
  );
}

const meta = {
  title: "Routes/Status",
  component: StatusRouteStory,
} satisfies Meta<typeof StatusRouteStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const PdUnsupported: Story = {
  render: () => <StatusRoutePdUnsupportedStory />,
};

export const PdLinkDown: Story = {
  render: () => <StatusRoutePdLinkDownStory />,
};

export const PdDetached: Story = {
  render: () => <StatusRoutePdDetachedStory />,
};

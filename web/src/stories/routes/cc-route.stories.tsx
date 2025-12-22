import type { Meta, StoryObj } from "@storybook/react";
import { expect, userEvent, within } from "@storybook/test";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function CcRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/cc" />;
}

const meta = {
  title: "Routes/CC",
  component: CcRouteStory,
} satisfies Meta<typeof CcRouteStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await canvas.findByRole("heading", { name: /Device control/i });
    await canvas.findByText(/1\.50\s*A\s*·\s*1500\s*mA/);

    const enableToggle = await canvas.findByRole("checkbox", {
      name: /Enable output/i,
    });

    await expect(enableToggle).not.toBeChecked();
    await userEvent.click(enableToggle);
    await expect(enableToggle).toBeChecked();
  },
};

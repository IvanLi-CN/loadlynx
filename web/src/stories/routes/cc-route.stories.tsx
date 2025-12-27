import type { Meta, StoryObj } from "@storybook/react";
import { within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
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
    await canvas.findByRole("heading", { name: /Presets/i });
    await canvas.findByText(/1500\s*mA/);

    const outputToggle = await canvas.findByRole("checkbox", {
      name: /Output enabled/i,
    });

    if ((outputToggle as HTMLInputElement).checked) {
      throw new Error("Expected Output enabled to start unchecked");
    }
    await userEvent.click(outputToggle);
    if (!(outputToggle as HTMLInputElement).checked) {
      throw new Error("Expected Output enabled to be checked after click");
    }
  },
};

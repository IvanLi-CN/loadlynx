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
    await canvas.findByText(/1\.50\s*A\s*·\s*1500\s*mA/);

    const enableToggle = await canvas.findByRole("checkbox", {
      name: /Load switch/i,
    });

    if ((enableToggle as HTMLInputElement).checked) {
      throw new Error("Expected Load switch to start unchecked");
    }
    await userEvent.click(enableToggle);
    if (!(enableToggle as HTMLInputElement).checked) {
      throw new Error("Expected Load switch to be checked after click");
    }
  },
};

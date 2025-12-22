import type { Meta, StoryObj } from "@storybook/react";
import { expect, userEvent, within } from "@storybook/test";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function CalibrationRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/calibration" />;
}

const meta = {
  title: "Routes/Calibration",
  component: CalibrationRouteStory,
} satisfies Meta<typeof CalibrationRouteStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await canvas.findByRole("heading", { name: "Calibration" });

    const currentCh1Tab = canvas.getByRole("tab", { name: "电流通道1" });
    await userEvent.click(currentCh1Tab);

    await canvas.findByText("电流单位");
    await expect(currentCh1Tab).toHaveClass("tab-active");
  },
};


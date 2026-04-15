import type { Meta, StoryObj } from "@storybook/react";
import { within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
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
    if (!(currentCh1Tab as HTMLElement).classList.contains("tab-active")) {
      throw new Error('Expected "电流通道1" tab to be active after click');
    }
  },
};

export const OutputControlApplied: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await canvas.findByRole("heading", { name: "Calibration" });

    await userEvent.click(canvas.getByRole("tab", { name: "电流通道1" }));
    await canvas.findByText("Output control (CC)");

    await userEvent.click(canvas.getByRole("button", { name: "2A" }));
    await userEvent.click(canvas.getByRole("button", { name: "Set Output" }));

    await canvas.findByText("1.7100 A");
    await canvas.findByText("1638");
  },
};

import type { Meta, StoryObj } from "@storybook/react";
import { expect, userEvent, waitFor, within } from "@storybook/test";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function SettingsRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/settings" />;
}

const meta = {
  title: "Routes/Settings",
  component: SettingsRouteStory,
} satisfies Meta<typeof SettingsRouteStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const SoftResetDialog: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(() => {
      expect(canvas.getByRole("button", { name: "Soft Reset" })).toBeTruthy();
    });

    await userEvent.click(canvas.getByRole("button", { name: "Soft Reset" }));

    await waitFor(() => {
      expect(canvas.getByRole("dialog")).toBeTruthy();
    });

    const dialog = within(canvas.getByRole("dialog"));
    await userEvent.click(dialog.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(canvas.queryByRole("dialog")).toBeNull();
    });
  },
};

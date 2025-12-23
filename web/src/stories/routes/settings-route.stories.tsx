import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
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
      canvas.getByRole("button", { name: "Soft Reset" });
    });

    await userEvent.click(canvas.getByRole("button", { name: "Soft Reset" }));

    await waitFor(() => {
      canvas.getByRole("dialog");
    });

    const dialog = within(canvas.getByRole("dialog"));
    await userEvent.click(dialog.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      if (canvas.queryByRole("dialog")) {
        throw new Error("Expected dialog to close after clicking Cancel");
      }
    });
  },
};

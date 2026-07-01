import type { Meta, StoryObj } from "@storybook/react";
import { expect, fn, within } from "storybook/test";
import { PwaUpdatePromptView } from "./pwa-update-prompt-view.tsx";

const meta = {
  title: "PWA/UpdatePrompt",
  component: PwaUpdatePromptView,
  args: {
    onClose: fn(),
    onUpdate: fn(),
  },
  decorators: [
    (Story) => (
      <div className="relative min-h-[20rem] overflow-hidden bg-base-100 p-6">
        <div className="ll-panel mx-auto max-w-3xl">
          <div className="ll-panel-body">
            <h2 className="ll-panel-title">LoadLynx Console Surface</h2>
            <p className="max-w-[58ch] text-sm text-base-content/75">
              PWA status appears above the working console without blocking
              readback, navigation or safety actions.
            </p>
            <div className="grid gap-3 sm:grid-cols-3">
              <button type="button" className="ll-button ll-button-primary">
                Enable output
              </button>
              <button type="button" className="ll-button ll-button-outline">
                Read status
              </button>
              <button type="button" className="ll-button ll-button-danger">
                Soft reset
              </button>
            </div>
          </div>
        </div>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof PwaUpdatePromptView>;

export default meta;

type Story = StoryObj<typeof meta>;

export const UpdateReady: Story = {
  args: {
    state: "update-ready",
  },
  play: async ({ args, canvasElement, userEvent }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("status")).toBeVisible();
    const upgrade = canvas.getByRole("button", { name: "升级" });
    await userEvent.click(upgrade);
    await expect(args.onUpdate).toHaveBeenCalled();
  },
};

export const OfflineReady: Story = {
  args: {
    state: "offline-ready",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("status")).toBeVisible();
    await expect(canvas.getByText("离线应用壳已就绪")).toBeVisible();
  },
};

export const RegistrationError: Story = {
  args: {
    state: "registration-error",
    errorMessage: "Service worker registration is blocked in this browser.",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("alert")).toBeVisible();
    await expect(canvas.getByText("离线缓存不可用")).toBeVisible();
  },
};

export const Hidden: Story = {
  args: {
    state: "hidden",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("status")).toBeNull();
    await expect(canvas.queryByRole("alert")).toBeNull();
  },
};

import type { Meta, StoryObj } from "@storybook/react";
import { expect, fn, userEvent, waitFor, within } from "@storybook/test";
import { useState } from "react";
import { ConfirmDialog, type ConfirmDialogProps } from "./confirm-dialog.tsx";

function ConfirmDialogHarness(args: ConfirmDialogProps) {
  const [open, setOpen] = useState(false);

  return (
    <div className="p-6">
      <button
        type="button"
        className="btn btn-primary"
        onClick={() => setOpen(true)}
      >
        Open
      </button>

      <ConfirmDialog
        {...args}
        open={open}
        onCancel={() => {
          args.onCancel();
          setOpen(false);
        }}
        onConfirm={() => {
          args.onConfirm();
          setOpen(false);
        }}
      />
    </div>
  );
}

const meta = {
  title: "Common/ConfirmDialog",
  component: ConfirmDialog,
  render: (args) => <ConfirmDialogHarness {...args} />,
  args: {
    open: false,
    title: "Confirm action",
    body: "Are you sure you want to continue?",
    details: ["This cannot be undone."],
    confirmLabel: "Confirm",
    destructive: false,
    confirmDisabled: false,
    onConfirm: () => {},
    onCancel: () => {},
  },
} satisfies Meta<typeof ConfirmDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const CancelCloses: Story = {
  args: {
    onConfirm: fn(),
    onCancel: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    expect(canvas.getByRole("dialog")).toBeTruthy();

    await userEvent.click(canvas.getByRole("button", { name: "Cancel" }));
    expect(args.onCancel).toHaveBeenCalledTimes(1);
    await waitFor(() => {
      expect(canvas.queryByRole("dialog")).toBeNull();
    });
  },
};

export const ConfirmCallsHandler: Story = {
  args: {
    onConfirm: fn(),
    onCancel: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    expect(canvas.getByRole("dialog")).toBeTruthy();

    await userEvent.click(canvas.getByRole("button", { name: "Confirm" }));
    expect(args.onConfirm).toHaveBeenCalledTimes(1);
    await waitFor(() => {
      expect(canvas.queryByRole("dialog")).toBeNull();
    });
  },
};

export const ConfirmDisabled: Story = {
  args: {
    confirmDisabled: true,
    onConfirm: fn(),
    onCancel: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    const confirmButton = canvas.getByRole("button", { name: "Confirm" });
    expect((confirmButton as HTMLButtonElement).disabled).toBe(true);

    expect(args.onConfirm).toHaveBeenCalledTimes(0);
    expect(canvas.getByRole("dialog")).toBeTruthy();
  },
};

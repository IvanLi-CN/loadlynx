import type { Meta, StoryObj } from "@storybook/react";
import { expect, fn, userEvent, waitFor, within } from "@storybook/test";
import { useState } from "react";
import { AlertDialog, type AlertDialogProps } from "./alert-dialog.tsx";

function AlertDialogHarness(args: AlertDialogProps) {
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

      <AlertDialog
        {...args}
        open={open}
        onClose={() => {
          args.onClose();
          setOpen(false);
        }}
      />
    </div>
  );
}

const meta = {
  title: "Common/AlertDialog",
  component: AlertDialog,
  render: (args) => <AlertDialogHarness {...args} />,
  args: {
    open: false,
    title: "Something happened",
    body: "This is an alert dialog.",
    details: ["Details go here."],
    onClose: () => {},
  },
} satisfies Meta<typeof AlertDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const CloseCloses: Story = {
  args: {
    onClose: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    expect(canvas.getByRole("dialog")).toBeTruthy();

    await userEvent.click(canvas.getByRole("button", { name: "Close" }));
    expect(args.onClose).toHaveBeenCalledTimes(1);
    await waitFor(() => {
      expect(canvas.queryByRole("dialog")).toBeNull();
    });
  },
};

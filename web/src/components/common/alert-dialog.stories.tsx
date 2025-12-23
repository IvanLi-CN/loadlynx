import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { AlertDialog, type AlertDialogProps } from "./alert-dialog.tsx";

let closeCalls = 0;

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
    onClose: () => {
      closeCalls += 1;
    },
  },
} satisfies Meta<typeof AlertDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const CloseCloses: Story = {
  play: async ({ canvasElement }) => {
    closeCalls = 0;
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    canvas.getByRole("dialog");

    await userEvent.click(canvas.getByText("Close"));
    if (closeCalls !== 1) {
      throw new Error(
        `Expected onClose to be called exactly once, got ${closeCalls}`,
      );
    }
    await waitFor(() => {
      if (canvas.queryByRole("dialog")) {
        throw new Error("Expected dialog to close after clicking Close");
      }
    });
  },
};

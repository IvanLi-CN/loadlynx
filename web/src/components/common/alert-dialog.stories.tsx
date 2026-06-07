import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";
import { waitFor } from "storybook/test";
import { AlertDialog, type AlertDialogProps } from "./alert-dialog.tsx";

let closeCalls = 0;

function AlertDialogHarness(args: AlertDialogProps) {
  const [open, setOpen] = useState(false);

  return (
    <div className="p-6">
      <button
        type="button"
        className="ll-button ll-button-primary"
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
  play: async ({ canvas, userEvent }) => {
    closeCalls = 0;

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

export const BackdropCloses: Story = {
  play: async ({ canvas, canvasElement, userEvent }) => {
    closeCalls = 0;

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    canvas.getByRole("dialog");

    const backdrop =
      canvasElement.querySelector<HTMLButtonElement>(".ll-modal-backdrop");
    if (!backdrop) {
      throw new Error("Expected modal backdrop to be present");
    }

    await userEvent.click(backdrop);
    if (closeCalls !== 1) {
      throw new Error(
        `Expected onClose to be called exactly once, got ${closeCalls}`,
      );
    }
    await waitFor(() => {
      if (canvas.queryByRole("dialog")) {
        throw new Error("Expected dialog to close after clicking backdrop");
      }
    });
  },
};

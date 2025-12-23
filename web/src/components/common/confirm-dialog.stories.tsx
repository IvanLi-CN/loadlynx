import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { ConfirmDialog, type ConfirmDialogProps } from "./confirm-dialog.tsx";

let cancelCalls = 0;
let confirmCalls = 0;

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
    onConfirm: () => {
      confirmCalls += 1;
    },
    onCancel: () => {
      cancelCalls += 1;
    },
  },
} satisfies Meta<typeof ConfirmDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const CancelCloses: Story = {
  play: async ({ canvasElement }) => {
    cancelCalls = 0;
    confirmCalls = 0;
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    canvas.getByRole("dialog");

    await userEvent.click(canvas.getByRole("button", { name: "Cancel" }));
    if (cancelCalls !== 1) {
      throw new Error(
        `Expected onCancel to be called exactly once, got ${cancelCalls}`,
      );
    }
    if (confirmCalls !== 0) {
      throw new Error(
        `Expected onConfirm to be called 0 times, got ${confirmCalls}`,
      );
    }
    await waitFor(() => {
      if (canvas.queryByRole("dialog")) {
        throw new Error("Expected dialog to close after clicking Cancel");
      }
    });
  },
};

export const ConfirmCallsHandler: Story = {
  play: async ({ canvasElement }) => {
    cancelCalls = 0;
    confirmCalls = 0;
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    canvas.getByRole("dialog");

    await userEvent.click(canvas.getByRole("button", { name: "Confirm" }));
    if (confirmCalls !== 1) {
      throw new Error(
        `Expected onConfirm to be called exactly once, got ${confirmCalls}`,
      );
    }
    if (cancelCalls !== 0) {
      throw new Error(
        `Expected onCancel to be called 0 times, got ${cancelCalls}`,
      );
    }
    await waitFor(() => {
      if (canvas.queryByRole("dialog")) {
        throw new Error("Expected dialog to close after clicking Confirm");
      }
    });
  },
};

export const ConfirmDisabled: Story = {
  args: {
    confirmDisabled: true,
  },
  play: async ({ canvasElement }) => {
    cancelCalls = 0;
    confirmCalls = 0;
    const canvas = within(canvasElement);

    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    const confirmButton = canvas.getByRole("button", { name: "Confirm" });
    if (!(confirmButton as HTMLButtonElement).disabled) {
      throw new Error("Expected Confirm button to be disabled");
    }

    if (confirmCalls !== 0) {
      throw new Error(
        `Expected onConfirm to be called 0 times, got ${confirmCalls}`,
      );
    }
    canvas.getByRole("dialog");
  },
};

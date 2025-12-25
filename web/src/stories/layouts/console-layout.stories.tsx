import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function ConsoleLayoutStory(props: { initialPath: string }) {
  return <RouteStoryHarness initialPath={props.initialPath} />;
}

function getInlineSidebar(canvasElement: HTMLElement): HTMLElement {
  const asides = canvasElement.querySelectorAll("aside");
  if (asides.length !== 1) {
    throw new Error(
      `Expected exactly 1 inline sidebar <aside>, found ${asides.length}`,
    );
  }
  const aside = asides[0] as HTMLElement;
  const display = window.getComputedStyle(aside).display;
  if (display === "none") {
    throw new Error("Expected inline sidebar <aside> to be visible");
  }
  return aside;
}

function getStatusLabelSpan(canvasElement: HTMLElement): HTMLSpanElement {
  const canvas = within(canvasElement);
  const statusLink = canvas.getByRole("link", { name: "Status" });
  const label = within(statusLink).getByText("Status");
  if (!(label instanceof HTMLSpanElement)) {
    throw new Error('Expected "Status" label to be a <span>');
  }
  return label;
}

function assertDisplayIsNone(element: HTMLElement, message: string) {
  const display = window.getComputedStyle(element).display;
  if (display !== "none") {
    throw new Error(`${message} (display was "${display}")`);
  }
}

function assertDisplayIsNotNone(element: HTMLElement, message: string) {
  const display = window.getComputedStyle(element).display;
  if (display === "none") {
    throw new Error(`${message} (display was "${display}")`);
  }
}

const meta = {
  title: "Layouts/ConsoleLayout",
  component: ConsoleLayoutStory,
  args: {
    initialPath: "/mock-001/cc",
  },
} satisfies Meta<typeof ConsoleLayoutStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Large: Story = {
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  play: async ({ canvasElement }) => {
    await waitFor(
      () => {
        getInlineSidebar(canvasElement);

        const statusLabel = getStatusLabelSpan(canvasElement);
        assertDisplayIsNotNone(
          statusLabel,
          'Expected "Status" label to be visible at Large',
        );
      },
      { timeout: 5000 },
    );

    const hamburger = canvasElement.querySelector<HTMLElement>(
      'button[aria-label="Open navigation drawer"]',
    );
    if (!hamburger) {
      throw new Error(
        'Expected hamburger button [aria-label="Open navigation drawer"] to exist',
      );
    }
    await waitFor(
      () => {
        assertDisplayIsNone(
          hamburger,
          "Expected hamburger button to be hidden at Large in non-tool layout",
        );
      },
      { timeout: 5000 },
    );
  },
};

export const Medium: Story = {
  globals: {
    viewport: { value: "loadlynxMedium", isRotated: false },
  },
  play: async ({ canvasElement }) => {
    let aside: HTMLElement | null = null;
    await waitFor(
      () => {
        aside = getInlineSidebar(canvasElement);
      },
      { timeout: 5000 },
    );
    if (!aside) throw new Error("Expected inline sidebar <aside> to exist");

    await waitFor(
      () => {
        const statusLabel = getStatusLabelSpan(canvasElement);
        assertDisplayIsNone(
          statusLabel,
          'Expected "Status" label to be hidden (rail) at Medium by default',
        );
      },
      { timeout: 5000 },
    );

    const canvas = within(aside);
    const expandButton = canvas.getByRole("button", { name: "Expand sidebar" });
    await userEvent.click(expandButton);

    await waitFor(
      () => {
        const statusLabel = getStatusLabelSpan(canvasElement);
        assertDisplayIsNotNone(
          statusLabel,
          'Expected "Status" label to be visible after expanding sidebar',
        );
      },
      { timeout: 5000 },
    );

    await waitFor(
      () => {
        within(aside).getByRole("button", { name: "Collapse sidebar" });
        if (within(aside).queryByRole("button", { name: "Expand sidebar" })) {
          throw new Error(
            'Expected toggle aria-label to flip from "Expand sidebar" to "Collapse sidebar"',
          );
        }
      },
      { timeout: 5000 },
    );
  },
};

export const Small: Story = {
  globals: {
    viewport: { value: "loadlynxSmall", isRotated: false },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const hamburger = canvas.getByRole("button", {
      name: "Open navigation drawer",
    });
    await waitFor(
      () => {
        assertDisplayIsNotNone(
          hamburger,
          "Expected hamburger to be visible at Small",
        );
      },
      { timeout: 5000 },
    );

    // Open drawer
    await userEvent.click(hamburger);
    await canvas.findByRole("dialog", {
      name: "Navigation drawer",
    });

    // Close drawer via Escape
    await userEvent.keyboard("{Escape}");
    await waitFor(() => {
      if (canvas.queryByRole("dialog", { name: "Navigation drawer" })) {
        throw new Error("Expected drawer to close after pressing Escape");
      }
    });

    // Re-open drawer and close via backdrop click
    await userEvent.click(hamburger);
    const dialog2 = await canvas.findByRole("dialog", {
      name: "Navigation drawer",
    });
    const drawerRoot = dialog2.parentElement;
    if (!drawerRoot) throw new Error("Expected drawer dialog to have a parent");
    const backdrop = drawerRoot.querySelector<HTMLElement>(
      'div[aria-hidden="true"]',
    );
    if (!backdrop) throw new Error("Expected drawer backdrop to exist");
    await userEvent.click(backdrop);
    await waitFor(() => {
      if (canvas.queryByRole("dialog", { name: "Navigation drawer" })) {
        throw new Error("Expected drawer to close after clicking backdrop");
      }
    });

    // Re-open drawer and validate device switcher + switch device
    await userEvent.click(hamburger);
    const dialog3 = await canvas.findByRole("dialog", {
      name: "Navigation drawer",
    });
    const drawer = within(dialog3);

    const deviceSelect = drawer.getByRole("combobox", {
      name: "Switch device",
    });
    const deviceOptions = within(deviceSelect);
    deviceOptions.getByRole("option", { name: "Demo Device #1 (mock-001)" });
    deviceOptions.getByRole("option", { name: "Demo Device #2 (mock-002)" });

    await userEvent.selectOptions(deviceSelect, "mock-002");

    await waitFor(() => {
      if (canvas.queryByRole("dialog", { name: "Navigation drawer" })) {
        throw new Error("Expected drawer to close after switching device");
      }
    });

    await waitFor(() => {
      canvas.getByRole("option", { name: "Demo Device #2 (mock-002)" });
    });
  },
};

export const ToolMode: Story = {
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  args: {
    initialPath: "/mock-001/calibration",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByRole("heading", { name: "Calibration" });

    await waitFor(
      () => {
        const asides = canvasElement.querySelectorAll("aside");
        if (asides.length !== 0) {
          throw new Error(
            "Expected sidebar <aside> to be hidden in tool layout",
          );
        }
      },
      { timeout: 5000 },
    );

    const hamburger = canvas.getByRole("button", {
      name: "Open navigation drawer",
    });
    await waitFor(
      () => {
        assertDisplayIsNotNone(
          hamburger,
          "Expected hamburger to be visible at Large in tool layout",
        );
      },
      { timeout: 5000 },
    );

    await userEvent.click(hamburger);
    await canvas.findByRole("dialog", { name: "Navigation drawer" });
  },
};

export const NoDeviceSelected: Story = {
  args: {
    initialPath: "/devices",
  },
};

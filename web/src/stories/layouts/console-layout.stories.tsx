import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "storybook/test";
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
  const statusLink = canvas.getByRole("link", { name: "状态" });
  const label = within(statusLink).getByText("状态");
  if (!(label instanceof HTMLSpanElement)) {
    throw new Error('Expected "状态" label to be a <span>');
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
          'Expected "状态" label to be visible at Large',
        );
      },
      { timeout: 5000 },
    );

    const hamburger = canvasElement.querySelector<HTMLElement>(
      'button[aria-label="打开导航抽屉"]',
    );
    if (!hamburger) {
      throw new Error(
        'Expected hamburger button [aria-label="打开导航抽屉"] to exist',
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
  play: async ({ canvasElement, userEvent }) => {
    let aside: HTMLElement | null = null;
    await waitFor(
      () => {
        aside = getInlineSidebar(canvasElement);
      },
      { timeout: 5000 },
    );
    if (!aside) throw new Error("Expected inline sidebar <aside> to exist");
    const asideEl = aside;

    await waitFor(
      () => {
        const statusLabel = getStatusLabelSpan(canvasElement);
        assertDisplayIsNone(
          statusLabel,
          'Expected "状态" label to be hidden (rail) at Medium by default',
        );
      },
      { timeout: 5000 },
    );

    const canvas = within(asideEl);
    const expandButton = canvas.getByRole("button", { name: "展开侧边栏" });
    await userEvent.click(expandButton);

    await waitFor(
      () => {
        const statusLabel = getStatusLabelSpan(canvasElement);
        assertDisplayIsNotNone(
          statusLabel,
          'Expected "状态" label to be visible after expanding sidebar',
        );
      },
      { timeout: 5000 },
    );

    await waitFor(
      () => {
        within(asideEl).getByRole("button", { name: "收起侧边栏" });
        if (within(asideEl).queryByRole("button", { name: "展开侧边栏" })) {
          throw new Error(
            'Expected toggle aria-label to flip from "展开侧边栏" to "收起侧边栏"',
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
  play: async ({ canvas, userEvent }) => {
    const hamburger = canvas.getByRole("button", {
      name: "打开导航抽屉",
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
      name: "导航",
    });

    // Close drawer via Escape
    await userEvent.keyboard("{Escape}");
    await waitFor(() => {
      if (canvas.queryByRole("dialog", { name: "导航" })) {
        throw new Error("Expected drawer to close after pressing Escape");
      }
    });

    // Re-open drawer and close via backdrop click
    await userEvent.click(hamburger);
    const dialog2 = await canvas.findByRole("dialog", {
      name: "导航",
    });
    const drawerRoot = dialog2.parentElement;
    if (!drawerRoot) throw new Error("Expected drawer dialog to have a parent");
    const backdrop = drawerRoot.querySelector<HTMLElement>(
      'div[aria-hidden="true"]',
    );
    if (!backdrop) throw new Error("Expected drawer backdrop to exist");
    await userEvent.click(backdrop);
    await waitFor(() => {
      if (canvas.queryByRole("dialog", { name: "导航" })) {
        throw new Error("Expected drawer to close after clicking backdrop");
      }
    });

    // Re-open drawer and validate device switcher + switch device
    await userEvent.click(hamburger);
    const dialog3 = await canvas.findByRole("dialog", {
      name: "导航",
    });
    const drawer = within(dialog3);

    const deviceSelect = drawer.getByRole("combobox", {
      name: "设备切换",
    });
    const deviceOptions = within(deviceSelect);
    deviceOptions.getByRole("option", { name: "Demo Device #1 (mock-001)" });
    deviceOptions.getByRole("option", { name: "Demo Device #2 (mock-002)" });

    await userEvent.selectOptions(deviceSelect, "mock-002");

    await waitFor(() => {
      if (canvas.queryByRole("dialog", { name: "导航" })) {
        throw new Error("Expected drawer to close after switching device");
      }
    });

    await waitFor(() => {
      canvas.getByRole("option", { name: "Demo Device #2 (mock-002)" });
    });
  },
};

export const CalibrationLarge: Story = {
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  args: {
    initialPath: "/mock-001/calibration",
  },
  play: async ({ canvas, canvasElement }) => {
    await canvas.findByRole(
      "heading",
      { name: "Calibration" },
      { timeout: 5000 },
    );

    await waitFor(
      () => {
        getInlineSidebar(canvasElement);
      },
      { timeout: 5000 },
    );

    const hamburger = canvasElement.querySelector<HTMLElement>(
      'button[aria-label="打开导航抽屉"]',
    );
    if (!hamburger) {
      throw new Error(
        'Expected hamburger button [aria-label="打开导航抽屉"] to exist',
      );
    }
    await waitFor(
      () => {
        assertDisplayIsNone(
          hamburger,
          "Expected hamburger to be hidden at Large in non-tool layout",
        );
      },
      { timeout: 5000 },
    );
  },
};

export const NoDeviceSelected: Story = {
  args: {
    initialPath: "/devices",
  },
};

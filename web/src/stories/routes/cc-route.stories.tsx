import type { Meta, StoryObj } from "@storybook/react";
import { expect, waitFor, within } from "storybook/test";
import type { StoredDevice } from "../../devices/device-store.ts";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function CcRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/cc" />;
}

const meta = {
  title: "Routes/CC",
  component: CcRouteStory,
  globals: {
    loadlynxLocale: "en",
  },
  parameters: {
    viewport: { defaultViewport: "loadlynxLarge" },
  },
} satisfies Meta<typeof CcRouteStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvas, userEvent }) => {
    await canvas.findByText(/Mode, output and setpoints/i, undefined, {
      timeout: 5_000,
    });
    await canvas.findByRole(
      "button",
      { name: /Current device/i },
      { timeout: 5_000 },
    );
    await canvas.findByRole("radio", { name: "CC" }, { timeout: 5_000 });
    await canvas.findByRole("radio", { name: "CV" }, { timeout: 5_000 });
    await canvas.findByRole("radio", { name: "CP" }, { timeout: 5_000 });
    const crBtn = await canvas.findByRole(
      "radio",
      { name: "CR" },
      { timeout: 5_000 },
    );
    if (!(crBtn as HTMLInputElement).disabled) {
      throw new Error("Expected CR button to be visible but read-only");
    }

    await waitFor(
      () => {
        const activePreset = canvas.getByTestId("control-active-preset");
        if ((activePreset.textContent ?? "").includes("—")) {
          throw new Error("Expected control state to be loaded");
        }
      },
      { timeout: 5_000 },
    );

    const outputToggle = await canvas.findByRole("switch", {
      name: /Load output switch|负载主开关/i,
    });

    await expect(outputToggle).toBeEnabled();
    const initialOutputState = outputToggle.getAttribute("aria-checked");
    await userEvent.click(outputToggle);
    await waitFor(
      () => {
        const toggled = canvas.getByRole("switch", {
          name: /Load output switch|负载主开关/i,
        });
        if (toggled.getAttribute("aria-checked") === initialOutputState) {
          throw new Error("Expected output switch state to change after click");
        }
      },
      { timeout: 5_000 },
    );
  },
};

export const LiveTelemetry: Story = {
  play: async ({ canvas }) => {
    await canvas.findByText(/Mode, output and setpoints/i, undefined, {
      timeout: 5_000,
    });
    await waitFor(
      () => {
        const controlSummary = canvas.getByText(/· Uptime:/i);
        const current = controlSummary.textContent ?? "";
        if (!current.includes("Setpoint:")) {
          throw new Error("Expected controls summary to include uptime");
        }
      },
      { timeout: 5_000 },
    );

    const firstSummary = (
      canvas.getByText(/· Uptime:/i).textContent ?? ""
    ).trim();
    await waitFor(
      () => {
        const nextSummary = (
          canvas.getByText(/· Uptime:/i).textContent ?? ""
        ).trim();
        if (nextSummary === firstSummary) {
          throw new Error("Expected live telemetry summary to refresh");
        }
      },
      { timeout: 1_500 },
    );
  },
};

export const PdPanelEmbedded: Story = {
  render: () => <RouteStoryHarness initialPath="/mock-001/cc?panel=pd" />,
  play: async ({ canvas }) => {
    await canvas.findByRole("heading", { name: "USB-PD" }, { timeout: 5_000 });
    const closeButtons = await canvas.findAllByRole("button", {
      name: /Close dashboard tools/i,
    });
    if (closeButtons.length < 2) {
      throw new Error("Expected drawer backdrop and close button controls");
    }
  },
};

const DEVICE_CP_SUPPORTED: StoredDevice[] = [
  { id: "mock-001", name: "Demo Device #1", baseUrl: "mock://demo-1" },
];

const DEVICE_CP_UNSUPPORTED: StoredDevice[] = [
  { id: "mock-001", name: "Demo Device #1", baseUrl: "mock://demo-no-cp" },
];

const DEVICE_LINK_DOWN: StoredDevice[] = [
  { id: "mock-001", name: "Demo Device #1", baseUrl: "mock://demo-link-down" },
];

const DEVICE_ANALOG_NOT_READY: StoredDevice[] = [
  {
    id: "mock-001",
    name: "Demo Device #1",
    baseUrl: "mock://demo-cal-missing",
  },
];

export const CpUnsupported: Story = {
  render: () => (
    <RouteStoryHarness
      initialPath="/mock-001/cc"
      devices={DEVICE_CP_UNSUPPORTED}
    />
  ),
  play: async ({ canvas }) => {
    await canvas.findByText(/Mode, output and setpoints/i, undefined, {
      timeout: 5_000,
    });
    const cpBtn = await canvas.findByRole(
      "radio",
      { name: "CP" },
      { timeout: 5_000 },
    );
    if (!(cpBtn as HTMLInputElement).disabled) {
      throw new Error(
        "Expected CP button to be disabled when cp_supported=false",
      );
    }
  },
};

export const LinkDown: Story = {
  render: () => (
    <RouteStoryHarness initialPath="/mock-001/cc" devices={DEVICE_LINK_DOWN} />
  ),
  play: async ({ canvas }) => {
    await canvas.findByText(/Link unavailable|UART link is down/i);
  },
};

export const AnalogNotReady: Story = {
  render: () => (
    <RouteStoryHarness
      initialPath="/mock-001/cc"
      devices={DEVICE_ANALOG_NOT_READY}
    />
  ),
  play: async ({ canvas }) => {
    const matches = await canvas.findAllByText(
      /Cal missing|ANALOG_NOT_READY|NOT_ATTACHED/i,
    );
    if (matches.length === 0) {
      throw new Error("Expected analog-not-ready state to be visible");
    }
  },
};

export const LimitViolationBlocked: Story = {
  render: () => (
    <RouteStoryHarness
      initialPath="/mock-001/cc"
      devices={DEVICE_CP_SUPPORTED}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(
      await canvas.findByRole("button", { name: /Advanced/i }),
    );
    await userEvent.click(
      await canvas.findByRole(
        "button",
        { name: /Expand/i },
        { timeout: 5_000 },
      ),
    );
    const advancedRegion = await canvas.findByRole("region", {
      name: /Advanced/i,
    });
    const advanced = within(advancedRegion);
    await userEvent.click(
      await advanced.findByRole("radio", { name: "CP" }, { timeout: 5_000 }),
    );

    const targetPowerInput = (
      await advanced.findAllByLabelText(/Target power/i)
    ).find((node) => node instanceof HTMLInputElement && node.type === "text");
    if (!(targetPowerInput instanceof HTMLInputElement)) {
      throw new Error("Expected Target power textbox inside Advanced region");
    }
    await userEvent.clear(targetPowerInput);
    await userEvent.type(targetPowerInput, "2000");
    await userEvent.tab();

    const maxPowerInput = (
      await advanced.findAllByLabelText(/Max power/i)
    ).find((node) => node instanceof HTMLInputElement && node.type === "text");
    if (!(maxPowerInput instanceof HTMLInputElement)) {
      throw new Error("Expected Max power textbox inside Advanced region");
    }
    await userEvent.clear(maxPowerInput);
    await userEvent.type(maxPowerInput, "1000");
    await userEvent.tab();

    await waitFor(() => {
      if (targetPowerInput.value !== "1000") {
        throw new Error(
          `Expected target power to clamp to max power, got ${targetPowerInput.value}`,
        );
      }
    });

    const saveBtn = await advanced.findByRole("button", {
      name: /Save Active Slot|Save Slot/i,
    });
    if ((saveBtn as HTMLButtonElement).disabled) {
      throw new Error("Expected save action to stay enabled after clamping");
    }
  },
};

import type { Meta, StoryObj } from "@storybook/react";
import { within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
import type { StoredDevice } from "../../devices/device-store.ts";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function CcRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/cc" />;
}

const meta = {
  title: "Routes/CC",
  component: CcRouteStory,
} satisfies Meta<typeof CcRouteStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await canvas.findByText(/MODE & OUTPUT/i);
    await canvas.findByText(/PRESETS/i);

    const outputToggle = await canvas.findByRole("checkbox", {
      name: /Output enabled/i,
    });

    if ((outputToggle as HTMLInputElement).checked) {
      throw new Error("Expected Output enabled to start unchecked");
    }
    await userEvent.click(outputToggle);
    if (!(outputToggle as HTMLInputElement).checked) {
      throw new Error("Expected Output enabled to be checked after click");
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
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByText(/MODE & OUTPUT/i);
    const cpBtn = await canvas.findByRole("button", { name: "CP" });
    if (!(cpBtn as HTMLButtonElement).disabled) {
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
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByText(/HTTP error: LINK_DOWN/i);
  },
};

export const AnalogNotReady: Story = {
  render: () => (
    <RouteStoryHarness
      initialPath="/mock-001/cc"
      devices={DEVICE_ANALOG_NOT_READY}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByText(/HTTP error: ANALOG_NOT_READY/i);
  },
};

export const LimitViolationBlocked: Story = {
  render: () => (
    <RouteStoryHarness
      initialPath="/mock-001/cc"
      devices={DEVICE_CP_SUPPORTED}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await canvas.findByText(/PRESETS/i);

    await userEvent.click(
      await canvas.findByRole("button", { name: /Advanced/i }),
    );

    const modeSelect = await canvas.findByLabelText(/Mode/i);
    await userEvent.selectOptions(modeSelect, "cp");

    await userEvent.clear(await canvas.findByLabelText(/Max power/i));
    await userEvent.type(await canvas.findByLabelText(/Max power/i), "1000");

    await userEvent.clear(await canvas.findByLabelText(/Target power/i));
    await userEvent.type(await canvas.findByLabelText(/Target power/i), "2000");

    await canvas.findByText(/target_p_mw must be ≤ max_p_mw/i);

    const advancedRegion = await canvas.findByRole("region", {
      name: /Advanced/i,
    });
    const advanced = within(advancedRegion);
    const saveBtn = await advanced.findByRole("button", {
      name: /Save Draft/i,
    });
    if (!(saveBtn as HTMLButtonElement).disabled) {
      throw new Error("Expected Save Draft to be disabled on limit violation");
    }
  },
};

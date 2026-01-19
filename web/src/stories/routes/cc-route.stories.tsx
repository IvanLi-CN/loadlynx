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

    await canvas.findByRole("heading", { name: /Device control/i });
    await canvas.findByRole("heading", { name: /Presets/i });
    await canvas.findByText(/1500\s*mA/);

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
    await canvas.findByRole("heading", { name: /Device control/i });
    await canvas.findByText(/CP: 固件不支持/i);
    const modeSelect = await canvas.findByLabelText(/Mode/i);
    if ((modeSelect as HTMLSelectElement).querySelector('option[value="cp"]')) {
      throw new Error(
        "Expected CP option to be hidden when cp_supported=false",
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
    await canvas.findByRole("heading", { name: /Device control/i });
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
    await canvas.findByRole("heading", { name: /Device control/i });
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

    await canvas.findByRole("heading", { name: /Device control/i });
    await canvas.findByRole("heading", { name: /Preset editor/i });

    const modeSelect = await canvas.findByLabelText(/Mode/i);
    await userEvent.selectOptions(modeSelect, "cp");

    await userEvent.clear(await canvas.findByLabelText(/Max power/i));
    await userEvent.type(await canvas.findByLabelText(/Max power/i), "1000");

    await userEvent.clear(await canvas.findByLabelText(/Target power/i));
    await userEvent.type(await canvas.findByLabelText(/Target power/i), "2000");

    await canvas.findByText(/target_p_mw must be ≤ max_p_mw/i);

    const saveBtn = await canvas.findByRole("button", { name: /Save preset/i });
    if (!(saveBtn as HTMLButtonElement).disabled) {
      throw new Error("Expected Save preset to be disabled on limit violation");
    }
  },
};

import type { Meta, StoryObj } from "@storybook/react";

import { MainDisplayCanvas } from "../../routes/device-cc.tsx";

const meta: Meta<typeof MainDisplayCanvas> = {
  title: "Main Display/Canvas",
  component: MainDisplayCanvas,
  args: {
    remoteVoltageV: 24.5,
    localVoltageV: 24.47,
    localCurrentA: 4.2,
    remoteCurrentA: 3.5,
    totalCurrentA: 12.0,
    totalPowerW: 294.0,
    controlMode: "cc",
    controlTargetMilli: 12_000,
    controlTargetUnit: "A",
    uptimeSeconds: 1 * 3600 + 32 * 60 + 10,
    tempCoreC: 42.3,
    tempSinkC: 38.1,
    tempMcuC: 35.0,
    remoteActive: true,
    analogState: "ready",
    faultFlags: 0,
  },
  parameters: {
    viewport: {
      defaultViewport: "loadlynxLarge",
    },
    layout: "centered",
  },
};

export default meta;

type Story = StoryObj<typeof MainDisplayCanvas>;

export const CC: Story = {};

export const CV: Story = {
  args: {
    controlMode: "cv",
    controlTargetMilli: 24_500,
    controlTargetUnit: "V",
  },
};

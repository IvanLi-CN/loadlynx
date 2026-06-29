import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";

import {
  LoadOutputSwitch,
  type LoadOutputSwitchProps,
} from "./load-output-switch.tsx";

function LoadOutputSwitchGallery(
  args: Omit<LoadOutputSwitchProps, "checked" | "onCheckedChange">,
) {
  const [checked, setChecked] = useState(false);

  return (
    <div className="flex w-full max-w-4xl flex-col gap-6 p-6">
      {(["sm", "md", "lg"] as const).map((size) => (
        <section
          key={size}
          className="rounded-2xl border border-slate-400/10 bg-black/16 p-5"
        >
          <div className="text-[11px] uppercase tracking-[0.14em] text-slate-200/45">
            {size}
          </div>
          <div className="mt-4 grid gap-4 md:grid-cols-2">
            <div className="max-w-xl">
              <LoadOutputSwitch
                {...args}
                size={size}
                checked={checked}
                onCheckedChange={setChecked}
              />
            </div>
            <div className="max-w-xl">
              <LoadOutputSwitch
                {...args}
                size={size}
                checked={!checked}
                onCheckedChange={(next) => setChecked(!next)}
                disabled
              />
            </div>
          </div>
        </section>
      ))}
    </div>
  );
}

const meta = {
  title: "Instrument/LoadOutputSwitch",
  component: LoadOutputSwitchGallery,
  parameters: {
    layout: "fullscreen",
    viewport: {
      defaultViewport: "loadlynxLarge",
    },
  },
} satisfies Meta<typeof LoadOutputSwitchGallery>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Gallery: Story = {
  args: {
    ariaLabel: "Load output switch",
    offLabel: "Off",
    onLabel: "On",
  },
};

import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";

import {
  SliderRadioGroup,
  type SliderRadioGroupProps,
} from "./slider-radio-group.tsx";

function SliderRadioGroupGallery(
  args: Omit<SliderRadioGroupProps<string>, "value" | "onValueChange">,
) {
  const [value, setValue] = useState("cc");

  return (
    <div className="flex w-full max-w-4xl flex-col gap-6 p-6">
      {(["fit", "fill"] as const).map((widthMode) => (
        <section
          key={widthMode}
          className="rounded-2xl border border-slate-400/10 bg-black/16 p-5"
        >
          <div className="instrument-label">{widthMode}</div>
          <div className="mt-4 flex flex-col gap-4">
            {(["sm", "md", "lg"] as const).map((size) => (
              <div key={`${widthMode}-${size}`}>
                <div className="text-[11px] uppercase tracking-[0.14em] text-slate-200/45">
                  {size}
                </div>
                <div className="mt-2 max-w-3xl">
                  <SliderRadioGroup
                    {...args}
                    size={size}
                    widthMode={widthMode}
                    value={value}
                    onValueChange={setValue}
                  />
                </div>
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

const meta = {
  title: "UI/SliderRadioGroup",
  component: SliderRadioGroupGallery,
  parameters: {
    layout: "fullscreen",
    viewport: {
      defaultViewport: "loadlynxLarge",
    },
  },
} satisfies Meta<typeof SliderRadioGroupGallery>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Gallery: Story = {
  args: {
    ariaLabel: "Instrument mode selection",
    options: [
      { value: "cc", label: "CC" },
      { value: "cv", label: "CV" },
      { value: "cp", label: "CP" },
      { value: "cr", label: "CR", disabled: true },
    ],
  },
};

export const GalleryCompact: Story = {
  args: {
    ariaLabel: "Preset slots",
    options: [
      { value: "cc", label: "#1" },
      { value: "cv", label: "#2" },
      { value: "cp", label: "#3" },
      { value: "cr", label: "#4" },
      { value: "slot-5", label: "#5" },
      { value: "slot-6", label: "#6", disabled: true },
    ],
  },
};

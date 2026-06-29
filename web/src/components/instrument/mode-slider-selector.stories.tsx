import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";

import {
  ModeSliderSelector,
  type ModeSliderSelectorProps,
} from "./mode-slider-selector.tsx";

function ModeSliderSelectorGallery(
  args: Omit<ModeSliderSelectorProps, "activeMode" | "onModeChange">,
) {
  const [activeMode, setActiveMode] = useState<"CC" | "CV" | "CP" | "CR">("CC");

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
                  <ModeSliderSelector
                    {...args}
                    size={size}
                    widthMode={widthMode}
                    activeMode={activeMode}
                    onModeChange={setActiveMode}
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
  title: "Instrument/ModeSliderSelector",
  component: ModeSliderSelectorGallery,
  parameters: {
    layout: "fullscreen",
    viewport: {
      defaultViewport: "loadlynxLarge",
    },
  },
} satisfies Meta<typeof ModeSliderSelectorGallery>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Gallery: Story = {
  args: {
    availableModes: ["CC", "CV", "CP"],
    ariaLabel: "Control mode selector",
  },
};

export const GalleryNoCp: Story = {
  args: {
    availableModes: ["CC", "CV"],
    ariaLabel: "Control mode selector without cp",
  },
};

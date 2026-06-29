import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";

import {
  BlockControlInputRow,
  BlockControlSliderRow,
} from "./block-control-row.tsx";

function BlockControlRowGallery() {
  const [current, setCurrent] = useState(1500);
  const [voltage, setVoltage] = useState(12000);
  const [power, setPower] = useState(10000);
  const [alias, setAlias] = useState("Preset Alpha");

  return (
    <div className="flex w-full max-w-5xl flex-col gap-6 p-6">
      <section className="rounded-2xl border border-slate-400/10 bg-black/16 p-5">
        <div className="instrument-label">Slider + input row</div>
        <div className="mt-4 flex flex-col gap-3">
          <BlockControlSliderRow
            id="story-target-current"
            label="Target current (mA)"
            value={current}
            min={0}
            max={10000}
            step={50}
            onValueChange={setCurrent}
          />
          <BlockControlSliderRow
            id="story-target-voltage"
            label="Target voltage (mV)"
            value={voltage}
            min={0}
            max={30000}
            step={100}
            onValueChange={setVoltage}
          />
          <BlockControlSliderRow
            id="story-target-power"
            label="Max power (mW)"
            value={power}
            min={0}
            max={150000}
            step={500}
            onValueChange={setPower}
          />
        </div>
      </section>

      <section className="rounded-2xl border border-slate-400/10 bg-black/16 p-5">
        <div className="instrument-label">Input row</div>
        <div className="mt-4 flex flex-col gap-3">
          <BlockControlInputRow
            id="story-preset-name"
            label="Preset alias"
            value={alias}
            onChange={(event) => setAlias(event.target.value)}
          />
          <BlockControlInputRow
            id="story-preset-readonly"
            label="Device protocol"
            value="API v2.0.0"
            readOnly
          />
        </div>
      </section>
    </div>
  );
}

const meta = {
  title: "UI/BlockControlRow",
  component: BlockControlRowGallery,
  parameters: {
    layout: "fullscreen",
    viewport: {
      defaultViewport: "loadlynxLarge",
    },
  },
} satisfies Meta<typeof BlockControlRowGallery>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Gallery: Story = {};

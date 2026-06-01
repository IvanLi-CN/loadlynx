import type { Meta, StoryObj } from "@storybook/react";

function Smoke() {
  return (
    <div className="mx-auto max-w-lg space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div className="space-y-1">
          <h1 className="text-lg font-semibold">LoadLynx UI smoke</h1>
          <p className="text-sm opacity-70">
            Tailwind 4 + DaisyUI should render with the app&apos;s dark styling.
          </p>
        </div>
        <span className="ll-badge ll-badge-outline">storybook</span>
      </div>

      <div className="ll-panel bg-base-200 shadow">
        <div className="ll-panel-body">
          <h2 className="ll-panel-title">Controls</h2>
          <p className="text-sm opacity-70">
            Buttons should use DaisyUI tokens from <code>src/index.css</code>.
          </p>
          <div className="flex flex-wrap gap-2">
            <button className="ll-button ll-button-primary" type="button">
              Primary
            </button>
            <button className="ll-button ll-button-ghost" type="button">
              Ghost
            </button>
            <button className="ll-button ll-button-outline" type="button">
              Outline
            </button>
          </div>
        </div>
      </div>

      <div className="ll-stats ll-stats-vertical bg-base-200 shadow sm:ll-stats-horizontal">
        <div className="ll-stat">
          <div className="ll-stat-title">Viewport</div>
          <div className="ll-stat-value text-primary">Ready</div>
          <div className="ll-stat-desc">Use the toolbar presets</div>
        </div>
        <div className="ll-stat">
          <div className="ll-stat-title">Theme</div>
          <div className="ll-stat-value">Dark</div>
          <div className="ll-stat-desc">data-theme=&quot;dark&quot;</div>
        </div>
      </div>
    </div>
  );
}

const meta = {
  title: "Smoke/DaisyUI",
  component: Smoke,
} satisfies Meta<typeof Smoke>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

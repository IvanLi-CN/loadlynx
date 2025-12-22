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
        <span className="badge badge-outline">storybook</span>
      </div>

      <div className="card bg-base-200 shadow">
        <div className="card-body">
          <h2 className="card-title">Controls</h2>
          <p className="text-sm opacity-70">
            Buttons should use DaisyUI tokens from <code>src/index.css</code>.
          </p>
          <div className="flex flex-wrap gap-2">
            <button className="btn btn-primary" type="button">
              Primary
            </button>
            <button className="btn btn-ghost" type="button">
              Ghost
            </button>
            <button className="btn btn-outline" type="button">
              Outline
            </button>
          </div>
        </div>
      </div>

      <div className="stats stats-vertical bg-base-200 shadow sm:stats-horizontal">
        <div className="stat">
          <div className="stat-title">Viewport</div>
          <div className="stat-value text-primary">Ready</div>
          <div className="stat-desc">Use the toolbar presets</div>
        </div>
        <div className="stat">
          <div className="stat-title">Theme</div>
          <div className="stat-value">Dark</div>
          <div className="stat-desc">data-theme=&quot;dark&quot;</div>
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

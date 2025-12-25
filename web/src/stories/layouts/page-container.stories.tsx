import type { Meta, StoryObj } from "@storybook/react";
import { PageContainer } from "../../components/layout/page-container.tsx";

function PageContainerStory(props: {
  variant?: "default" | "full";
  className?: string;
}) {
  return (
    <PageContainer
      variant={props.variant}
      className={[
        "border border-dashed border-base-content/20 rounded-box p-4",
        props.className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="space-y-1">
        <h2 className="text-lg font-semibold">PageContainer</h2>
        <p className="text-sm text-base-content/70">
          Variant:{" "}
          <code className="font-mono">{props.variant ?? "default"}</code>
        </p>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <div className="card bg-base-200 shadow">
          <div className="card-body">
            <h3 className="card-title text-base">Content block</h3>
            <p className="text-sm text-base-content/70">
              This card helps visualize the container width and spacing.
            </p>
          </div>
        </div>
        <div className="card bg-base-200 shadow">
          <div className="card-body">
            <h3 className="card-title text-base">Second block</h3>
            <p className="text-sm text-base-content/70">
              Default variant should center with <code>max-w-5xl</code>.
            </p>
          </div>
        </div>
      </div>
    </PageContainer>
  );
}

const meta = {
  title: "Layouts/PageContainer",
  component: PageContainerStory,
  args: {
    variant: "default",
    className: "space-y-6",
  },
} satisfies Meta<typeof PageContainerStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const FullWidth: Story = {
  args: { variant: "full" },
};

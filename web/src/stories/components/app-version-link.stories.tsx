import type { Meta, StoryObj } from "@storybook/react";
import { expect, within } from "storybook/test";
import { AppVersionLink } from "../../components/layout/app-version-link.tsx";

const meta = {
  title: "Components/App Version Link",
  component: AppVersionLink,
  tags: ["autodocs"],
  args: {
    version: "v0.1.0",
    repo: "IvanLi-CN/loadlynx",
    sha: "451328f6def6a29b717f714dced490ffe67b667b",
    tag: "v0.1.0",
  },
} satisfies Meta<typeof AppVersionLink>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Release: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const link = canvas.getByRole("link", { name: /open v0\.1\.0 on github/i });
    await expect(link.className).not.toMatch(/(?:^|\s)link(?:\s|$)/);
  },
};

export const ReleaseVisual: Story = {
  render: (args) => (
    <div
      className="inline-flex bg-base-100 px-2 pt-[5px] pb-[7px] text-base-content"
      data-testid="app-version-link-evidence"
    >
      <AppVersionLink {...args} />
    </div>
  ),
};

export const Unlinked: Story = {
  args: {
    repo: null,
    sha: null,
    tag: null,
  },
};

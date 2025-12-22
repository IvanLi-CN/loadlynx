import type { Meta, StoryObj } from "@storybook/react";
import { App } from "../../app.tsx";

const meta = {
  title: "Legacy/App (scaffold)",
  component: App,
} satisfies Meta<typeof App>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

import type { Meta, StoryObj } from "@storybook/react";
import { within } from "storybook/test";
import { RoutePendingView } from "../../components/layout/route-pending-view.tsx";

const meta = {
  title: "Layouts/RoutePendingView",
  component: RoutePendingView,
  tags: ["autodocs"],
  args: {
    title: "正在打开设置",
    description: "正在加载设备设置",
  },
  parameters: {
    docs: {
      description: {
        component:
          "Route-level pending state used while lazy route chunks or route loaders are resolving.",
      },
    },
  },
} satisfies Meta<typeof RoutePendingView>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Settings: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByRole("status");
    await canvas.findByText("正在打开设置");
  },
};

export const Devices: Story = {
  args: {
    title: "正在打开设备列表",
    description: "正在准备设备与 devd 状态",
  },
};

export const Compact: Story = {
  args: {
    title: "正在切换视图",
    description: "正在加载目标面板",
    compact: true,
  },
};

export const StateGallery: Story = {
  render: () => (
    <div className="grid gap-4 bg-base-100 p-4 md:grid-cols-2">
      <RoutePendingView
        compact
        title="正在打开设置"
        description="正在加载设备设置"
      />
      <RoutePendingView
        compact
        title="正在打开 USB-PD"
        description="正在加载 USB-PD 面板"
      />
    </div>
  ),
  parameters: {
    docs: {
      description: {
        story:
          "Curated gallery for the route transition states most likely to be seen from the device sidebar.",
      },
    },
  },
};

import type { Meta, StoryObj } from "@storybook/react";
import { waitFor } from "storybook/test";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function ConsoleLayoutStory(props: { initialPath: string }) {
  return <RouteStoryHarness initialPath={props.initialPath} />;
}

const meta = {
  title: "Layouts/ConsoleLayout",
  component: ConsoleLayoutStory,
  args: {
    initialPath: "/mock-001/cc",
  },
} satisfies Meta<typeof ConsoleLayoutStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Large: Story = {
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(
      () => {
        canvas.getByText("LoadLynx Web Console");
        canvas.getByRole("navigation", { name: "主导航" });
        canvas.getByRole("link", { name: "总览" });
        canvas.getByRole("button", { name: "仪表盘" });
        canvas.getByRole("button", { name: "系统" });
      },
      { timeout: 5_000 },
    );

    const asides = canvasElement.querySelectorAll("aside");
    if (asides.length > 1) {
      throw new Error(
        "Expected the header shell to render without legacy side navigation",
      );
    }
  },
};

export const Medium: Story = {
  globals: {
    viewport: { value: "loadlynxMedium", isRotated: false },
  },
  play: async ({ canvas }) => {
    await waitFor(
      () => {
        canvas.getByText("LoadLynx Web Console");
        canvas.getByRole("navigation", { name: "主导航" });
        canvas.getByRole("button", { name: /当前设备：/ });
      },
      { timeout: 5_000 },
    );
  },
};

export const SmallSelectionFlow: Story = {
  globals: {
    viewport: { value: "loadlynxSmall", isRotated: false },
  },
  play: async ({ canvas, userEvent }) => {
    await canvas.findByText("LoadLynx Web Console");
    const deviceButton = await canvas.findByRole("button", {
      name: /当前设备：/,
    });
    await userEvent.click(deviceButton);

    await waitFor(() => {
      canvas.getByRole("heading", { name: "选择设备" });
    });
    await waitFor(() => {
      if (canvas.getAllByRole("link", { name: "使用此设备" }).length < 2) {
        throw new Error("Expected mobile selection CTAs for overview devices");
      }
    });
  },
};

export const NoDeviceSelected: Story = {
  args: {
    initialPath: "/devices",
  },
};

export const OverviewKeepsLastActiveDevice: Story = {
  render: () => (
    <RouteStoryHarness initialPath="/devices" lastActiveDeviceId="mock-001" />
  ),
  globals: {
    viewport: { value: "loadlynxLarge", isRotated: false },
  },
  play: async ({ canvas }) => {
    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "总览" });
        const deviceButton = canvas.getByRole("button", {
          name: /当前设备：/,
        });
        if (!deviceButton.textContent?.includes("Demo Device #1")) {
          throw new Error(
            "Expected header device switcher to keep Demo Device #1",
          );
        }
        if (!deviceButton.textContent?.includes("mock-001")) {
          throw new Error("Expected header device switcher to keep mock-001");
        }
      },
      { timeout: 5_000 },
    );
  },
};

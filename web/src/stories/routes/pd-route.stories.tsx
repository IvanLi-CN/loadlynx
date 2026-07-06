import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "storybook/test";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function PdRouteDefaultStory() {
  return <RouteStoryHarness initialPath="/mock-001/cc?panel=pd" />;
}

function PdRouteExtendedVoltageStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/cc?panel=pd"
      devices={[
        {
          id: "mock-001",
          name: "PD Extended Enabled",
          baseUrl: "mock://demo-extended",
        },
      ]}
    />
  );
}

function PdRouteUnsupportedStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/cc?panel=pd"
      devices={[
        {
          id: "mock-001",
          name: "PD Unsupported",
          baseUrl: "mock://demo-no-pd",
        },
      ]}
    />
  );
}

function PdRouteLinkDownStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/cc?panel=pd"
      devices={[
        {
          id: "mock-001",
          name: "PD Link Down",
          baseUrl: "mock://demo-link-down",
        },
      ]}
    />
  );
}

function PdRouteHiddenFixed28Story() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/cc?panel=pd"
      devices={[
        {
          id: "mock-001",
          name: "PD Hidden Saved 28V",
          baseUrl: "mock://demo-hidden-fixed-28",
        },
      ]}
    />
  );
}

function PdRouteRealFixed28Story() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/cc?panel=pd"
      devices={[
        {
          id: "mock-001",
          name: "PD Real Fixed 28V",
          baseUrl: "mock://demo-real-fixed-28",
        },
      ]}
    />
  );
}

const meta = {
  title: "Routes/Dashboard PD Panel",
  component: PdRouteDefaultStory,
} satisfies Meta<typeof PdRouteDefaultStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ExtendedVoltageEnabled: Story = {
  render: () => <PdRouteExtendedVoltageStory />,
};

export const Unsupported: Story = {
  render: () => <PdRouteUnsupportedStory />,
};

export const LinkDown: Story = {
  render: () => <PdRouteLinkDownStory />,
};

export const HiddenSavedFixed28: Story = {
  render: () => <PdRouteHiddenFixed28Story />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByText("LoadLynx Web Console");
        canvas.getByRole("heading", { name: "USB-PD" });
        canvas.getByText(/配置列表/i);
        canvas.getByText(/Fixed PDO/i);
      },
      { timeout: 5_000 },
    );

    await waitFor(() => {
      if (canvas.queryByText("28.0 V")) {
        throw new Error("Did not expect a synthetic 28V row");
      }
      if (canvas.queryByText(/PDO #8/i)) {
        throw new Error("Did not expect the hidden saved PDO #8 to be shown");
      }
    });

    const applyButton = await canvas.findByRole("button", {
      name: "Apply profile",
    });
    if ((applyButton as HTMLButtonElement).disabled) {
      throw new Error(
        "Expected Apply to stay enabled when a visible fallback Fixed PDO is selected",
      );
    }

    await userEvent.click(
      await canvas.findByRole(
        "button",
        { name: /15\.0 V/i },
        { timeout: 5000 },
      ),
    );

    await waitFor(() => {
      const nextApplyButton = canvas.getByRole("button", {
        name: "Apply profile",
      }) as HTMLButtonElement;
      if (nextApplyButton.disabled) {
        throw new Error("Expected Apply to enable after selecting a real PDO");
      }
    });
  },
};

export const RealFixed28: Story = {
  render: () => <PdRouteRealFixed28Story />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByText("LoadLynx Web Console");
        canvas.getByRole("heading", { name: "USB-PD" });
        canvas.getByText(/配置列表/i);
        canvas.getByText("28.0 V");
        canvas.getByText(/已保存配置/i);
        canvas.getByText(/Fixed · PDO #8 · 28\.0 V · 3000 mA/i);
      },
      { timeout: 5_000 },
    );
  },
};

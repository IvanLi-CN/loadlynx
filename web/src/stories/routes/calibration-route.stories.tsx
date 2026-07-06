import type { Meta, StoryObj } from "@storybook/react";
import { userEvent, waitFor, within } from "storybook/test";
import type { StoredDevice } from "../../devices/device-store.ts";
import type { MemoryCalibrationStore } from "../../routes/device-calibration/store.ts";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

type CalibrationRouteStoryProps = {
  beforeMount?: (stores: { calibrationStore: MemoryCalibrationStore }) => void;
  devices?: StoredDevice[];
};

function clearCalibrationDraftStorage(
  calibrationStore: MemoryCalibrationStore,
  deviceId: string,
  baseUrl: string,
) {
  calibrationStore.setDraft(deviceId, baseUrl, null);
}

function seedCurrentCh2Draft(
  calibrationStore: MemoryCalibrationStore,
  deviceId: string,
  baseUrl: string,
) {
  clearCalibrationDraftStorage(calibrationStore, deviceId, baseUrl);
  calibrationStore.setDraft(deviceId, baseUrl, {
    version: 4,
    saved_at: "2026-04-16T00:00:00.000Z",
    device_id: deviceId,
    base_url: baseUrl,
    active_tab: "current_ch2",
    draft_profile: {
      v_local_points: [],
      v_remote_points: [],
      current_ch1_points: [],
      current_ch2_points: [
        [[5300, 685], 989600],
        [[10680, 1375], 1984700],
      ],
    },
  });
}

function findCalModeBadge(canvasElement: HTMLElement): HTMLElement | null {
  return (
    Array.from(canvasElement.querySelectorAll<HTMLElement>(".ll-badge")).find(
      (element) => element.textContent?.includes("cal_mode:"),
    ) ?? null
  );
}

function findCalibrationStat(
  canvasElement: HTMLElement,
  title: string,
): HTMLElement | null {
  const matches = Array.from(
    canvasElement.querySelectorAll<HTMLElement>(".ll-stat"),
  ).filter(
    (element) =>
      element.querySelector(".ll-stat-title")?.textContent?.trim() === title,
  );
  return matches.at(-1) ?? null;
}

function CalibrationRouteStory(props: CalibrationRouteStoryProps) {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/calibration"
      devices={props.devices}
      beforeMount={({ calibrationStore }) => {
        clearCalibrationDraftStorage(
          calibrationStore,
          "mock-001",
          "mock://demo-1",
        );
        props.beforeMount?.({ calibrationStore });
      }}
    />
  );
}

const meta = {
  title: "Routes/Calibration",
  component: CalibrationRouteStory,
} satisfies Meta<typeof CalibrationRouteStory>;

export default meta;

type Story = StoryObj<typeof meta>;

const DEVICE_CAL_OUTPUT_APPLIED: StoredDevice[] = [
  {
    id: "mock-001",
    name: "Demo Device #1",
    baseUrl: "mock://demo-calibration-output-applied",
  },
];

export const Default: Story = {
  play: async ({ canvas, canvasElement }) => {
    await canvas.findByRole(
      "heading",
      { name: "Calibration" },
      { timeout: 5_000 },
    );
    const nav = within(
      canvasElement.querySelector('[aria-label="系统页导航"]') as HTMLElement,
    );
    await userEvent.click(nav.getByRole("link", { name: "电流通道1" }));
    await canvas.findByText("电流单位");
    await waitFor(() => {
      const currentCh1Link = nav.getByRole("link", { name: "电流通道1" });
      if (currentCh1Link.getAttribute("aria-current") !== "page") {
        throw new Error('Expected "电流通道1" nav item to become active');
      }
    });
  },
};

export const OutputControlApplied: Story = {
  render: () => <CalibrationRouteStory devices={DEVICE_CAL_OUTPUT_APPLIED} />,
  play: async ({ canvas, canvasElement }) => {
    await canvas.findByRole(
      "heading",
      { name: "Calibration" },
      { timeout: 5_000 },
    );
    const nav = within(
      canvasElement.querySelector('[aria-label="系统页导航"]') as HTMLElement,
    );
    await userEvent.click(nav.getByRole("link", { name: "电流通道1" }));
    await canvas.findByText("Output control (CC)");

    await waitFor(() => {
      const modeBadge = findCalModeBadge(canvasElement);
      const currentStat = findCalibrationStat(canvasElement, "Active Current");
      const dacStat = findCalibrationStat(canvasElement, "DAC Code");
      if (!modeBadge || !currentStat || !dacStat) {
        throw new Error("Expected calibration current ll-stats to be rendered");
      }
      if (!(modeBadge.textContent?.includes("current_ch1") ?? false)) {
        throw new Error('Expected "cal_mode: current_ch1" badge on load');
      }
      if (!/1\.(6|7)\d{3} A/.test(currentStat.textContent ?? "")) {
        throw new Error(
          'Expected "Active Current" to show a non-zero applied current',
        );
      }
      if (!/16\d{2}|17\d{2}/.test(dacStat.textContent ?? "")) {
        throw new Error('Expected "DAC Code" to update to an applied value');
      }
    });
  },
};

export const RestoresStoredCurrentTab: Story = {
  render: () => (
    <CalibrationRouteStory
      beforeMount={({ calibrationStore }) =>
        seedCurrentCh2Draft(calibrationStore, "mock-001", "mock://demo-1")
      }
    />
  ),
  play: async ({ canvas, canvasElement }) => {
    await canvas.findByRole(
      "heading",
      { name: "Calibration" },
      { timeout: 5_000 },
    );
    const nav = within(
      canvasElement.querySelector('[aria-label="系统页导航"]') as HTMLElement,
    );
    await waitFor(() => {
      const currentCh2Link = nav.getByRole("link", { name: "电流通道2" });
      if (currentCh2Link.getAttribute("aria-current") !== "page") {
        throw new Error('Expected stored "电流通道2" nav item to be active');
      }
    });

    await waitFor(() => {
      const modeBadge = findCalModeBadge(canvasElement);
      if (!modeBadge?.textContent?.includes("current_ch2")) {
        throw new Error('Expected "cal_mode: current_ch2" badge on load');
      }
    });

    await canvas.findByRole("spinbutton", {
      name: "Meter Reading (Remote) (A) Capture",
    });
  },
};

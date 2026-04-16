import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
import type { StoredDevice } from "../../devices/device-store.ts";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

type CalibrationRouteStoryProps = {
  beforeMount?: () => void;
  devices?: StoredDevice[];
};

function getCalibrationDraftStorageKey(
  deviceId: string,
  baseUrl: string,
  version = 4,
): string {
  return `loadlynx:calibration-draft:v${version}:${deviceId}:${encodeURIComponent(baseUrl)}`;
}

function clearCalibrationDraftStorage(deviceId: string, baseUrl: string) {
  if (typeof window === "undefined") {
    return;
  }
  for (const version of [2, 3, 4]) {
    window.localStorage.removeItem(
      getCalibrationDraftStorageKey(deviceId, baseUrl, version),
    );
  }
}

function seedCurrentCh2Draft(deviceId: string, baseUrl: string) {
  if (typeof window === "undefined") {
    return;
  }
  clearCalibrationDraftStorage(deviceId, baseUrl);
  window.localStorage.setItem(
    getCalibrationDraftStorageKey(deviceId, baseUrl),
    JSON.stringify({
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
    }),
  );
}

function findCalModeBadge(canvasElement: HTMLElement): HTMLElement | null {
  return (
    Array.from(canvasElement.querySelectorAll<HTMLElement>(".badge")).find(
      (element) => element.textContent?.includes("cal_mode:"),
    ) ?? null
  );
}

function findCalibrationStat(
  canvasElement: HTMLElement,
  title: string,
): HTMLElement | null {
  const matches = Array.from(
    canvasElement.querySelectorAll<HTMLElement>(".stat"),
  ).filter(
    (element) =>
      element.querySelector(".stat-title")?.textContent?.trim() === title,
  );
  return matches.at(-1) ?? null;
}

function CalibrationRouteStory(props: CalibrationRouteStoryProps) {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/calibration"
      devices={props.devices}
      beforeMount={() => {
        clearCalibrationDraftStorage("mock-001", "mock://demo-1");
        props.beforeMount?.();
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
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await canvas.findByRole("heading", { name: "Calibration" });

    const currentCh1Tab = canvas.getByRole("tab", { name: "电流通道1" });
    await userEvent.click(currentCh1Tab);

    await canvas.findByText("电流单位");
    if (!(currentCh1Tab as HTMLElement).classList.contains("tab-active")) {
      throw new Error('Expected "电流通道1" tab to be active after click');
    }
  },
};

export const OutputControlApplied: Story = {
  render: () => <CalibrationRouteStory devices={DEVICE_CAL_OUTPUT_APPLIED} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await canvas.findByRole("heading", { name: "Calibration" });
    await userEvent.click(canvas.getByRole("tab", { name: "电流通道1" }));
    await canvas.findByText("Output control (CC)");

    await waitFor(() => {
      const modeBadge = findCalModeBadge(canvasElement);
      const currentStat = findCalibrationStat(canvasElement, "Active Current");
      const dacStat = findCalibrationStat(canvasElement, "DAC Code");
      if (!modeBadge || !currentStat || !dacStat) {
        throw new Error("Expected calibration current stats to be rendered");
      }
      if (!(modeBadge.textContent?.includes("current_ch1") ?? false)) {
        throw new Error('Expected "cal_mode: current_ch1" badge on load');
      }
      if (!(currentStat.textContent?.includes("1.7100 A") ?? false)) {
        throw new Error('Expected "Active Current" to update to 1.7100 A');
      }
      if (!(dacStat.textContent?.includes("1638") ?? false)) {
        throw new Error('Expected "DAC Code" to update to 1638');
      }
    });
  },
};

export const RestoresStoredCurrentTab: Story = {
  render: () => (
    <CalibrationRouteStory
      beforeMount={() => seedCurrentCh2Draft("mock-001", "mock://demo-1")}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await canvas.findByRole("heading", { name: "Calibration" });

    const currentCh2Tab = canvas.getByRole("tab", { name: "电流通道2" });
    if (!(currentCh2Tab as HTMLElement).classList.contains("tab-active")) {
      throw new Error('Expected stored "电流通道2" tab to be active on load');
    }

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

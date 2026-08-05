import { expect, test, vi } from "vitest";
import { createCalibrationModeCoordinator } from "./calibration-mode-coordinator.ts";

test("serializes calibration mode writes for one device", async () => {
  const releases: Array<() => void> = [];
  const write = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        releases.push(resolve);
      }),
  );
  const coordinate = createCalibrationModeCoordinator(write);

  const first = coordinate("http://device", { kind: "voltage" });
  const second = coordinate("http://device", { kind: "current_ch2" });
  await vi.waitFor(() => expect(write).toHaveBeenCalledTimes(1));

  releases.shift()?.();
  await first;
  await vi.waitFor(() => expect(write).toHaveBeenCalledTimes(2));

  releases.shift()?.();
  await second;
});

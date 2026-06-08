import { useCallback, useRef } from "react";
import { getStatus, postCalibrationMode } from "../../api/client.ts";
import type { CalibrationModeRequest } from "../../api/types.ts";
import type { CalibrationTab } from "./shared.ts";
import {
  formatDeviceCalKind,
  retryDeviceCall,
  type WithStatusStreamPaused,
} from "./shared.ts";

export function useCalibrationModeSync(input: {
  activeTab: CalibrationTab;
  baseUrl: string;
  expectedCalKind: number;
  getLatestCalKind: () => number | null;
  getLatestStatus: () => { raw: { cal_kind?: number | null } } | null;
  isOffline: boolean;
  onAlert: (title: string, body: string, details?: string[]) => void;
  publishStatusSnapshot: (view: Awaited<ReturnType<typeof getStatus>>) => void;
  waitForStatus: (
    predicate: (view: Awaited<ReturnType<typeof getStatus>>) => boolean,
    timeoutMs: number,
  ) => Promise<Awaited<ReturnType<typeof getStatus>>>;
  withStatusStreamPaused: WithStatusStreamPaused;
}) {
  const {
    activeTab,
    baseUrl,
    expectedCalKind,
    getLatestCalKind,
    getLatestStatus,
    isOffline,
    onAlert,
    publishStatusSnapshot,
    waitForStatus,
    withStatusStreamPaused,
  } = input;
  const modeSyncInFlightRef = useRef<Promise<void> | null>(null);

  return useCallback(
    async (action: string, opts?: { silent?: boolean }): Promise<boolean> => {
      if (modeSyncInFlightRef.current) {
        try {
          await modeSyncInFlightRef.current;
        } catch {
          // ignore
        }
      }

      if (getLatestStatus() !== null && isOffline) {
        return false;
      }

      const already = getLatestCalKind();
      if (already === expectedCalKind) {
        return true;
      }

      const kind: CalibrationModeRequest["kind"] =
        activeTab === "voltage" ? "voltage" : activeTab;

      let snapshotAfterCalKind: number | null = null;
      const attempt = (async (): Promise<void> => {
        await withStatusStreamPaused(async () => {
          await retryDeviceCall(() => postCalibrationMode(baseUrl, { kind }), {
            attempts: 4,
            firstDelayMs: 120,
            maxDelayMs: 600,
          });

          try {
            const snapshot = await retryDeviceCall(() => getStatus(baseUrl), {
              attempts: 2,
              firstDelayMs: 80,
              maxDelayMs: 300,
            });
            snapshotAfterCalKind = snapshot.raw.cal_kind ?? null;
            publishStatusSnapshot(snapshot);
          } catch {
            // keep waiting for the stream/fallback path
          }
        });
      })();

      modeSyncInFlightRef.current = attempt;
      try {
        await attempt;
      } catch {
        if (!opts?.silent) {
          onAlert(
            `Cannot ${action}`,
            "Failed to set device calibration mode. Check network/API availability.",
          );
        }
        return false;
      } finally {
        if (modeSyncInFlightRef.current === attempt) {
          modeSyncInFlightRef.current = null;
        }
      }

      if (snapshotAfterCalKind === expectedCalKind) {
        return true;
      }

      try {
        await waitForStatus(
          (view) => (view.raw.cal_kind ?? null) === expectedCalKind,
          1500,
        );
        return true;
      } catch {
        try {
          const snapshot = await retryDeviceCall(() => getStatus(baseUrl), {
            attempts: 2,
            firstDelayMs: 100,
            maxDelayMs: 400,
          });
          publishStatusSnapshot(snapshot);
          if ((snapshot.raw.cal_kind ?? null) === expectedCalKind) {
            return true;
          }
        } catch {
          // ignore final readback failure
        }

        if (!opts?.silent) {
          const seen = getLatestCalKind();
          onAlert(
            "Calibration mode mismatch",
            "Device did not switch to the expected calibration mode.",
            [
              `expected=${formatDeviceCalKind(expectedCalKind)}`,
              `device=${formatDeviceCalKind(seen)}`,
            ],
          );
        }
        return false;
      }
    },
    [
      activeTab,
      baseUrl,
      expectedCalKind,
      getLatestCalKind,
      getLatestStatus,
      isOffline,
      onAlert,
      publishStatusSnapshot,
      waitForStatus,
      withStatusStreamPaused,
    ],
  );
}

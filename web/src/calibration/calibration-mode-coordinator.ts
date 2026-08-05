import { postCalibrationMode } from "../api/client.ts";
import type { CalibrationModeRequest } from "../api/types.ts";

type WriteCalibrationMode = (
  baseUrl: string,
  request: CalibrationModeRequest,
) => Promise<unknown>;

export function createCalibrationModeCoordinator(write: WriteCalibrationMode) {
  const pendingByDevice = new Map<string, Promise<void>>();

  return async (baseUrl: string, request: CalibrationModeRequest) => {
    const previous = pendingByDevice.get(baseUrl) ?? Promise.resolve();
    const current = previous
      .catch(() => undefined)
      .then(async () => {
        await write(baseUrl, request);
      });
    pendingByDevice.set(baseUrl, current);

    try {
      await current;
    } finally {
      if (pendingByDevice.get(baseUrl) === current) {
        pendingByDevice.delete(baseUrl);
      }
    }
  };
}

export const coordinateCalibrationMode =
  createCalibrationModeCoordinator(postCalibrationMode);

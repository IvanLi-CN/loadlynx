import { isDevdCompatBaseUrl } from "../../api/client.ts";

export const HTTP_FAST_STATUS_REFETCH_MS = 400;
export const DEVD_FAST_STATUS_TARGET_PERIOD_MS = 200;

export function getFastStatusRefetchIntervalMs(
  baseUrl: string | undefined,
): number | false {
  if (!baseUrl) {
    return HTTP_FAST_STATUS_REFETCH_MS;
  }

  if (isDevdCompatBaseUrl(baseUrl)) {
    return false;
  }

  return HTTP_FAST_STATUS_REFETCH_MS;
}

export function usesManualDevdStatusPolling(
  baseUrl: string | undefined,
): boolean {
  return Boolean(baseUrl && isDevdCompatBaseUrl(baseUrl));
}

export function getManualStatusPollDelayMs(
  targetPeriodMs: number,
  cycleStartedAtMs: number,
  nowMs: number,
): number {
  return Math.max(0, targetPeriodMs - Math.max(0, nowMs - cycleStartedAtMs));
}

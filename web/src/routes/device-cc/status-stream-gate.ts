export const STREAM_UI_INTERVAL_MS = 200;

export function getStatusRenderDelay(
  lastCommitAtMs: number | null,
  nowMs: number,
  minIntervalMs = STREAM_UI_INTERVAL_MS,
): number {
  if (lastCommitAtMs == null) {
    return 0;
  }

  return Math.max(0, minIntervalMs - (nowMs - lastCommitAtMs));
}

export function shouldCommitStatusImmediately(
  lastCommitAtMs: number | null,
  nowMs: number,
  minIntervalMs = STREAM_UI_INTERVAL_MS,
): boolean {
  return getStatusRenderDelay(lastCommitAtMs, nowMs, minIntervalMs) === 0;
}

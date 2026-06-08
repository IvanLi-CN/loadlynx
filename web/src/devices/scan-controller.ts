export function beginManagedScan(
  activeController: AbortController | null,
): AbortController {
  activeController?.abort();
  return new AbortController();
}

export function cancelManagedScan(
  activeController: AbortController | null,
): null {
  activeController?.abort();
  return null;
}

export function isManagedScanCurrent(
  activeController: AbortController | null,
  candidate: AbortController,
): boolean {
  return activeController === candidate && !candidate.signal.aborted;
}

export function clearManagedScan(
  activeController: AbortController | null,
  candidate: AbortController,
): AbortController | null {
  return activeController === candidate ? null : activeController;
}

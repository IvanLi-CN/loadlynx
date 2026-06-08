import { expect, test } from "vitest";

import {
  beginManagedScan,
  cancelManagedScan,
  clearManagedScan,
  isManagedScanCurrent,
} from "./scan-controller.ts";

test("beginManagedScan aborts the previous controller before replacing it", () => {
  const previous = new AbortController();

  const next = beginManagedScan(previous);

  expect(previous.signal.aborted).toBe(true);
  expect(next).not.toBe(previous);
  expect(next.signal.aborted).toBe(false);
});

test("cancelManagedScan aborts the active controller and clears it", () => {
  const active = new AbortController();

  const next = cancelManagedScan(active);

  expect(active.signal.aborted).toBe(true);
  expect(next).toBeNull();
});

test("clearManagedScan only clears the currently active controller", () => {
  const active = new AbortController();
  const stale = new AbortController();

  expect(clearManagedScan(active, stale)).toBe(active);
  expect(clearManagedScan(active, active)).toBeNull();
});

test("isManagedScanCurrent rejects stale or aborted controllers", () => {
  const active = new AbortController();
  const stale = new AbortController();

  expect(isManagedScanCurrent(active, active)).toBe(true);
  expect(isManagedScanCurrent(active, stale)).toBe(false);

  active.abort();
  expect(isManagedScanCurrent(active, active)).toBe(false);
});

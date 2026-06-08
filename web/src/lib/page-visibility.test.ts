import { expect, test, vi } from "vitest";

import {
  observePageVisibility,
  readPageVisibility,
} from "./page-visibility.ts";

test("readPageVisibility defaults to visible when document is unavailable", () => {
  expect(readPageVisibility(null)).toBe(true);
});

test("readPageVisibility reflects hidden documents", () => {
  expect(readPageVisibility({ visibilityState: "hidden" })).toBe(false);
});

test("observePageVisibility emits current visibility changes and unsubscribes", () => {
  let listener: (() => void) | null = null;
  const doc = {
    visibilityState: "visible",
    addEventListener: vi.fn((_type: "visibilitychange", next: () => void) => {
      listener = next;
    }),
    removeEventListener: vi.fn(),
  };
  const onChange = vi.fn();

  const cleanup = observePageVisibility(doc, onChange);

  expect(doc.addEventListener).toHaveBeenCalledTimes(1);
  doc.visibilityState = "hidden";
  listener?.();
  expect(onChange).toHaveBeenCalledWith(false);

  cleanup();

  expect(doc.removeEventListener).toHaveBeenCalledTimes(1);
});

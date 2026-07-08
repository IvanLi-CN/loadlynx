import { expect, test } from "vitest";
import { hasRemoteAppUpdate } from "./version-check.ts";

test("reports an update when version.json differs from the running build", () => {
  expect(
    hasRemoteAppUpdate("0.1.0+00eea1b", {
      version: "0.1.0+11aa22b",
      builtAt: "2026-07-08T03:34:01.828Z",
    }),
  ).toBe(true);
});

test("does not report an update when the versions match", () => {
  expect(
    hasRemoteAppUpdate("0.1.0+00eea1b", {
      version: "0.1.0+00eea1b",
      builtAt: "2026-07-08T03:34:01.828Z",
    }),
  ).toBe(false);
});

test("ignores empty or malformed version values", () => {
  expect(hasRemoteAppUpdate("0.1.0+00eea1b", null)).toBe(false);
  expect(hasRemoteAppUpdate("0.1.0+00eea1b", { version: "" })).toBe(false);
  expect(hasRemoteAppUpdate("", { version: "0.1.0+11aa22b" })).toBe(false);
});

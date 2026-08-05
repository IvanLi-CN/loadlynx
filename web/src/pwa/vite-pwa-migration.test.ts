import { describe, expect, test } from "vitest";
import { pwaMigrationWorkerOptions } from "../../vite.config.ts";

describe("PWA migration worker options", () => {
  test("ordinary production builds remain prompt-driven", () => {
    expect(pwaMigrationWorkerOptions(undefined)).toEqual({
      clientsClaim: false,
      skipWaiting: false,
    });
  });

  test("the explicit migration build enables takeover", () => {
    expect(pwaMigrationWorkerOptions("1")).toEqual({
      clientsClaim: true,
      skipWaiting: true,
    });
  });
});

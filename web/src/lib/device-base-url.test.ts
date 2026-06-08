import { expect, test } from "vitest";

import { requireDeviceBaseUrl } from "./device-base-url.ts";

test("requireDeviceBaseUrl returns the url when present", () => {
  expect(requireDeviceBaseUrl("http://device.local")).toBe(
    "http://device.local",
  );
});

test("requireDeviceBaseUrl throws for missing urls", () => {
  expect(() => requireDeviceBaseUrl(undefined)).toThrow(
    "Device base URL is not available",
  );
});

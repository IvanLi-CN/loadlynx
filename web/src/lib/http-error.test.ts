import { expect, test } from "vitest";

import { HttpApiError } from "../api/client.ts";
import {
  formatHttpApiErrorSummary,
  getNetworkErrorHint,
  getUsbSerialErrorHint,
  isAnalogNotReadyError,
  isLinkUnavailableError,
  isUnsupportedOperationError,
  isUsbSerialUnavailableError,
} from "./http-error.ts";

function makeError(
  input: Partial<ConstructorParameters<typeof HttpApiError>[0]>,
): HttpApiError {
  return new HttpApiError({
    status: input.status ?? 500,
    code: input.code ?? "HTTP_ERROR",
    message: input.message ?? "boom",
    retryable: input.retryable ?? false,
    details: input.details ?? null,
  });
}

test("formatHttpApiErrorSummary uses code and message", () => {
  expect(
    formatHttpApiErrorSummary(
      makeError({ code: "NETWORK_ERROR", message: "offline" }),
    ),
  ).toBe("NETWORK_ERROR — offline");
});

test("helpers classify unsupported and transient device errors", () => {
  expect(
    isUnsupportedOperationError(
      makeError({ status: 404, code: "UNSUPPORTED_OPERATION" }),
    ),
  ).toBe(true);
  expect(isLinkUnavailableError(makeError({ code: "LINK_DOWN" }))).toBe(true);
  expect(isUsbSerialUnavailableError(makeError({ code: "device_busy" }))).toBe(
    true,
  );
  expect(
    isUsbSerialUnavailableError(makeError({ code: "web_session_expired" })),
  ).toBe(true);
  expect(isAnalogNotReadyError(makeError({ code: "ANALOG_NOT_READY" }))).toBe(
    true,
  );
  expect(isAnalogNotReadyError(makeError({ code: "NOT_ATTACHED" }))).toBe(true);
});

test("getNetworkErrorHint includes baseUrl when provided", () => {
  expect(getNetworkErrorHint("http://device.local")).toContain(
    "baseUrl=http://device.local",
  );
  expect(getNetworkErrorHint()).not.toContain("baseUrl=");
});

test("getUsbSerialErrorHint explains the affected baseUrl", () => {
  expect(getUsbSerialErrorHint("http://127.0.0.1:19390")).toContain(
    "baseUrl=http://127.0.0.1:19390",
  );
});

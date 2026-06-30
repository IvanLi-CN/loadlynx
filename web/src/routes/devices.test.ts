import { expect, test } from "vitest";

import { HttpApiError } from "../api/client.ts";
import type { FastStatusView } from "../api/types.ts";
import { getOverviewTopRightDetail } from "./devices.tsx";

function t(key: string): string {
  const translations: Record<string, string> = {
    "overview.checkingRequest": "Probing /api/v1/identity",
    "overview.networkRetry": "Network error, retrying automatically.",
    "overview.offline": "Offline",
  };
  return translations[key] ?? key;
}

function makeHttpError(input: {
  status: number;
  code: string;
  message?: string;
}): HttpApiError {
  return new HttpApiError({
    status: input.status,
    code: input.code,
    message: input.message ?? input.code,
    retryable: false,
    details: null,
  });
}

test("overview top-right detail hides raw endpoint while identity loads", () => {
  expect(
    getOverviewTopRightDetail({
      t,
      status: undefined,
      statusError: null,
      identityError: null,
      identityLoading: true,
      linkValue: "Checking",
      protectionValue: "Pending",
      fallbackDetail: "mock://demo-1",
    }),
  ).toBe("Probing /api/v1/identity");
});

test("overview top-right detail hides raw endpoint on network identity failure", () => {
  expect(
    getOverviewTopRightDetail({
      t,
      status: undefined,
      statusError: null,
      identityError: makeHttpError({
        status: 0,
        code: "NETWORK_ERROR",
      }),
      identityLoading: false,
      linkValue: "Offline",
      protectionValue: "Pending",
      fallbackDetail: "http://192.168.0.20",
    }),
  ).toBe("Network error, retrying automatically.");
});

test("overview top-right detail prioritizes live status summary", () => {
  expect(
    getOverviewTopRightDetail({
      t,
      status: {} as FastStatusView,
      statusError: null,
      identityError: null,
      identityLoading: false,
      linkValue: "Link up",
      protectionValue: "All clear",
      fallbackDetail: "No live identity",
    }),
  ).toBe("Link up · All clear");
});

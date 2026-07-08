import { expect, test } from "vitest";
import {
  assertWifiWriteAllowed,
  deleteWifiConfig,
  makeWifiSetRequest,
  postWifiConfig,
} from "./client-backup.ts";
import { HttpApiError } from "./client-core.ts";

test("blocks WiFi writes over unverified direct HTTP", () => {
  expect(() => assertWifiWriteAllowed("http://192.0.2.55")).toThrow(
    HttpApiError,
  );
  try {
    assertWifiWriteAllowed("http://192.0.2.55");
  } catch (error) {
    expect(error).toMatchObject({
      code: "WIFI_WRITE_REQUIRES_LOCAL_CONNECTION",
      retryable: false,
    });
  }
});

test("requires verified identity for devd WiFi writes", () => {
  expect(() =>
    assertWifiWriteAllowed(
      "http://127.0.0.1:30180/?device_id=digital-aabbcc&lease_id=lease-1",
    ),
  ).toThrow(HttpApiError);
  try {
    assertWifiWriteAllowed(
      "http://127.0.0.1:30180/?device_id=digital-aabbcc&lease_id=lease-1",
    );
  } catch (error) {
    expect(error).toMatchObject({
      code: "WIFI_WRITE_REQUIRES_VERIFIED_LOCAL_CONNECTION",
      retryable: false,
    });
  }
});

test("allows WiFi writes only over mock and verified devd compat links", () => {
  expect(() => assertWifiWriteAllowed("mock://demo-1")).not.toThrow();
  expect(() =>
    assertWifiWriteAllowed(
      "http://127.0.0.1:30180/?device_id=digital-aabbcc&lease_id=lease-1",
      { identityVerified: true },
    ),
  ).not.toThrow();
});

test("acks WiFi save after storing credentials instead of waiting for connection", () => {
  expect(makeWifiSetRequest("BenchNet", "bench-pass")).toMatchObject({
    ssid: "BenchNet",
    psk: "bench-pass",
    wait: false,
  });
});

test("mock WiFi clear can reproduce a device that ignores clear", async () => {
  const wifi = await deleteWifiConfig("mock://wifi-clear-noop");

  expect(wifi).toMatchObject({
    ssid: "BenchNet",
    source: "user",
    state: "connected",
    ip: "192.168.31.216",
  });
});

test("recovers WiFi save when USB response times out after device applied it", async () => {
  const wifi = await postWifiConfig("mock://wifi-set-timeout-success", {
    ssid: "BenchNet",
    psk: "bench-pass",
    wait: false,
  });

  expect(wifi).toMatchObject({
    ssid: "BenchNet",
    source: "user",
    state: "configured",
    last_error: null,
  });
});

test("does not recover explicit EEPROM write failure as success", async () => {
  await expect(
    postWifiConfig("mock://wifi-set-eeprom-error", {
      ssid: "BenchNet",
      psk: "bench-pass",
      wait: false,
    }),
  ).rejects.toMatchObject({
    code: "UNAVAILABLE",
    message: "EEPROM write failed",
  });
});

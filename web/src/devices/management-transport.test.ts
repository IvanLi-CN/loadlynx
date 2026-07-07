import { expect, test } from "vitest";

import {
  formatDeviceSwitcherLabel,
  getManagementTransport,
  getManagementTransportLabel,
  isWifiTransportBaseUrl,
  isWifiWriteTransportVerified,
} from "./management-transport.ts";

test("getManagementTransport classifies LAN/WiFi and USB devd base URLs", () => {
  expect(getManagementTransport("http://192.168.31.216")).toBe("lan-http");
  expect(getManagementTransport("http://loadlynx-d68638.local")).toBe(
    "lan-http",
  );
  expect(
    getManagementTransport(
      "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
    ),
  ).toBe("usb-devd");
  expect(getManagementTransport("mock://demo-1")).toBe("mock");
  expect(getManagementTransport(undefined)).toBe("unknown");
});

test("isWifiTransportBaseUrl only flags direct HTTP management paths", () => {
  expect(isWifiTransportBaseUrl("http://192.168.31.216")).toBe(true);
  expect(
    isWifiTransportBaseUrl(
      "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
    ),
  ).toBe(false);
});

test("isWifiWriteTransportVerified requires a verified independent path", () => {
  const devdUrl =
    "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1";

  expect(isWifiWriteTransportVerified("mock://demo-1", false)).toBe(true);
  expect(isWifiWriteTransportVerified(devdUrl, true)).toBe(true);
  expect(isWifiWriteTransportVerified(devdUrl, false)).toBe(false);
  expect(isWifiWriteTransportVerified("http://192.168.31.216", true)).toBe(
    false,
  );
  expect(isWifiWriteTransportVerified(undefined, true)).toBe(false);
});

test("formatDeviceSwitcherLabel includes the active management transport", () => {
  expect(
    formatDeviceSwitcherLabel({
      id: "device-001",
      name: "LoadLynx d68638 WiFi",
      baseUrl:
        "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
    }),
  ).toBe("USB · LoadLynx d68638 WiFi (device-001)");
  expect(getManagementTransportLabel("http://192.168.31.216")).toBe("WiFi");
});

import type { StoredDevice } from "./device-store.ts";

export type ManagementTransport = "lan-http" | "usb-devd" | "mock" | "unknown";

export function getManagementTransport(
  baseUrl: string | undefined,
): ManagementTransport {
  if (!baseUrl) {
    return "unknown";
  }

  const normalized = baseUrl.trim().toLowerCase();
  if (normalized.startsWith("mock://")) {
    return "mock";
  }

  try {
    const url = new URL(baseUrl);
    if (url.searchParams.get("device_id") && url.searchParams.get("lease_id")) {
      return "usb-devd";
    }
    if (url.protocol === "http:" || url.protocol === "https:") {
      return "lan-http";
    }
  } catch {
    return "unknown";
  }

  return "unknown";
}

export function isWifiTransportBaseUrl(baseUrl: string | undefined): boolean {
  return getManagementTransport(baseUrl) === "lan-http";
}

export function isWifiWriteTransportVerified(
  baseUrl: string | undefined,
  identityVerified: boolean,
): boolean {
  const transport = getManagementTransport(baseUrl);
  return transport === "mock" || (transport === "usb-devd" && identityVerified);
}

export function getManagementTransportLabel(
  baseUrl: string | undefined,
): string {
  const transport = getManagementTransport(baseUrl);
  if (transport === "lan-http") {
    return "WiFi";
  }
  if (transport === "usb-devd") {
    return "USB";
  }
  if (transport === "mock") {
    return "Mock";
  }
  return "Unknown";
}

export function formatDeviceSwitcherLabel(device: StoredDevice): string {
  return `${getManagementTransportLabel(device.baseUrl)} · ${device.name} (${device.id})`;
}

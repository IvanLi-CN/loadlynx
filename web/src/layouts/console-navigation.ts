import type { StoredDevice } from "../devices/device-store.ts";

export type PrimarySection = "overview" | "dashboard" | "system";
export type DeviceRouteKind =
  | "cc"
  | "status"
  | "settings"
  | "calibration"
  | "firmware"
  | "about";

export function getPrimarySection(
  pathname: string,
  deviceId?: string,
): PrimarySection {
  if (pathname === "/" || pathname === "/devices") {
    return "overview";
  }
  if (!deviceId) {
    return "overview";
  }

  const segments = pathname.split("/").filter(Boolean);
  if (segments[0] !== deviceId) {
    return "overview";
  }
  if (segments[1] === "cc" || segments[1] === "pd") {
    return "dashboard";
  }
  return "system";
}

export function getConnectionLabels(device: StoredDevice): string[] {
  const labels = new Set<string>();
  const normalizedBaseUrl = device.baseUrl.trim().toLowerCase();

  if (device.connectionMarks?.includes("lan")) {
    labels.add("LAN");
  }
  if (
    normalizedBaseUrl.startsWith("http://") ||
    normalizedBaseUrl.startsWith("https://")
  ) {
    labels.add("HTTP");
  }
  if (device.connectionMarks?.includes("usb") || device.devd) {
    labels.add("USB");
  }
  if (device.devd) {
    labels.add("devd");
  }
  if (device.webSerial) {
    labels.add("Web Serial");
  }
  if (device.connectionMarks?.includes("digital_flash")) {
    labels.add("Digital Flash");
  }
  if (device.connectionMarks?.includes("analog_flash")) {
    labels.add("Analog Flash");
  }

  return Array.from(labels);
}

export function getDeviceRouteIntentFromHref(href: string | null | undefined): {
  route: DeviceRouteKind;
  panel?: "pd";
} {
  const fallback = { route: "cc" as const };
  if (!href) {
    return fallback;
  }

  let url: URL;
  try {
    url = new URL(href, "https://loadlynx.local");
  } catch {
    return fallback;
  }

  const segments = url.pathname.split("/").filter(Boolean);
  const route = segments[1];

  if (
    route === "status" ||
    route === "settings" ||
    route === "calibration" ||
    route === "firmware" ||
    route === "about"
  ) {
    return { route };
  }

  if (route === "pd") {
    return { route: "cc", panel: "pd" };
  }

  if (route === "cc") {
    return {
      route: "cc",
      panel: url.searchParams.get("panel") === "pd" ? "pd" : undefined,
    };
  }

  return fallback;
}

export function buildDeviceHref(
  deviceId: string,
  route: DeviceRouteKind,
  panel?: "pd",
): string {
  const path = route === "cc" ? `/${deviceId}/cc` : `/${deviceId}/${route}`;
  return route === "cc" && panel === "pd" ? `${path}?panel=pd` : path;
}

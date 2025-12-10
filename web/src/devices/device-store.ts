import { ENABLE_MOCK } from "../api/client.ts";

export interface StoredDevice {
  id: string;
  name: string;
  baseUrl: string;
}

const STORAGE_KEY = "loadlynx.devices";

const DEFAULT_DEVICES: StoredDevice[] = [
  {
    id: "llx-dev-001",
    name: "Demo Device #1",
    baseUrl: "mock://demo-1",
  },
];

function getDefaultDevices(): StoredDevice[] {
  if (!ENABLE_MOCK) {
    return [];
  }
  return DEFAULT_DEVICES;
}

function isBrowser(): boolean {
  return (
    typeof window !== "undefined" && typeof window.localStorage !== "undefined"
  );
}

export function loadDevices(): StoredDevice[] {
  if (!isBrowser()) {
    return getDefaultDevices();
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return getDefaultDevices();
    }
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) {
      return getDefaultDevices();
    }

    const devices: StoredDevice[] = [];
    for (const item of parsed) {
      if (
        item &&
        typeof item === "object" &&
        typeof (item as StoredDevice).id === "string" &&
        typeof (item as StoredDevice).name === "string" &&
        typeof (item as StoredDevice).baseUrl === "string"
      ) {
        devices.push({
          id: (item as StoredDevice).id,
          name: (item as StoredDevice).name,
          baseUrl: (item as StoredDevice).baseUrl,
        });
      }
    }

    const normalized = ENABLE_MOCK
      ? devices
      : devices.filter(
          (device) => !device.baseUrl.toLowerCase().startsWith("mock://"),
        );

    if (normalized.length > 0) {
      return normalized;
    }

    return getDefaultDevices();
  } catch {
    // If parsing fails, fall back to a safe default.
    return getDefaultDevices();
  }
}

export function saveDevices(devices: StoredDevice[]): void {
  if (!isBrowser()) {
    return;
  }
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(devices));
  } catch {
    // Best-effort only; UI can still function from in-memory state.
  }
}

export interface StoredDevice {
  id: string;
  name: string;
  baseUrl: string;
}

const STORAGE_KEY = "loadlynx.devices";

function getDefaultDevices(): StoredDevice[] {
  return [];
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

    if (devices.length > 0) {
      return devices;
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

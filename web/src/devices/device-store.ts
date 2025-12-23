export interface StoredDevice {
  id: string;
  name: string;
  baseUrl: string;
}

const STORAGE_KEY = "loadlynx.devices";

export interface DeviceStore {
  getDevices(): StoredDevice[];
  setDevices(devices: StoredDevice[]): void;
}

function sanitizeDevices(input: unknown): StoredDevice[] {
  if (!Array.isArray(input)) {
    return [];
  }

  const devices: StoredDevice[] = [];
  for (const item of input) {
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

  return devices;
}

export class LocalStorageDeviceStore implements DeviceStore {
  readonly #storage: Storage;
  readonly #key: string;

  constructor(storage: Storage, key: string = STORAGE_KEY) {
    this.#storage = storage;
    this.#key = key;
  }

  getDevices(): StoredDevice[] {
    try {
      const raw = this.#storage.getItem(this.#key);
      if (!raw) {
        return [];
      }
      const parsed = JSON.parse(raw) as unknown;
      return sanitizeDevices(parsed);
    } catch {
      return [];
    }
  }

  setDevices(devices: StoredDevice[]): void {
    try {
      this.#storage.setItem(this.#key, JSON.stringify(devices));
    } catch {
      // Best-effort only; UI can still function from in-memory state.
    }
  }
}

export class MemoryDeviceStore implements DeviceStore {
  #devices: StoredDevice[];

  constructor(initialDevices: StoredDevice[] = []) {
    this.#devices = [...initialDevices];
  }

  getDevices(): StoredDevice[] {
    return [...this.#devices];
  }

  setDevices(devices: StoredDevice[]): void {
    this.#devices = [...devices];
  }
}

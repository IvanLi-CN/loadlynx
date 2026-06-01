import { readStoredDemoMode } from "../lib/demo-mode.ts";

export interface StoredDevice {
  id: string;
  name: string;
  baseUrl: string;
  connectionMarks?: Array<"lan" | "usb" | "digital_flash" | "analog_flash">;
  devd?: {
    baseUrl: string;
    deviceId: string;
    leaseId?: string;
  };
}

const STORAGE_KEY = "loadlynx.devices";
const DEMO_STORAGE_KEY = "loadlynx.demo.devices";

export const DEMO_DEVICES: StoredDevice[] = [
  {
    id: "mock-001",
    name: "Demo Device #1",
    baseUrl: "mock://demo-1",
  },
  {
    id: "mock-002",
    name: "Demo Device #2",
    baseUrl: "mock://demo-2",
  },
];

export interface DeviceStore {
  getDevices(): StoredDevice[];
  setDevices(devices: StoredDevice[]): void;
}

function isDemoDevice(device: StoredDevice): boolean {
  return device.baseUrl.trim().toLowerCase().startsWith("mock://");
}

function sanitizeDevices(input: unknown): StoredDevice[] {
  if (!Array.isArray(input)) {
    return [];
  }

  const devices: StoredDevice[] = [];
  for (const item of input) {
    const stored = item as StoredDevice;
    const devd = stored.devd;
    if (
      item &&
      typeof item === "object" &&
      typeof stored.id === "string" &&
      typeof stored.name === "string" &&
      typeof stored.baseUrl === "string"
    ) {
      devices.push({
        id: stored.id,
        name: stored.name,
        baseUrl: stored.baseUrl,
        connectionMarks: Array.isArray(stored.connectionMarks)
          ? stored.connectionMarks
          : undefined,
        devd:
          devd &&
          typeof devd.baseUrl === "string" &&
          typeof devd.deviceId === "string"
            ? {
                baseUrl: devd.baseUrl,
                deviceId: devd.deviceId,
                leaseId:
                  typeof devd.leaseId === "string" ? devd.leaseId : undefined,
              }
            : undefined,
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

export class DemoAwareDeviceStore implements DeviceStore {
  readonly #storage: Storage;
  readonly #realStore: DeviceStore;
  readonly #demoStore: DeviceStore;

  constructor(storage: Storage) {
    this.#storage = storage;
    this.#realStore = new LocalStorageDeviceStore(storage, STORAGE_KEY);
    this.#demoStore = new LocalStorageDeviceStore(storage, DEMO_STORAGE_KEY);
  }

  getDevices(): StoredDevice[] {
    const store = this.#getActiveStore();
    const devices = store.getDevices();

    if (store === this.#demoStore) {
      const demoDevices = devices.filter(isDemoDevice);
      if (demoDevices.length !== devices.length) {
        store.setDevices(demoDevices);
      }
      if (demoDevices.length > 0) {
        return demoDevices;
      }

      store.setDevices(DEMO_DEVICES);
      return [...DEMO_DEVICES];
    }

    return devices;
  }

  setDevices(devices: StoredDevice[]): void {
    const store = this.#getActiveStore();
    store.setDevices(
      store === this.#demoStore ? devices.filter(isDemoDevice) : devices,
    );
  }

  #getActiveStore(): DeviceStore {
    return readStoredDemoMode(this.#storage) === true
      ? this.#demoStore
      : this.#realStore;
  }
}

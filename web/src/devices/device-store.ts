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
  webSerial?: {
    identityDeviceId: string;
    displayName?: string;
    profileCapturedAt?: string;
  };
}

const STORAGE_KEY = "loadlynx.devices";
const DEMO_STORAGE_KEY = "loadlynx.demo.devices";
const LAST_ACTIVE_DEVICE_KEY = "loadlynx.last-active-device-id";

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
  getLastActiveDeviceId(): string | null;
  setLastActiveDeviceId(deviceId: string | null): void;
}

function cloneStoredDevice(device: StoredDevice): StoredDevice {
  return {
    id: device.id,
    name: device.name,
    baseUrl: device.baseUrl,
    connectionMarks: device.connectionMarks
      ? [...device.connectionMarks]
      : undefined,
    devd: device.devd
      ? {
          baseUrl: device.devd.baseUrl,
          deviceId: device.devd.deviceId,
          leaseId: device.devd.leaseId,
        }
      : undefined,
    webSerial: device.webSerial
      ? {
          identityDeviceId: device.webSerial.identityDeviceId,
          displayName: device.webSerial.displayName,
          profileCapturedAt: device.webSerial.profileCapturedAt,
        }
      : undefined,
  };
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
    const webSerial = stored.webSerial;
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
        webSerial:
          webSerial && typeof webSerial.identityDeviceId === "string"
            ? {
                identityDeviceId: webSerial.identityDeviceId,
                displayName:
                  typeof webSerial.displayName === "string"
                    ? webSerial.displayName
                    : undefined,
                profileCapturedAt:
                  typeof webSerial.profileCapturedAt === "string"
                    ? webSerial.profileCapturedAt
                    : undefined,
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
  readonly #lastActiveDeviceKey: string;

  constructor(
    storage: Storage,
    key: string = STORAGE_KEY,
    lastActiveDeviceKey: string = LAST_ACTIVE_DEVICE_KEY,
  ) {
    this.#storage = storage;
    this.#key = key;
    this.#lastActiveDeviceKey = lastActiveDeviceKey;
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

  getLastActiveDeviceId(): string | null {
    try {
      const raw = this.#storage.getItem(this.#lastActiveDeviceKey);
      return raw && raw.trim().length > 0 ? raw : null;
    } catch {
      return null;
    }
  }

  setLastActiveDeviceId(deviceId: string | null): void {
    try {
      if (!deviceId) {
        this.#storage.removeItem(this.#lastActiveDeviceKey);
        return;
      }
      this.#storage.setItem(this.#lastActiveDeviceKey, deviceId);
    } catch {
      // Best-effort only; UI can still function from in-memory state.
    }
  }
}

export class MemoryDeviceStore implements DeviceStore {
  #devices: StoredDevice[];
  #lastActiveDeviceId: string | null = null;

  constructor(initialDevices: StoredDevice[] = []) {
    this.#devices = initialDevices.map(cloneStoredDevice);
  }

  getDevices(): StoredDevice[] {
    return this.#devices.map(cloneStoredDevice);
  }

  setDevices(devices: StoredDevice[]): void {
    this.#devices = devices.map(cloneStoredDevice);
  }

  getLastActiveDeviceId(): string | null {
    return this.#lastActiveDeviceId;
  }

  setLastActiveDeviceId(deviceId: string | null): void {
    this.#lastActiveDeviceId = deviceId;
  }
}

export class DemoAwareDeviceStore implements DeviceStore {
  readonly #storage: Storage;
  readonly #realStore: DeviceStore;
  readonly #demoStore: DeviceStore;
  readonly #realLastActiveStore: LocalStorageDeviceStore;
  readonly #demoLastActiveStore: LocalStorageDeviceStore;

  constructor(storage: Storage) {
    this.#storage = storage;
    this.#realLastActiveStore = new LocalStorageDeviceStore(
      storage,
      STORAGE_KEY,
      LAST_ACTIVE_DEVICE_KEY,
    );
    this.#demoLastActiveStore = new LocalStorageDeviceStore(
      storage,
      DEMO_STORAGE_KEY,
      `${LAST_ACTIVE_DEVICE_KEY}.demo`,
    );
    this.#realStore = this.#realLastActiveStore;
    this.#demoStore = this.#demoLastActiveStore;
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
        return demoDevices.map(cloneStoredDevice);
      }

      store.setDevices(DEMO_DEVICES);
      return DEMO_DEVICES.map(cloneStoredDevice);
    }

    return devices;
  }

  setDevices(devices: StoredDevice[]): void {
    const store = this.#getActiveStore();
    store.setDevices(
      store === this.#demoStore ? devices.filter(isDemoDevice) : devices,
    );
  }

  getLastActiveDeviceId(): string | null {
    return this.#getActiveStore().getLastActiveDeviceId();
  }

  setLastActiveDeviceId(deviceId: string | null): void {
    this.#getActiveStore().setLastActiveDeviceId(deviceId);
  }

  #getActiveStore(): DeviceStore {
    return readStoredDemoMode(this.#storage) === true
      ? this.#demoStore
      : this.#realStore;
  }
}

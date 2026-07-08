import { readStoredDemoMode } from "../lib/demo-mode.ts";

export interface StoredDevice {
  id: string;
  name: string;
  baseUrl: string;
  identityDeviceId?: string;
  connectionMarks?: Array<"lan" | "usb" | "digital_flash" | "analog_flash">;
  lan?: {
    baseUrl: string;
  };
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
    identityDeviceId: device.identityDeviceId,
    connectionMarks: device.connectionMarks
      ? [...device.connectionMarks]
      : undefined,
    lan: device.lan
      ? {
          baseUrl: device.lan.baseUrl,
        }
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

function normalizeIdentityDeviceId(device: StoredDevice): string | undefined {
  const identityDeviceId =
    typeof device.identityDeviceId === "string"
      ? device.identityDeviceId.trim()
      : "";
  if (identityDeviceId) {
    return identityDeviceId;
  }

  const webSerialIdentityDeviceId =
    typeof device.webSerial?.identityDeviceId === "string"
      ? device.webSerial.identityDeviceId.trim()
      : "";
  return webSerialIdentityDeviceId || undefined;
}

const CONNECTION_MARK_ORDER: NonNullable<StoredDevice["connectionMarks"]> = [
  "lan",
  "usb",
  "digital_flash",
  "analog_flash",
];

function mergeConnectionMarks(
  first: StoredDevice["connectionMarks"],
  second: StoredDevice["connectionMarks"],
): StoredDevice["connectionMarks"] {
  const marks = new Set([...(first ?? []), ...(second ?? [])]);
  const ordered = CONNECTION_MARK_ORDER.filter((mark) => marks.has(mark));
  return ordered.length > 0 ? ordered : undefined;
}

function isLanBaseUrl(baseUrl: string): boolean {
  try {
    const url = new URL(baseUrl);
    const hostname = url.hostname.toLowerCase();
    return (
      hostname !== "localhost" && hostname !== "127.0.0.1" && hostname !== "::1"
    );
  } catch {
    return false;
  }
}

function normalizeLanEndpoint(device: StoredDevice): StoredDevice["lan"] {
  const storedLanBaseUrl =
    typeof device.lan?.baseUrl === "string" ? device.lan.baseUrl.trim() : "";
  if (storedLanBaseUrl && isLanBaseUrl(storedLanBaseUrl)) {
    return { baseUrl: storedLanBaseUrl };
  }

  const baseUrl = device.baseUrl.trim();
  if (baseUrl && isLanBaseUrl(baseUrl)) {
    return { baseUrl };
  }

  return undefined;
}

function isUsbDevdBaseUrl(device: StoredDevice): boolean {
  if (device.devd) {
    return true;
  }

  try {
    const url = new URL(device.baseUrl);
    const hostname = url.hostname.toLowerCase();
    return (
      (hostname === "localhost" ||
        hostname === "127.0.0.1" ||
        hostname === "::1") &&
      url.searchParams.has("lease_id")
    );
  } catch {
    return false;
  }
}

function inferConnectionMarks(
  device: StoredDevice,
): StoredDevice["connectionMarks"] {
  const marks = new Set(device.connectionMarks ?? []);
  if (isLanBaseUrl(device.baseUrl)) {
    marks.add("lan");
  }
  if (isUsbDevdBaseUrl(device)) {
    marks.add("usb");
  }
  return mergeConnectionMarks([...marks], undefined);
}

function preferredPrimaryDevice(
  existing: StoredDevice,
  incoming: StoredDevice,
): StoredDevice {
  const existingIsLan = isLanBaseUrl(existing.baseUrl);
  const incomingIsLan = isLanBaseUrl(incoming.baseUrl);
  if (incomingIsLan && !existingIsLan) {
    return incoming;
  }
  if (existingIsLan && !incomingIsLan) {
    return existing;
  }
  return incoming;
}

function mergeStoredDevice(existing: StoredDevice, incoming: StoredDevice) {
  const primary = preferredPrimaryDevice(existing, incoming);
  const lan = normalizeLanEndpoint(incoming) ?? normalizeLanEndpoint(existing);

  return {
    ...existing,
    name: primary.name || existing.name || incoming.name,
    baseUrl: primary.baseUrl || existing.baseUrl || incoming.baseUrl,
    identityDeviceId:
      normalizeIdentityDeviceId(incoming) ??
      normalizeIdentityDeviceId(existing),
    connectionMarks: mergeConnectionMarks(
      inferConnectionMarks(existing),
      inferConnectionMarks(incoming),
    ),
    lan,
    devd: incoming.devd ?? existing.devd,
    webSerial: incoming.webSerial ?? existing.webSerial,
  } satisfies StoredDevice;
}

export function coalesceStoredDevicesByIdentity(
  devices: StoredDevice[],
): StoredDevice[] {
  const next: StoredDevice[] = [];
  const identityIndex = new Map<string, number>();

  for (const device of devices) {
    const identityDeviceId = normalizeIdentityDeviceId(device);
    if (!identityDeviceId) {
      next.push(cloneStoredDevice(device));
      continue;
    }

    const existingIndex = identityIndex.get(identityDeviceId);
    if (existingIndex === undefined) {
      identityIndex.set(identityDeviceId, next.length);
      next.push({
        ...cloneStoredDevice(device),
        identityDeviceId,
      });
      continue;
    }

    next[existingIndex] = mergeStoredDevice(next[existingIndex], {
      ...cloneStoredDevice(device),
      identityDeviceId,
    });
  }

  return next;
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
    const lan = stored.lan;
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
        identityDeviceId:
          typeof stored.identityDeviceId === "string"
            ? stored.identityDeviceId
            : undefined,
        connectionMarks: Array.isArray(stored.connectionMarks)
          ? stored.connectionMarks
          : undefined,
        lan:
          lan && typeof lan.baseUrl === "string"
            ? {
                baseUrl: lan.baseUrl,
              }
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

  return coalesceStoredDevicesByIdentity(devices);
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
      this.#storage.setItem(
        this.#key,
        JSON.stringify(coalesceStoredDevicesByIdentity(devices)),
      );
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
    this.#devices = coalesceStoredDevicesByIdentity(initialDevices);
  }

  getDevices(): StoredDevice[] {
    return this.#devices.map(cloneStoredDevice);
  }

  setDevices(devices: StoredDevice[]): void {
    this.#devices = coalesceStoredDevicesByIdentity(devices);
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

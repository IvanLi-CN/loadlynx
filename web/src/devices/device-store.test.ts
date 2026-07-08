import { expect, test } from "vitest";

import { DEMO_MODE_STORAGE_KEY } from "../lib/demo-mode.ts";
import {
  DEMO_DEVICES,
  DemoAwareDeviceStore,
  LocalStorageDeviceStore,
  MemoryDeviceStore,
} from "./device-store.ts";

class MemoryStorage implements Storage {
  readonly #map = new Map<string, string>();

  get length(): number {
    return this.#map.size;
  }

  clear(): void {
    this.#map.clear();
  }

  getItem(key: string): string | null {
    return this.#map.get(key) ?? null;
  }

  key(index: number): string | null {
    return [...this.#map.keys()][index] ?? null;
  }

  removeItem(key: string): void {
    this.#map.delete(key);
  }

  setItem(key: string, value: string): void {
    this.#map.set(key, value);
  }
}

test("LocalStorageDeviceStore ignores invalid JSON instead of throwing", () => {
  const storage = new MemoryStorage();
  storage.setItem("loadlynx.devices", "{");

  const store = new LocalStorageDeviceStore(storage);

  expect(store.getDevices()).toEqual([]);
});

test("LocalStorageDeviceStore sanitizes malformed stored device entries", () => {
  const storage = new MemoryStorage();
  storage.setItem(
    "loadlynx.devices",
    JSON.stringify([
      {
        id: "llx-1",
        name: "Bench",
        baseUrl: "http://192.168.1.23",
        identityDeviceId: "loadlynx-d68638",
        devd: {
          baseUrl: "http://127.0.0.1:30180",
          deviceId: "digital-aabbcc",
          leaseId: 123,
        },
        webSerial: {
          identityDeviceId: "llx-1",
          displayName: 42,
        },
      },
      {
        id: "bad",
        name: "Missing base URL",
      },
    ]),
  );

  const store = new LocalStorageDeviceStore(storage);

  expect(store.getDevices()).toEqual([
    {
      id: "llx-1",
      name: "Bench",
      baseUrl: "http://192.168.1.23",
      identityDeviceId: "loadlynx-d68638",
      devd: {
        baseUrl: "http://127.0.0.1:30180",
        deviceId: "digital-aabbcc",
      },
      webSerial: {
        identityDeviceId: "llx-1",
      },
    },
  ]);
});

test("LocalStorageDeviceStore coalesces duplicate transports by hardware identity", () => {
  const storage = new MemoryStorage();
  storage.setItem(
    "loadlynx.devices",
    JSON.stringify([
      {
        id: "device-001",
        name: "ESP32-S3 USB CDC",
        baseUrl:
          "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
        identityDeviceId: "loadlynx-d68638",
        connectionMarks: ["usb"],
        devd: {
          baseUrl: "http://127.0.0.1:19390",
          deviceId: "digital-2bdf",
          leaseId: "lease-1",
        },
      },
      {
        id: "device-002",
        name: "LoadLynx d68638 WiFi",
        baseUrl: "http://192.168.31.216",
        identityDeviceId: "loadlynx-d68638",
        connectionMarks: ["lan"],
      },
    ]),
  );

  const store = new LocalStorageDeviceStore(storage);

  expect(store.getDevices()).toEqual([
    {
      id: "device-001",
      name: "LoadLynx d68638 WiFi",
      baseUrl: "http://192.168.31.216",
      identityDeviceId: "loadlynx-d68638",
      connectionMarks: ["lan", "usb"],
      lan: {
        baseUrl: "http://192.168.31.216",
      },
      devd: {
        baseUrl: "http://127.0.0.1:19390",
        deviceId: "digital-2bdf",
        leaseId: "lease-1",
      },
    },
  ]);
});

test("LocalStorageDeviceStore prefers LAN base URL when coalescing duplicate transports", () => {
  const storage = new MemoryStorage();
  storage.setItem(
    "loadlynx.devices",
    JSON.stringify([
      {
        id: "device-001",
        name: "LoadLynx d68638 WiFi",
        baseUrl: "http://192.168.31.216",
        identityDeviceId: "loadlynx-d68638",
      },
      {
        id: "device-002",
        name: "ESP32-S3 USB CDC",
        baseUrl:
          "http://127.0.0.1:19390/?device_id=digital-2bdf&lease_id=lease-1",
        identityDeviceId: "loadlynx-d68638",
        devd: {
          baseUrl: "http://127.0.0.1:19390",
          deviceId: "digital-2bdf",
          leaseId: "lease-1",
        },
      },
    ]),
  );

  const store = new LocalStorageDeviceStore(storage);

  expect(store.getDevices()).toEqual([
    {
      id: "device-001",
      name: "LoadLynx d68638 WiFi",
      baseUrl: "http://192.168.31.216",
      identityDeviceId: "loadlynx-d68638",
      connectionMarks: ["lan", "usb"],
      lan: {
        baseUrl: "http://192.168.31.216",
      },
      devd: {
        baseUrl: "http://127.0.0.1:19390",
        deviceId: "digital-2bdf",
        leaseId: "lease-1",
      },
    },
  ]);
});

test("MemoryDeviceStore returns defensive copies", () => {
  const store = new MemoryDeviceStore([
    {
      id: "llx-1",
      name: "Bench",
      baseUrl: "http://192.168.1.23",
    },
  ]);

  const devices = store.getDevices();
  const first = devices[0];
  if (!first) {
    throw new Error("expected seeded device");
  }
  first.name = "Mutated";

  expect(store.getDevices()[0]?.name).toBe("Bench");
});

test("DemoAwareDeviceStore filters non-demo devices and persists demo defaults", () => {
  const storage = new MemoryStorage();
  storage.setItem(DEMO_MODE_STORAGE_KEY, "true");
  storage.setItem(
    "loadlynx.demo.devices",
    JSON.stringify([
      {
        id: "mock-001",
        name: "Demo Device #1",
        baseUrl: "mock://demo-1",
      },
      {
        id: "llx-real",
        name: "Real Device",
        baseUrl: "http://192.168.1.23",
      },
    ]),
  );

  const store = new DemoAwareDeviceStore(storage);

  expect(store.getDevices()).toEqual([
    {
      id: "mock-001",
      name: "Demo Device #1",
      baseUrl: "mock://demo-1",
    },
  ]);
  expect(JSON.parse(storage.getItem("loadlynx.demo.devices") ?? "[]")).toEqual([
    {
      id: "mock-001",
      name: "Demo Device #1",
      baseUrl: "mock://demo-1",
    },
  ]);

  storage.removeItem("loadlynx.demo.devices");
  expect(store.getDevices()).toEqual(DEMO_DEVICES);
});

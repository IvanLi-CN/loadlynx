import type { ReactNode } from "react";
import { createContext, useContext } from "react";
import type { DeviceStore } from "./device-store.ts";

const DeviceStoreContext = createContext<DeviceStore | null>(null);

export function DeviceStoreProvider(props: {
  store: DeviceStore;
  children: ReactNode;
}) {
  return (
    <DeviceStoreContext.Provider value={props.store}>
      {props.children}
    </DeviceStoreContext.Provider>
  );
}

export function useDeviceStore(): DeviceStore {
  const store = useContext(DeviceStoreContext);
  if (!store) {
    throw new Error("DeviceStoreProvider is missing");
  }
  return store;
}

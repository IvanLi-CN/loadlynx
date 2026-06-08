import type { ReactNode } from "react";
import { createContext, useContext } from "react";
import type { CalibrationStore } from "./store.ts";

const CalibrationStoreContext = createContext<CalibrationStore | null>(null);

export function CalibrationStoreProvider(props: {
  store: CalibrationStore;
  children: ReactNode;
}) {
  return (
    <CalibrationStoreContext.Provider value={props.store}>
      {props.children}
    </CalibrationStoreContext.Provider>
  );
}

export function useCalibrationStore(): CalibrationStore {
  const store = useContext(CalibrationStoreContext);
  if (!store) {
    throw new Error(
      "CalibrationStoreProvider is missing. This component/page cannot run in the current environment. Wrap it with <CalibrationStoreProvider store={...}> (MemoryCalibrationStore for Storybook/tests, LocalStorageCalibrationStore(window.localStorage) for the app).",
    );
  }
  return store;
}

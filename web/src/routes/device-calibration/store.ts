import type {
  CurrentInputUnit,
  ParsedCalibrationCurrentOptions,
  ParsedCalibrationDraft,
  ParsedCalibrationVoltageOptions,
  StoredCalibrationDraftV4,
  VoltageInputUnit,
} from "./shared.ts";
import {
  readCalibrationCurrentOptionsFromStorage,
  readCalibrationDraftFromStorage,
  readCalibrationVoltageOptionsFromStorage,
  writeCalibrationCurrentOptionsToStorage,
  writeCalibrationDraftToStorage,
  writeCalibrationVoltageOptionsToStorage,
} from "./shared.ts";

type CurveKey = "current_ch1" | "current_ch2";

export interface CalibrationStore {
  getDraft(deviceId: string, baseUrl: string): ParsedCalibrationDraft | null;
  setDraft(
    deviceId: string,
    baseUrl: string,
    draft: StoredCalibrationDraftV4 | null,
  ): void;
  getCurrentOptions(
    deviceId: string,
    baseUrl: string,
    curve: CurveKey,
  ): ParsedCalibrationCurrentOptions;
  setCurrentOptions(
    deviceId: string,
    baseUrl: string,
    curve: CurveKey,
    options: { baselineUa: number; unit: CurrentInputUnit },
  ): void;
  getVoltageOptions(
    deviceId: string,
    baseUrl: string,
  ): ParsedCalibrationVoltageOptions;
  setVoltageOptions(
    deviceId: string,
    baseUrl: string,
    options: { inputUv: number; unit: VoltageInputUnit },
  ): void;
}

class MapStorageAdapter
  implements Pick<Storage, "getItem" | "setItem" | "removeItem">
{
  readonly #values = new Map<string, string>();

  constructor(initialEntries?: Iterable<readonly [string, string]>) {
    if (!initialEntries) {
      return;
    }
    for (const [key, value] of initialEntries) {
      this.#values.set(key, value);
    }
  }

  getItem(key: string): string | null {
    return this.#values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.#values.set(key, value);
  }

  removeItem(key: string): void {
    this.#values.delete(key);
  }
}

export class LocalStorageCalibrationStore implements CalibrationStore {
  readonly #storage: Storage;

  constructor(storage: Storage) {
    this.#storage = storage;
  }

  getDraft(deviceId: string, baseUrl: string): ParsedCalibrationDraft | null {
    return readCalibrationDraftFromStorage(this.#storage, deviceId, baseUrl);
  }

  setDraft(
    deviceId: string,
    baseUrl: string,
    draft: StoredCalibrationDraftV4 | null,
  ): void {
    writeCalibrationDraftToStorage(this.#storage, deviceId, baseUrl, draft);
  }

  getCurrentOptions(
    deviceId: string,
    baseUrl: string,
    curve: CurveKey,
  ): ParsedCalibrationCurrentOptions {
    return readCalibrationCurrentOptionsFromStorage(
      this.#storage,
      deviceId,
      baseUrl,
      curve,
    );
  }

  setCurrentOptions(
    deviceId: string,
    baseUrl: string,
    curve: CurveKey,
    options: { baselineUa: number; unit: CurrentInputUnit },
  ): void {
    writeCalibrationCurrentOptionsToStorage(
      this.#storage,
      deviceId,
      baseUrl,
      curve,
      options,
    );
  }

  getVoltageOptions(
    deviceId: string,
    baseUrl: string,
  ): ParsedCalibrationVoltageOptions {
    return readCalibrationVoltageOptionsFromStorage(
      this.#storage,
      deviceId,
      baseUrl,
    );
  }

  setVoltageOptions(
    deviceId: string,
    baseUrl: string,
    options: { inputUv: number; unit: VoltageInputUnit },
  ): void {
    writeCalibrationVoltageOptionsToStorage(
      this.#storage,
      deviceId,
      baseUrl,
      options,
    );
  }
}

export class MemoryCalibrationStore implements CalibrationStore {
  readonly #storage: MapStorageAdapter;

  constructor(initialEntries?: Iterable<readonly [string, string]>) {
    this.#storage = new MapStorageAdapter(initialEntries);
  }

  getDraft(deviceId: string, baseUrl: string): ParsedCalibrationDraft | null {
    return readCalibrationDraftFromStorage(this.#storage, deviceId, baseUrl);
  }

  setDraft(
    deviceId: string,
    baseUrl: string,
    draft: StoredCalibrationDraftV4 | null,
  ): void {
    writeCalibrationDraftToStorage(this.#storage, deviceId, baseUrl, draft);
  }

  getCurrentOptions(
    deviceId: string,
    baseUrl: string,
    curve: CurveKey,
  ): ParsedCalibrationCurrentOptions {
    return readCalibrationCurrentOptionsFromStorage(
      this.#storage,
      deviceId,
      baseUrl,
      curve,
    );
  }

  setCurrentOptions(
    deviceId: string,
    baseUrl: string,
    curve: CurveKey,
    options: { baselineUa: number; unit: CurrentInputUnit },
  ): void {
    writeCalibrationCurrentOptionsToStorage(
      this.#storage,
      deviceId,
      baseUrl,
      curve,
      options,
    );
  }

  getVoltageOptions(
    deviceId: string,
    baseUrl: string,
  ): ParsedCalibrationVoltageOptions {
    return readCalibrationVoltageOptionsFromStorage(
      this.#storage,
      deviceId,
      baseUrl,
    );
  }

  setVoltageOptions(
    deviceId: string,
    baseUrl: string,
    options: { inputUv: number; unit: VoltageInputUnit },
  ): void {
    writeCalibrationVoltageOptionsToStorage(
      this.#storage,
      deviceId,
      baseUrl,
      options,
    );
  }
}

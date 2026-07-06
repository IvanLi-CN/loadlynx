import type { Preset, PresetId } from "../../api/types.ts";

export type PresetEditableField =
  | "target_i_ma"
  | "target_v_mv"
  | "target_p_mw"
  | "min_v_mv"
  | "max_i_ma_total"
  | "max_p_mw";

export type PresetInputUnitKind = "current" | "voltage" | "power";

export type PresetInputMemoryEntry = {
  value: number;
  text: string;
};

export type PresetInputMemoryStore = Record<string, PresetInputMemoryEntry>;

export const PRESET_INPUT_MEMORY_STORAGE_KEY =
  "loadlynx.preset-input-memory.v1";

const UNIT_KIND_MAP = {
  current: [
    { tokens: ["a"], factor: 1000, display: "A" },
    { tokens: ["ma"], factor: 1, display: "mA" },
  ],
  voltage: [
    { tokens: ["v"], factor: 1000, display: "V" },
    { tokens: ["mv"], factor: 1, display: "mV" },
  ],
  power: [
    { tokens: ["w"], factor: 1000, display: "W" },
    { tokens: ["mw"], factor: 1, display: "mW" },
  ],
} as const satisfies Record<
  PresetInputUnitKind,
  ReadonlyArray<{
    tokens: readonly string[];
    factor: number;
    display: string;
  }>
>;

export type ParsedPresetInputValue =
  | {
      ok: true;
      value: number;
      displayText: string;
    }
  | {
      ok: false;
      error: string;
    };

function normalizeNumberToken(raw: string): string {
  const normalized = raw.replace(",", ".").trim();
  if (normalized.length === 0) {
    return normalized;
  }

  const parsed = Number(normalized);
  if (!Number.isFinite(parsed)) {
    return normalized;
  }

  return Number.isInteger(parsed) ? String(parsed) : parsed.toString();
}

export function formatPresetRawValue(value: number): string {
  return String(value);
}

export function parsePresetInputValue(
  raw: string,
  unitKind: PresetInputUnitKind,
): ParsedPresetInputValue {
  const trimmed = raw.trim();
  if (trimmed.length === 0) {
    return {
      ok: false,
      error: "Enter a numeric value.",
    };
  }

  const match = trimmed.match(
    /^([+-]?(?:\d+(?:[.,]\d+)?|[.,]\d+))(?:\s*([a-zA-Z]+))?$/,
  );
  if (!match) {
    return {
      ok: false,
      error: "Enter a number such as 2300, 2.3 A, 12 V, or 15 W.",
    };
  }

  const numericToken = normalizeNumberToken(match[1]);
  const parsedNumber = Number(numericToken);
  if (!Number.isFinite(parsedNumber)) {
    return {
      ok: false,
      error: "Enter a valid numeric value.",
    };
  }

  const normalizedUnitToken = match[2]?.trim().toLowerCase() ?? null;
  if (normalizedUnitToken == null) {
    return {
      ok: true,
      value: Math.max(0, Math.round(parsedNumber)),
      displayText: normalizeNumberToken(numericToken),
    };
  }

  const unitDescriptor = UNIT_KIND_MAP[unitKind].find((entry) =>
    entry.tokens.some((token) => token === normalizedUnitToken),
  );
  if (!unitDescriptor) {
    return {
      ok: false,
      error: "Unsupported unit for this field.",
    };
  }

  return {
    ok: true,
    value: Math.max(0, Math.round(parsedNumber * unitDescriptor.factor)),
    displayText: `${normalizeNumberToken(numericToken)} ${unitDescriptor.display}`,
  };
}

export function makePresetInputMemoryKey(params: {
  deviceId: string;
  baseUrl: string | undefined;
  presetId: PresetId;
  field: PresetEditableField;
}): string {
  const scope = params.baseUrl?.trim() ? params.baseUrl.trim() : "no-base-url";
  return [params.deviceId, scope, params.presetId, params.field].join("::");
}

export function readPresetInputMemory(
  storage: Pick<Storage, "getItem">,
): PresetInputMemoryStore {
  try {
    const raw = storage.getItem(PRESET_INPUT_MEMORY_STORAGE_KEY);
    if (!raw) {
      return {};
    }

    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") {
      return {};
    }

    const result: PresetInputMemoryStore = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (!value || typeof value !== "object") {
        continue;
      }
      const entry = value as Partial<PresetInputMemoryEntry>;
      if (
        typeof entry.value === "number" &&
        Number.isFinite(entry.value) &&
        typeof entry.text === "string" &&
        entry.text.trim().length > 0
      ) {
        result[key] = { value: entry.value, text: entry.text };
      }
    }
    return result;
  } catch {
    return {};
  }
}

export function writePresetInputMemory(
  storage: Pick<Storage, "setItem">,
  data: PresetInputMemoryStore,
) {
  storage.setItem(PRESET_INPUT_MEMORY_STORAGE_KEY, JSON.stringify(data));
}

const PRESET_FIELDS: PresetEditableField[] = [
  "target_i_ma",
  "target_v_mv",
  "target_p_mw",
  "min_v_mv",
  "max_i_ma_total",
  "max_p_mw",
];

export function reconcilePresetInputMemory(params: {
  store: PresetInputMemoryStore;
  deviceId: string;
  baseUrl: string | undefined;
  presets: Preset[];
}): PresetInputMemoryStore {
  const next: PresetInputMemoryStore = {};
  const scope = params.baseUrl?.trim() ? params.baseUrl.trim() : "no-base-url";
  const expectedValueByKey = new Map<string, number>();

  for (const preset of params.presets) {
    for (const field of PRESET_FIELDS) {
      expectedValueByKey.set(
        makePresetInputMemoryKey({
          deviceId: params.deviceId,
          baseUrl: params.baseUrl,
          presetId: preset.preset_id,
          field,
        }),
        preset[field],
      );
    }
  }

  for (const [key, entry] of Object.entries(params.store)) {
    if (!key.startsWith(`${params.deviceId}::${scope}::`)) {
      next[key] = entry;
      continue;
    }

    const expectedValue = expectedValueByKey.get(key);
    if (expectedValue === undefined || expectedValue === entry.value) {
      next[key] = entry;
    }
  }

  return next;
}

export const UNBOUND_DEVICE_BASE_URL = "__no_base_url__";

export const DEVICE_QUERY_PARTS = {
  identity: ["identity"],
  status: ["status"],
  pd: ["pd"],
  control: ["control"],
  presets: ["presets"],
  wifi: ["wifi"],
  backupPd: ["backup-pd"],
  calibrationProfile: ["calibration", "profile"],
  calibrationStatusFallback: ["status", "calibration-fallback"],
} as const;

export type DeviceQueryParts =
  (typeof DEVICE_QUERY_PARTS)[keyof typeof DEVICE_QUERY_PARTS];

export function makeDeviceQueryKey(
  deviceId: string | null | undefined,
  baseUrl: string | null | undefined,
  ...parts: DeviceQueryParts
) {
  return [
    "device",
    deviceId ?? "unknown",
    baseUrl ?? UNBOUND_DEVICE_BASE_URL,
    ...parts,
  ] as const;
}

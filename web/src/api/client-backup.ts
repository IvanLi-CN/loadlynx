import {
  getCalibrationProfile,
  mapCalibrationProfileUiToWire,
  mapCalibrationProfileWireToUi,
  postCalibrationCommit,
  postCalibrationReset,
} from "./client-calibration.ts";
import {
  HttpApiError,
  httpJsonQueued,
  isDevdCompatBaseUrl,
  isMockBaseUrl,
} from "./client-core.ts";
import {
  getControl,
  getPd,
  getPresets,
  postPd,
  updateControl,
  updatePreset,
} from "./client-device.ts";
import { getOrCreateMockDevice } from "./client-mock.ts";
import type {
  BackupRestoreResult,
  BackupSectionKey,
  CalibrationProfileWire,
  ControlUpdateRequest,
  ControlView,
  DiagnosticsExport,
  LoadLynxBackup,
  PdFixedUpdateRequest,
  PdPpsUpdateRequest,
  Preset,
  SoftResetRequest,
  WifiCredentials,
  WifiSetRequest,
  WifiStatus,
  WifiStatusResponse,
} from "./types.ts";

export const BACKUP_SECTION_KEYS: BackupSectionKey[] = [
  "presets",
  "calibration",
  "settings.wifi",
  "settings.pd",
];

function unwrapWifiStatusResponse(response: WifiStatusResponse): WifiStatus {
  return "wifi" in response ? response.wifi : response;
}

export function makeWifiSetRequest(ssid: string, psk: string): WifiSetRequest {
  return {
    ssid,
    psk,
    wait: false,
  };
}

export function makeManualSoftResetRequest(): SoftResetRequest {
  return { reason: "manual" };
}

export interface WifiWriteGuard {
  identityVerified?: boolean;
}

const WIFI_SET_RECOVERY_ATTEMPTS = 8;
const WIFI_SET_RECOVERY_DELAY_MS = 1_000;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => globalThis.setTimeout(resolve, ms));
}

function isRecoverableWifiSetError(error: unknown): error is HttpApiError {
  return (
    error instanceof HttpApiError &&
    (error.code === "serial_response_timeout" ||
      error.code === "serial_response_mismatch")
  );
}

function wifiSetMatchesPayload(
  wifi: WifiStatus,
  payload: WifiSetRequest,
): boolean {
  return (
    wifi.source === "user" &&
    wifi.ssid === payload.ssid &&
    wifi.state !== "error"
  );
}

async function recoverWifiSetAfterError(
  baseUrl: string,
  payload: WifiSetRequest,
): Promise<WifiStatus | null> {
  for (let attempt = 0; attempt < WIFI_SET_RECOVERY_ATTEMPTS; attempt += 1) {
    if (attempt > 0) {
      await delay(WIFI_SET_RECOVERY_DELAY_MS);
    }

    try {
      const wifi = await getWifiStatus(baseUrl);
      if (wifiSetMatchesPayload(wifi, payload)) {
        return wifi;
      }
    } catch {
      // Preserve the original write error when readback cannot confirm success.
    }
  }
  return null;
}

export function assertWifiWriteAllowed(
  baseUrl: string,
  guard: WifiWriteGuard = {},
): void {
  if (isMockBaseUrl(baseUrl)) {
    return;
  }

  if (isDevdCompatBaseUrl(baseUrl)) {
    if (guard.identityVerified === true) {
      return;
    }
    throw new HttpApiError({
      status: 0,
      code: "WIFI_WRITE_REQUIRES_VERIFIED_LOCAL_CONNECTION",
      message:
        "WiFi settings require a verified USB/devd lease and device identity before writing.",
      retryable: false,
    });
  }

  throw new HttpApiError({
    status: 0,
    code: "WIFI_WRITE_REQUIRES_LOCAL_CONNECTION",
    message:
      "WiFi settings can only be changed from a non-WiFi connection. Switch to USB/devd before changing WiFi.",
    retryable: false,
  });
}

export async function getWifiStatus(baseUrl: string): Promise<WifiStatus> {
  if (isMockBaseUrl(baseUrl)) {
    const state = getOrCreateMockDevice(baseUrl);
    if (state.wifiConnectPollsRemaining > 0) {
      state.wifiConnectPollsRemaining -= 1;
      if (state.wifiConnectPollsRemaining === 0) {
        state.wifi = {
          ...state.wifi,
          state: "connected",
          ip: "192.0.2.11",
          last_error: null,
        };
      }
    }
    return structuredClone(state.wifi);
  }
  return httpJsonQueued<WifiStatus>(baseUrl, "/api/v1/wifi");
}

export async function getWifiCredentials(
  baseUrl: string,
): Promise<WifiCredentials> {
  if (isMockBaseUrl(baseUrl)) {
    const state = getOrCreateMockDevice(baseUrl);
    return {
      ssid: state.wifi.ssid ?? "",
      psk: state.wifiPsk,
      source: state.wifi.source === "user" ? "user" : "factory",
    };
  }
  return httpJsonQueued<WifiCredentials>(baseUrl, "/api/v1/wifi/credentials");
}

export async function postWifiConfig(
  baseUrl: string,
  payload: WifiSetRequest,
  guard?: WifiWriteGuard,
): Promise<WifiStatus> {
  try {
    if (isMockBaseUrl(baseUrl)) {
      const state = getOrCreateMockDevice(baseUrl);
      const normalizedBaseUrl = baseUrl.toLowerCase();
      if (normalizedBaseUrl.includes("wifi-set-eeprom-error")) {
        throw new HttpApiError({
          status: 503,
          code: "UNAVAILABLE",
          message: "EEPROM write failed",
          retryable: true,
        });
      }

      state.wifi = {
        ssid: payload.ssid,
        source: "user",
        state: payload.wait ? "connected" : "configured",
        ip: payload.wait ? "192.0.2.11" : null,
        last_error: null,
      };
      state.wifiConnectPollsRemaining = payload.wait ? 0 : 2;
      state.wifiPsk = payload.psk;

      if (normalizedBaseUrl.includes("wifi-set-timeout-success")) {
        throw new HttpApiError({
          status: 504,
          code: "serial_response_timeout",
          message:
            "USB request devd-set-wifi-config-test did not receive a matching response",
          retryable: true,
        });
      }

      return structuredClone(state.wifi);
    }
    assertWifiWriteAllowed(baseUrl, guard);
    const response = await httpJsonQueued<WifiStatusResponse>(
      baseUrl,
      "/api/v1/wifi",
      {
        method: "POST",
        body: JSON.stringify(payload),
        headers: {
          "Content-Type": "application/json",
        },
      },
    );
    return unwrapWifiStatusResponse(response);
  } catch (error) {
    if (isRecoverableWifiSetError(error)) {
      const recovered = await recoverWifiSetAfterError(baseUrl, payload);
      if (recovered) {
        return recovered;
      }
    }
    throw error;
  }
}

export async function deleteWifiConfig(
  baseUrl: string,
  guard?: WifiWriteGuard,
): Promise<WifiStatus> {
  if (isMockBaseUrl(baseUrl)) {
    const state = getOrCreateMockDevice(baseUrl);
    const normalizedBaseUrl = baseUrl.toLowerCase();
    if (normalizedBaseUrl.includes("wifi-clear-error")) {
      throw new HttpApiError({
        status: 503,
        code: "MOCK_WIFI_CLEAR_FAILED",
        message: "Mock WiFi clear failed.",
        retryable: true,
      });
    }
    if (normalizedBaseUrl.includes("wifi-clear-noop")) {
      return structuredClone(state.wifi);
    }
    if (normalizedBaseUrl.includes("wifi-clear-timeout-success")) {
      state.wifi = {
        ssid: "",
        source: "none",
        state: "idle",
        ip: null,
        last_error: null,
      };
      state.wifiConnectPollsRemaining = 0;
      state.wifiPsk = "";
      throw new HttpApiError({
        status: 504,
        code: "serial_response_timeout",
        message:
          "USB request devd-clear-wifi-config-test did not receive a matching response",
        retryable: true,
      });
    }
    state.wifi = {
      ssid: "LoadLynx Lab",
      source: "factory",
      state: "configured",
      ip: null,
      last_error: null,
    };
    state.wifiConnectPollsRemaining = 2;
    state.wifiPsk = "factory-mock-psk";
    return structuredClone(state.wifi);
  }
  assertWifiWriteAllowed(baseUrl, guard);
  const response = await httpJsonQueued<WifiStatusResponse>(
    baseUrl,
    "/api/v1/wifi",
    {
      method: "DELETE",
    },
  );
  return unwrapWifiStatusResponse(response);
}

export function getSupportedBackupSections(
  backup: LoadLynxBackup | null,
): BackupSectionKey[] {
  if (!backup?.sections) {
    return [];
  }
  const sections: BackupSectionKey[] = [];
  if (backup.sections.presets) sections.push("presets");
  if (backup.sections.calibration) sections.push("calibration");
  if (backup.sections.settings?.wifi) sections.push("settings.wifi");
  if (backup.sections.settings?.pd) sections.push("settings.pd");
  return sections;
}

export function getBackupUnknownWarnings(
  backup: LoadLynxBackup | null,
): string[] {
  if (!backup?.sections) {
    return [];
  }
  const warnings: string[] = [];
  for (const key of Object.keys(backup.sections)) {
    if (key !== "presets" && key !== "calibration" && key !== "settings") {
      warnings.push(`Unknown section ignored: ${key}`);
    }
  }
  for (const key of Object.keys(backup.sections.settings ?? {})) {
    if (key !== "wifi" && key !== "pd") {
      warnings.push(`Unknown section ignored: settings.${key}`);
    }
  }
  return warnings;
}

export function validateBackupEnvelope(value: unknown): LoadLynxBackup {
  if (!value || typeof value !== "object") {
    throw new Error("Backup JSON must be an object.");
  }
  const backup = value as LoadLynxBackup;
  if (backup.kind !== "loadlynx.backup") {
    throw new Error("Backup kind must be loadlynx.backup.");
  }
  if (backup.schema_version !== 1) {
    throw new Error("Unsupported backup schema_version.");
  }
  if (!backup.sections || typeof backup.sections !== "object") {
    throw new Error("Backup sections must be an object.");
  }
  return backup;
}

export async function exportDeviceBackup(
  baseUrl: string,
  selected: BackupSectionKey[],
): Promise<LoadLynxBackup> {
  const sections: LoadLynxBackup["sections"] = {};

  if (selected.includes("presets")) {
    const [presets, control] = await Promise.all([
      getPresets(baseUrl),
      getControl(baseUrl),
    ]);
    sections.presets = {
      presets,
      active_preset_id: control.active_preset_id,
    };
  }

  if (selected.includes("calibration")) {
    sections.calibration = mapCalibrationProfileUiToWire(
      await getCalibrationProfile(baseUrl),
    );
  }

  const settings: NonNullable<LoadLynxBackup["sections"]["settings"]> = {};
  if (selected.includes("settings.wifi")) {
    settings.wifi = await getWifiCredentials(baseUrl);
  }
  if (selected.includes("settings.pd")) {
    const pd = await getPd(baseUrl);
    if (!pd) {
      throw new Error("USB-PD settings are not available on this device.");
    }
    settings.pd = {
      saved: pd.saved,
      allow_extended_voltage: pd.allow_extended_voltage ?? false,
    };
  }
  if (Object.keys(settings).length > 0) {
    sections.settings = settings;
  }

  return {
    kind: "loadlynx.backup",
    schema_version: 1,
    created_at: new Date().toISOString(),
    selected_sections: selected,
    sections,
  };
}

export async function restoreDeviceBackup(
  baseUrl: string,
  backup: LoadLynxBackup,
  selected: BackupSectionKey[],
  guard?: WifiWriteGuard,
): Promise<BackupRestoreResult> {
  validateBackupEnvelope(backup);
  const warnings = getBackupUnknownWarnings(backup);

  let control: ControlView;
  try {
    const payload: ControlUpdateRequest = { output_enabled: false };
    control = await updateControl(baseUrl, payload);
  } catch (error) {
    throw new HttpApiError({
      status: 409,
      code: "SAFETY_BLOCKED",
      message: `Output disable failed: ${formatUnknownError(error)}`,
      retryable: true,
      details: null,
    });
  }
  if (control.output_enabled !== false) {
    throw new HttpApiError({
      status: 409,
      code: "SAFETY_BLOCKED",
      message: "Output disable was not confirmed.",
      retryable: true,
      details: control,
    });
  }

  const restored: BackupRestoreResult["restored"] = [];
  const run = async (section: BackupSectionKey, fn: () => Promise<void>) => {
    try {
      await fn();
      restored.push({ section, ok: true });
    } catch (error) {
      restored.push({
        section,
        ok: false,
        message: formatUnknownError(error),
      });
    }
  };

  if (selected.includes("presets") && backup.sections.presets) {
    await run("presets", async () => {
      const currentPresets = await getPresets(baseUrl);
      for (const preset of backup.sections.presets?.presets ?? []) {
        const current = currentPresets.find(
          (candidate) => candidate.preset_id === preset.preset_id,
        );
        if (!current || !presetsEqual(current, preset)) {
          await updatePreset(baseUrl, preset);
        }
      }
    });
  }

  const calibrationSection = backup.sections.calibration;
  if (selected.includes("calibration") && calibrationSection) {
    await run("calibration", async () => {
      await restoreCalibrationBackup(baseUrl, calibrationSection);
    });
  }

  const settingsSection = backup.sections.settings;
  const pdSection = settingsSection?.pd;
  if (selected.includes("settings.pd") && pdSection) {
    await run("settings.pd", async () => {
      await restorePdBackup(baseUrl, pdSection);
    });
  }

  const wifiSection = settingsSection?.wifi;
  if (selected.includes("settings.wifi") && wifiSection) {
    await run("settings.wifi", async () => {
      await restoreWifiBackup(baseUrl, wifiSection, guard);
    });
  }

  return {
    ok: restored.every((entry) => entry.ok),
    safety: { output_disabled: true },
    restored,
    warnings,
  };
}

async function restoreWifiBackup(
  baseUrl: string,
  wifi: NonNullable<
    NonNullable<LoadLynxBackup["sections"]["settings"]>["wifi"]
  >,
  guard?: WifiWriteGuard,
): Promise<void> {
  try {
    const readback = await getWifiCredentials(baseUrl);
    if (
      readback.ssid === wifi.ssid &&
      readback.psk === wifi.psk &&
      readback.source === wifi.source
    ) {
      return;
    }
  } catch {
    // ignore
  }

  try {
    if (wifi.source === "factory") {
      await deleteWifiConfig(baseUrl, guard);
    } else {
      const payload = makeWifiSetRequest(wifi.ssid, wifi.psk);
      await postWifiConfig(baseUrl, payload, guard);
    }
  } catch (error) {
    try {
      const readback = await getWifiCredentials(baseUrl);
      if (
        readback.ssid === wifi.ssid &&
        readback.psk === wifi.psk &&
        readback.source === wifi.source
      ) {
        return;
      }
    } catch {
      // preserve original write error
    }
    throw error;
  }
}

async function restoreCalibrationBackup(
  baseUrl: string,
  profile: CalibrationProfileWire,
): Promise<void> {
  const allEmpty =
    profile.current_ch1_points.length === 0 &&
    profile.current_ch2_points.length === 0 &&
    profile.v_local_points.length === 0 &&
    profile.v_remote_points.length === 0;
  if (allEmpty || profile.active.source === "factory-default") {
    await postCalibrationReset(baseUrl, { kind: "all" });
    return;
  }

  const ui = mapCalibrationProfileWireToUi(profile);
  const curves = [
    { kind: "current_ch1" as const, points: ui.current_ch1_points },
    { kind: "current_ch2" as const, points: ui.current_ch2_points },
    { kind: "v_local" as const, points: ui.v_local_points },
    { kind: "v_remote" as const, points: ui.v_remote_points },
  ];

  for (const curve of curves) {
    if (curve.points.length === 0) {
      await postCalibrationReset(baseUrl, { kind: curve.kind });
    } else {
      await postCalibrationCommit(baseUrl, curve);
    }
  }
}

async function restorePdBackup(
  baseUrl: string,
  pd: NonNullable<NonNullable<LoadLynxBackup["sections"]["settings"]>["pd"]>,
): Promise<void> {
  if (pd.saved.mode === "fixed") {
    const request: PdFixedUpdateRequest = {
      mode: "fixed",
      object_pos: pd.saved.fixed_object_pos,
      target_mv: pd.saved.target_mv,
      i_req_ma: pd.saved.i_req_ma,
      allow_extended_voltage: pd.allow_extended_voltage,
    };
    await postPd(baseUrl, request);
    return;
  }

  const request: PdPpsUpdateRequest = {
    mode: "pps",
    object_pos: pd.saved.pps_object_pos,
    target_mv: pd.saved.pps_target_mv ?? pd.saved.target_mv,
    i_req_ma: pd.saved.i_req_ma,
    allow_extended_voltage: pd.allow_extended_voltage,
  };
  await postPd(baseUrl, request);
}

function formatUnknownError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function presetsEqual(a: Preset, b: Preset): boolean {
  return (
    a.preset_id === b.preset_id &&
    a.mode === b.mode &&
    a.target_p_mw === b.target_p_mw &&
    a.target_i_ma === b.target_i_ma &&
    a.target_v_mv === b.target_v_mv &&
    a.min_v_mv === b.min_v_mv &&
    a.max_i_ma_total === b.max_i_ma_total &&
    a.max_p_mw === b.max_p_mw
  );
}

export async function exportDiagnostics(
  baseUrl: string,
): Promise<DiagnosticsExport> {
  if (isMockBaseUrl(baseUrl)) {
    const state = getOrCreateMockDevice(baseUrl);
    return {
      schema_version: 1,
      redaction: { psk: true },
      firmware_version: state.identity.digital_fw_version,
      wifi: {
        ...state.wifi,
        psk: "<redacted>",
      },
      link_up: state.status.link_up,
      calibration_persistence: state.calibration.ram.persistence ?? {
        status: "unknown",
      },
      last_status: {
        uptime_ms: state.status.raw.uptime_ms,
        fault_flags: state.status.raw.fault_flags,
      },
    };
  }
  return httpJsonQueued<DiagnosticsExport>(
    baseUrl,
    "/api/v1/diagnostics/export",
  );
}

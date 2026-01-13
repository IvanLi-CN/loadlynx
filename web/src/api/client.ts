import { CALIBRATION_MAX_POINTS } from "../calibration/validation.ts";
import type {
  CalibrationApplyRequest,
  CalibrationCommitRequest,
  CalibrationModeRequest,
  CalibrationPointCurrentWireCompact,
  CalibrationPointVoltageWireCompact,
  CalibrationProfile,
  CalibrationProfileWire,
  CalibrationResetRequest,
  CalibrationWriteRequestWire,
  CcControlView,
  CcUpdateRequest,
  ControlView,
  FastStatusJson,
  FastStatusView,
  Identity,
  PdUpdateRequest,
  PdView,
  Preset,
  PresetId,
} from "./types.ts";

const TAB_ID =
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `tab-${Math.random().toString(16).slice(2)}`;

// Mock backend selection is based solely on the device URL scheme. The
// ENABLE_MOCK flag remains exported for other modules but no longer gates the
// mock backend here.
// ENABLE_MOCK remains exported for compatibility but no longer gates the backend choice.
export const ENABLE_MOCK = import.meta.env.VITE_ENABLE_MOCK_BACKEND !== "false";

// Controls visibility of developer-facing mock controls (e.g. "Add demo device" button).
// Defaults to TRUE in DEV mode unless explicitly disabled.
// Always TRUE if VITE_ENABLE_MOCK_DEVTOOLS is "true".
// Always FALSE if VITE_ENABLE_MOCK_DEVTOOLS is "false".
export const ENABLE_MOCK_DEVTOOLS =
  import.meta.env.VITE_ENABLE_MOCK_DEVTOOLS === "true" ||
  (import.meta.env.DEV &&
    import.meta.env.VITE_ENABLE_MOCK_DEVTOOLS !== "false");

export function isMockBaseUrl(baseUrl: string): boolean {
  if (!baseUrl) {
    return false;
  }
  const normalized = baseUrl.trim().toLowerCase();
  return normalized.startsWith("mock://");
}

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export interface HttpApiErrorInit {
  status: number;
  code?: string;
  message: string;
  retryable?: boolean;
  details?: unknown;
}

export class HttpApiError extends Error {
  readonly status: number;
  readonly code?: string;
  readonly retryable?: boolean;
  readonly details?: unknown;

  constructor(init: HttpApiErrorInit) {
    super(init.message);
    this.name = "HttpApiError";
    this.status = init.status;
    this.code = init.code;
    this.retryable = init.retryable;
    this.details = init.details;
  }
}

export function isHttpApiError(error: unknown): error is HttpApiError {
  return error instanceof HttpApiError;
}

interface ErrorEnvelope {
  error?: {
    code?: string;
    message?: string;
    retryable?: boolean;
    details?: unknown;
  };
}

function mapHttpError(status: number, data: unknown): HttpApiError {
  const envelope = (data ?? {}) as ErrorEnvelope;
  const inner = envelope.error ?? {};

  const code =
    typeof inner.code === "string" && inner.code.length > 0
      ? inner.code
      : undefined;
  const message =
    typeof inner.message === "string" && inner.message.length > 0
      ? inner.message
      : `HTTP ${status}`;
  const retryable =
    typeof inner.retryable === "boolean"
      ? inner.retryable
      : status >= 500 || status === 0;

  return new HttpApiError({
    status,
    code,
    message,
    retryable,
    details: inner.details ?? data,
  });
}

// Per-device FIFO queue to serialize HTTP calls to a single device baseUrl.
// Errors from previous requests must not stall the queue, so we always
// advance the tail even when a request fails.
const deviceQueues = new Map<string, Promise<unknown>>();

function enqueueForDevice<T>(
  baseUrl: string,
  op: () => Promise<T>,
): Promise<T> {
  const tail = deviceQueues.get(baseUrl) ?? Promise.resolve();
  const next = tail.catch(() => undefined).then(() => op());

  deviceQueues.set(
    baseUrl,
    next.catch(() => undefined),
  );

  return next;
}

async function httpJson<T>(
  baseUrl: string,
  path: string,
  init?: RequestInit,
): Promise<T> {
  const method = init?.method ?? "GET";

  if (isStorybookRuntime() && !isMockBaseUrl(baseUrl)) {
    throw new Error(
      `[LoadLynx] Real device HTTP is disabled in Storybook. This request tried to call ${method} ${path} with baseUrl="${baseUrl}". Use a mock:// baseUrl instead.`,
    );
  }

  const url = new URL(path, baseUrl);

  const headers: Record<string, string> = {
    ...(init?.headers as Record<string, string> | undefined),
  };

  // Embedded servers often have tiny connection limits; explicitly request
  // connection close to avoid keeping sockets busy between polls/mutations.
  headers.Connection ||= "close";

  const hasBody = init?.body !== undefined && init.body !== null;
  if (hasBody || method.toUpperCase() !== "GET") {
    headers["Content-Type"] ||= "application/json";
  }

  let response: Response;
  try {
    response = await fetch(url.toString(), {
      method,
      ...init,
      headers,
    });
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Network request failed";
    throw new HttpApiError({
      status: 0,
      code: "NETWORK_ERROR",
      message,
      retryable: true,
      details: null,
    });
  }

  const text = await response.text();
  let data: unknown = null;

  if (text.length > 0) {
    try {
      data = JSON.parse(text) as unknown;
    } catch {
      throw new HttpApiError({
        status: response.status,
        code: "INVALID_JSON",
        message: `Invalid JSON from ${path}`,
        retryable: false,
        details: text.slice(0, 200),
      });
    }
  }

  if (!response.ok) {
    throw mapHttpError(response.status, data);
  }

  return data as T;
}

async function httpJsonQueued<T>(
  baseUrl: string,
  path: string,
  init?: RequestInit,
): Promise<T> {
  return enqueueForDevice(baseUrl, () => httpJson<T>(baseUrl, path, init));
}

// Test-only export to validate queue behaviour without widening public surface.
export const __testHttpJsonQueued = httpJsonQueued;
export const __testEnqueueForDevice = enqueueForDevice;
export const __testClearDeviceQueues = () => deviceQueues.clear();

// Simple in-memory mock of the HTTP API.
// All functions mimic the shape of the real endpoints so we can later swap
// the internals for real fetch() calls without touching callers.

interface MockDeviceState {
  identity: Identity;
  status: FastStatusView;
  cc: CcControlView;
  pd: PdView | null;
  presets: Preset[];
  active_preset_id: PresetId;
  output_enabled: boolean;
  uv_latched: boolean;
  calibrationMode: CalibrationModeRequest["kind"];
  calibration: MockCalibrationState;
}

interface MockCalibrationState {
  factory: CalibrationProfileWire;
  ram: CalibrationProfileWire;
  eeprom: CalibrationProfileWire | null;
}

function createInitialCalibrationProfileWire(): CalibrationProfileWire {
  // Match firmware expectations: raw_100uv is i16, points are 1..7, and meas is
  // strictly increasing (after raw-sorted normalization).
  const active = {
    source: "factory-default" as const,
    fmt_version: 3,
    hw_rev: 1,
  };

  const v_local_points = [
    { raw_100uv: 0, meas_mv: 0 },
    { raw_100uv: 30_000, meas_mv: 12_000 },
  ];
  const v_remote_points = [
    { raw_100uv: 0, meas_mv: 0 },
    { raw_100uv: 30_000, meas_mv: 12_000 },
  ];

  const current_ch1_points = [
    { raw_100uv: 0, raw_dac_code: 0, meas_ma: 0 },
    { raw_100uv: 25_000, raw_dac_code: 4095, meas_ma: 5_000 },
  ];
  const current_ch2_points = [
    { raw_100uv: 0, raw_dac_code: 0, meas_ma: 0 },
    { raw_100uv: 25_000, raw_dac_code: 4095, meas_ma: 5_000 },
  ];

  return {
    active,
    current_ch1_points,
    current_ch2_points,
    v_local_points,
    v_remote_points,
  };
}

const mockDevices = new Map<string, MockDeviceState>();

function clampI16(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(-32768, Math.min(32767, value));
}

function createInitialStatus(baseUrl?: string): FastStatusView {
  const raw: FastStatusJson = {
    uptime_ms: 123_456,
    mode: 0,
    state_flags: 0,
    enable: false,
    target_value: 0,
    i_local_ma: 0,
    i_remote_ma: 0,
    v_local_mv: 12_000,
    v_remote_mv: 11_950,
    calc_p_mw: 0,
    dac_headroom_mv: 500,
    loop_error: 0,
    sink_core_temp_mc: 45_000,
    sink_exhaust_temp_mc: 42_000,
    mcu_temp_mc: 40_000,
    fault_flags: 0,
  };

  const view: FastStatusView = {
    raw,
    link_up: true,
    hello_seen: true,
    analog_state: "ready",
    fault_flags_decoded: [],
  };

  if (baseUrl) {
    const normalized = baseUrl.toLowerCase();
    if (normalized.includes("link-down")) {
      view.link_up = false;
      view.hello_seen = false;
      view.analog_state = "offline";
    } else if (normalized.includes("cal-missing")) {
      view.analog_state = "cal_missing";
    } else if (normalized.includes("faulted")) {
      view.analog_state = "faulted";
    }
  }

  return view;
}

function createInitialPd(baseUrl: string): PdView | null {
  const normalized = baseUrl.toLowerCase();
  if (normalized.includes("no-pd")) {
    return null;
  }

  const fixed_pdos = [
    { pos: 1, mv: 5_000, max_ma: 3_000 },
    { pos: 2, mv: 9_000, max_ma: 3_000 },
    { pos: 3, mv: 12_000, max_ma: 3_000 },
    { pos: 4, mv: 15_000, max_ma: 3_000 },
    { pos: 5, mv: 20_000, max_ma: 1_500 },
  ];

  const pps_pdos = [
    { pos: 3, min_mv: 3_300, max_mv: 21_000, max_ma: 3_000 },
    { pos: 4, min_mv: 5_000, max_mv: 11_000, max_ma: 2_000 },
  ];

  return {
    attached: true,
    contract_mv: 9_000,
    contract_ma: 2_000,
    fixed_pdos,
    pps_pdos,
    saved: {
      mode: "pps",
      fixed_object_pos: 5,
      pps_object_pos: 3,
      target_mv: 9_000,
      i_req_ma: 2_000,
    },
    apply: {
      pending: false,
      last: { code: "ok", at_ms: 123_456 },
    },
  };
}

function createInitialCc(): CcControlView {
  return {
    enable: false,
    target_i_ma: 1_500,
    effective_i_ma: 0,
    limit_profile: {
      max_i_ma: 5_000,
      max_p_mw: 60_000,
      ovp_mv: 40_000,
      temp_trip_mc: 80_000,
      thermal_derate_pct: 100,
    },
    protection: {
      voltage_mode: "protect",
      power_mode: "protect",
    },
    i_total_ma: 0,
    v_main_mv: 12_000,
    p_main_mw: 0,
  };
}

function createInitialPresets(): Preset[] {
  const presets: Preset[] = [];
  for (let idx = 1 as PresetId; idx <= 5; idx = (idx + 1) as PresetId) {
    presets.push({
      preset_id: idx,
      mode: "cc",
      target_i_ma: 1_500 + (idx - 1) * 250,
      target_v_mv: 12_000,
      min_v_mv: 0,
      max_i_ma_total: 10_000,
      max_p_mw: 150_000,
    });
  }
  return presets;
}

function createInitialIdentity(baseUrl: string, index: number): Identity {
  const deviceId = `llx-mock-${String(index).padStart(3, "0")}`;

  return {
    device_id: deviceId,
    digital_fw_version:
      "digital 0.1.0 (profile mock, v0.1.0-mock, src 0x0000000000000000)",
    analog_fw_version:
      "analog 0.1.0 (profile mock, v0.1.0-mock, src 0x0000000000000000)",
    protocol_version: 1,
    uptime_ms: 123_456,
    network: {
      ip: "127.0.0.1",
      mac: "00:00:00:00:00:00",
      hostname: new URL(baseUrl).hostname || "loadlynx-mock",
    },
    // Mock FQDN: loadlynx-<short_id>.local
    hostname: `loadlynx-${String(index).padStart(6, "a")}.local`,
    // Mock Short ID: 6-char hex-like string
    short_id: String(index).padStart(6, "a"),
    capabilities: {
      cc_supported: true,
      cv_supported: true,
      cp_supported: false,
      presets_supported: true,
      preset_count: 5,
      api_version: "2.0.0-mock",
    },
  };
}

function getOrCreateMockDevice(baseUrl: string): MockDeviceState {
  const existing = mockDevices.get(baseUrl);
  if (existing) {
    return existing;
  }

  const index = mockDevices.size + 1;
  const identity = createInitialIdentity(baseUrl, index);
  const status = createInitialStatus(baseUrl);
  const cc = createInitialCc();
  const pd = createInitialPd(baseUrl);
  const presets = createInitialPresets();
  const factoryProfile = createInitialCalibrationProfileWire();
  const calibration: MockCalibrationState = {
    factory: structuredClone(factoryProfile),
    ram: structuredClone(factoryProfile),
    eeprom: null,
  };

  const state: MockDeviceState = {
    identity,
    status,
    cc,
    pd,
    presets,
    active_preset_id: 1,
    output_enabled: false,
    uv_latched: false,
    calibrationMode: "off",
    calibration,
  };

  mockDevices.set(baseUrl, state);
  return state;
}

async function mockGetIdentity(baseUrl: string): Promise<Identity> {
  const state = getOrCreateMockDevice(baseUrl);
  return structuredClone(state.identity);
}

async function mockGetStatus(baseUrl: string): Promise<FastStatusView> {
  const state = getOrCreateMockDevice(baseUrl);
  const next = { ...state.status, raw: { ...state.status.raw } };
  // Simple uptime tick to show the value changing between refreshes.
  next.raw.uptime_ms += 1_000;

  // Injection of raw values based on calibration mode
  switch (state.calibrationMode) {
    case "voltage":
      // Inject dummy raw voltage values
      // In a real device these would fluctuate; here we can just mirror the parsed values * 10 or similar
      next.raw.cal_kind = 1; // dummy enum value for voltage
      // Keep within firmware range (i16) so captured candidates can be applied.
      // Treat raw_*_100uv as a scaled-down ADC-domain representation (~V/4).
      next.raw.raw_v_nr_100uv = clampI16(Math.round(next.raw.v_local_mv * 2.5));
      next.raw.raw_v_rmt_100uv = clampI16(
        Math.round(next.raw.v_remote_mv * 2.5),
      );
      break;
    case "current_ch1":
      next.raw.cal_kind = 2; // dummy
      // Keep within firmware range (i16). Model a small shunt voltage at ADC.
      next.raw.raw_cur_100uv = clampI16(Math.round(next.raw.i_local_ma / 2));
      next.raw.raw_dac_code = Math.floor(
        (next.raw.target_value /
          (state.cc.limit_profile.max_i_ma > 0
            ? state.cc.limit_profile.max_i_ma
            : 1)) *
          4095,
      );
      break;
    case "current_ch2":
      next.raw.cal_kind = 3; // dummy
      next.raw.raw_cur_100uv = clampI16(Math.round(next.raw.i_remote_ma / 2));
      next.raw.raw_dac_code = Math.floor(
        (next.raw.target_value /
          (state.cc.limit_profile.max_i_ma > 0
            ? state.cc.limit_profile.max_i_ma
            : 1)) *
          4095,
      );
      break;
    default:
      delete next.raw.cal_kind;
      delete next.raw.raw_v_nr_100uv;
      delete next.raw.raw_v_rmt_100uv;
      delete next.raw.raw_cur_100uv;
      delete next.raw.raw_dac_code;
      break;
  }

  state.status = next;
  return structuredClone(next);
}

async function mockGetCc(baseUrl: string): Promise<CcControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  return structuredClone(state.cc);
}

async function mockUpdateCc(
  baseUrl: string,
  payload: CcUpdateRequest,
): Promise<CcControlView> {
  const state = getOrCreateMockDevice(baseUrl);

  // Rule A: when target is 0, force enable=false.
  const nextTargetIMa = payload.target_i_ma;
  const nextEnable = nextTargetIMa === 0 ? false : payload.enable;
  const nextEffectiveIMa = nextEnable ? nextTargetIMa : 0;

  const nextCc: CcControlView = {
    ...state.cc,
    enable: nextEnable,
    target_i_ma: nextTargetIMa,
    effective_i_ma: nextEffectiveIMa,
    limit_profile: {
      ...state.cc.limit_profile,
      max_i_ma: payload.max_i_ma ?? state.cc.limit_profile.max_i_ma,
      max_p_mw: payload.max_p_mw ?? state.cc.limit_profile.max_p_mw,
      ovp_mv: payload.ovp_mv ?? state.cc.limit_profile.ovp_mv,
      temp_trip_mc: payload.temp_trip_mc ?? state.cc.limit_profile.temp_trip_mc,
      thermal_derate_pct:
        payload.thermal_derate_pct ?? state.cc.limit_profile.thermal_derate_pct,
    },
    protection: {
      ...state.cc.protection,
      voltage_mode: payload.voltage_mode ?? state.cc.protection.voltage_mode,
      power_mode: payload.power_mode ?? state.cc.protection.power_mode,
    },
    i_total_ma: state.cc.i_total_ma,
    v_main_mv: state.cc.v_main_mv,
    p_main_mw: state.cc.p_main_mw,
  };

  // Very simple output model: if enabled, we assume actual current tracks
  // effective at ~95%, otherwise 0. Power is derived from voltage and current.
  if (nextCc.effective_i_ma > 0) {
    const clampedTarget = Math.min(
      nextCc.effective_i_ma,
      nextCc.limit_profile.max_i_ma,
    );
    nextCc.i_total_ma = Math.round(clampedTarget * 0.95);
    nextCc.p_main_mw = Math.round(
      (nextCc.i_total_ma * nextCc.v_main_mv) / 1_000,
    );
  } else {
    nextCc.i_total_ma = 0;
    nextCc.p_main_mw = 0;
  }

  state.cc = nextCc;

  // Keep FastStatusView roughly in sync with CC control.
  const nextStatus: FastStatusView = {
    ...state.status,
    raw: {
      ...state.status.raw,
      enable: nextCc.enable,
      target_value: nextCc.effective_i_ma,
      i_local_ma: Math.round(nextCc.i_total_ma * 0.9),
      i_remote_ma: nextCc.i_total_ma - Math.round(nextCc.i_total_ma * 0.9),
      v_local_mv: nextCc.v_main_mv,
      v_remote_mv: nextCc.v_main_mv - 20,
      calc_p_mw: nextCc.p_main_mw,
    },
  };

  state.status = nextStatus;

  return structuredClone(nextCc);
}

function mockInvalidRequest(message: string): never {
  throw new HttpApiError({
    status: 400,
    code: "INVALID_REQUEST",
    message,
    retryable: false,
    details: null,
  });
}

function assertPresetId(presetId: number): PresetId {
  if (!Number.isFinite(presetId) || !Number.isInteger(presetId)) {
    mockInvalidRequest("preset_id must be an integer");
  }
  if (presetId < 1 || presetId > 5) {
    mockInvalidRequest("preset_id out of range (expected 1..=5)");
  }
  return presetId as PresetId;
}

function mockGetActivePreset(state: MockDeviceState): Preset {
  const preset = state.presets.find(
    (p) => p.preset_id === state.active_preset_id,
  );
  if (!preset) {
    mockInvalidRequest("active preset missing");
  }
  return preset;
}

function mockMakeControlView(state: MockDeviceState): ControlView {
  return {
    active_preset_id: state.active_preset_id,
    output_enabled: state.output_enabled,
    uv_latched: state.uv_latched,
    preset: structuredClone(mockGetActivePreset(state)),
  };
}

function mockUpdateStatusFromControl(state: MockDeviceState) {
  const preset = mockGetActivePreset(state);
  const next = { ...state.status, raw: { ...state.status.raw } };

  next.raw.mode = preset.mode === "cc" ? 1 : 2;
  next.raw.enable = state.output_enabled;

  const vMv = next.raw.v_remote_mv ?? 0;

  if (!state.output_enabled) {
    next.raw.target_value = 0;
    next.raw.i_local_ma = 0;
    next.raw.i_remote_ma = 0;
    next.raw.calc_p_mw = 0;
    state.status = next;
    return;
  }

  if (preset.mode === "cc") {
    const unclampedIMa = Math.max(0, preset.target_i_ma);
    const iLimit = Math.max(0, preset.max_i_ma_total);
    const pLimitMw = Math.max(0, preset.max_p_mw);
    const pLimitedIMa =
      vMv > 0 ? Math.floor((pLimitMw * 1000) / vMv) : unclampedIMa;
    const iMa = Math.min(unclampedIMa, iLimit, pLimitedIMa);
    next.raw.target_value = iMa;
    next.raw.i_local_ma = Math.round(iMa * 0.9);
    next.raw.i_remote_ma = iMa - next.raw.i_local_ma;
    next.raw.calc_p_mw = Math.round((iMa * vMv) / 1000);
  } else {
    // Extremely simple CV approximation: track target voltage, draw a small current.
    next.raw.v_remote_mv = Math.max(0, preset.target_v_mv);
    next.raw.v_local_mv = next.raw.v_remote_mv + 50;
    const iMa = Math.min(Math.max(0, preset.max_i_ma_total), 1_000);
    next.raw.target_value = iMa;
    next.raw.i_local_ma = Math.round(iMa * 0.9);
    next.raw.i_remote_ma = iMa - next.raw.i_local_ma;
    next.raw.calc_p_mw = Math.round((iMa * next.raw.v_remote_mv) / 1000);
  }

  state.status = next;
}

async function mockGetPresets(baseUrl: string): Promise<{ presets: Preset[] }> {
  const state = getOrCreateMockDevice(baseUrl);
  return { presets: structuredClone(state.presets) };
}

async function mockUpdatePreset(
  baseUrl: string,
  payload: Preset,
): Promise<Preset> {
  const state = getOrCreateMockDevice(baseUrl);
  const presetId = assertPresetId(payload.preset_id);

  const idx = state.presets.findIndex((p) => p.preset_id === presetId);
  if (idx < 0) {
    mockInvalidRequest("preset not found");
  }

  const nextPreset: Preset = {
    preset_id: presetId,
    mode: payload.mode,
    target_i_ma: Number.isFinite(payload.target_i_ma) ? payload.target_i_ma : 0,
    target_v_mv: Number.isFinite(payload.target_v_mv) ? payload.target_v_mv : 0,
    min_v_mv: Number.isFinite(payload.min_v_mv) ? payload.min_v_mv : 0,
    max_i_ma_total: Number.isFinite(payload.max_i_ma_total)
      ? payload.max_i_ma_total
      : 0,
    max_p_mw: Number.isFinite(payload.max_p_mw) ? payload.max_p_mw : 0,
  };

  state.presets[idx] = nextPreset;
  mockUpdateStatusFromControl(state);
  return structuredClone(nextPreset);
}

async function mockApplyPreset(
  baseUrl: string,
  preset_id: number,
): Promise<ControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  const presetId = assertPresetId(preset_id);

  state.active_preset_id = presetId;
  // Spec requirement: applying a preset forces output off.
  state.output_enabled = false;
  mockUpdateStatusFromControl(state);

  return mockMakeControlView(state);
}

async function mockGetControl(baseUrl: string): Promise<ControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  mockUpdateStatusFromControl(state);
  return mockMakeControlView(state);
}

async function mockUpdateControl(
  baseUrl: string,
  payload: { output_enabled: boolean },
): Promise<ControlView> {
  const state = getOrCreateMockDevice(baseUrl);

  const nextOutputEnabled = Boolean(payload.output_enabled);
  const prevOutputEnabled = state.output_enabled;

  state.output_enabled = nextOutputEnabled;

  // Spec requirement: uv_latched clears only on output off->on edge.
  if (!prevOutputEnabled && nextOutputEnabled && state.uv_latched) {
    state.uv_latched = false;
  }

  // Derived rule (mock-only): latch UV when enabled and v < min_v_mv.
  if (nextOutputEnabled) {
    const preset = mockGetActivePreset(state);
    const vMv = state.status.raw.v_remote_mv ?? 0;
    if (preset.min_v_mv > 0 && vMv < preset.min_v_mv) {
      state.uv_latched = true;
    }
  }

  mockUpdateStatusFromControl(state);
  return mockMakeControlView(state);
}

async function mockDebugSetUvLatched(
  baseUrl: string,
  uv_latched: boolean,
): Promise<ControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  state.uv_latched = Boolean(uv_latched);
  return mockMakeControlView(state);
}

async function mockSoftReset(
  baseUrl: string,
  reason: string,
): Promise<{ accepted: boolean; reason: string }> {
  const state = getOrCreateMockDevice(baseUrl);
  // Simulate a reboot: calibration apply is RAM-only, commit persists.
  state.calibrationMode = "off";
  if (state.calibration.eeprom) {
    state.calibration.ram = structuredClone(state.calibration.eeprom);
    state.calibration.ram.active.source = "user-calibrated";
  } else {
    state.calibration.ram = structuredClone(state.calibration.factory);
  }
  return {
    accepted: true,
    reason,
  };
}

function mockRequirePdSupported(state: MockDeviceState): PdView {
  if (!state.pd) {
    throw new HttpApiError({
      status: 404,
      code: "UNSUPPORTED_OPERATION",
      message: "USB-PD API not supported by this device",
      retryable: false,
      details: null,
    });
  }
  return state.pd;
}

function mockRequirePdReady(state: MockDeviceState): void {
  if (!state.status.link_up) {
    throw new HttpApiError({
      status: 503,
      code: "LINK_DOWN",
      message: "UART link is down",
      retryable: true,
      details: null,
    });
  }
  if (state.status.analog_state === "cal_missing") {
    throw new HttpApiError({
      status: 503,
      code: "ANALOG_NOT_READY",
      message: "Analog is not ready (calibration missing)",
      retryable: true,
      details: null,
    });
  }
  if (state.status.analog_state === "faulted") {
    throw new HttpApiError({
      status: 409,
      code: "ANALOG_FAULTED",
      message: "Analog is faulted",
      retryable: false,
      details: null,
    });
  }
}

async function mockGetPd(baseUrl: string): Promise<PdView> {
  const state = getOrCreateMockDevice(baseUrl);
  mockRequirePdReady(state);
  return structuredClone(mockRequirePdSupported(state));
}

async function mockUpdatePd(
  baseUrl: string,
  payload: PdUpdateRequest,
): Promise<PdView> {
  const state = getOrCreateMockDevice(baseUrl);
  mockRequirePdReady(state);

  const current = mockRequirePdSupported(state);
  if (!current.attached) {
    throw new HttpApiError({
      status: 409,
      code: "NOT_ATTACHED",
      message: "PD is not attached",
      retryable: true,
      details: null,
    });
  }

  const next = structuredClone(current);
  const nowMs = state.status.raw.uptime_ms ?? 0;

  const limitViolation = (message: string, details?: unknown) => {
    throw new HttpApiError({
      status: 422,
      code: "LIMIT_VIOLATION",
      message,
      retryable: false,
      details: details ?? null,
    });
  };

  const objectPos = payload.object_pos;
  if (payload.mode === "fixed") {
    const pdo = next.fixed_pdos.find((entry) => entry.pos === objectPos);
    if (!pdo) {
      limitViolation("Selected fixed PDO does not exist", {
        object_pos: objectPos,
      });
    }
    if (payload.i_req_ma < 0 || payload.i_req_ma > pdo.max_ma) {
      limitViolation("Ireq exceeds Imax for selected PDO", {
        i_req_ma: payload.i_req_ma,
        max_ma: pdo.max_ma,
      });
    }

    next.saved = {
      ...next.saved,
      mode: "fixed",
      fixed_object_pos: objectPos,
      i_req_ma: payload.i_req_ma,
    };
    next.contract_mv = pdo.mv;
    next.contract_ma = payload.i_req_ma;
  } else {
    const apdo = next.pps_pdos.find((entry) => entry.pos === objectPos);
    if (!apdo) {
      limitViolation("Selected PPS APDO does not exist", {
        object_pos: objectPos,
      });
    }
    if (payload.target_mv < apdo.min_mv || payload.target_mv > apdo.max_mv) {
      limitViolation("Vreq is out of range for selected APDO", {
        target_mv: payload.target_mv,
        min_mv: apdo.min_mv,
        max_mv: apdo.max_mv,
      });
    }
    if (payload.i_req_ma < 0 || payload.i_req_ma > apdo.max_ma) {
      limitViolation("Ireq exceeds Imax for selected APDO", {
        i_req_ma: payload.i_req_ma,
        max_ma: apdo.max_ma,
      });
    }

    next.saved = {
      ...next.saved,
      mode: "pps",
      pps_object_pos: objectPos,
      target_mv: payload.target_mv,
      i_req_ma: payload.i_req_ma,
    };
    next.contract_mv = payload.target_mv;
    next.contract_ma = payload.i_req_ma;
  }

  next.apply = {
    pending: false,
    last: { code: "ok", at_ms: nowMs },
  };

  state.pd = next;
  return structuredClone(next);
}

export async function getIdentity(baseUrl: string): Promise<Identity> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetIdentity(baseUrl);
  }
  return httpJsonQueued<Identity>(baseUrl, "/api/v1/identity");
}

export async function getStatus(baseUrl: string): Promise<FastStatusView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetStatus(baseUrl);
  }
  interface FastStatusHttpResponse {
    status: FastStatusJson;
    link_up: boolean;
    hello_seen: boolean;
    analog_state: FastStatusView["analog_state"];
    fault_flags_decoded: FastStatusView["fault_flags_decoded"];
  }

  const payload = await httpJsonQueued<FastStatusHttpResponse>(
    baseUrl,
    "/api/v1/status",
  );

  return {
    raw: payload.status,
    link_up: payload.link_up,
    hello_seen: payload.hello_seen,
    analog_state: payload.analog_state,
    fault_flags_decoded: payload.fault_flags_decoded,
  };
}

export function subscribeStatusStream(
  baseUrl: string,
  onMessage: (view: FastStatusView) => void,
  onError?: (error: Event | Error) => void,
): () => void {
  if (isMockBaseUrl(baseUrl)) {
    let stopped = false;
    const timer = setInterval(async () => {
      if (stopped) {
        return;
      }
      try {
        const next = await mockGetStatus(baseUrl);
        onMessage(next);
      } catch (error) {
        if (onError) {
          onError(
            error instanceof Error ? error : new Error("mock stream error"),
          );
        }
      }
    }, 300);

    return () => {
      stopped = true;
      clearInterval(timer);
    };
  }

  if (isStorybookRuntime()) {
    throw new Error(
      `[LoadLynx] Real device status streaming is disabled in Storybook. Use a mock:// baseUrl instead (attempted baseUrl="${baseUrl}").`,
    );
  }

  const url = new URL("/api/v1/status", baseUrl);
  let closed = false;

  const isFastStatusView = (val: unknown): val is FastStatusView => {
    return (
      typeof val === "object" &&
      val !== null &&
      "raw" in val &&
      "link_up" in val &&
      "hello_seen" in val
    );
  };

  const emitMessage = (view: FastStatusView) => {
    if (closed) {
      return;
    }
    onMessage(view);
  };

  const emitError = (error: Event | Error) => {
    if (closed) {
      return;
    }
    if (onError) {
      onError(error);
    }
  };

  const parseAndEmit = (payload: string) => {
    try {
      const parsed = JSON.parse(payload) as
        | FastStatusView
        | {
            status: FastStatusJson;
            link_up: boolean;
            hello_seen: boolean;
            analog_state: FastStatusView["analog_state"];
            fault_flags_decoded: FastStatusView["fault_flags_decoded"];
          };

      const view: FastStatusView = isFastStatusView(parsed)
        ? parsed
        : {
            raw: parsed.status,
            link_up: parsed.link_up,
            hello_seen: parsed.hello_seen,
            analog_state: parsed.analog_state,
            fault_flags_decoded: parsed.fault_flags_decoded ?? [],
          };

      emitMessage(view);
    } catch (error) {
      emitError(
        error instanceof Error ? error : new Error("invalid SSE payload"),
      );
    }
  };

  const handleStatus = (event: MessageEvent) => {
    parseAndEmit(event.data);
  };

  const handleError = (event: Event) => {
    emitError(event);
  };

  // Prevent multiple browser tabs from opening parallel SSE connections to the
  // device. The embedded HTTP server has a small worker pool; one SSE stream is
  // enough and can be fan-out broadcast to other tabs.
  const canShareAcrossTabs =
    typeof BroadcastChannel !== "undefined" &&
    typeof navigator !== "undefined" &&
    "locks" in navigator;

  if (!canShareAcrossTabs) {
    const source = new EventSource(url.toString());
    source.addEventListener("status", handleStatus as EventListener);
    source.addEventListener("message", handleStatus as EventListener);
    source.addEventListener("error", handleError);

    return () => {
      closed = true;
      source.removeEventListener("status", handleStatus as EventListener);
      source.removeEventListener("message", handleStatus as EventListener);
      source.removeEventListener("error", handleError);
      source.close();
    };
  }

  const lockName = `llx-status-sse:${new URL(baseUrl).origin}`;
  const channel = new BroadcastChannel(lockName);

  let releaseLeader: (() => void) | null = null;

  type BroadcastEnvelope =
    | { t: "status"; d: string; from: string }
    | { t: "bye"; from: string };

  void navigator.locks
    .request(lockName, { mode: "exclusive" }, async () => {
      if (closed) {
        return;
      }

      let resolveRelease: (() => void) | null = null;
      const waitRelease = new Promise<void>((resolve) => {
        resolveRelease = resolve;
      });
      releaseLeader = () => {
        resolveRelease?.();
      };

      const leaderSource = new EventSource(url.toString());

      const broadcastStatus = (event: MessageEvent) => {
        const msg: BroadcastEnvelope = {
          t: "status",
          d: event.data,
          from: TAB_ID,
        };
        try {
          channel.postMessage(msg);
        } catch {
          // Best-effort fan-out; the channel may already be closed during cleanup.
        }
        parseAndEmit(event.data);
      };
      const broadcastError = (event: Event) => {
        handleError(event);
      };

      leaderSource.addEventListener("status", broadcastStatus as EventListener);
      leaderSource.addEventListener(
        "message",
        broadcastStatus as EventListener,
      );
      leaderSource.addEventListener("error", broadcastError);

      try {
        await waitRelease;
      } finally {
        leaderSource.removeEventListener(
          "status",
          broadcastStatus as EventListener,
        );
        leaderSource.removeEventListener(
          "message",
          broadcastStatus as EventListener,
        );
        leaderSource.removeEventListener("error", broadcastError);
        leaderSource.close();

        try {
          channel.postMessage({
            t: "bye",
            from: TAB_ID,
          } satisfies BroadcastEnvelope);
        } catch {
          // ignore
        }
        releaseLeader = null;
      }
    })
    .catch((error) => {
      emitError(
        error instanceof Error ? error : new Error("status lock error"),
      );
    });

  const onChannelMessage = (event: MessageEvent) => {
    if (closed) {
      return;
    }
    const payload = event.data as Partial<BroadcastEnvelope> | null;
    if (!payload || typeof payload !== "object" || payload.from === TAB_ID) {
      return;
    }
    if (payload.t === "status" && typeof payload.d === "string") {
      parseAndEmit(payload.d);
      return;
    }
  };

  channel.addEventListener("message", onChannelMessage);

  return () => {
    closed = true;
    releaseLeader?.();
    releaseLeader = null;
    channel.removeEventListener("message", onChannelMessage);
    channel.close();
  };
}

export async function getCc(baseUrl: string): Promise<CcControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetCc(baseUrl);
  }
  return httpJsonQueued<CcControlView>(baseUrl, "/api/v1/cc");
}

export async function getPd(baseUrl: string): Promise<PdView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetPd(baseUrl);
  }
  return httpJsonQueued<PdView>(baseUrl, "/api/v1/pd");
}

export async function postPd(
  baseUrl: string,
  payload: PdUpdateRequest,
): Promise<PdView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockUpdatePd(baseUrl, payload);
  }

  const body = JSON.stringify(payload);

  // Use POST + text/plain to stay within the CORS simple-request surface and
  // avoid私网预检；body is JSON string as documented in docs/interfaces/network-http-api.md.
  return httpJsonQueued<PdView>(baseUrl, "/api/v1/pd", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function updateCc(
  baseUrl: string,
  payload: CcUpdateRequest,
): Promise<CcControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockUpdateCc(baseUrl, payload);
  }

  const body = JSON.stringify(payload);

  // Use POST + text/plain to stay within the CORS simple-request surface and
  // avoid私网预检；fetch 会发送 Content-Length，兼容设备端的小栈。
  return httpJsonQueued<CcControlView>(baseUrl, "/api/v1/cc", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

// Presets / unified control view (docs/interfaces/network-http-api.md §3.6..§3.10)

export async function getPresets(baseUrl: string): Promise<Preset[]> {
  if (isMockBaseUrl(baseUrl)) {
    const payload = await mockGetPresets(baseUrl);
    return payload.presets;
  }
  const payload = await httpJsonQueued<{ presets: Preset[] }>(
    baseUrl,
    "/api/v1/presets",
  );
  return payload.presets;
}

export async function updatePreset(
  baseUrl: string,
  payload: Preset,
): Promise<Preset> {
  if (isMockBaseUrl(baseUrl)) {
    return mockUpdatePreset(baseUrl, payload);
  }

  const body = JSON.stringify(payload);

  // Use POST + text/plain to stay within the CORS simple-request surface.
  return httpJsonQueued<Preset>(baseUrl, "/api/v1/presets", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function applyPreset(
  baseUrl: string,
  preset_id: number,
): Promise<ControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockApplyPreset(baseUrl, preset_id);
  }

  const body = JSON.stringify({ preset_id });

  return httpJsonQueued<ControlView>(baseUrl, "/api/v1/presets/apply", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function getControl(baseUrl: string): Promise<ControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetControl(baseUrl);
  }
  return httpJsonQueued<ControlView>(baseUrl, "/api/v1/control");
}

export async function updateControl(
  baseUrl: string,
  payload: { output_enabled: boolean },
): Promise<ControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockUpdateControl(baseUrl, payload);
  }

  const body = JSON.stringify(payload);

  // Use POST + text/plain to stay within the CORS simple-request surface.
  return httpJsonQueued<ControlView>(baseUrl, "/api/v1/control", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function __debugSetUvLatched(
  baseUrl: string,
  uv_latched: boolean,
): Promise<ControlView> {
  if (!isMockBaseUrl(baseUrl)) {
    throw new Error(
      "UV latch debug toggle is only available for mock:// devices",
    );
  }
  return mockDebugSetUvLatched(baseUrl, uv_latched);
}

export async function postSoftReset(
  baseUrl: string,
  reason:
    | "manual"
    | "firmware_update"
    | "ui_recover"
    | "link_recover" = "manual",
): Promise<{ accepted: boolean; reason: string }> {
  if (isMockBaseUrl(baseUrl)) {
    return mockSoftReset(baseUrl, reason);
  }

  const body = JSON.stringify({ reason });

  return httpJsonQueued<{ accepted: boolean; reason: string }>(
    baseUrl,
    "/api/v1/soft-reset",
    {
      method: "POST",
      body,
      headers: {
        "Content-Type": "application/json; charset=utf-8",
      },
    },
  );
}

// Calibration API

function mapCalibrationProfileWireToUi(
  profile: CalibrationProfileWire,
): CalibrationProfile {
  return {
    active: profile.active,
    v_local_points: profile.v_local_points.map((point) => ({
      raw: point.raw_100uv,
      mv: point.meas_mv,
    })),
    v_remote_points: profile.v_remote_points.map((point) => ({
      raw: point.raw_100uv,
      mv: point.meas_mv,
    })),
    current_ch1_points: profile.current_ch1_points.map((point) => ({
      raw: point.raw_100uv,
      ua: point.meas_ma * 1000,
      dac_code: point.raw_dac_code,
    })),
    current_ch2_points: profile.current_ch2_points.map((point) => ({
      raw: point.raw_100uv,
      ua: point.meas_ma * 1000,
      dac_code: point.raw_dac_code,
    })),
  };
}

function mapCalibrationWriteRequestToWire(
  payload: CalibrationApplyRequest,
): CalibrationWriteRequestWire {
  switch (payload.kind) {
    case "v_local":
    case "v_remote":
      return {
        kind: payload.kind,
        points: payload.points.map(
          (point): CalibrationPointVoltageWireCompact => [point.raw, point.mv],
        ),
      };
    case "current_ch1":
    case "current_ch2":
      return {
        kind: payload.kind,
        points: payload.points.map(
          (point): CalibrationPointCurrentWireCompact => [
            point.raw,
            point.dac_code,
            Math.floor((point.ua + 500) / 1000),
          ],
        ),
      };
  }
}

export async function getCalibrationProfile(
  baseUrl: string,
): Promise<CalibrationProfile> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetCalibrationProfile(baseUrl);
  }
  const payload = await httpJsonQueued<CalibrationProfileWire>(
    baseUrl,
    "/api/v1/calibration/profile",
  );
  return mapCalibrationProfileWireToUi(payload);
}

export async function postCalibrationApply(
  baseUrl: string,
  payload: CalibrationApplyRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationApply(baseUrl, payload);
  }
  const body = JSON.stringify(mapCalibrationWriteRequestToWire(payload));
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/apply", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function postCalibrationCommit(
  baseUrl: string,
  payload: CalibrationCommitRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationCommit(baseUrl, payload);
  }
  const body = JSON.stringify(mapCalibrationWriteRequestToWire(payload));
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/commit", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function postCalibrationReset(
  baseUrl: string,
  payload: CalibrationResetRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationReset(baseUrl, payload);
  }
  const body = JSON.stringify(payload);
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/reset", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function postCalibrationMode(
  baseUrl: string,
  payload: CalibrationModeRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationMode(baseUrl, payload);
  }
  const body = JSON.stringify(payload);
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/mode", {
    method: "POST",
    body,
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

// Mock Implementation Extensions (Calibration)

function mockCalValidationError(message: string): never {
  throw new HttpApiError({
    status: 400,
    code: "INVALID_REQUEST",
    message,
    retryable: false,
    details: null,
  });
}

function mockNormalizeWirePointsByRaw100uv<T extends { raw_100uv: number }>(
  kind: string,
  points: T[],
  measKey: string,
  getMeas: (point: T) => number,
): T[] {
  for (const point of points) {
    const raw = point.raw_100uv;
    if (!Number.isFinite(raw) || !Number.isInteger(raw)) {
      mockCalValidationError("raw_100uv must be an integer");
    }
    if (raw < -32768 || raw > 32767) {
      mockCalValidationError("raw_100uv out of range for i16");
    }

    const meas = getMeas(point);
    if (!Number.isFinite(meas) || !Number.isInteger(meas)) {
      mockCalValidationError(`${measKey} must be an integer`);
    }
  }

  // Allow repeated captures at the same measured value (keep the most recent).
  const measDeduped: T[] = [];
  for (const point of points) {
    const meas = getMeas(point);
    const idx = measDeduped.findIndex((p) => getMeas(p) === meas);
    if (idx < 0) measDeduped.push(point);
    else measDeduped[idx] = point;
  }

  // Small N (<=24): stable insertion sort by raw_100uv, then drop duplicates.
  const sorted = measDeduped.slice();
  for (let i = 1; i < sorted.length; i++) {
    let j = i;
    while (j > 0 && sorted[j - 1].raw_100uv > sorted[j].raw_100uv) {
      const tmp = sorted[j - 1];
      sorted[j - 1] = sorted[j];
      sorted[j] = tmp;
      j -= 1;
    }
  }

  // Dedup by raw_100uv (keep last occurrence).
  const deduped: T[] = [];
  for (const point of sorted) {
    const last = deduped[deduped.length - 1];
    if (last && last.raw_100uv === point.raw_100uv) {
      deduped[deduped.length - 1] = point;
    } else {
      deduped.push(point);
    }
  }

  for (let i = 1; i < deduped.length; i++) {
    if (getMeas(deduped[i]) <= getMeas(deduped[i - 1])) {
      mockCalValidationError(`meas must be strictly increasing for ${kind}`);
    }
  }

  if (deduped.length === 0) {
    mockCalValidationError(
      `points must contain 1..${CALIBRATION_MAX_POINTS} items`,
    );
  }
  if (deduped.length > CALIBRATION_MAX_POINTS) {
    mockCalValidationError(`too many points (max ${CALIBRATION_MAX_POINTS})`);
  }

  return deduped;
}

function mockNormalizeVoltageWirePoints(
  kind: "v_local" | "v_remote",
  points: CalibrationPointVoltageWireCompact[],
): CalibrationProfileWire["v_local_points"] {
  const normalized = points.map((point) => ({
    raw_100uv: point[0],
    meas_mv: point[1],
  }));
  return mockNormalizeWirePointsByRaw100uv(
    kind,
    normalized,
    "meas_mv",
    (p) => p.meas_mv,
  );
}

function mockNormalizeCurrentWirePoints(
  kind: "current_ch1" | "current_ch2",
  points: CalibrationPointCurrentWireCompact[],
): CalibrationProfileWire["current_ch1_points"] {
  const normalized = points.map((point) => ({
    raw_100uv: point[0],
    raw_dac_code: point[1],
    meas_ma: point[2],
  }));

  for (const point of normalized) {
    const dac = point.raw_dac_code;
    if (!Number.isFinite(dac) || !Number.isInteger(dac)) {
      mockCalValidationError("raw_dac_code must be an integer");
    }
    if (dac < 0 || dac > 65535) {
      mockCalValidationError("raw_dac_code out of range for u16");
    }
  }

  return mockNormalizeWirePointsByRaw100uv(
    kind,
    normalized,
    "meas_ma",
    (p) => p.meas_ma,
  );
}

function mockProfileWireEqualsFactory(
  profile: CalibrationProfileWire,
  factory: CalibrationProfileWire,
): boolean {
  if (
    profile.active.fmt_version !== factory.active.fmt_version ||
    profile.active.hw_rev !== factory.active.hw_rev
  ) {
    return false;
  }
  const eqVoltage = (
    a: { raw_100uv: number; meas_mv: number }[],
    b: { raw_100uv: number; meas_mv: number }[],
  ) => {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (a[i].raw_100uv !== b[i].raw_100uv || a[i].meas_mv !== b[i].meas_mv) {
        return false;
      }
    }
    return true;
  };

  const eqCurrent = (
    a: { raw_100uv: number; raw_dac_code: number; meas_ma: number }[],
    b: { raw_100uv: number; raw_dac_code: number; meas_ma: number }[],
  ) => {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (
        a[i].raw_100uv !== b[i].raw_100uv ||
        a[i].raw_dac_code !== b[i].raw_dac_code ||
        a[i].meas_ma !== b[i].meas_ma
      ) {
        return false;
      }
    }
    return true;
  };

  return (
    eqCurrent(profile.current_ch1_points, factory.current_ch1_points) &&
    eqCurrent(profile.current_ch2_points, factory.current_ch2_points) &&
    eqVoltage(profile.v_local_points, factory.v_local_points) &&
    eqVoltage(profile.v_remote_points, factory.v_remote_points)
  );
}

async function mockGetCalibrationProfile(
  baseUrl: string,
): Promise<CalibrationProfile> {
  const state = getOrCreateMockDevice(baseUrl);
  return mapCalibrationProfileWireToUi(structuredClone(state.calibration.ram));
}

async function mockPostCalibrationApply(
  baseUrl: string,
  payload: CalibrationApplyRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  const wire = mapCalibrationWriteRequestToWire(payload);
  const ram = state.calibration.ram;
  ram.active.source = "user-calibrated";

  switch (wire.kind) {
    case "v_local":
      ram.v_local_points = mockNormalizeVoltageWirePoints(
        wire.kind,
        wire.points,
      );
      break;
    case "v_remote":
      ram.v_remote_points = mockNormalizeVoltageWirePoints(
        wire.kind,
        wire.points,
      );
      break;
    case "current_ch1":
      ram.current_ch1_points = mockNormalizeCurrentWirePoints(
        wire.kind,
        wire.points,
      );
      break;
    case "current_ch2":
      ram.current_ch2_points = mockNormalizeCurrentWirePoints(
        wire.kind,
        wire.points,
      );
      break;
  }
}

async function mockPostCalibrationCommit(
  baseUrl: string,
  payload: CalibrationCommitRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  await mockPostCalibrationApply(baseUrl, payload);
  state.calibration.eeprom = structuredClone(state.calibration.ram);
}

async function mockPostCalibrationReset(
  baseUrl: string,
  payload: CalibrationResetRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  const { kind } = payload;

  if (kind === "all") {
    state.calibration.ram = structuredClone(state.calibration.factory);
    state.calibration.eeprom = null;
    return;
  }

  const factory = state.calibration.factory;
  const ram = state.calibration.ram;

  switch (kind) {
    case "v_local":
      ram.v_local_points = structuredClone(factory.v_local_points);
      break;
    case "v_remote":
      ram.v_remote_points = structuredClone(factory.v_remote_points);
      break;
    case "current_ch1":
      ram.current_ch1_points = structuredClone(factory.current_ch1_points);
      break;
    case "current_ch2":
      ram.current_ch2_points = structuredClone(factory.current_ch2_points);
      break;
  }

  if (mockProfileWireEqualsFactory(ram, factory)) {
    state.calibration.ram = structuredClone(factory);
    state.calibration.eeprom = null;
  } else {
    ram.active.source = "user-calibrated";
    state.calibration.eeprom = structuredClone(ram);
  }
}

async function mockPostCalibrationMode(
  baseUrl: string,
  payload: CalibrationModeRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  state.calibrationMode = payload.kind;
}

import type {
  CcControlView,
  CcUpdateRequest,
  FastStatusJson,
  FastStatusView,
  Identity,
} from "./types.ts";

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
  const url = new URL(path, baseUrl);

  const headers: Record<string, string> = {
    ...(init?.headers as Record<string, string> | undefined),
  };

  // Embedded servers often have tiny connection limits; explicitly request
  // connection close to avoid keeping sockets busy between polls/mutations.
  headers.Connection ||= "close";

  const method = init?.method ?? "GET";
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

// Mock Calibration Types & State
import type {
  CalibrationApplyRequest,
  CalibrationCommitRequest,
  CalibrationModeRequest,
  CalibrationProfile,
  CalibrationResetRequest,
} from "./types.ts";

interface MockDeviceState {
  identity: Identity;
  status: FastStatusView;
  cc: CcControlView;
  calibrationMode: CalibrationModeRequest["kind"];
  calibrationProfile: CalibrationProfile;
}

function createInitialCalibrationProfile(): CalibrationProfile {
  return {
    v_local_points: [
      { raw: 0, mv: 0 },
      // raw_v_* is reported in 100uV units => 40V is 400_000 * 100uV.
      { raw: 400_000, mv: 40_000 },
    ],
    v_remote_points: [
      { raw: 0, mv: 0 },
      { raw: 400_000, mv: 40_000 },
    ],
    current_ch1_points: [
      { raw: 0, ma: 0, dac_code: 0 },
      // raw_cur_100uv is an arbitrary mock scaling of i_ma * 100.
      { raw: 500_000, ma: 5_000, dac_code: 4095 },
    ],
    current_ch2_points: [
      { raw: 0, ma: 0, dac_code: 0 },
      { raw: 500_000, ma: 5_000, dac_code: 4095 },
    ],
  };
}

const mockDevices = new Map<string, MockDeviceState>();

function createInitialStatus(): FastStatusView {
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

  return {
    raw,
    link_up: true,
    hello_seen: true,
    analog_state: "ready",
    fault_flags_decoded: [],
  };
}

function createInitialCc(): CcControlView {
  return {
    enable: false,
    target_i_ma: 1_500,
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
      cv_supported: false,
      cp_supported: false,
      api_version: "1.0.0-mock",
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
  const status = createInitialStatus();
  const cc = createInitialCc();
  const calibrationProfile = createInitialCalibrationProfile();

  const state: MockDeviceState = {
    identity,
    status,
    cc,
    calibrationMode: "off",
    calibrationProfile,
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
      next.raw.raw_v_nr_100uv = next.raw.v_local_mv * 10;
      next.raw.raw_v_rmt_100uv = next.raw.v_remote_mv * 10;
      break;
    case "current_ch1":
      next.raw.cal_kind = 2; // dummy
      next.raw.raw_cur_100uv = next.raw.i_local_ma * 100; // mA to 100uv roughly
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
      next.raw.raw_cur_100uv = next.raw.i_remote_ma * 100;
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

  const nextCc: CcControlView = {
    ...state.cc,
    enable: payload.enable,
    target_i_ma: payload.target_i_ma,
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
  // target at ~95%, otherwise 0. Power is derived from voltage and current.
  if (nextCc.enable) {
    const clampedTarget = Math.min(
      nextCc.target_i_ma,
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
      target_value: nextCc.target_i_ma,
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

async function mockSoftReset(
  baseUrl: string,
  reason: string,
): Promise<{ accepted: boolean; reason: string }> {
  // Ensure the device exists in the mock registry so identity/status remain
  // consistent, but we do not currently simulate side effects.
  getOrCreateMockDevice(baseUrl);
  return {
    accepted: true,
    reason,
  };
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

  const url = new URL("/api/v1/status", baseUrl);
  const source = new EventSource(url.toString());

  const isFastStatusView = (val: unknown): val is FastStatusView => {
    return (
      typeof val === "object" &&
      val !== null &&
      "raw" in val &&
      "link_up" in val &&
      "hello_seen" in val
    );
  };

  const handleStatus = (event: MessageEvent) => {
    try {
      const parsed = JSON.parse(event.data) as
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

      onMessage(view);
    } catch (error) {
      if (onError) {
        onError(
          error instanceof Error ? error : new Error("invalid SSE payload"),
        );
      }
    }
  };

  const handleError = (event: Event) => {
    if (onError) {
      onError(event);
    }
  };

  source.addEventListener("status", handleStatus as EventListener);
  source.addEventListener("message", handleStatus as EventListener);
  source.addEventListener("error", handleError);

  return () => {
    source.removeEventListener("status", handleStatus as EventListener);
    source.removeEventListener("message", handleStatus as EventListener);
    source.removeEventListener("error", handleError);
    source.close();
  };
}

export async function getCc(baseUrl: string): Promise<CcControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetCc(baseUrl);
  }
  return httpJsonQueued<CcControlView>(baseUrl, "/api/v1/cc");
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

export async function getCalibrationProfile(
  baseUrl: string,
): Promise<CalibrationProfile> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetCalibrationProfile(baseUrl);
  }
  return httpJsonQueued<CalibrationProfile>(
    baseUrl,
    "/api/v1/calibration/profile",
  );
}

export async function postCalibrationApply(
  baseUrl: string,
  payload: CalibrationApplyRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationApply(baseUrl, payload);
  }
  const body = JSON.stringify(payload);
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/apply", {
    method: "POST",
    body,
  });
}

export async function postCalibrationCommit(
  baseUrl: string,
  payload: CalibrationCommitRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationCommit(baseUrl, payload);
  }
  const body = JSON.stringify(payload);
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/commit", {
    method: "POST",
    body,
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
  });
}

// Mock Implementation Extensions (Calibration)

async function mockGetCalibrationProfile(
  baseUrl: string,
): Promise<CalibrationProfile> {
  const state = getOrCreateMockDevice(baseUrl);
  return structuredClone(state.calibrationProfile);
}

async function mockPostCalibrationApply(
  baseUrl: string,
  payload: CalibrationApplyRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  // Update the transient profile
  if (payload.kind === "v_local")
    state.calibrationProfile.v_local_points = payload.points;
  if (payload.kind === "v_remote")
    state.calibrationProfile.v_remote_points = payload.points;
  if (payload.kind === "current_ch1")
    state.calibrationProfile.current_ch1_points = payload.points;
  if (payload.kind === "current_ch2")
    state.calibrationProfile.current_ch2_points = payload.points;
}

async function mockPostCalibrationCommit(
  baseUrl: string,
  _payload: CalibrationCommitRequest,
): Promise<void> {
  getOrCreateMockDevice(baseUrl);
  // In mock, commit effectively just means "we accepted it".
  // The applied profile is already in memory.
  // The applied profile is already in memory.
}

async function mockPostCalibrationReset(
  baseUrl: string,
  payload: CalibrationResetRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  const initial = createInitialCalibrationProfile();

  if (payload.kind === "v_local" || payload.kind === "both") {
    state.calibrationProfile.v_local_points = initial.v_local_points;
  }
  if (payload.kind === "v_remote" || payload.kind === "both") {
    state.calibrationProfile.v_remote_points = initial.v_remote_points;
  }
  if (payload.kind === "current_ch1") {
    state.calibrationProfile.current_ch1_points = initial.current_ch1_points;
  }
  if (payload.kind === "current_ch2") {
    state.calibrationProfile.current_ch2_points = initial.current_ch2_points;
  }
}

async function mockPostCalibrationMode(
  baseUrl: string,
  payload: CalibrationModeRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  state.calibrationMode = payload.kind;
}

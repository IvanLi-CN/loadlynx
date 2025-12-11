import type {
  CcControlView,
  CcUpdateRequest,
  FastStatusJson,
  FastStatusView,
  Identity,
} from "./types.ts";

// Mock backend is enabled on a per-device basis. Devices whose baseUrl starts
// with "mock://" use the in-memory backend when ENABLE_MOCK is true; all other
// devices use the real HTTP backend.
export const ENABLE_MOCK = import.meta.env.VITE_ENABLE_MOCK_BACKEND !== "false";

export function isMockBaseUrl(baseUrl: string): boolean {
  return ENABLE_MOCK && baseUrl.startsWith("mock://");
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

interface MockDeviceState {
  identity: Identity;
  status: FastStatusView;
  cc: CcControlView;
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

  const state: MockDeviceState = { identity, status, cc };
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

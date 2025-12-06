import type {
  CcControlView,
  CcUpdateRequest,
  FastStatusJson,
  FastStatusView,
  Identity,
} from "./types.ts";

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

export async function getIdentity(baseUrl: string): Promise<Identity> {
  const state = getOrCreateMockDevice(baseUrl);
  return structuredClone(state.identity);
}

export async function getStatus(baseUrl: string): Promise<FastStatusView> {
  const state = getOrCreateMockDevice(baseUrl);
  const next = { ...state.status, raw: { ...state.status.raw } };
  // Simple uptime tick to show the value changing between refreshes.
  next.raw.uptime_ms += 1_000;
  state.status = next;
  return structuredClone(next);
}

export async function getCc(baseUrl: string): Promise<CcControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  return structuredClone(state.cc);
}

export async function updateCc(
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

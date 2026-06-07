import type {
  CalibrationModeRequest,
  CalibrationProfileWire,
  CcControlView,
  FastStatusJson,
  FastStatusView,
  Identity,
  PdView,
  Preset,
  PresetId,
  WifiStatus,
} from "./types.ts";

export interface MockDeviceState {
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
  wifi: WifiStatus;
  wifiPsk: string;
}

export interface MockCalibrationState {
  factory: CalibrationProfileWire;
  ram: CalibrationProfileWire;
  eeprom: CalibrationProfileWire | null;
}

function createInitialCalibrationProfileWire(): CalibrationProfileWire {
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

export function clampI16(value: number): number {
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

  const hasRealFixed28 =
    normalized.includes("real-fixed28") || normalized.includes("real-fixed-28");
  const hasHiddenSavedFixed28 =
    normalized.includes("hidden-fixed28") ||
    normalized.includes("hidden-fixed-28");

  const fixed_pdos = [
    { pos: 1, mv: 5_000, max_ma: 3_000 },
    { pos: 2, mv: 9_000, max_ma: 3_000 },
    { pos: 3, mv: 12_000, max_ma: 3_000 },
    { pos: 4, mv: 15_000, max_ma: 3_000 },
    { pos: 5, mv: 20_000, max_ma: 1_500 },
  ];
  if (hasRealFixed28) {
    fixed_pdos.push({ pos: 8, mv: 28_000, max_ma: 5_000 });
  }

  const pps_pdos = [
    { pos: 3, min_mv: 3_300, max_mv: 21_000, max_ma: 3_000 },
    { pos: 4, min_mv: 5_000, max_mv: 11_000, max_ma: 2_000 },
  ];

  const detached =
    normalized.includes("detached") || normalized.includes("not-attached");
  const allow_extended_voltage =
    normalized.includes("extended") || hasRealFixed28 || hasHiddenSavedFixed28;

  const saved: PdView["saved"] = {
    mode: hasRealFixed28 || hasHiddenSavedFixed28 ? "fixed" : "pps",
    fixed_object_pos: hasRealFixed28 || hasHiddenSavedFixed28 ? 8 : 5,
    pps_object_pos: 3,
    target_mv: hasRealFixed28 || hasHiddenSavedFixed28 ? 28_000 : 9_000,
    pps_target_mv: 9_000,
    i_req_ma: hasRealFixed28 ? 3_000 : 2_000,
  };

  const view: PdView = {
    attached: !detached,
    contract_mv: null,
    contract_ma: null,
    fixed_pdos,
    pps_pdos,
    allow_extended_voltage,
    saved,
    apply: {
      pending: false,
      last: { code: "ok", at_ms: 123_456 },
    },
  };

  if (!detached) {
    if (!allow_extended_voltage) {
      const safePdo =
        fixed_pdos.find((entry) => entry.mv === 5_000) ?? fixed_pdos[0];
      const safeMaxMa = safePdo?.max_ma ?? saved.i_req_ma;
      view.contract_mv = 5_000;
      view.contract_ma = Math.min(saved.i_req_ma, safeMaxMa);
    } else if (saved.mode === "fixed") {
      const pdo =
        fixed_pdos.find((entry) => entry.pos === saved.fixed_object_pos) ??
        fixed_pdos[0];
      view.contract_mv = pdo?.mv ?? 5_000;
      view.contract_ma = saved.i_req_ma;
    } else {
      view.contract_mv = saved.target_mv;
      view.contract_ma = saved.i_req_ma;
    }
  }

  if (hasHiddenSavedFixed28) {
    view.contract_mv = 5_000;
    view.contract_ma = Math.min(saved.i_req_ma, fixed_pdos[0]?.max_ma ?? 3_000);
  }

  return view;
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
      target_p_mw: 10_000 + (idx - 1) * 2_000,
      min_v_mv: 0,
      max_i_ma_total: 10_000,
      max_p_mw: 150_000,
    });
  }
  return presets;
}

function createInitialIdentity(baseUrl: string, index: number): Identity {
  const deviceId = `llx-mock-${String(index).padStart(3, "0")}`;
  const normalized = baseUrl.toLowerCase();
  const cpSupported = !normalized.includes("no-cp");

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
    hostname: `loadlynx-${String(index).padStart(6, "a")}.local`,
    short_id: String(index).padStart(6, "a"),
    capabilities: {
      cc_supported: true,
      cv_supported: true,
      cp_supported: cpSupported,
      presets_supported: true,
      preset_count: 5,
      api_version: "2.0.0-mock",
    },
  };
}

export function normalizeDevdIdentity(
  baseUrl: string,
  payload: Partial<Identity> & { firmware_version?: unknown },
): Identity {
  const url = new URL(baseUrl);
  const deviceId =
    payload.device_id ?? url.searchParams.get("device_id") ?? "devd-usb";
  const hostname =
    payload.hostname ?? payload.network?.hostname ?? "loadlynx-devd-usb";
  return {
    device_id: deviceId,
    digital_fw_version:
      payload.digital_fw_version ??
      (typeof payload.firmware_version === "string"
        ? payload.firmware_version
        : "unknown"),
    analog_fw_version: payload.analog_fw_version ?? "unknown",
    protocol_version: payload.protocol_version ?? 1,
    uptime_ms: payload.uptime_ms ?? 0,
    network: {
      ip: payload.network?.ip ?? url.hostname,
      mac: payload.network?.mac ?? "unknown",
      hostname,
    },
    hostname,
    short_id: payload.short_id ?? deviceId,
    capabilities: {
      cc_supported: payload.capabilities?.cc_supported ?? true,
      cv_supported: payload.capabilities?.cv_supported ?? true,
      cp_supported: payload.capabilities?.cp_supported ?? true,
      presets_supported: payload.capabilities?.presets_supported ?? false,
      preset_count: payload.capabilities?.preset_count,
      api_version: payload.capabilities?.api_version ?? "devd-usb",
    },
  };
}

export function normalizeDevdStatus(payload: {
  status: Partial<FastStatusJson>;
  link_up?: boolean;
  hello_seen?: boolean;
  analog_state?: FastStatusView["analog_state"];
  fault_flags_decoded?: FastStatusView["fault_flags_decoded"];
  control?: { mode?: string; output_enabled?: boolean; target_p_mw?: number };
}): FastStatusView {
  const raw: FastStatusJson = {
    uptime_ms: payload.status.uptime_ms ?? 0,
    mode:
      payload.status.mode ??
      (payload.control?.mode === "cc"
        ? 1
        : payload.control?.mode === "cv"
          ? 2
          : 3),
    state_flags: payload.status.state_flags ?? 0,
    enable: payload.status.enable ?? payload.control?.output_enabled ?? false,
    target_value:
      payload.status.target_value ?? payload.control?.target_p_mw ?? 0,
    i_local_ma: payload.status.i_local_ma ?? 0,
    i_remote_ma: payload.status.i_remote_ma ?? 0,
    v_local_mv: payload.status.v_local_mv ?? 0,
    v_remote_mv: payload.status.v_remote_mv ?? 0,
    calc_p_mw: payload.status.calc_p_mw ?? 0,
    dac_headroom_mv: payload.status.dac_headroom_mv ?? 0,
    loop_error: payload.status.loop_error ?? 0,
    sink_core_temp_mc: payload.status.sink_core_temp_mc ?? 0,
    sink_exhaust_temp_mc: payload.status.sink_exhaust_temp_mc ?? 0,
    mcu_temp_mc: payload.status.mcu_temp_mc ?? 0,
    fault_flags: payload.status.fault_flags ?? 0,
    cal_kind: payload.status.cal_kind,
    raw_v_nr_100uv: payload.status.raw_v_nr_100uv,
    raw_v_rmt_100uv: payload.status.raw_v_rmt_100uv,
    raw_cur_100uv: payload.status.raw_cur_100uv,
    raw_dac_code: payload.status.raw_dac_code,
  };
  return {
    raw,
    link_up: payload.link_up ?? true,
    hello_seen: payload.hello_seen ?? true,
    analog_state: payload.analog_state ?? "ready",
    fault_flags_decoded: payload.fault_flags_decoded ?? [],
  };
}

export function getOrCreateMockDevice(baseUrl: string): MockDeviceState {
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
  const wifi: WifiStatus = {
    ssid: "LoadLynx Lab",
    source: "factory",
    state: "connected",
    ip: "192.0.2.10",
    last_error: null,
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
    wifi,
    wifiPsk: "factory-mock-psk",
  };

  if (baseUrl.toLowerCase().includes("calibration-output-applied")) {
    state.calibrationMode = "current_ch1";
    state.cc = {
      ...state.cc,
      enable: true,
      target_i_ma: 2_000,
      effective_i_ma: 2_000,
      i_total_ma: 1_900,
      p_main_mw: 22_800,
    };
    state.status = {
      ...state.status,
      raw: {
        ...state.status.raw,
        enable: true,
        target_value: 2_000,
        i_local_ma: 1_710,
        i_remote_ma: 190,
        calc_p_mw: 22_800,
      },
    };
  }

  mockDevices.set(baseUrl, state);
  return state;
}

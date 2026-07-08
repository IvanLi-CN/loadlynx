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
  simulation: MockSimulationState;
  wifiConnectPollsRemaining: number;
}

export interface MockCalibrationState {
  factory: CalibrationProfileWire;
  ram: CalibrationProfileWire;
  eeprom: CalibrationProfileWire | null;
}

export interface MockSimulationProfile {
  scenarioId: string;
  openCircuitMv: number;
  sourceResistanceMilliohm: number;
  localSenseBiasMv: number;
  rippleMv: number;
  currentNoiseMa: number;
  localCurrentShare: number;
  ambientTempMc: number;
  coreTempRisePerW: number;
  exhaustTempRisePerW: number;
  mcuTempRisePerW: number;
  phaseOffset: number;
}

export interface MockSimulationState {
  profile: MockSimulationProfile;
  lastWallClockMs: number;
}

interface MockScenarioBootState {
  activePresetId: PresetId;
  outputEnabled: boolean;
}

export type DevdIdentityPayload = Partial<Identity> & {
  firmware_version?: unknown;
};

export interface DevdControlCompatPayload {
  mode?: string;
  output_enabled?: boolean;
  target_p_mw?: number;
}

export interface DevdStatusPayload {
  status: Partial<FastStatusJson>;
  uptime_ms?: number;
  sink_core_temp_mc?: number;
  sink_exhaust_temp_mc?: number;
  mcu_temp_mc?: number;
  link_up?: boolean;
  hello_seen?: boolean;
  analog_state?: FastStatusView["analog_state"];
  fault_flags_decoded?: FastStatusView["fault_flags_decoded"];
  state_flags_decoded?: FastStatusView["state_flags_decoded"];
  control?: DevdControlCompatPayload;
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

function createMockSimulationProfile(
  baseUrl: string,
  index: number,
): MockSimulationProfile {
  const normalized = baseUrl.toLowerCase();
  const isDemo2 = normalized.includes("demo-2");

  if (isDemo2) {
    return {
      scenarioId: "bench-24v-cp",
      openCircuitMv: 19_800,
      sourceResistanceMilliohm: 820,
      localSenseBiasMv: 55,
      rippleMv: 110,
      currentNoiseMa: 55,
      localCurrentShare: 0.88,
      ambientTempMc: 27_800,
      coreTempRisePerW: 540,
      exhaustTempRisePerW: 340,
      mcuTempRisePerW: 190,
      phaseOffset: 1.35 + index * 0.31,
    };
  }

  return {
    scenarioId: "bench-12v-cc",
    openCircuitMv: 12_120,
    sourceResistanceMilliohm: 135,
    localSenseBiasMv: 42,
    rippleMv: 65,
    currentNoiseMa: 32,
    localCurrentShare: 0.91,
    ambientTempMc: 28_600,
    coreTempRisePerW: 710,
    exhaustTempRisePerW: 460,
    mcuTempRisePerW: 220,
    phaseOffset: 0.55 + index * 0.21,
  };
}

function createMockScenarioBootState(
  profile: MockSimulationProfile,
): MockScenarioBootState {
  if (profile.scenarioId === "bench-24v-cp") {
    return {
      activePresetId: 3,
      outputEnabled: true,
    };
  }

  return {
    activePresetId: 1,
    outputEnabled: true,
  };
}

const mockDevices = new Map<string, MockDeviceState>();

export function clampI16(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(-32768, Math.min(32767, value));
}

function createInitialStatus(
  baseUrl: string | undefined,
  profile: MockSimulationProfile,
): FastStatusView {
  const raw = {
    uptime_ms: 123_456,
    mode: 1,
    state_flags: 0,
    enable: false,
    target_value: 0,
    i_local_ma: 0,
    i_remote_ma: 0,
    v_local_mv: profile.openCircuitMv + profile.localSenseBiasMv,
    v_remote_mv: profile.openCircuitMv,
    calc_p_mw: 0,
    dac_headroom_mv: 500,
    loop_error: 0,
    sink_core_temp_mc: profile.ambientTempMc + 2_400,
    sink_exhaust_temp_mc: profile.ambientTempMc + 1_200,
    mcu_temp_mc: profile.ambientTempMc + 1_600,
    fault_flags: 0,
  };

  const view: FastStatusView = {
    raw,
    link_up: true,
    hello_seen: true,
    analog_state: "ready",
    fault_flags_decoded: [],
    state_flags_decoded: [],
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
    epr_active: false,
    epr_avs_pdos: [],
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

function createInitialCc(profile: MockSimulationProfile): CcControlView {
  const isHighPowerBench = profile.scenarioId === "bench-24v-cp";
  return {
    enable: false,
    target_i_ma: isHighPowerBench ? 2_400 : 1_850,
    effective_i_ma: 0,
    limit_profile: {
      max_i_ma: isHighPowerBench ? 6_000 : 5_000,
      max_p_mw: isHighPowerBench ? 120_000 : 60_000,
      ovp_mv: 40_000,
      temp_trip_mc: 80_000,
      thermal_derate_pct: 100,
    },
    protection: {
      voltage_mode: "protect",
      power_mode: "protect",
    },
    i_total_ma: 0,
    v_main_mv: profile.openCircuitMv,
    p_main_mw: 0,
  };
}

function createInitialPresets(profile: MockSimulationProfile): Preset[] {
  if (profile.scenarioId === "bench-24v-cp") {
    return [
      {
        preset_id: 1,
        mode: "cc",
        target_i_ma: 1_200,
        target_v_mv: 16_000,
        target_p_mw: 18_000,
        min_v_mv: 14_000,
        max_i_ma_total: 4_500,
        max_p_mw: 65_000,
      },
      {
        preset_id: 2,
        mode: "cv",
        target_i_ma: 1_000,
        target_v_mv: 15_000,
        target_p_mw: 22_000,
        min_v_mv: 13_500,
        max_i_ma_total: 2_800,
        max_p_mw: 50_000,
      },
      {
        preset_id: 3,
        mode: "cp",
        target_i_ma: 2_100,
        target_v_mv: 18_000,
        target_p_mw: 42_000,
        min_v_mv: 13_000,
        max_i_ma_total: 3_600,
        max_p_mw: 60_000,
      },
      {
        preset_id: 4,
        mode: "cp",
        target_i_ma: 2_500,
        target_v_mv: 19_000,
        target_p_mw: 58_000,
        min_v_mv: 12_500,
        max_i_ma_total: 4_200,
        max_p_mw: 75_000,
      },
      {
        preset_id: 5,
        mode: "cc",
        target_i_ma: 2_900,
        target_v_mv: 12_000,
        target_p_mw: 30_000,
        min_v_mv: 11_000,
        max_i_ma_total: 5_200,
        max_p_mw: 90_000,
      },
    ];
  }

  return [
    {
      preset_id: 1,
      mode: "cc",
      target_i_ma: 1_850,
      target_v_mv: 11_500,
      target_p_mw: 16_000,
      min_v_mv: 10_800,
      max_i_ma_total: 4_500,
      max_p_mw: 42_000,
    },
    {
      preset_id: 2,
      mode: "cv",
      target_i_ma: 900,
      target_v_mv: 9_000,
      target_p_mw: 8_000,
      min_v_mv: 7_500,
      max_i_ma_total: 1_800,
      max_p_mw: 18_000,
    },
    {
      preset_id: 3,
      mode: "cp",
      target_i_ma: 2_300,
      target_v_mv: 12_000,
      target_p_mw: 28_000,
      min_v_mv: 9_500,
      max_i_ma_total: 3_200,
      max_p_mw: 36_000,
    },
    {
      preset_id: 4,
      mode: "cc",
      target_i_ma: 3_100,
      target_v_mv: 10_500,
      target_p_mw: 24_000,
      min_v_mv: 9_800,
      max_i_ma_total: 3_800,
      max_p_mw: 32_000,
    },
    {
      preset_id: 5,
      mode: "cp",
      target_i_ma: 1_500,
      target_v_mv: 12_000,
      target_p_mw: 12_000,
      min_v_mv: 10_500,
      max_i_ma_total: 2_500,
      max_p_mw: 20_000,
    },
  ];
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
    firmware: {
      target: "digital_esp32s3",
      package_version: "0.1.0",
      build_id:
        "digital 0.1.0 (profile mock, v0.1.0-mock, src 0x0000000000000000)",
      build_profile: "mock",
      target_triple: "xtensa-esp32s3-none-elf",
      source_digest: "src 0x0000000000000000",
      features: ["net_http", "mdns_dns_sd", "usb_cdc_jsonl"],
      protocol: "loadlynx.cdc.v1",
      defmt: {
        enabled: true,
        encoding: "defmt-espflash",
      },
    },
    usb_bridge: {
      transport: "usb_cdc_jsonl",
      protocol: "loadlynx.cdc.v1",
      lease_required: true,
      framing: "lf_json",
    },
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
  payload: DevdIdentityPayload,
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
    firmware: payload.firmware,
    usb_bridge: payload.usb_bridge ?? {
      transport: "usb_cdc_jsonl",
      protocol: "loadlynx.cdc.v1",
      lease_required: true,
      framing: "lf_json",
    },
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

export function normalizeDevdStatus(
  payload: DevdStatusPayload,
  previous?: FastStatusView,
): FastStatusView {
  const previousRaw = previous?.raw;
  const fallbackMode =
    payload.control?.mode === "cc"
      ? 1
      : payload.control?.mode === "cv"
        ? 2
        : payload.control?.mode === "cp"
          ? 3
          : (previousRaw?.mode ?? 1);
  const raw: FastStatusJson = {
    uptime_ms:
      payload.uptime_ms ?? payload.status.uptime_ms ?? previousRaw?.uptime_ms ?? 0,
    mode: payload.status.mode ?? previousRaw?.mode ?? fallbackMode,
    state_flags: payload.status.state_flags ?? previousRaw?.state_flags ?? 0,
    enable:
      payload.status.enable ??
      payload.control?.output_enabled ??
      previousRaw?.enable ??
      false,
    target_value:
      payload.status.target_value ??
      previousRaw?.target_value ??
      (fallbackMode === 3 ? (payload.control?.target_p_mw ?? 0) : 0),
    i_local_ma: payload.status.i_local_ma ?? previousRaw?.i_local_ma ?? 0,
    i_remote_ma: payload.status.i_remote_ma ?? previousRaw?.i_remote_ma ?? 0,
    v_local_mv: payload.status.v_local_mv ?? previousRaw?.v_local_mv ?? 0,
    v_remote_mv: payload.status.v_remote_mv ?? previousRaw?.v_remote_mv ?? 0,
    calc_p_mw: payload.status.calc_p_mw ?? previousRaw?.calc_p_mw ?? 0,
    dac_headroom_mv:
      payload.status.dac_headroom_mv ?? previousRaw?.dac_headroom_mv ?? 0,
    loop_error: payload.status.loop_error ?? previousRaw?.loop_error ?? 0,
    sink_core_temp_mc:
      payload.sink_core_temp_mc ??
      payload.status.sink_core_temp_mc ??
      previousRaw?.sink_core_temp_mc,
    sink_exhaust_temp_mc:
      payload.sink_exhaust_temp_mc ??
      payload.status.sink_exhaust_temp_mc ??
      previousRaw?.sink_exhaust_temp_mc,
    mcu_temp_mc:
      payload.mcu_temp_mc ??
      payload.status.mcu_temp_mc ??
      previousRaw?.mcu_temp_mc,
    fault_flags: payload.status.fault_flags ?? previousRaw?.fault_flags ?? 0,
    cal_kind: payload.status.cal_kind ?? previousRaw?.cal_kind,
    raw_v_nr_100uv: payload.status.raw_v_nr_100uv ?? previousRaw?.raw_v_nr_100uv,
    raw_v_rmt_100uv:
      payload.status.raw_v_rmt_100uv ?? previousRaw?.raw_v_rmt_100uv,
    raw_cur_100uv: payload.status.raw_cur_100uv ?? previousRaw?.raw_cur_100uv,
    raw_dac_code: payload.status.raw_dac_code ?? previousRaw?.raw_dac_code,
  } as FastStatusJson;
  return {
    raw,
    link_up: payload.link_up ?? previous?.link_up ?? true,
    hello_seen: payload.hello_seen ?? previous?.hello_seen ?? true,
    analog_state: payload.analog_state ?? previous?.analog_state ?? "ready",
    fault_flags_decoded:
      payload.fault_flags_decoded ?? previous?.fault_flags_decoded ?? [],
    state_flags_decoded:
      payload.state_flags_decoded ?? previous?.state_flags_decoded ?? [],
  };
}

export function getOrCreateMockDevice(baseUrl: string): MockDeviceState {
  const existing = mockDevices.get(baseUrl);
  if (existing) {
    return existing;
  }

  const index = mockDevices.size + 1;
  const identity = createInitialIdentity(baseUrl, index);
  const simulationProfile = createMockSimulationProfile(baseUrl, index);
  const bootState = createMockScenarioBootState(simulationProfile);
  const status = createInitialStatus(baseUrl, simulationProfile);
  const cc = createInitialCc(simulationProfile);
  const pd = createInitialPd(baseUrl);
  const presets = createInitialPresets(simulationProfile);
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

  const normalizedBaseUrl = baseUrl.toLowerCase();
  if (
    normalizedBaseUrl.includes("wifi-connect-failed") ||
    normalizedBaseUrl.includes("wifi-clear-noop") ||
    normalizedBaseUrl.includes("wifi-clear-error") ||
    normalizedBaseUrl.includes("wifi-clear-timeout-success")
  ) {
    wifi.ssid = "BenchNet";
    wifi.source = "user";
    wifi.state = "connected";
    wifi.ip = "192.168.31.216";
    wifi.last_error = null;
  }

  if (normalizedBaseUrl.includes("wifi-connect-failed")) {
    wifi.state = "error";
    wifi.ip = null;
    wifi.last_error = "connect_failed";
  }

  const state: MockDeviceState = {
    identity,
    status,
    cc,
    pd,
    presets,
    active_preset_id: bootState.activePresetId,
    output_enabled: bootState.outputEnabled,
    uv_latched: false,
    calibrationMode: "off",
    calibration,
    wifi,
    wifiPsk: "factory-mock-psk",
    simulation: {
      profile: simulationProfile,
      lastWallClockMs: Date.now(),
    },
    wifiConnectPollsRemaining: 0,
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
    state.output_enabled = true;
  }

  mockDevices.set(baseUrl, state);
  return state;
}

import { HttpApiError } from "./client-core.ts";
import {
  clampI16,
  getOrCreateMockDevice,
  type MockDeviceState,
  type MockSimulationProfile,
} from "./client-mock-state.ts";
import type {
  CcControlView,
  CcUpdateRequest,
  ControlUpdateRequest,
  ControlView,
  FastStatusView,
  Identity,
  LoadMode,
  Preset,
  PresetId,
  PresetsResponse,
  SoftResetReason,
  SoftResetResponse,
} from "./types.ts";

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

function noiseWave(
  uptimeSeconds: number,
  phase: number,
  periodSeconds: number,
) {
  return Math.sin((uptimeSeconds / periodSeconds) * Math.PI * 2 + phase);
}

function getModeCode(mode: LoadMode) {
  if (mode === "cv") return 2;
  if (mode === "cp") return 3;
  return 1;
}

function computeMockElectricalState(params: {
  profile: MockSimulationProfile;
  preset: Preset;
  outputEnabled: boolean;
  uptimeSeconds: number;
}) {
  const { profile, preset, outputEnabled, uptimeSeconds } = params;
  const ripple = noiseWave(uptimeSeconds, profile.phaseOffset, 5.5);
  const loadDrift = noiseWave(uptimeSeconds, profile.phaseOffset * 1.7, 13.5);
  const thermalSwing = noiseWave(uptimeSeconds, profile.phaseOffset * 0.8, 21);

  const minVoltageMv = Math.max(0, preset.min_v_mv);
  const maxCurrentMa = Math.max(0, preset.max_i_ma_total);
  const maxPowerMw = Math.max(0, preset.max_p_mw);

  if (!outputEnabled) {
    const idleRemoteMv = Math.round(
      profile.openCircuitMv + ripple * profile.rippleMv * 0.4,
    );
    const idleLocalMv = idleRemoteMv + profile.localSenseBiasMv;
    return {
      modeCode: getModeCode(preset.mode),
      targetValue: 0,
      vRemoteMv: idleRemoteMv,
      vLocalMv: idleLocalMv,
      iTotalMa: 0,
      iLocalMa: 0,
      iRemoteMa: 0,
      pMainMw: 0,
      loopError: Math.round(8 * thermalSwing),
      dacHeadroomMv: Math.max(180, Math.round(idleRemoteMv * 0.12)),
      sinkCoreTempMc: profile.ambientTempMc + 2_500,
      sinkExhaustTempMc: profile.ambientTempMc + 1_300,
      mcuTempMc: profile.ambientTempMc + 1_700,
    };
  }

  let desiredCurrentMa = 0;
  let desiredRemoteMv = profile.openCircuitMv;

  if (preset.mode === "cc") {
    desiredCurrentMa = Math.max(0, preset.target_i_ma);
    desiredRemoteMv =
      profile.openCircuitMv -
      desiredCurrentMa * (profile.sourceResistanceMilliohm / 1000);
  } else if (preset.mode === "cv") {
    desiredRemoteMv = clamp(
      preset.target_v_mv,
      Math.max(1_000, minVoltageMv || 1_000),
      profile.openCircuitMv,
    );
    const availableCurrentMa = Math.max(
      0,
      (profile.openCircuitMv - desiredRemoteMv) /
        (profile.sourceResistanceMilliohm / 1000),
    );
    desiredCurrentMa = Math.min(maxCurrentMa, availableCurrentMa);
  } else {
    desiredRemoteMv = clamp(
      profile.openCircuitMv - 250 + ripple * profile.rippleMv * 0.6,
      Math.max(1_000, minVoltageMv || 1_000),
      profile.openCircuitMv,
    );
    const desiredPowerMw = Math.max(0, preset.target_p_mw);
    desiredCurrentMa =
      desiredRemoteMv > 0 ? (desiredPowerMw * 1000) / desiredRemoteMv : 0;
  }

  desiredCurrentMa = Math.max(0, desiredCurrentMa * (1 + loadDrift * 0.035));

  const powerLimitedCurrentMa =
    desiredRemoteMv > 0
      ? (maxPowerMw * 1000) / desiredRemoteMv
      : desiredCurrentMa;
  const totalCurrentMa = clamp(
    Math.round(Math.min(desiredCurrentMa, maxCurrentMa, powerLimitedCurrentMa)),
    0,
    Math.max(0, maxCurrentMa),
  );

  let vRemoteMv = Math.round(
    clamp(
      profile.openCircuitMv -
        totalCurrentMa * (profile.sourceResistanceMilliohm / 1000) +
        ripple * profile.rippleMv,
      Math.max(0, minVoltageMv),
      profile.openCircuitMv + profile.rippleMv,
    ),
  );

  if (preset.mode === "cv") {
    vRemoteMv = Math.round(
      clamp(
        preset.target_v_mv + ripple * profile.rippleMv * 0.45,
        Math.max(0, minVoltageMv),
        profile.openCircuitMv,
      ),
    );
  }

  const vLocalMv = vRemoteMv + profile.localSenseBiasMv;
  const pMainMw = Math.round((totalCurrentMa * vRemoteMv) / 1000);
  const iLocalMa = Math.round(totalCurrentMa * profile.localCurrentShare);
  const iRemoteMa = totalCurrentMa - iLocalMa;
  const thermalPowerW = pMainMw / 1000;
  const sinkCoreTempMc = Math.round(
    profile.ambientTempMc +
      thermalPowerW * profile.coreTempRisePerW +
      thermalSwing * 350,
  );
  const sinkExhaustTempMc = Math.round(
    profile.ambientTempMc +
      thermalPowerW * profile.exhaustTempRisePerW +
      thermalSwing * 240,
  );
  const mcuTempMc = Math.round(
    profile.ambientTempMc +
      thermalPowerW * profile.mcuTempRisePerW +
      thermalSwing * 160,
  );
  const loopError = Math.round(
    (desiredCurrentMa - totalCurrentMa) * 0.08 + ripple * 12,
  );
  const dacHeadroomMv = Math.round(
    clamp(vRemoteMv * 0.08 + (maxCurrentMa - totalCurrentMa) * 0.03, 80, 1800),
  );
  const targetValue =
    preset.mode === "cv"
      ? vRemoteMv
      : preset.mode === "cp"
        ? pMainMw
        : totalCurrentMa;

  return {
    modeCode: getModeCode(preset.mode),
    targetValue,
    vRemoteMv,
    vLocalMv,
    iTotalMa: totalCurrentMa,
    iLocalMa,
    iRemoteMa,
    pMainMw,
    loopError,
    dacHeadroomMv,
    sinkCoreTempMc,
    sinkExhaustTempMc,
    mcuTempMc,
  };
}

function syncCcFromPreset(state: MockDeviceState, preset: Preset) {
  state.cc = {
    ...state.cc,
    enable: state.output_enabled,
    target_i_ma: preset.target_i_ma,
    effective_i_ma: state.output_enabled ? preset.target_i_ma : 0,
    limit_profile: {
      ...state.cc.limit_profile,
      max_i_ma: preset.max_i_ma_total,
      max_p_mw: preset.max_p_mw,
    },
  };
}

export async function mockGetIdentity(baseUrl: string): Promise<Identity> {
  return structuredClone(getOrCreateMockDevice(baseUrl).identity);
}

export async function mockGetStatus(baseUrl: string): Promise<FastStatusView> {
  const state = getOrCreateMockDevice(baseUrl);
  const preset = mockGetActivePreset(state);
  syncCcFromPreset(state, preset);
  const next = { ...state.status, raw: { ...state.status.raw } };
  const now = Date.now();
  const elapsedMs = Math.max(300, now - state.simulation.lastWallClockMs);
  state.simulation.lastWallClockMs = now;
  next.raw.uptime_ms += elapsedMs;
  state.identity.uptime_ms = next.raw.uptime_ms;

  const electrical = computeMockElectricalState({
    profile: state.simulation.profile,
    preset,
    outputEnabled: state.output_enabled,
    uptimeSeconds: next.raw.uptime_ms / 1000,
  });

  next.raw.mode = electrical.modeCode;
  next.raw.enable = state.output_enabled;
  next.raw.target_value = electrical.targetValue;
  next.raw.i_local_ma = electrical.iLocalMa;
  next.raw.i_remote_ma = electrical.iRemoteMa;
  next.raw.v_local_mv = electrical.vLocalMv;
  next.raw.v_remote_mv = electrical.vRemoteMv;
  next.raw.calc_p_mw = electrical.pMainMw;
  next.raw.dac_headroom_mv = electrical.dacHeadroomMv;
  next.raw.loop_error = electrical.loopError;
  next.raw.sink_core_temp_mc = electrical.sinkCoreTempMc;
  next.raw.sink_exhaust_temp_mc = electrical.sinkExhaustTempMc;
  next.raw.mcu_temp_mc = electrical.mcuTempMc;

  state.cc = {
    ...state.cc,
    enable: state.output_enabled,
    effective_i_ma:
      preset.mode === "cc" && state.output_enabled ? electrical.iTotalMa : 0,
    i_total_ma: electrical.iTotalMa,
    v_main_mv: electrical.vRemoteMv,
    p_main_mw: electrical.pMainMw,
  };

  next.state_flags_decoded = [];
  if (state.output_enabled) {
    next.state_flags_decoded.push("ENABLED");
    next.state_flags_decoded.push("REMOTE_ACTIVE");
  }
  if (next.link_up) {
    next.state_flags_decoded.push("LINK_GOOD");
  }
  if (state.uv_latched) {
    next.state_flags_decoded.push("UV_LATCHED");
  }
  if (
    state.output_enabled &&
    preset.max_i_ma_total > 0 &&
    electrical.iTotalMa >= preset.max_i_ma_total - 20
  ) {
    next.state_flags_decoded.push("CURRENT_LIMITED");
  }
  if (
    state.output_enabled &&
    preset.max_p_mw > 0 &&
    electrical.pMainMw >= preset.max_p_mw - 250
  ) {
    next.state_flags_decoded.push("POWER_LIMITED");
  }

  switch (state.calibrationMode) {
    case "voltage":
      next.raw.cal_kind = 1;
      next.raw.raw_v_nr_100uv = clampI16(Math.round(next.raw.v_local_mv * 2.5));
      next.raw.raw_v_rmt_100uv = clampI16(
        Math.round(next.raw.v_remote_mv * 2.5),
      );
      break;
    case "current_ch1":
      next.raw.cal_kind = 2;
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
      next.raw.cal_kind = 3;
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

export async function mockGetCc(baseUrl: string): Promise<CcControlView> {
  return structuredClone(getOrCreateMockDevice(baseUrl).cc);
}

export async function mockUpdateCc(
  baseUrl: string,
  payload: CcUpdateRequest,
): Promise<CcControlView> {
  const state = getOrCreateMockDevice(baseUrl);
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
  state.status = {
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
    (entry) => entry.preset_id === state.active_preset_id,
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

export function mockRequireControlReady(state: MockDeviceState): void {
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

function mockUpdateStatusFromControl(state: MockDeviceState) {
  const preset = mockGetActivePreset(state);
  syncCcFromPreset(state, preset);
}

export async function mockGetPresets(
  baseUrl: string,
): Promise<PresetsResponse> {
  const state = getOrCreateMockDevice(baseUrl);
  mockRequireControlReady(state);
  return { presets: structuredClone(state.presets) };
}

export async function mockUpdatePreset(
  baseUrl: string,
  payload: Preset,
): Promise<Preset> {
  const state = getOrCreateMockDevice(baseUrl);
  mockRequireControlReady(state);
  const presetId = assertPresetId(payload.preset_id);
  const idx = state.presets.findIndex((entry) => entry.preset_id === presetId);
  if (idx < 0) {
    mockInvalidRequest("preset not found");
  }

  if (payload.mode === "cp") {
    const targetPMw = Number.isFinite(payload.target_p_mw)
      ? payload.target_p_mw
      : 0;
    const maxPMw = Number.isFinite(payload.max_p_mw) ? payload.max_p_mw : 0;
    if (targetPMw < 0) {
      throw new HttpApiError({
        status: 422,
        code: "LIMIT_VIOLATION",
        message: "target_p_mw must be >= 0",
        retryable: false,
        details: { target_p_mw: targetPMw },
      });
    }
    if (maxPMw < 0) {
      throw new HttpApiError({
        status: 422,
        code: "LIMIT_VIOLATION",
        message: "max_p_mw must be >= 0",
        retryable: false,
        details: { max_p_mw: maxPMw },
      });
    }
    if (targetPMw > maxPMw) {
      throw new HttpApiError({
        status: 422,
        code: "LIMIT_VIOLATION",
        message: "target_p_mw exceeds max_p_mw",
        retryable: false,
        details: { target_p_mw: targetPMw, max_p_mw: maxPMw },
      });
    }
  }

  const nextPreset: Preset = {
    preset_id: presetId,
    mode: payload.mode,
    target_i_ma: Number.isFinite(payload.target_i_ma) ? payload.target_i_ma : 0,
    target_v_mv: Number.isFinite(payload.target_v_mv) ? payload.target_v_mv : 0,
    target_p_mw: Number.isFinite(payload.target_p_mw) ? payload.target_p_mw : 0,
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

export async function mockApplyPreset(
  baseUrl: string,
  preset_id: number,
): Promise<ControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  mockRequireControlReady(state);
  state.active_preset_id = assertPresetId(preset_id);
  state.output_enabled = false;
  mockUpdateStatusFromControl(state);
  return mockMakeControlView(state);
}

export async function mockGetControl(baseUrl: string): Promise<ControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  mockRequireControlReady(state);
  mockUpdateStatusFromControl(state);
  return mockMakeControlView(state);
}

export async function mockUpdateControl(
  baseUrl: string,
  payload: ControlUpdateRequest,
): Promise<ControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  if (
    baseUrl.toLowerCase().includes("restore-safety-blocked") &&
    payload.output_enabled === false
  ) {
    throw new HttpApiError({
      status: 409,
      code: "SAFETY_BLOCKED",
      message: "mock output disable refused",
      retryable: true,
      details: null,
    });
  }

  const nextOutputEnabled = Boolean(payload.output_enabled);
  const prevOutputEnabled = state.output_enabled;
  state.output_enabled = nextOutputEnabled;

  if (!prevOutputEnabled && nextOutputEnabled && state.uv_latched) {
    state.uv_latched = false;
  }

  if (nextOutputEnabled) {
    const preset = mockGetActivePreset(state);
    const preview = computeMockElectricalState({
      profile: state.simulation.profile,
      preset,
      outputEnabled: true,
      uptimeSeconds: (state.status.raw.uptime_ms ?? 0) / 1000,
    });
    const vMv = preview.vRemoteMv;
    if (preset.min_v_mv > 0 && vMv < preset.min_v_mv) {
      state.uv_latched = true;
    }
  }

  mockUpdateStatusFromControl(state);
  return mockMakeControlView(state);
}

export async function mockDebugSetUvLatched(
  baseUrl: string,
  uv_latched: boolean,
): Promise<ControlView> {
  const state = getOrCreateMockDevice(baseUrl);
  state.uv_latched = Boolean(uv_latched);
  return mockMakeControlView(state);
}

export async function mockSoftReset(
  baseUrl: string,
  reason: SoftResetReason,
): Promise<SoftResetResponse> {
  const state = getOrCreateMockDevice(baseUrl);
  state.calibrationMode = "off";
  if (state.calibration.eeprom) {
    state.calibration.ram = structuredClone(state.calibration.eeprom);
    state.calibration.ram.active.source = "user-calibrated";
  } else {
    state.calibration.ram = structuredClone(state.calibration.factory);
  }
  return { accepted: true, reason };
}

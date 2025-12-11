// Core HTTP API types for the LoadLynx network control surface (trimmed for web mock).
// Field names follow docs/interfaces/network-http-api.md so we can swap in real HTTP later.

export type DeviceId = string;

export interface NetworkInfo {
  ip: string;
  mac: string;
  hostname: string;
}

export interface DeviceCapabilities {
  cc_supported: boolean;
  cv_supported: boolean;
  cp_supported: boolean;
  api_version: string;
}

export interface Identity {
  device_id: DeviceId;
  digital_fw_version: string;
  analog_fw_version: string;
  protocol_version: number;
  uptime_ms: number;
  network: NetworkInfo;
  capabilities: DeviceCapabilities;

  // Added fields in digital firmware /api/v1/identity
  hostname?: string;
  short_id?: string;
}

export type AnalogState = "offline" | "cal_missing" | "faulted" | "ready";

export type FaultFlag =
  | "OVERCURRENT"
  | "OVERVOLTAGE"
  | "MCU_OVER_TEMP"
  | "SINK_OVER_TEMP";

export interface FastStatusJson {
  uptime_ms: number;
  mode: number;
  state_flags: number;
  enable: boolean;
  target_value: number;
  i_local_ma: number;
  i_remote_ma: number;
  v_local_mv: number;
  v_remote_mv: number;
  calc_p_mw: number;
  dac_headroom_mv: number;
  loop_error: number;
  sink_core_temp_mc: number;
  sink_exhaust_temp_mc: number;
  mcu_temp_mc: number;
  fault_flags: number;
}

export interface FastStatusView {
  raw: FastStatusJson;
  link_up: boolean;
  hello_seen: boolean;
  analog_state: AnalogState;
  fault_flags_decoded: FaultFlag[];
}

export type CcProtectionMode = "off" | "protect" | "maintain";

export interface CcLimitProfile {
  max_i_ma: number;
  max_p_mw: number;
  ovp_mv: number;
  temp_trip_mc: number;
  thermal_derate_pct: number;
}

export interface CcProtectionConfig {
  voltage_mode: CcProtectionMode;
  power_mode: CcProtectionMode;
}

export interface CcControlView {
  enable: boolean;
  target_i_ma: number;
  limit_profile: CcLimitProfile;
  protection: CcProtectionConfig;
  // Derived / live values; simplified from the full docs.
  i_total_ma: number;
  v_main_mv: number;
  p_main_mw: number;
}

// Shape of PUT /api/v1/cc payload.
export interface CcUpdateRequest {
  enable: boolean;
  target_i_ma: number;
  max_i_ma?: number;
  max_p_mw?: number;
  ovp_mv?: number;
  temp_trip_mc?: number;
  thermal_derate_pct?: number;
  voltage_mode?: CcProtectionMode;
  power_mode?: CcProtectionMode;
}

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
  presets_supported?: boolean;
  preset_count?: number; // 固定为 5（见 docs/interfaces/network-http-api.md）
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
  // -----------------------------------------------------------------------
  // hostname: The mDNS FQDN, e.g. "loadlynx-d68638.local".
  // short_id: The unique 6-char hex ID derived from MAC, e.g. "d68638".
  // Note: network.hostname is legay/NetBIOS and may differ or be less specific.
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
  // Optional raw fields for calibration (only present when mode != off)
  cal_kind?: number;
  raw_v_nr_100uv?: number;
  raw_v_rmt_100uv?: number;
  raw_cur_100uv?: number;
  raw_dac_code?: number;
}

export type CalibrationCurveKind =
  | "v_local"
  | "v_remote"
  | "current_ch1"
  | "current_ch2";

export interface CalibrationActiveProfile {
  source: "factory-default" | "user-calibrated";
  fmt_version: number;
  hw_rev: number;
}

export interface CalibrationPointVoltage {
  raw: number;
  mv: number;
}

export interface CalibrationPointCurrent {
  raw: number;
  ua: number;
  dac_code: number;
}

export interface CalibrationProfile {
  active: CalibrationActiveProfile;
  v_local_points: CalibrationPointVoltage[];
  v_remote_points: CalibrationPointVoltage[];
  current_ch1_points: CalibrationPointCurrent[];
  current_ch2_points: CalibrationPointCurrent[];
}

export type CalibrationWriteRequest =
  | { kind: "v_local"; points: CalibrationPointVoltage[] }
  | { kind: "v_remote"; points: CalibrationPointVoltage[] }
  | { kind: "current_ch1"; points: CalibrationPointCurrent[] }
  | { kind: "current_ch2"; points: CalibrationPointCurrent[] };

export type CalibrationApplyRequest = CalibrationWriteRequest;

export type CalibrationCommitRequest = CalibrationWriteRequest;

export interface CalibrationResetRequest {
  kind: "all" | CalibrationCurveKind;
}

export interface CalibrationModeRequest {
  kind: "off" | "voltage" | "current_ch1" | "current_ch2";
}

// Calibration wire protocol types (ESP32-S3 firmware net_http)

export interface CalibrationPointVoltageWire {
  raw_100uv: number;
  meas_mv: number;
}

export interface CalibrationPointCurrentWire {
  raw_100uv: number;
  raw_dac_code: number;
  meas_ma: number;
}

// Compact request encoding (reduces payload size for embedded HTTP parsers).
export type CalibrationPointVoltageWireCompact = [
  raw_100uv: number,
  meas_mv: number,
];

export type CalibrationPointCurrentWireCompact = [
  raw_100uv: number,
  raw_dac_code: number,
  meas_ma: number,
];

export interface CalibrationProfileWire {
  active: CalibrationActiveProfile;
  current_ch1_points: CalibrationPointCurrentWire[];
  current_ch2_points: CalibrationPointCurrentWire[];
  v_local_points: CalibrationPointVoltageWire[];
  v_remote_points: CalibrationPointVoltageWire[];
}

export type CalibrationWriteRequestWire =
  | {
      kind: "v_local";
      points: CalibrationPointVoltageWireCompact[];
    }
  | {
      kind: "v_remote";
      points: CalibrationPointVoltageWireCompact[];
    }
  | {
      kind: "current_ch1";
      points: CalibrationPointCurrentWireCompact[];
    }
  | {
      kind: "current_ch2";
      points: CalibrationPointCurrentWireCompact[];
    };

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
  // NOTE: For capabilities.api_version >= 2.0.0, enable/target_i_ma model
  // the "load switch + setpoint" semantics (see docs/interfaces/network-http-api.md).
  enable: boolean; // load switch: when false, output is effectively 0 mA
  target_i_ma: number; // setpoint (mA), editable even when enable=false
  effective_i_ma: number; // effective output target (mA): enable ? target_i_ma : 0
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

// Preset/Control (docs/interfaces/network-http-api.md §2.5)

export type LoadMode = "cc" | "cv";

export type PresetId = 1 | 2 | 3 | 4 | 5;

export interface Preset {
  preset_id: PresetId; // 1..=5
  mode: LoadMode;
  target_i_ma: number; // mA (used when mode="cc")
  target_v_mv: number; // mV (used when mode="cv")
  min_v_mv: number; // mV (undervoltage latch threshold)
  max_i_ma_total: number; // mA
  max_p_mw: number; // mW
}

export interface ControlView {
  active_preset_id: PresetId; // 1..=5
  output_enabled: boolean;
  uv_latched: boolean;
  preset: Preset; // snapshot of active preset
}

// USB-PD (docs/interfaces/network-http-api.md §3.5..§3.6)

export interface PdFixedPdo {
  pos: number; // object position (1-based)
  mv: number;
  max_ma: number;
}

export interface PdPpsPdo {
  pos: number; // object position (1-based)
  min_mv: number;
  max_mv: number;
  max_ma: number;
}

export type PdSavedMode = "fixed" | "pps";

export interface PdSavedConfig {
  mode: PdSavedMode;
  fixed_object_pos: number;
  pps_object_pos: number;
  target_mv: number; // PPS target; ignored for fixed negotiation
  i_req_ma: number;
}

export interface PdApplyLast {
  code: string; // "ok" | "ack" | "nack" | ...
  at_ms: number;
}

export interface PdApplyState {
  pending: boolean;
  last: PdApplyLast | null;
}

export interface PdView {
  attached: boolean;
  contract_mv: number | null;
  contract_ma: number | null;
  fixed_pdos: PdFixedPdo[];
  pps_pdos: PdPpsPdo[];
  saved: PdSavedConfig;
  apply: PdApplyState;
}

export type PdUpdateRequest =
  | { mode: "fixed"; object_pos: number; i_req_ma: number }
  | { mode: "pps"; object_pos: number; target_mv: number; i_req_ma: number };

// Core HTTP API types for the LoadLynx network control surface (trimmed for web mock).
// Field names follow docs/interfaces/network-http-api.md so we can swap in real HTTP later.

export type DeviceId = string;

export interface NetworkInfo {
  ip: string;
  mac: string;
  hostname: string;
}

export interface WifiStatus {
  ssid: string | null;
  source: "factory" | "user" | "none";
  state: "idle" | "configured" | "connecting" | "connected" | "error";
  ip: string | null;
  last_error: string | null;
}

export interface WifiCredentials {
  ssid: string;
  psk: string;
  source: "factory" | "user";
}

export interface WifiSetRequest {
  ssid: string;
  psk: string;
  wait?: boolean;
}

export type WifiStatusResponse = WifiStatus | { wifi: WifiStatus };

export interface DiagnosticsLastStatus {
  uptime_ms: number;
  fault_flags: number;
}

export interface DiagnosticsExport {
  schema_version: 1;
  redaction: {
    psk: true;
  };
  firmware_version: string;
  wifi: WifiStatus & {
    psk: "<redacted>";
  };
  link_up: boolean;
  calibration_persistence: CalibrationPersistence;
  last_status: DiagnosticsLastStatus | null;
}

export interface DeviceCapabilities {
  cc_supported: boolean;
  cv_supported: boolean;
  cp_supported: boolean;
  presets_supported?: boolean;
  preset_count?: number; // 固定为 5（见 docs/interfaces/network-http-api.md）
  api_version: string;
}

export interface FirmwareIdentity {
  target: "digital_esp32s3";
  package_version: string;
  build_id: string;
  build_profile: string;
  target_triple: string;
  source_digest: string;
  features: string[];
  protocol: "loadlynx.cdc.v1";
  defmt: {
    enabled: boolean;
    encoding: string;
  };
}

export interface UsbBridgeIdentity {
  transport: "usb_cdc_jsonl";
  protocol: "loadlynx.cdc.v1";
  lease_required: true;
  framing: "lf_json";
}

export interface Identity {
  device_id: DeviceId;
  digital_fw_version: string;
  analog_fw_version: string;
  protocol_version: number;
  uptime_ms: number;
  network: NetworkInfo;
  capabilities: DeviceCapabilities;
  firmware?: FirmwareIdentity;
  usb_bridge?: UsbBridgeIdentity;

  // Added fields in digital firmware /api/v1/identity
  // -----------------------------------------------------------------------
  // hostname: The mDNS FQDN, e.g. "loadlynx-d68638.local".
  // short_id: The unique 6-char hex ID derived from MAC, e.g. "d68638".
  // Note: network.hostname is legay/NetBIOS and may differ or be less specific.
  hostname?: string;
  short_id?: string;
}

export type AnalogState =
  | "offline"
  | "cal_missing"
  | "faulted"
  | "ready"
  | "measurement_invalid";

export type FaultFlag =
  | "OVERCURRENT"
  | "OVERVOLTAGE"
  | "MCU_OVER_TEMP"
  | "SINK_OVER_TEMP";

export type StateFlag =
  | "REMOTE_ACTIVE"
  | "LINK_GOOD"
  | "ENABLED"
  | "UV_LATCHED"
  | "POWER_LIMITED"
  | "CURRENT_LIMITED";

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

export interface CalibrationPersistence {
  status: string;
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
  persistence?: CalibrationPersistence;
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
  persistence?: CalibrationPersistence;
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
  state_flags_decoded: StateFlag[];
}

export interface FastStatusResponse {
  status: FastStatusJson;
  link_up: boolean;
  hello_seen: boolean;
  analog_state: AnalogState;
  fault_flags_decoded: FaultFlag[];
  state_flags_decoded?: StateFlag[];
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

export type LoadMode = "cc" | "cv" | "cp" | "cr";

export type PresetId = 1 | 2 | 3 | 4 | 5;

export interface Preset {
  preset_id: PresetId; // 1..=5
  mode: LoadMode;
  target_i_ma: number; // mA (used when mode="cc")
  target_v_mv: number; // mV (used when mode="cv")
  target_p_mw: number; // mW (used when mode="cp")
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

export interface ApplyPresetRequest {
  preset_id: number;
}

export interface ControlUpdateRequest {
  output_enabled: boolean;
}

export interface PresetsResponse {
  presets: Preset[];
}

export type SoftResetReason =
  | "manual"
  | "firmware_update"
  | "ui_recover"
  | "link_recover";

export interface SoftResetRequest {
  reason: SoftResetReason;
}

export interface SoftResetResponse {
  accepted: boolean;
  reason: SoftResetReason;
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

export interface PdEprAvsPdo {
  pos: number; // object position (1-based)
  min_mv: number;
  max_mv: number;
  pdp_w: number;
}

export type PdSavedMode = "fixed" | "pps";

export interface PdSavedConfig {
  mode: PdSavedMode;
  fixed_object_pos: number;
  pps_object_pos: number;
  // Active target voltage (mV).
  // - mode="fixed": mirrors the selected PDO voltage
  // - mode="pps": mirrors the current PPS Vreq
  target_mv: number;
  // Sticky PPS Vreq cache so the UI can restore the last PPS voltage even if
  // the saved mode is currently "fixed".
  pps_target_mv?: number;
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
  epr_active?: boolean;
  epr_avs_pdos?: PdEprAvsPdo[];
  // Safe5V gate; older firmware may omit this field.
  allow_extended_voltage?: boolean;
  saved: PdSavedConfig;
  apply: PdApplyState;
}

export interface PdFixedUpdateRequest {
  mode: "fixed";
  object_pos: number;
  target_mv?: number;
  i_req_ma: number;
  allow_extended_voltage?: boolean;
}

export interface PdPpsUpdateRequest {
  mode: "pps";
  object_pos: number;
  target_mv: number;
  i_req_ma: number;
  allow_extended_voltage?: boolean;
}

export interface PdVoltageGateUpdateRequest {
  allow_extended_voltage: boolean;
}

export type PdUpdateRequest =
  | PdFixedUpdateRequest
  | PdPpsUpdateRequest
  | PdVoltageGateUpdateRequest;

export type BackupSectionKey =
  | "presets"
  | "calibration"
  | "settings.wifi"
  | "settings.pd";

export interface BackupPresetsSection {
  presets: Preset[];
  active_preset_id?: PresetId;
}

export interface BackupPdSection {
  saved: PdSavedConfig;
  allow_extended_voltage: boolean;
}

export interface LoadLynxBackup {
  kind: "loadlynx.backup";
  schema_version: 1;
  created_at: string;
  selected_sections?: BackupSectionKey[];
  sections: {
    presets?: BackupPresetsSection;
    calibration?: CalibrationProfileWire;
    settings?: {
      wifi?: WifiCredentials;
      pd?: BackupPdSection;
      [key: string]: unknown;
    };
    [key: string]: unknown;
  };
  warnings?: unknown[];
}

export interface BackupRestoreSectionResult {
  section: BackupSectionKey;
  ok: boolean;
  message?: string;
}

export interface BackupRestoreResult {
  ok: boolean;
  safety: { output_disabled: boolean };
  restored: BackupRestoreSectionResult[];
  warnings: string[];
}

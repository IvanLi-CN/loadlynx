import type { Identity } from "../api/types.ts";

export type DevdTargetKind =
  | "digital_esp32s3"
  | "analog_stm32g431"
  | "lan_http"
  | "mock";

export interface DevdTargetCandidate {
  kind: DevdTargetKind;
  display_name: string;
  port_path?: string | null;
  probe_selector?: string | null;
  lan_base_url?: string | null;
  selector_source?: string | null;
}

export interface DevdLogDecode {
  status: "verified" | "unverified" | string;
  reason?: string | null;
  artifact_id?: string | null;
}

export interface DevdDevice {
  id: string;
  display_name: string;
  connection: "disconnected" | "connected" | "busy" | "error";
  digital_target?: DevdTargetCandidate | null;
  analog_target?: DevdTargetCandidate | null;
  lan_endpoint?: string | null;
  identity?: Identity | null;
  selected_artifact_id?: string | null;
  log_decode: DevdLogDecode;
}

export interface DevdScanResponse {
  devices: DevdDevice[];
}

export interface DevdLease {
  lease_id: string;
  device_id: string;
  identity_device_id?: string | null;
  heartbeat_interval_ms: number;
  lease_ttl_ms: number;
}

export interface DevdSession {
  connected: boolean;
  log_decode: DevdLogDecode;
  logs: Array<{
    id: string;
    timestamp: string;
    level: string;
    target: string;
    message: string;
  }>;
  trace: Array<{
    id: string;
    timestamp: string;
    direction: string;
    summary: string;
    payload: unknown;
  }>;
}

export interface DevdArtifactSelectResponse {
  artifact: unknown;
  log_decode: DevdLogDecode;
}

export interface DevdFlashResponse {
  ok: boolean;
  dry_run: boolean;
  action: "flash" | string;
  target_evidence: unknown;
  post_flash_identity?: unknown;
}

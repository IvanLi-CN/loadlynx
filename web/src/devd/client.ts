import type {
  DevdArtifactSelectResponse,
  DevdDevice,
  DevdFlashResponse,
  DevdLease,
  DevdScanResponse,
  DevdSession,
  DevdTargetKind,
} from "./types.ts";

export const DEFAULT_DEVD_BASE_URL =
  import.meta.env.VITE_LOADLYNX_DEVD_URL ?? "http://127.0.0.1:30180";

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export function buildDevdCompatBaseUrl(input: {
  baseUrl?: string;
  deviceId: string;
  leaseId?: string;
}): string {
  if (isStorybookRuntime()) {
    return `mock://devd-${input.deviceId}`;
  }

  const url = new URL(input.baseUrl ?? DEFAULT_DEVD_BASE_URL);
  url.searchParams.set("device_id", input.deviceId);
  if (input.leaseId) {
    url.searchParams.set("lease_id", input.leaseId);
  }
  return url.toString();
}

const MOCK_DEVD_DEVICES: DevdDevice[] = [
  {
    id: "mock-loadlynx-devd",
    display_name: "Mock LoadLynx devd device",
    connection: "connected",
    digital_target: {
      kind: "digital_esp32s3",
      display_name: "Mock ESP32-S3 USB CDC",
      port_path: "mock://esp32s3",
      selector_source: "mock",
    },
    analog_target: {
      kind: "analog_stm32g431",
      display_name: "Mock STM32G431 probe",
      probe_selector: "mock-probe",
      selector_source: "mock",
    },
    lan_endpoint: "mock://devd-lan",
    log_decode: {
      status: "unverified",
      reason: "no artifact selected",
      artifact_id: null,
    },
  },
  {
    id: "digital-aabbcc",
    display_name: "ESP32-S3 USB CDC (/dev/cu.usbmodem-aabbcc)",
    connection: "disconnected",
    digital_target: {
      kind: "digital_esp32s3",
      display_name: "ESP32-S3 USB CDC",
      port_path: "/dev/cu.usbmodem-aabbcc",
      selector_source: "mock serial scan",
    },
    log_decode: {
      status: "unverified",
      reason: "no artifact selected",
      artifact_id: null,
    },
  },
];

export class DevdApiError extends Error {
  readonly status: number;
  readonly code?: string;

  constructor(input: { status: number; code?: string; message: string }) {
    super(input.message);
    this.name = "DevdApiError";
    this.status = input.status;
    this.code = input.code;
  }
}

async function devdJson<T>(
  baseUrl: string,
  path: string,
  init?: RequestInit,
): Promise<T> {
  if (isStorybookRuntime()) {
    return mockDevd<T>(path, init);
  }

  const response = await fetch(new URL(path, baseUrl).toString(), {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers as Record<string, string> | undefined),
    },
  });
  const text = await response.text();
  const data = text ? (JSON.parse(text) as unknown) : null;
  if (!response.ok) {
    const envelope = data as { error?: { message?: string; code?: string } };
    const message = envelope.error?.message ?? `devd HTTP ${response.status}`;
    throw new DevdApiError({
      status: response.status,
      code: envelope.error?.code,
      message,
    });
  }
  return data as T;
}

function mockDevd<T>(path: string, init?: RequestInit): T {
  if (path === "/api/v1/devices/scan") {
    return { devices: MOCK_DEVD_DEVICES } as T;
  }
  if (path === "/api/v1/devices") {
    return { devices: MOCK_DEVD_DEVICES, leases: [] } as T;
  }
  if (path === "/api/v1/serial/lease") {
    const body = init?.body ? JSON.parse(String(init.body)) : {};
    return {
      lease_id: "mock-lease-1",
      device_id: body.device_id ?? "mock-loadlynx-devd",
      identity_device_id: "mock-loadlynx-devd",
      heartbeat_interval_ms: 2000,
      lease_ttl_ms: 8000,
    } as T;
  }
  if (/\/api\/v1\/serial\/lease\/.+/.test(path)) {
    return {
      lease_id: path.split("/").at(-1) ?? "mock-lease-1",
      device_id: "mock-loadlynx-devd",
      identity_device_id: "mock-loadlynx-devd",
      heartbeat_interval_ms: 2000,
      lease_ttl_ms: 8000,
    } as T;
  }
  if (/\/api\/v1\/devices\/.+\/session/.test(path)) {
    return {
      connected: true,
      log_decode: { status: "unverified", reason: "mock session" },
      logs: [
        {
          id: "log-1",
          timestamp: new Date(0).toISOString(),
          level: "info",
          target: "devd",
          message: "mock USB session active",
        },
      ],
      trace: [
        {
          id: "trace-1",
          timestamp: new Date(0).toISOString(),
          direction: "rx",
          summary: "hello",
          payload: { type: "hello", protocol: "loadlynx.cdc.v1" },
        },
      ],
    } as T;
  }
  if (/\/api\/v1\/devices\/.+\/flash/.test(path)) {
    return {
      ok: true,
      dry_run: true,
      action: "flash",
      target_evidence: { device_id: "mock-loadlynx-devd", target: "mock" },
    } as T;
  }
  if (/\/api\/v1\/devices\/.+\/artifact/.test(path)) {
    const body = init?.body ? JSON.parse(String(init.body)) : {};
    return {
      artifact: {
        artifact_id: body.artifact_id ?? "digital-release-aabbcc",
        target: "digital_esp32s3",
      },
      log_decode: {
        status: "unverified",
        reason: "mock artifact",
        artifact_id: body.artifact_id ?? "digital-release-aabbcc",
      },
    } as T;
  }
  return { ok: true } as T;
}

export async function scanDevdDevices(
  baseUrl: string = DEFAULT_DEVD_BASE_URL,
): Promise<DevdScanResponse> {
  return devdJson<DevdScanResponse>(baseUrl, "/api/v1/devices/scan", {
    method: "POST",
  });
}

export async function createDevdLease(
  deviceId: string,
  baseUrl: string = DEFAULT_DEVD_BASE_URL,
): Promise<DevdLease> {
  return devdJson<DevdLease>(baseUrl, "/api/v1/serial/lease", {
    method: "POST",
    body: JSON.stringify({ device_id: deviceId }),
  });
}

export async function heartbeatDevdLease(
  leaseId: string,
  baseUrl: string = DEFAULT_DEVD_BASE_URL,
): Promise<DevdLease> {
  return devdJson<DevdLease>(
    baseUrl,
    `/api/v1/serial/lease/${encodeURIComponent(leaseId)}`,
    { method: "POST" },
  );
}

export async function getDevdSession(
  deviceId: string,
  leaseId: string,
  baseUrl: string = DEFAULT_DEVD_BASE_URL,
): Promise<DevdSession> {
  return devdJson<DevdSession>(
    baseUrl,
    `/api/v1/devices/${encodeURIComponent(deviceId)}/session?lease_id=${encodeURIComponent(leaseId)}`,
  );
}

export async function selectDevdArtifact(input: {
  baseUrl?: string;
  deviceId: string;
  manifestPath: string;
  artifactId?: string;
}): Promise<DevdArtifactSelectResponse> {
  return devdJson<DevdArtifactSelectResponse>(
    input.baseUrl ?? DEFAULT_DEVD_BASE_URL,
    `/api/v1/devices/${encodeURIComponent(input.deviceId)}/artifact`,
    {
      method: "POST",
      body: JSON.stringify({
        manifest_path: input.manifestPath,
        artifact_id: input.artifactId || undefined,
      }),
    },
  );
}

export async function flashDevdDevice(input: {
  baseUrl?: string;
  deviceId: string;
  leaseId?: string;
  target: DevdTargetKind;
  artifactId?: string;
  dryRun: boolean;
  confirmationPhrase?: string;
  expectedIdentityDeviceId?: string;
  acknowledgeNonProjectFirmware?: boolean;
}): Promise<DevdFlashResponse> {
  return devdJson<DevdFlashResponse>(
    input.baseUrl ?? DEFAULT_DEVD_BASE_URL,
    `/api/v1/devices/${encodeURIComponent(input.deviceId)}/flash`,
    {
      method: "POST",
      body: JSON.stringify({
        target: input.target,
        artifact_id: input.artifactId || undefined,
        lease_id: input.leaseId || undefined,
        dry_run: input.dryRun,
        confirmation_phrase: input.confirmationPhrase || undefined,
        expected_identity_device_id:
          input.expectedIdentityDeviceId || undefined,
        acknowledge_non_project_firmware:
          input.acknowledgeNonProjectFirmware ?? false,
      }),
    },
  );
}

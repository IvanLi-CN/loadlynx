export const WEB_SERIAL_FLASH_CONFIRMATION_TEXT = "yes";

export interface WebSerialIdentityProfile {
  deviceId: string;
  displayName?: string;
  product?: string;
  firmware?: unknown;
  capturedAt: string;
}

export interface FirmwareCatalogFile {
  kind: string;
  path: string;
  sha256: string;
  size: number;
  flash_address?: number | null;
}

export interface FirmwareCatalogArtifact {
  artifact_id: string;
  name: string;
  target: "digital_esp32s3" | "analog_stm32g431" | string;
  protocol: string;
  files: FirmwareCatalogFile[];
}

export interface FirmwareCatalog {
  schema_version: string;
  artifacts: FirmwareCatalogArtifact[];
}

type WebSerialFlashFile = FirmwareCatalogFile & { flash_address: number };

export interface WebSerialFlashInput {
  catalog: FirmwareCatalog;
  artifactId?: string;
  firmwareFile: File;
  confirmationPhrase: string;
  acknowledgeNonProjectFirmware: boolean;
  expectedIdentityDeviceId?: string;
}

export interface WebSerialFlashResult {
  artifact: FirmwareCatalogArtifact;
  file: FirmwareCatalogFile;
  sha256: string;
  preFlashIdentity?: WebSerialIdentityProfile;
  postFlashIdentity?: WebSerialIdentityProfile;
}

const PROFILE_STORAGE_KEY = "loadlynx.webSerial.profiles";

type SerialPortLike = {
  open(options: { baudRate: number }): Promise<void>;
  close(): Promise<void>;
  readable: ReadableStream<Uint8Array> | null;
  writable: WritableStream<Uint8Array> | null;
};

type SerialNavigator = Navigator & {
  serial?: {
    requestPort(options?: unknown): Promise<SerialPortLike>;
    getPorts(): Promise<SerialPortLike[]>;
  };
};

export function hasWebSerialSupport(): boolean {
  return typeof navigator !== "undefined" && "serial" in navigator;
}

export async function parseFirmwareCatalog(
  file: File,
): Promise<FirmwareCatalog> {
  const text = await file.text();
  const parsed = JSON.parse(text) as FirmwareCatalog;
  if (!Array.isArray(parsed.artifacts)) {
    throw new Error("Firmware catalog is missing artifacts");
  }
  return parsed;
}

export async function runWebSerialDigitalFlash(
  input: WebSerialFlashInput,
): Promise<WebSerialFlashResult> {
  if (!hasWebSerialSupport()) {
    throw new Error("This browser does not expose Web Serial");
  }
  if (
    input.confirmationPhrase.trim().toLowerCase() !==
    WEB_SERIAL_FLASH_CONFIRMATION_TEXT
  ) {
    throw new Error(`Type ${WEB_SERIAL_FLASH_CONFIRMATION_TEXT} to confirm`);
  }
  const artifact = selectDigitalArtifact(input.catalog, input.artifactId);
  if (!isLoadLynxArtifact(artifact) && !input.acknowledgeNonProjectFirmware) {
    throw new Error(
      "Non-project or unknown firmware requires explicit risk acknowledgement",
    );
  }
  const file = selectFlashFile(artifact);
  const bytes = new Uint8Array(await input.firmwareFile.arrayBuffer());
  const sha256 = await sha256Hex(bytes);
  if (sha256 !== file.sha256.toLowerCase()) {
    throw new Error(
      `Firmware SHA-256 mismatch for ${input.firmwareFile.name}: expected ${file.sha256}, got ${sha256}`,
    );
  }

  const serial = (navigator as SerialNavigator).serial;
  if (!serial) {
    throw new Error("Web Serial is unavailable");
  }
  const port = await serial.requestPort();
  const preFlashIdentity = await tryCaptureIdentity(port);
  if (
    input.expectedIdentityDeviceId &&
    preFlashIdentity?.deviceId !== input.expectedIdentityDeviceId
  ) {
    throw new Error(
      `Expected identity ${input.expectedIdentityDeviceId}, current identity is ${preFlashIdentity?.deviceId ?? "<unknown>"}`,
    );
  }

  await flashEsp32s3(port, bytes, file.flash_address);
  const postFlashIdentity = await tryCaptureIdentity(port);
  if (postFlashIdentity) {
    saveWebSerialIdentityProfile(postFlashIdentity);
  }

  return {
    artifact,
    file,
    sha256,
    preFlashIdentity,
    postFlashIdentity,
  };
}

export async function listAuthorizedWebSerialProfiles(): Promise<
  WebSerialIdentityProfile[]
> {
  if (!hasWebSerialSupport()) {
    return [];
  }
  const profiles = readProfiles();
  const ports = await (navigator as SerialNavigator).serial?.getPorts();
  if (!ports?.length) {
    return [];
  }
  return profiles;
}

function selectDigitalArtifact(
  catalog: FirmwareCatalog,
  artifactId?: string,
): FirmwareCatalogArtifact {
  const artifacts = catalog.artifacts.filter(
    (artifact) => artifact.target === "digital_esp32s3",
  );
  const artifact = artifactId
    ? artifacts.find((candidate) => candidate.artifact_id === artifactId)
    : artifacts[0];
  if (!artifact) {
    throw new Error(
      "Catalog does not contain the requested digital ESP32-S3 artifact",
    );
  }
  return artifact;
}

function selectFlashFile(
  artifact: FirmwareCatalogArtifact,
): WebSerialFlashFile {
  const file = artifact.files.find((candidate) => candidate.kind === "image");
  if (!file) {
    throw new Error(
      "Web Serial flashing requires a raw image artifact; ELF files are only supported by CLI/devd flashing",
    );
  }
  if (file.flash_address == null) {
    throw new Error("Raw image artifacts require flash_address");
  }
  return file as WebSerialFlashFile;
}

function isLoadLynxArtifact(artifact: FirmwareCatalogArtifact): boolean {
  return `${artifact.artifact_id} ${artifact.name} ${artifact.protocol}`
    .toLowerCase()
    .includes("loadlynx");
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    bytes.slice().buffer as ArrayBuffer,
  );
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

async function flashEsp32s3(
  port: SerialPortLike,
  bytes: Uint8Array,
  flashAddress: number,
): Promise<void> {
  const esptool = (await import("esptool-js")) as Record<string, unknown>;
  const Transport = esptool.Transport as new (
    port: SerialPortLike,
    tracing?: boolean,
  ) => unknown;
  const ESPLoader = esptool.ESPLoader as new (
    options: unknown,
  ) => {
    main(): Promise<void>;
    writeFlash(options: unknown): Promise<void>;
  };
  if (!Transport || !ESPLoader) {
    throw new Error("esptool-js did not expose Transport and ESPLoader");
  }
  const transport = new Transport(port, false);
  const loader = new ESPLoader({
    transport,
    baudrate: 460800,
    terminal: {
      clean: () => undefined,
      writeLine: () => undefined,
      write: () => undefined,
    },
  });
  await loader.main();
  await loader.writeFlash({
    fileArray: [{ data: bytes, address: flashAddress }],
    flashSize: "keep",
    eraseAll: false,
    compress: true,
  });
}

async function tryCaptureIdentity(
  port: SerialPortLike,
): Promise<WebSerialIdentityProfile | undefined> {
  try {
    const payload = await jsonlRequest(port, "get_identity");
    const identity = (
      payload.data && typeof payload.data === "object" ? payload.data : payload
    ) as Record<string, unknown>;
    const deviceId =
      typeof identity.device_id === "string" ? identity.device_id : undefined;
    if (!deviceId) {
      return undefined;
    }
    return {
      deviceId,
      displayName:
        typeof identity.display_name === "string"
          ? identity.display_name
          : undefined,
      product:
        typeof identity.product === "string" ? identity.product : undefined,
      firmware: identity.firmware,
      capturedAt: new Date().toISOString(),
    };
  } catch {
    return undefined;
  }
}

async function jsonlRequest(
  port: SerialPortLike,
  op: string,
): Promise<Record<string, unknown>> {
  const wasOpen = Boolean(port.readable || port.writable);
  if (!wasOpen) {
    await port.open({ baudRate: 115200 });
  }
  try {
    if (!port.writable || !port.readable) {
      throw new Error("Serial port is not readable/writable");
    }
    const requestId = `web-${op}-${crypto.randomUUID()}`;
    const writer = port.writable.getWriter();
    try {
      const encoded = new TextEncoder().encode(
        `${JSON.stringify({ id: requestId, op })}\n`,
      );
      await writer.write(encoded);
    } finally {
      writer.releaseLock();
    }
    const reader = port.readable.getReader();
    const decoder = new TextDecoder();
    let buffered = "";
    try {
      const deadline = Date.now() + 5000;
      while (Date.now() < deadline) {
        const { value, done } = await reader.read();
        if (done) break;
        buffered += decoder.decode(value, { stream: true });
        for (const line of buffered.split(/\r?\n/)) {
          if (!line.trim().startsWith("{")) continue;
          const parsed = JSON.parse(line) as Record<string, unknown>;
          if (parsed.id === requestId) {
            return parsed;
          }
        }
      }
    } finally {
      reader.releaseLock();
    }
    throw new Error("Timed out waiting for Web Serial identity response");
  } finally {
    if (!wasOpen) {
      await port.close().catch(() => undefined);
    }
  }
}

function saveWebSerialIdentityProfile(profile: WebSerialIdentityProfile): void {
  const profiles = readProfiles().filter(
    (candidate) => candidate.deviceId !== profile.deviceId,
  );
  profiles.unshift(profile);
  localStorage.setItem(
    PROFILE_STORAGE_KEY,
    JSON.stringify(profiles.slice(0, 20)),
  );
}

function readProfiles(): WebSerialIdentityProfile[] {
  try {
    const parsed = JSON.parse(
      localStorage.getItem(PROFILE_STORAGE_KEY) ?? "[]",
    );
    return Array.isArray(parsed) ? (parsed as WebSerialIdentityProfile[]) : [];
  } catch {
    return [];
  }
}

const TAB_ID =
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `tab-${Math.random().toString(16).slice(2)}`;

export { TAB_ID };

export const ENABLE_MOCK = import.meta.env.VITE_ENABLE_MOCK_BACKEND !== "false";

export const ENABLE_MOCK_DEVTOOLS =
  import.meta.env.VITE_ENABLE_MOCK_DEVTOOLS === "true" ||
  (import.meta.env.DEV &&
    import.meta.env.VITE_ENABLE_MOCK_DEVTOOLS !== "false");

export function isMockBaseUrl(baseUrl: string): boolean {
  if (!baseUrl) {
    return false;
  }
  const normalized = baseUrl.trim().toLowerCase();
  return normalized.startsWith("mock://");
}

export function makeApiUrl(baseUrl: string, path: string): URL {
  const base = new URL(baseUrl);
  const url = new URL(path, base);
  base.searchParams.forEach((value, key) => {
    if (!url.searchParams.has(key)) {
      url.searchParams.append(key, value);
    }
  });
  return url;
}

export function isDevdCompatBaseUrl(baseUrl: string): boolean {
  if (!baseUrl || isMockBaseUrl(baseUrl)) {
    return false;
  }
  try {
    const url = new URL(baseUrl);
    return Boolean(
      url.searchParams.get("device_id") && url.searchParams.get("lease_id"),
    );
  } catch {
    return false;
  }
}

export function supportsBackupWifiCredentials(baseUrl: string): boolean {
  return Boolean(baseUrl);
}

export function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export interface HttpApiErrorInit {
  status: number;
  code?: string;
  message: string;
  retryable?: boolean;
  details?: unknown;
}

export class HttpApiError extends Error {
  readonly status: number;
  readonly code?: string;
  readonly retryable?: boolean;
  readonly details?: unknown;

  constructor(init: HttpApiErrorInit) {
    super(init.message);
    this.name = "HttpApiError";
    this.status = init.status;
    this.code = init.code;
    this.retryable = init.retryable;
    this.details = init.details;
  }
}

export function isHttpApiError(error: unknown): error is HttpApiError {
  return error instanceof HttpApiError;
}

interface ErrorEnvelope {
  error?: {
    code?: string;
    message?: string;
    retryable?: boolean;
    details?: unknown;
  };
}

function mapHttpError(status: number, data: unknown): HttpApiError {
  const envelope = (data ?? {}) as ErrorEnvelope;
  const inner = envelope.error ?? {};

  const code =
    typeof inner.code === "string" && inner.code.length > 0
      ? inner.code
      : undefined;
  const message =
    typeof inner.message === "string" && inner.message.length > 0
      ? inner.message
      : `HTTP ${status}`;
  const retryable =
    typeof inner.retryable === "boolean"
      ? inner.retryable
      : status >= 500 || status === 0;

  return new HttpApiError({
    status,
    code,
    message,
    retryable,
    details: inner.details ?? data,
  });
}

const deviceQueues = new Map<string, Promise<unknown>>();

function enqueueForDevice<T>(
  baseUrl: string,
  op: () => Promise<T>,
): Promise<T> {
  const tail = deviceQueues.get(baseUrl) ?? Promise.resolve();
  const next = tail.catch(() => undefined).then(() => op());

  deviceQueues.set(
    baseUrl,
    next.catch(() => undefined),
  );

  return next;
}

async function httpJson<T>(
  baseUrl: string,
  path: string,
  init?: RequestInit,
): Promise<T> {
  const method = init?.method ?? "GET";

  if (isStorybookRuntime() && !isMockBaseUrl(baseUrl)) {
    throw new Error(
      `[LoadLynx] Real device HTTP is disabled in Storybook. This request tried to call ${method} ${path} with baseUrl="${baseUrl}". Use a mock:// baseUrl instead.`,
    );
  }

  const url = makeApiUrl(baseUrl, path);

  const headers: Record<string, string> = {
    ...(init?.headers as Record<string, string> | undefined),
  };

  headers.Connection ||= "close";

  const hasBody = init?.body !== undefined && init.body !== null;
  if (hasBody || method.toUpperCase() !== "GET") {
    headers["Content-Type"] ||= "application/json";
  }

  let response: Response;
  try {
    response = await fetch(url.toString(), {
      method,
      ...init,
      headers,
    });
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Network request failed";
    throw new HttpApiError({
      status: 0,
      code: "NETWORK_ERROR",
      message,
      retryable: true,
      details: null,
    });
  }

  const text = await response.text();
  let data: unknown = null;

  if (text.length > 0) {
    try {
      data = JSON.parse(text) as unknown;
    } catch {
      throw new HttpApiError({
        status: response.status,
        code: "INVALID_JSON",
        message: `Invalid JSON from ${path}`,
        retryable: false,
        details: text.slice(0, 200),
      });
    }
  }

  if (!response.ok) {
    throw mapHttpError(response.status, data);
  }

  return data as T;
}

export async function httpJsonQueued<T>(
  baseUrl: string,
  path: string,
  init?: RequestInit,
): Promise<T> {
  return enqueueForDevice(baseUrl, () => httpJson<T>(baseUrl, path, init));
}

export const __testHttpJsonQueued = httpJsonQueued;
export const __testEnqueueForDevice = enqueueForDevice;
export const __testClearDeviceQueues = () => deviceQueues.clear();

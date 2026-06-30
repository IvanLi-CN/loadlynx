import {
  httpJsonQueued,
  isDevdCompatBaseUrl,
  isMockBaseUrl,
  isStorybookRuntime,
  makeApiUrl,
  TAB_ID,
} from "./client-core.ts";
import {
  type DevdIdentityPayload,
  type DevdStatusPayload,
  mockApplyPreset,
  mockDebugSetUvLatched,
  mockGetCc,
  mockGetControl,
  mockGetIdentity,
  mockGetPd,
  mockGetPresets,
  mockGetStatus,
  mockSoftReset,
  mockUpdateCc,
  mockUpdateControl,
  mockUpdatePd,
  mockUpdatePreset,
  normalizeDevdIdentity,
  normalizeDevdStatus,
} from "./client-mock.ts";
import { subscribeMockStatusStream } from "./mock-status-stream.ts";
import type {
  ApplyPresetRequest,
  CcControlView,
  CcUpdateRequest,
  ControlUpdateRequest,
  ControlView,
  FastStatusResponse,
  FastStatusView,
  Identity,
  PdUpdateRequest,
  PdView,
  Preset,
  PresetsResponse,
  SoftResetReason,
  SoftResetRequest,
  SoftResetResponse,
} from "./types.ts";

function toFastStatusView(payload: FastStatusResponse): FastStatusView {
  return {
    raw: payload.status,
    link_up: payload.link_up,
    hello_seen: payload.hello_seen,
    analog_state: payload.analog_state,
    fault_flags_decoded: payload.fault_flags_decoded,
    state_flags_decoded: payload.state_flags_decoded ?? [],
  };
}

function makeApplyPresetRequest(preset_id: number): ApplyPresetRequest {
  return { preset_id };
}

function makeSoftResetRequest(reason: SoftResetReason): SoftResetRequest {
  return { reason };
}

export async function getIdentity(baseUrl: string): Promise<Identity> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetIdentity(baseUrl);
  }
  if (isDevdCompatBaseUrl(baseUrl)) {
    try {
      const payload = await httpJsonQueued<DevdIdentityPayload>(
        baseUrl,
        "/api/v1/identity",
      );
      return normalizeDevdIdentity(baseUrl, payload);
    } catch {
      const status = await httpJsonQueued<DevdStatusPayload>(
        baseUrl,
        "/api/v1/status",
      );
      return normalizeDevdIdentity(baseUrl, {
        device_id: new URL(baseUrl).searchParams.get("device_id") ?? undefined,
        uptime_ms: status.status.uptime_ms,
      });
    }
  }
  return httpJsonQueued<Identity>(baseUrl, "/api/v1/identity");
}

export async function getStatus(baseUrl: string): Promise<FastStatusView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetStatus(baseUrl);
  }
  if (isDevdCompatBaseUrl(baseUrl)) {
    const payload = await httpJsonQueued<DevdStatusPayload>(
      baseUrl,
      "/api/v1/status",
    );
    return normalizeDevdStatus(payload);
  }

  const payload = await httpJsonQueued<FastStatusResponse>(
    baseUrl,
    "/api/v1/status",
  );
  return toFastStatusView(payload);
}

export function subscribeStatusStream(
  baseUrl: string,
  onMessage: (view: FastStatusView) => void,
  onError?: (error: Event | Error) => void,
): () => void {
  if (isMockBaseUrl(baseUrl)) {
    return subscribeMockStatusStream({
      baseUrl,
      onMessage,
      onError,
      readStatus: mockGetStatus,
    });
  }

  if (isDevdCompatBaseUrl(baseUrl)) {
    return () => undefined;
  }

  if (isStorybookRuntime()) {
    throw new Error(
      `[LoadLynx] Real device status streaming is disabled in Storybook. Use a mock:// baseUrl instead (attempted baseUrl="${baseUrl}").`,
    );
  }

  const url = makeApiUrl(baseUrl, "/api/v1/status");
  let closed = false;

  const isFastStatusView = (value: unknown): value is FastStatusView => {
    return (
      typeof value === "object" &&
      value !== null &&
      "raw" in value &&
      "link_up" in value &&
      "hello_seen" in value
    );
  };

  const emitMessage = (view: FastStatusView) => {
    if (!closed) {
      onMessage(view);
    }
  };

  const emitError = (error: Event | Error) => {
    if (!closed && onError) {
      onError(error);
    }
  };

  const parseAndEmit = (payload: string) => {
    try {
      const parsed = JSON.parse(payload) as FastStatusView | FastStatusResponse;

      const view: FastStatusView = isFastStatusView(parsed)
        ? parsed
        : toFastStatusView(parsed);

      emitMessage(view);
    } catch (error) {
      emitError(
        error instanceof Error ? error : new Error("invalid SSE payload"),
      );
    }
  };

  const handleStatus = (event: MessageEvent) => {
    parseAndEmit(event.data);
  };
  const handleError = (event: Event) => {
    emitError(event);
  };

  const canShareAcrossTabs =
    typeof BroadcastChannel !== "undefined" &&
    typeof navigator !== "undefined" &&
    "locks" in navigator;

  if (!canShareAcrossTabs) {
    const source = new EventSource(url.toString());
    source.addEventListener("status", handleStatus as EventListener);
    source.addEventListener("message", handleStatus as EventListener);
    source.addEventListener("error", handleError);

    return () => {
      closed = true;
      source.removeEventListener("status", handleStatus as EventListener);
      source.removeEventListener("message", handleStatus as EventListener);
      source.removeEventListener("error", handleError);
      source.close();
    };
  }

  const lockName = `llx-status-sse:${new URL(baseUrl).origin}`;
  const channel = new BroadcastChannel(lockName);
  let releaseLeader: (() => void) | null = null;

  type BroadcastEnvelope =
    | { t: "status"; d: string; from: string }
    | { t: "bye"; from: string };

  void navigator.locks
    .request(lockName, { mode: "exclusive" }, async () => {
      if (closed) {
        return;
      }

      let resolveRelease: (() => void) | null = null;
      const waitRelease = new Promise<void>((resolve) => {
        resolveRelease = resolve;
      });
      releaseLeader = () => {
        resolveRelease?.();
      };

      const leaderSource = new EventSource(url.toString());

      const broadcastStatus = (event: MessageEvent) => {
        const msg: BroadcastEnvelope = {
          t: "status",
          d: event.data,
          from: TAB_ID,
        };
        try {
          channel.postMessage(msg);
        } catch {
          // ignore
        }
        parseAndEmit(event.data);
      };
      const broadcastError = (event: Event) => {
        handleError(event);
      };

      leaderSource.addEventListener("status", broadcastStatus as EventListener);
      leaderSource.addEventListener(
        "message",
        broadcastStatus as EventListener,
      );
      leaderSource.addEventListener("error", broadcastError);

      try {
        await waitRelease;
      } finally {
        leaderSource.removeEventListener(
          "status",
          broadcastStatus as EventListener,
        );
        leaderSource.removeEventListener(
          "message",
          broadcastStatus as EventListener,
        );
        leaderSource.removeEventListener("error", broadcastError);
        leaderSource.close();

        try {
          channel.postMessage({
            t: "bye",
            from: TAB_ID,
          } satisfies BroadcastEnvelope);
        } catch {
          // ignore
        }
        releaseLeader = null;
      }
    })
    .catch((error) => {
      emitError(
        error instanceof Error ? error : new Error("status lock error"),
      );
    });

  const onChannelMessage = (event: MessageEvent) => {
    if (closed) {
      return;
    }
    const payload = event.data as Partial<BroadcastEnvelope> | null;
    if (!payload || typeof payload !== "object" || payload.from === TAB_ID) {
      return;
    }
    if (payload.t === "status" && typeof payload.d === "string") {
      parseAndEmit(payload.d);
    }
  };

  channel.addEventListener("message", onChannelMessage);

  return () => {
    closed = true;
    releaseLeader?.();
    releaseLeader = null;
    channel.removeEventListener("message", onChannelMessage);
    channel.close();
  };
}

export async function getCc(baseUrl: string): Promise<CcControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetCc(baseUrl);
  }
  return httpJsonQueued<CcControlView>(baseUrl, "/api/v1/cc");
}

export async function getPd(baseUrl: string): Promise<PdView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetPd(baseUrl);
  }
  return httpJsonQueued<PdView>(baseUrl, "/api/v1/pd");
}

export async function postPd(
  baseUrl: string,
  payload: PdUpdateRequest,
): Promise<PdView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockUpdatePd(baseUrl, payload);
  }
  return httpJsonQueued<PdView>(baseUrl, "/api/v1/pd", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function updateCc(
  baseUrl: string,
  payload: CcUpdateRequest,
): Promise<CcControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockUpdateCc(baseUrl, payload);
  }
  return httpJsonQueued<CcControlView>(baseUrl, "/api/v1/cc", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function getPresets(baseUrl: string): Promise<Preset[]> {
  if (isMockBaseUrl(baseUrl)) {
    return (await mockGetPresets(baseUrl)).presets;
  }
  const payload = await httpJsonQueued<PresetsResponse>(
    baseUrl,
    "/api/v1/presets",
  );
  return payload.presets;
}

export async function updatePreset(
  baseUrl: string,
  payload: Preset,
): Promise<Preset> {
  if (isMockBaseUrl(baseUrl)) {
    return mockUpdatePreset(baseUrl, payload);
  }
  return httpJsonQueued<Preset>(baseUrl, "/api/v1/presets", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function applyPreset(
  baseUrl: string,
  preset_id: number,
): Promise<ControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockApplyPreset(baseUrl, preset_id);
  }
  const payload = makeApplyPresetRequest(preset_id);
  return httpJsonQueued<ControlView>(baseUrl, "/api/v1/presets/apply", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function getControl(baseUrl: string): Promise<ControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetControl(baseUrl);
  }
  return httpJsonQueued<ControlView>(baseUrl, "/api/v1/control");
}

export async function updateControl(
  baseUrl: string,
  payload: ControlUpdateRequest,
): Promise<ControlView> {
  if (isMockBaseUrl(baseUrl)) {
    return mockUpdateControl(baseUrl, payload);
  }
  return httpJsonQueued<ControlView>(baseUrl, "/api/v1/control", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function __debugSetUvLatched(
  baseUrl: string,
  uv_latched: boolean,
): Promise<ControlView> {
  if (!isMockBaseUrl(baseUrl)) {
    throw new Error(
      "UV latch debug toggle is only available for mock:// devices",
    );
  }
  return mockDebugSetUvLatched(baseUrl, uv_latched);
}

export async function postSoftReset(
  baseUrl: string,
  reason: SoftResetReason = "manual",
): Promise<SoftResetResponse> {
  if (isMockBaseUrl(baseUrl)) {
    return mockSoftReset(baseUrl, reason);
  }
  const payload = makeSoftResetRequest(reason);
  return httpJsonQueued<SoftResetResponse>(baseUrl, "/api/v1/soft-reset", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json; charset=utf-8",
    },
  });
}

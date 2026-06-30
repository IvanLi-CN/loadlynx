import type { FastStatusView } from "./types.ts";

export const DEFAULT_MOCK_STATUS_STREAM_INTERVAL_MS = 500;

type StatusListener = (view: FastStatusView) => void;
type ErrorListener = (error: Event | Error) => void;

type MockStatusStreamChannel = {
  listeners: Set<StatusListener>;
  errorListeners: Set<ErrorListener>;
  inFlight: boolean;
  timerId: ReturnType<typeof globalThis.setTimeout> | null;
};

type MockStatusStreamRegistry = Map<string, MockStatusStreamChannel>;

type SubscribeMockStatusStreamParams = {
  baseUrl: string;
  onMessage: StatusListener;
  onError?: ErrorListener;
  readStatus: (baseUrl: string) => Promise<FastStatusView>;
  intervalMs?: number;
};

declare global {
  // HMR-safe singleton registry for mock streams.
  var __LOADLYNX_MOCK_STATUS_STREAMS__: MockStatusStreamRegistry | undefined;
}

function getRegistry(): MockStatusStreamRegistry {
  if (!globalThis.__LOADLYNX_MOCK_STATUS_STREAMS__) {
    globalThis.__LOADLYNX_MOCK_STATUS_STREAMS__ = new Map();
  }
  return globalThis.__LOADLYNX_MOCK_STATUS_STREAMS__;
}

function createChannel(): MockStatusStreamChannel {
  return {
    listeners: new Set(),
    errorListeners: new Set(),
    inFlight: false,
    timerId: null,
  };
}

export function subscribeMockStatusStream({
  baseUrl,
  onMessage,
  onError,
  readStatus,
  intervalMs = DEFAULT_MOCK_STATUS_STREAM_INTERVAL_MS,
}: SubscribeMockStatusStreamParams): () => void {
  const registry = getRegistry();
  const channel = registry.get(baseUrl) ?? createChannel();
  registry.set(baseUrl, channel);

  channel.listeners.add(onMessage);
  if (onError) {
    channel.errorListeners.add(onError);
  }

  const scheduleNextTick = () => {
    if (
      channel.timerId != null ||
      channel.inFlight ||
      channel.listeners.size === 0
    ) {
      return;
    }

    channel.timerId = globalThis.setTimeout(async () => {
      channel.timerId = null;
      if (channel.listeners.size === 0 || channel.inFlight) {
        return;
      }

      channel.inFlight = true;
      try {
        const next = await readStatus(baseUrl);
        for (const listener of [...channel.listeners]) {
          listener(next);
        }
      } catch (error) {
        const nextError =
          error instanceof Error ? error : new Error("mock stream error");
        for (const listener of [...channel.errorListeners]) {
          listener(nextError);
        }
      } finally {
        channel.inFlight = false;
        if (channel.listeners.size === 0) {
          registry.delete(baseUrl);
        } else {
          scheduleNextTick();
        }
      }
    }, intervalMs);
  };

  scheduleNextTick();

  return () => {
    channel.listeners.delete(onMessage);
    if (onError) {
      channel.errorListeners.delete(onError);
    }

    if (channel.listeners.size > 0) {
      return;
    }

    if (channel.timerId != null) {
      globalThis.clearTimeout(channel.timerId);
      channel.timerId = null;
    }
    if (!channel.inFlight) {
      registry.delete(baseUrl);
    }
  };
}

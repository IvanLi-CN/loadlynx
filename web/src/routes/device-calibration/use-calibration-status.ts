import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  type HttpApiError,
  isMockBaseUrl,
  subscribeStatusStream,
} from "../../api/client.ts";
import type { FastStatusView } from "../../api/types.ts";
import { DEVICE_QUERY_PARTS } from "../../devices/device-query-key.ts";
import { getDeviceStatusQueryOptions } from "../../devices/hooks.ts";
import { makeCalibrationRuntimeId } from "./runtime-id.ts";
import type { WithStatusStreamPaused } from "./shared.ts";

const CALIBRATION_STATUS_FALLBACK_REFETCH_MS = 500;
const CALIBRATION_STATUS_STALE_TIMEOUT_MS = 2_500;
const CALIBRATION_STATUS_STREAM_STARTUP_TIMEOUT_MS = 1_500;
const calibrationStatusRetryDelay = () => 200 + Math.random() * 300;

interface StatusWaiter {
  id: string;
  predicate: (view: FastStatusView) => boolean;
  resolve: (view: FastStatusView) => void;
  reject: (error: Error) => void;
  timeoutId: number;
}

export function useCalibrationStatus(input: {
  deviceId: string;
  baseUrl: string;
  isPageVisible: boolean;
}) {
  const { deviceId, baseUrl, isPageVisible } = input;
  const [status, setStatus] = useState<FastStatusView | null>(null);
  const [statusStreamPaused, setStatusStreamPaused] = useState(false);
  const [statusStreamConnected, setStatusStreamConnected] = useState(false);
  const [statusFallbackArmed, setStatusFallbackArmed] = useState(false);
  const latestStatusRef = useRef<FastStatusView | null>(status);
  const lastStatusAtRef = useRef<number | null>(null);
  const statusPauseDepthRef = useRef(0);
  const statusWaitersRef = useRef<StatusWaiter[]>([]);

  const publishStatusSnapshot = useCallback((view: FastStatusView | null) => {
    latestStatusRef.current = view;
    if (view) {
      lastStatusAtRef.current = Date.now();
      const remaining: StatusWaiter[] = [];
      for (const waiter of statusWaitersRef.current) {
        if (waiter.predicate(view)) {
          window.clearTimeout(waiter.timeoutId);
          waiter.resolve(view);
        } else {
          remaining.push(waiter);
        }
      }
      statusWaitersRef.current = remaining;
    }
    setStatus(view);
  }, []);

  const withStatusStreamPaused = useCallback<WithStatusStreamPaused>(
    async (op) => {
      if (isMockBaseUrl(baseUrl)) {
        return await op();
      }

      statusPauseDepthRef.current += 1;
      if (statusPauseDepthRef.current === 1) {
        setStatusStreamPaused(true);
        await new Promise((resolve) => window.setTimeout(resolve, 350));
      }
      try {
        return await op();
      } finally {
        statusPauseDepthRef.current -= 1;
        if (statusPauseDepthRef.current === 0) {
          setStatusStreamPaused(false);
        }
      }
    },
    [baseUrl],
  );

  const rejectStatusWaiters = useCallback((error: Error) => {
    for (const waiter of statusWaitersRef.current) {
      window.clearTimeout(waiter.timeoutId);
      waiter.reject(error);
    }
    statusWaitersRef.current = [];
  }, []);

  useEffect(() => {
    latestStatusRef.current = status;
  }, [status]);

  const getLatestStatus = useCallback(() => latestStatusRef.current, []);
  const getLatestCalKind = useCallback(
    () => latestStatusRef.current?.raw.cal_kind ?? null,
    [],
  );

  const waitForStatus = useCallback(
    (
      predicate: (view: FastStatusView) => boolean,
      timeoutMs: number,
    ): Promise<FastStatusView> => {
      const current = latestStatusRef.current;
      if (current && predicate(current)) {
        return Promise.resolve(current);
      }
      return new Promise((resolve, reject) => {
        const id = makeCalibrationRuntimeId("status");
        const timeoutId = window.setTimeout(
          () => {
            statusWaitersRef.current = statusWaitersRef.current.filter(
              (waiter) => waiter.id !== id,
            );
            reject(new Error("Timed out waiting for device status"));
          },
          Math.max(0, timeoutMs),
        );
        statusWaitersRef.current.push({
          id,
          predicate,
          resolve,
          reject,
          timeoutId,
        });
      });
    },
    [],
  );

  useEffect(() => {
    lastStatusAtRef.current = null;
    publishStatusSnapshot(null);
    setStatusStreamConnected(false);
    setStatusFallbackArmed(false);
    rejectStatusWaiters(new Error(`Status stream reset for ${baseUrl}`));
  }, [baseUrl, publishStatusSnapshot, rejectStatusWaiters]);

  useEffect(() => {
    if (statusStreamPaused) {
      return undefined;
    }

    const unsubscribe = subscribeStatusStream(
      baseUrl,
      (view) => {
        setStatusStreamConnected(true);
        setStatusFallbackArmed(false);
        publishStatusSnapshot(view);
      },
      () => {
        setStatusStreamConnected(false);
        setStatusFallbackArmed(true);
      },
    );

    return () => {
      unsubscribe();
      setStatusStreamConnected(false);
      rejectStatusWaiters(new Error("Status stream closed"));
    };
  }, [baseUrl, publishStatusSnapshot, rejectStatusWaiters, statusStreamPaused]);

  useEffect(() => {
    if (statusStreamPaused || statusStreamConnected || statusFallbackArmed) {
      return;
    }

    const timeoutId = window.setTimeout(() => {
      if (statusPauseDepthRef.current === 0 && !statusStreamConnected) {
        setStatusFallbackArmed(true);
      }
    }, CALIBRATION_STATUS_STREAM_STARTUP_TIMEOUT_MS);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [statusFallbackArmed, statusStreamConnected, statusStreamPaused]);

  const statusFallbackQuery = useQuery<FastStatusView, HttpApiError>(
    getDeviceStatusQueryOptions({
      deviceId,
      baseUrl,
      enabled:
        Boolean(baseUrl) &&
        isPageVisible &&
        statusFallbackArmed &&
        !statusStreamPaused &&
        !statusStreamConnected,
      parts: DEVICE_QUERY_PARTS.calibrationStatusFallback,
      refetchInterval: isPageVisible
        ? CALIBRATION_STATUS_FALLBACK_REFETCH_MS
        : false,
      refetchOnWindowFocus: false,
      retry: 2,
      retryDelay: calibrationStatusRetryDelay,
    }),
  );

  useEffect(() => {
    if (!statusFallbackQuery.data || statusFallbackQuery.dataUpdatedAt === 0) {
      return;
    }
    publishStatusSnapshot(statusFallbackQuery.data);
  }, [
    publishStatusSnapshot,
    statusFallbackQuery.data,
    statusFallbackQuery.dataUpdatedAt,
  ]);

  useEffect(() => {
    if (
      statusStreamPaused ||
      statusStreamConnected ||
      !statusFallbackQuery.isError ||
      statusFallbackQuery.fetchStatus === "fetching"
    ) {
      return;
    }

    const lastStatusAt = lastStatusAtRef.current;
    if (lastStatusAt === null) {
      publishStatusSnapshot(null);
      return;
    }

    const remainingMs =
      CALIBRATION_STATUS_STALE_TIMEOUT_MS - (Date.now() - lastStatusAt);
    if (remainingMs <= 0) {
      lastStatusAtRef.current = null;
      publishStatusSnapshot(null);
      return;
    }

    const timeoutId = window.setTimeout(() => {
      if (
        statusPauseDepthRef.current === 0 &&
        !statusStreamConnected &&
        statusFallbackQuery.isError &&
        statusFallbackQuery.fetchStatus !== "fetching"
      ) {
        lastStatusAtRef.current = null;
        publishStatusSnapshot(null);
      }
    }, remainingMs);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [
    publishStatusSnapshot,
    statusFallbackQuery.fetchStatus,
    statusFallbackQuery.isError,
    statusStreamConnected,
    statusStreamPaused,
  ]);

  return {
    getLatestCalKind,
    getLatestStatus,
    latestStatusRef,
    publishStatusSnapshot,
    status,
    waitForStatus,
    withStatusStreamPaused,
  };
}

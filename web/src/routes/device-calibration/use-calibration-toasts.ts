import { useCallback, useEffect, useState } from "react";
import { makeCalibrationRuntimeId } from "./runtime-id.ts";
import type { UndoAction } from "./shared.ts";

interface UndoToastEntry {
  id: string;
  message: string;
  action: UndoAction;
  expiresAt: number;
  timeoutId: number;
}

interface InfoToastEntry {
  id: string;
  message: string;
  timeoutId: number;
}

export function useCalibrationToasts() {
  const [undoToasts, setUndoToasts] = useState<UndoToastEntry[]>([]);
  const [infoToasts, setInfoToasts] = useState<InfoToastEntry[]>([]);
  const [undoNow, setUndoNow] = useState(() => Date.now());

  const clearToasts = useCallback(() => {
    setUndoToasts((prev) => {
      for (const toast of prev) {
        window.clearTimeout(toast.timeoutId);
      }
      return [];
    });
    setInfoToasts((prev) => {
      for (const toast of prev) {
        window.clearTimeout(toast.timeoutId);
      }
      return [];
    });
  }, []);

  useEffect(() => {
    return () => {
      clearToasts();
    };
  }, [clearToasts]);

  const enqueueInfoToast = useCallback((message: string) => {
    const id = makeCalibrationRuntimeId("toast");
    const timeoutId = window.setTimeout(() => {
      setInfoToasts((prev) => prev.filter((toast) => toast.id !== id));
    }, 2_500);
    setInfoToasts((prev) => [...prev, { id, message, timeoutId }]);
  }, []);

  const undoToast = useCallback(
    (toast: UndoToastEntry, onUndoAction: (action: UndoAction) => void) => {
      window.clearTimeout(toast.timeoutId);
      setUndoToasts((prev) => prev.filter((entry) => entry.id !== toast.id));
      onUndoAction(toast.action);
    },
    [],
  );

  const enqueueUndo = useCallback((action: UndoAction, message: string) => {
    const id = makeCalibrationRuntimeId("undo");
    const expiresAt = Date.now() + 5_000;
    const timeoutId = window.setTimeout(() => {
      setUndoToasts((prev) => prev.filter((toast) => toast.id !== id));
    }, 5_000);
    setUndoToasts((prev) => [
      ...prev,
      { id, message, action, expiresAt, timeoutId },
    ]);
  }, []);

  useEffect(() => {
    if (undoToasts.length === 0) return;
    const intervalId = window.setInterval(() => setUndoNow(Date.now()), 250);
    return () => window.clearInterval(intervalId);
  }, [undoToasts.length]);

  return {
    clearToasts,
    enqueueInfoToast,
    enqueueUndo,
    infoToasts,
    undoNow,
    undoToast,
    undoToasts,
  };
}

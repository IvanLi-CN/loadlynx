import type { UndoAction } from "./shared.ts";

interface InfoToastView {
  id: string;
  message: string;
}

interface UndoToastView {
  id: string;
  message: string;
  action: UndoAction;
  expiresAt: number;
  timeoutId: number;
}

export function CalibrationToastStack(input: {
  applyUndoAction: (action: UndoAction) => void;
  infoToasts: InfoToastView[];
  undoNow: number;
  undoToast: (
    toast: UndoToastView,
    onUndoAction: (action: UndoAction) => void,
  ) => void;
  undoToasts: UndoToastView[];
}) {
  const { applyUndoAction, infoToasts, undoNow, undoToast, undoToasts } = input;

  if (infoToasts.length === 0 && undoToasts.length === 0) {
    return null;
  }

  return (
    <div className="toast toast-end toast-bottom z-50">
      {infoToasts.map((toast) => (
        <div key={toast.id} className="ll-alert ll-alert-success text-sm">
          <div className="flex items-center justify-between gap-3 w-full">
            <div className="flex-1">{toast.message}</div>
          </div>
        </div>
      ))}
      {undoToasts.map((toast) => {
        const remaining = Math.max(
          0,
          Math.ceil((toast.expiresAt - undoNow) / 1000),
        );
        return (
          <div key={toast.id} className="ll-alert ll-alert-info text-sm">
            <div className="flex items-center justify-between gap-3 w-full">
              <div className="flex-1">{toast.message}</div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  className="ll-button ll-button-xs ll-button-outline"
                  onClick={() => undoToast(toast, applyUndoAction)}
                >
                  Undo
                </button>
                <span className="font-mono text-xs text-base-content/60">
                  {remaining}s
                </span>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  body: string;
  details?: string[];
  confirmLabel: string;
  cancelLabel?: string;
  destructive?: boolean;
  confirmDisabled?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open,
  title,
  body,
  details = [],
  confirmLabel,
  cancelLabel = "Cancel",
  destructive = false,
  confirmDisabled = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  if (!open) return null;

  return (
    <div className="modal modal-open" role="dialog" aria-modal="true">
      <div className="modal-box">
        <h3 className="font-bold text-lg">{title}</h3>
        <p className="py-3 text-sm">{body}</p>
        {details.length > 0 && (
          <ul className="list-disc pl-5 text-sm space-y-1">
            {details.map((line, idx) => (
              <li key={`${idx}:${line}`}>{line}</li>
            ))}
          </ul>
        )}

        <div className="modal-action">
          <button type="button" className="btn" onClick={onCancel}>
            {cancelLabel}
          </button>
          <button
            type="button"
            className={`btn ${destructive ? "btn-error" : "btn-primary"}`}
            disabled={confirmDisabled}
            onClick={onConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Close"
        onClick={onCancel}
      />
    </div>
  );
}

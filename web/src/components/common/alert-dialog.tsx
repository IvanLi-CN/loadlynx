export interface AlertDialogProps {
  open: boolean;
  title: string;
  body: string;
  details?: string[];
  closeLabel?: string;
  onClose: () => void;
}

export function AlertDialog({
  open,
  title,
  body,
  details = [],
  closeLabel = "Close",
  onClose,
}: AlertDialogProps) {
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
          <button type="button" className="btn btn-primary" onClick={onClose}>
            {closeLabel}
          </button>
        </div>
      </div>
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Close"
        onClick={onClose}
      />
    </div>
  );
}

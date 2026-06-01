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
    <div className="ll-modal" role="dialog" aria-modal="true">
      <div className="ll-modal-box">
        <h3 className="font-bold text-lg">{title}</h3>
        <p className="py-3 text-sm">{body}</p>
        {details.length > 0 && (
          <ul className="list-disc pl-5 text-sm space-y-1">
            {details.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        )}

        <div className="ll-modal-action">
          <button
            type="button"
            className="ll-button ll-button-primary"
            onClick={onClose}
          >
            {closeLabel}
          </button>
        </div>
      </div>
      <button
        type="button"
        className="ll-modal-backdrop"
        aria-label="Close"
        onClick={onClose}
      />
    </div>
  );
}

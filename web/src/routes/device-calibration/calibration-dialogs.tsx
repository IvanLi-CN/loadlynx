import { AlertDialog } from "../../components/common/alert-dialog.tsx";
import { ConfirmDialog } from "../../components/common/confirm-dialog.tsx";
import type { CalibrationAlertDialogState } from "./use-calibration-alert.ts";

export function CalibrationDialogs(input: {
  alertDialog: CalibrationAlertDialogState | null;
  confirmReadDeviceToDraft: boolean;
  isOffline: boolean;
  onCloseAlert: () => void;
  onConfirmReadDeviceToDraft: () => void;
  onDismissReadDeviceToDraft: () => void;
  readDeviceToDraftPending: boolean;
}) {
  const {
    alertDialog,
    confirmReadDeviceToDraft,
    isOffline,
    onCloseAlert,
    onConfirmReadDeviceToDraft,
    onDismissReadDeviceToDraft,
    readDeviceToDraftPending,
  } = input;

  return (
    <>
      <ConfirmDialog
        open={confirmReadDeviceToDraft}
        title="Read Device Calibration → Draft"
        body="This reads the current calibration profile from the device and overwrites the local web draft."
        details={[
          "Affects: v_local, v_remote, current_ch1, current_ch2 (local draft only).",
          "Writes device: No.",
          "Preview: cleared (returns to device preview).",
          "Irreversible locally: Yes (export draft first if needed).",
        ]}
        confirmLabel="Overwrite Draft"
        destructive
        confirmDisabled={readDeviceToDraftPending || isOffline}
        onCancel={onDismissReadDeviceToDraft}
        onConfirm={onConfirmReadDeviceToDraft}
      />

      <AlertDialog
        open={alertDialog !== null}
        title={alertDialog?.title ?? ""}
        body={alertDialog?.body ?? ""}
        details={alertDialog?.details ?? []}
        onClose={onCloseAlert}
      />
    </>
  );
}

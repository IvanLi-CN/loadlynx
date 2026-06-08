import { useCallback, useState } from "react";

export interface CalibrationAlertDialogState {
  title: string;
  body: string;
  details: string[];
}

export function useCalibrationAlert() {
  const [alertDialog, setAlertDialog] =
    useState<CalibrationAlertDialogState | null>(null);

  const showAlert = useCallback(
    (title: string, body: string, details: string[] = []) => {
      setAlertDialog({ title, body, details });
    },
    [],
  );

  const clearAlert = useCallback(() => {
    setAlertDialog(null);
  }, []);

  return {
    alertDialog,
    clearAlert,
    showAlert,
  };
}

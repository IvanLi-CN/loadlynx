import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

export function CalibrationDeviceViewPanel(input: {
  children: ReactNode;
  deviceProfileSource?: string;
  onReadDeviceProfile: () => void;
  onRequestReset: () => void;
  readPending: boolean;
  resetDisabled: boolean;
}) {
  const { t } = useTranslation();
  const {
    children,
    deviceProfileSource,
    onReadDeviceProfile,
    onRequestReset,
    readPending,
    resetDisabled,
  } = input;

  return (
    <div className="ll-panel bg-base-100 shadow-xl border border-base-200 mt-4">
      <div className="ll-panel-body gap-4">
        <div className="flex items-start justify-between gap-3">
          <h3 className="ll-panel-title flex flex-col items-start leading-tight">
            <span>{t("calibration.deviceData")}</span>
            <span className="text-sm font-normal text-base-content/60">
              Hardware
            </span>
          </h3>
          <div className="flex items-center gap-2">
            <div className="ll-badge ll-badge-info whitespace-nowrap">
              {t("calibration.readDevice")}
            </div>
            <div className="ll-badge ll-badge-warning whitespace-nowrap">
              {t("calibration.writeDevice")}
            </div>
          </div>
        </div>

        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="ll-button ll-button-sm ll-button-outline"
            onClick={onReadDeviceProfile}
            disabled={readPending}
          >
            Read
          </button>
          <button
            type="button"
            className="ll-button ll-button-sm ll-button-danger"
            onClick={onRequestReset}
            disabled={resetDisabled}
          >
            Reset
          </button>
        </div>

        <div className="divider my-0"></div>

        <h4 className="font-bold text-sm">
          {deviceProfileSource === "factory-default"
            ? "Device defaults"
            : "Device profile"}
        </h4>

        {children}
      </div>
    </div>
  );
}

import { useTranslation } from "react-i18next";

export function CalibrationViewTabs(input: {
  activeView: "draft" | "device";
  className?: string;
  onSelectView: (view: "draft" | "device") => void;
}) {
  const { t } = useTranslation();
  const { activeView, className, onSelectView } = input;

  return (
    <div role="tablist" className={className ?? "ll-tabs"}>
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeView === "draft" ? "ll-tab-active" : ""}`}
        aria-selected={activeView === "draft"}
        tabIndex={activeView === "draft" ? 0 : -1}
        onClick={() => onSelectView("draft")}
      >
        {t("calibration.localDraft")}
      </button>
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeView === "device" ? "ll-tab-active" : ""}`}
        aria-selected={activeView === "device"}
        tabIndex={activeView === "device" ? 0 : -1}
        onClick={() => onSelectView("device")}
      >
        {t("calibration.deviceData")}
      </button>
    </div>
  );
}

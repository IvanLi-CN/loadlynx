import { useTranslation } from "react-i18next";
import type { CalibrationTab } from "./shared.ts";

export function CalibrationTabList(input: {
  activeTab: CalibrationTab;
  onSelectTab: (tab: CalibrationTab) => void;
}) {
  const { t } = useTranslation();
  const { activeTab, onSelectTab } = input;

  return (
    <div role="tablist" className="ll-tabs mt-4">
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeTab === "voltage" ? "ll-tab-active" : ""}`}
        aria-selected={activeTab === "voltage"}
        tabIndex={activeTab === "voltage" ? 0 : -1}
        onClick={() => onSelectTab("voltage")}
      >
        {t("calibration.voltage")}
      </button>
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeTab === "current_ch1" ? "ll-tab-active" : ""}`}
        aria-selected={activeTab === "current_ch1"}
        tabIndex={activeTab === "current_ch1" ? 0 : -1}
        onClick={() => onSelectTab("current_ch1")}
      >
        {t("calibration.currentCh1")}
      </button>
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeTab === "current_ch2" ? "ll-tab-active" : ""}`}
        aria-selected={activeTab === "current_ch2"}
        tabIndex={activeTab === "current_ch2" ? 0 : -1}
        onClick={() => onSelectTab("current_ch2")}
      >
        {t("calibration.currentCh2")}
      </button>
    </div>
  );
}

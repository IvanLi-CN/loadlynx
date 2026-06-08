import type { CalibrationTab } from "./shared.ts";

export function CalibrationTabList(input: {
  activeTab: CalibrationTab;
  onSelectTab: (tab: CalibrationTab) => void;
}) {
  const { activeTab, onSelectTab } = input;

  return (
    <div role="tablist" className="ll-tabs mt-4">
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeTab === "voltage" ? "ll-tab-active" : ""}`}
        onClick={() => onSelectTab("voltage")}
      >
        电压
      </button>
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeTab === "current_ch1" ? "ll-tab-active" : ""}`}
        onClick={() => onSelectTab("current_ch1")}
      >
        电流通道1
      </button>
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeTab === "current_ch2" ? "ll-tab-active" : ""}`}
        onClick={() => onSelectTab("current_ch2")}
      >
        电流通道2
      </button>
    </div>
  );
}

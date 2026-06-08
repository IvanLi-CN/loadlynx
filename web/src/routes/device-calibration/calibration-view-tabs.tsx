export function CalibrationViewTabs(input: {
  activeView: "draft" | "device";
  className?: string;
  onSelectView: (view: "draft" | "device") => void;
}) {
  const { activeView, className, onSelectView } = input;

  return (
    <div role="tablist" className={className ?? "ll-tabs"}>
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeView === "draft" ? "ll-tab-active" : ""}`}
        onClick={() => onSelectView("draft")}
      >
        本地草稿
      </button>
      <button
        type="button"
        role="tab"
        className={`ll-tab ${activeView === "device" ? "ll-tab-active" : ""}`}
        onClick={() => onSelectView("device")}
      >
        设备数据
      </button>
    </div>
  );
}

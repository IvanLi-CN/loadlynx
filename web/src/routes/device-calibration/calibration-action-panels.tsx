import type { ReactNode } from "react";

export function CalibrationDraftActionsPanel(input: {
  disableApplyPreview: boolean;
  disableExport: boolean;
  disableResetDraft: boolean;
  exportTitle?: string;
  extraActions?: ReactNode;
  importInputId: string;
  onApplyPreview: () => void;
  onExportDraft: () => void;
  onImportDraftFile: (file: File | null) => Promise<void>;
  onResetDraft: () => void;
}) {
  const {
    disableApplyPreview,
    disableExport,
    disableResetDraft,
    exportTitle,
    extraActions,
    importInputId,
    onApplyPreview,
    onExportDraft,
    onImportDraftFile,
    onResetDraft,
  } = input;

  return (
    <div className="ll-panel bg-base-200/40 border border-base-200">
      <div className="ll-panel-body py-4 gap-3">
        <div className="flex items-start justify-between gap-3">
          <h4 className="font-bold text-sm">仅本地（不读写设备）</h4>
          <div className="ll-badge ll-badge-neutral whitespace-nowrap shrink-0">
            不读写设备
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="ll-button ll-button-sm ll-button-outline"
            onClick={onApplyPreview}
            disabled={disableApplyPreview}
          >
            Apply Preview
          </button>
          <button
            type="button"
            className="ll-button ll-button-sm ll-button-outline"
            onClick={onResetDraft}
            disabled={disableResetDraft}
          >
            Reset Draft
          </button>
          {extraActions}
          <button
            type="button"
            className="ll-button ll-button-sm ll-button-outline"
            onClick={onExportDraft}
            disabled={disableExport}
            title={exportTitle}
          >
            Export
          </button>
          <label
            htmlFor={importInputId}
            className="ll-button ll-button-sm ll-button-outline"
          >
            Import
          </label>
          <input
            id={importInputId}
            type="file"
            accept="application/json"
            className="hidden"
            onChange={(event) => {
              const file = event.currentTarget.files?.[0] ?? null;
              void onImportDraftFile(file);
              event.currentTarget.value = "";
            }}
          />
        </div>
      </div>
    </div>
  );
}

export function CalibrationHardwareIoPanel(input: {
  actionButtons: ReactNode;
  children?: ReactNode;
  disableReadDeviceToDraft: boolean;
  onReadDeviceToDraft: () => void;
}) {
  const {
    actionButtons,
    children,
    disableReadDeviceToDraft,
    onReadDeviceToDraft,
  } = input;

  return (
    <div className="ll-panel bg-base-200/40 border border-base-200">
      <div className="ll-panel-body py-4 gap-3">
        <div className="flex items-start justify-between gap-3">
          <h4 className="font-bold text-sm">硬件 I/O</h4>
          <div className="flex items-center gap-2">
            <div className="ll-badge ll-badge-info whitespace-nowrap">
              读设备
            </div>
            <div className="ll-badge ll-badge-warning whitespace-nowrap">
              写设备
            </div>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="ll-button ll-button-sm ll-button-outline"
            onClick={onReadDeviceToDraft}
            disabled={disableReadDeviceToDraft}
          >
            Read Device → Draft
          </button>
          {actionButtons}
        </div>
        {children}
      </div>
    </div>
  );
}

export function CalibrationDeviceWriteButtons(input: {
  applyPending: boolean;
  commitPending: boolean;
  disableApply: boolean;
  disableCommit: boolean;
  onApply: () => void;
  onCommit: () => void;
}) {
  const {
    applyPending,
    commitPending,
    disableApply,
    disableCommit,
    onApply,
    onCommit,
  } = input;

  return (
    <>
      <button
        type="button"
        className="ll-button ll-button-sm ll-button-outline"
        onClick={onApply}
        disabled={disableApply || applyPending}
      >
        Apply
      </button>
      <button
        type="button"
        className="ll-button ll-button-sm ll-button-secondary"
        onClick={onCommit}
        disabled={disableCommit || commitPending}
      >
        Commit
      </button>
    </>
  );
}

export type PresetsPanelProps = {
  presets: Array<{
    id: number;
    label: string;
    active: boolean;
    disabled?: boolean;
  }>;
  selectedPresetId: number | null;
  onPresetSelect: (id: number) => void;
  onApply: () => void;
  onSave: () => void;
  applyDisabled: boolean;
  saveDisabled: boolean;
  applying?: boolean;
  saving?: boolean;
};

export function PresetsPanel({
  presets,
  selectedPresetId,
  onPresetSelect,
  onApply,
  onSave,
  applyDisabled,
  saveDisabled,
  applying = false,
  saving = false,
}: PresetsPanelProps) {
  return (
    <section aria-label="Presets" className="instrument-card p-5">
      <div className="instrument-label">Presets</div>

      <div className="mt-4 grid grid-cols-4 gap-2">
        {presets.map((preset) => {
          const isSelected = selectedPresetId === preset.id;
          const isDisabled = Boolean(preset.disabled);
          return (
            <button
              key={preset.id}
              type="button"
              disabled={isDisabled}
              className={[
                "h-9 rounded-lg border text-xs font-semibold tracking-wide transition-colors",
                isSelected
                  ? "border-[rgba(111,234,249,0.28)] bg-[rgba(111,234,249,0.10)] text-slate-100"
                  : preset.active
                    ? "border-slate-200/20 bg-white/5 text-slate-100/90"
                    : "border-slate-400/10 bg-black/20 text-slate-200/70",
                isDisabled ? "opacity-35 cursor-not-allowed" : "cursor-pointer",
              ].join(" ")}
              onClick={() => onPresetSelect(preset.id)}
            >
              {preset.label}
            </button>
          );
        })}
      </div>

      <div className="mt-4 flex items-center justify-between gap-3">
        <button
          type="button"
          className="h-9 flex-1 rounded-lg border border-[rgba(111,234,249,0.28)] bg-[rgba(111,234,249,0.10)] px-3 text-xs font-semibold tracking-[0.14em] text-slate-100 uppercase disabled:opacity-50"
          disabled={applyDisabled}
          onClick={onApply}
        >
          {applying ? "Applying…" : "Apply Preset"}
        </button>
        <button
          type="button"
          className="h-9 flex-1 rounded-lg border border-[rgba(253,212,94,0.28)] bg-[rgba(253,212,94,0.10)] px-3 text-xs font-semibold tracking-[0.14em] text-slate-100 uppercase disabled:opacity-50"
          disabled={saveDisabled}
          onClick={onSave}
        >
          {saving ? "Saving…" : "Save Draft"}
        </button>
      </div>

      <div className="mt-3 text-[11px] text-slate-200/55">
        Apply preset turns output off.
      </div>
    </section>
  );
}

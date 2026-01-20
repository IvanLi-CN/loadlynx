export type PdSummaryPanelProps = {
  visible: boolean;
  contractText: string | null;
  ppsText: string | null;
  savedText: string | null;
};

export function PdSummaryPanel({
  visible,
  contractText,
  ppsText,
  savedText,
}: PdSummaryPanelProps) {
  if (!visible) {
    return null;
  }

  return (
    <section aria-label="USB-PD summary" className="instrument-card p-5">
      <div className="instrument-label">USB‑PD Summary</div>
      <div className="mt-3 space-y-1 text-[12px] text-slate-200/65">
        <div className="font-mono">{contractText ?? "Contract: —"}</div>
        <div className="font-mono">{ppsText ?? "PPS: —"}</div>
        <div className="font-mono">{savedText ?? "Saved: —"}</div>
      </div>
    </section>
  );
}

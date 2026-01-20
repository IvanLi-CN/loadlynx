import { Link } from "@tanstack/react-router";

export type DiagnosticsPanelProps = {
  analogLinkText: string;
  loopText: string;
  lastApplyText: string;
  to?: { to: string; params: Record<string, string> };
};

export function DiagnosticsPanel({
  analogLinkText,
  loopText,
  lastApplyText,
  to,
}: DiagnosticsPanelProps) {
  return (
    <section aria-label="Diagnostics" className="instrument-card p-5">
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="instrument-label">Diagnostics</div>
          <div className="mt-3 space-y-1 text-[12px] text-slate-200/65">
            <div className="font-mono">{analogLinkText}</div>
            <div className="font-mono">{loopText}</div>
            <div className="font-mono">{lastApplyText}</div>
          </div>
        </div>

        {to ? (
          <Link
            to={to.to as never}
            params={to.params as never}
            className="rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[11px] font-semibold text-slate-200/60 hover:text-slate-100"
          >
            Open
          </Link>
        ) : null}
      </div>
    </section>
  );
}


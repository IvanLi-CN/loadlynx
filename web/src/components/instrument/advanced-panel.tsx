import type { ReactNode } from "react";

export type AdvancedPanelProps = {
  summary: string;
  collapsed: boolean;
  onToggle: (collapsed: boolean) => void;
  children?: ReactNode;
};

export function AdvancedPanel({
  summary,
  collapsed,
  onToggle,
  children,
}: AdvancedPanelProps) {
  return (
    <section aria-label="Advanced" className="instrument-card p-5">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-4 text-left"
        aria-expanded={!collapsed}
        onClick={() => onToggle(!collapsed)}
      >
        <div>
          <div className="instrument-label">Advanced</div>
          <div className="mt-2 text-[12px] text-slate-200/60">{summary}</div>
        </div>
        <div className="text-[12px] font-semibold text-slate-200/55">
          {collapsed ? "Expand" : "Collapse"}
        </div>
      </button>

      {!collapsed ? (
        <div className="mt-4 border-t border-slate-400/10 pt-4">{children}</div>
      ) : null}
    </section>
  );
}


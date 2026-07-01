import { LoaderCircle } from "lucide-react";

export type RoutePendingViewProps = {
  title?: string;
  description?: string;
  compact?: boolean;
};

export function RoutePendingView({
  title = "正在打开页面",
  description = "正在加载目标视图",
  compact = false,
}: RoutePendingViewProps) {
  return (
    <section
      role="status"
      aria-live="polite"
      aria-busy="true"
      className={[
        "ll-panel w-full overflow-hidden",
        compact ? "min-h-64" : "min-h-[calc(100dvh-13rem)]",
      ].join(" ")}
    >
      <div className="h-1 w-full overflow-hidden bg-primary/10">
        <div className="h-full w-1/2 animate-[ll-route-pending_900ms_var(--ease-console)_infinite] bg-primary shadow-[0_0_18px_oklch(0.82_0.17_210_/_0.52)] motion-reduce:animate-none" />
      </div>

      <div className="grid min-h-64 content-center gap-8 p-4 sm:p-6">
        <div className="flex flex-wrap items-center gap-4">
          <span className="grid size-11 shrink-0 place-items-center rounded-lg border border-primary/45 bg-primary/10 text-primary shadow-[0_0_18px_oklch(0.82_0.17_210_/_0.16)]">
            <LoaderCircle
              aria-hidden="true"
              className="size-5 animate-spin motion-reduce:animate-none"
            />
          </span>
          <div className="min-w-0">
            <h1 className="text-lg font-bold text-base-content">{title}</h1>
            <p className="mt-1 font-mono text-sm text-base-content/70">
              {description}
            </p>
          </div>
        </div>

        <div className="grid gap-4 md:grid-cols-2">
          <div className="rounded-lg border border-primary/12 bg-base-100/45 p-4">
            <div className="h-3 w-28 rounded-full bg-primary/30" />
            <div className="mt-6 grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <div className="h-2 w-20 rounded-full bg-base-content/18" />
                <div className="h-7 w-24 rounded bg-base-content/12" />
              </div>
              <div className="space-y-2">
                <div className="h-2 w-24 rounded-full bg-base-content/18" />
                <div className="h-7 w-20 rounded bg-base-content/12" />
              </div>
            </div>
          </div>

          <div className="rounded-lg border border-primary/12 bg-base-100/45 p-4">
            <div className="h-3 w-40 rounded-full bg-base-content/18" />
            <div className="mt-6 space-y-3">
              <div className="h-3 w-full rounded-full bg-base-content/12" />
              <div className="h-3 w-4/5 rounded-full bg-base-content/12" />
              <div className="h-3 w-3/5 rounded-full bg-base-content/12" />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

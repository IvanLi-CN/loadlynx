import { useEffect, useMemo, useState } from "react";

const BREAKPOINT_MD = 768;
const BREAKPOINT_LG = 1024;

type BreakpointMode = "Small" | "Medium" | "Large";

function computeMode(width: number): BreakpointMode {
  if (width < BREAKPOINT_MD) return "Small";
  if (width < BREAKPOINT_LG) return "Medium";
  return "Large";
}

export function BreakpointRulerOverlay() {
  const [width, setWidth] = useState(() => window.innerWidth);

  useEffect(() => {
    const onResize = () => setWidth(window.innerWidth);
    window.addEventListener("resize", onResize, { passive: true });
    return () => window.removeEventListener("resize", onResize);
  }, []);

  const mode = useMemo(() => computeMode(width), [width]);
  const mdPosPct = useMemo(
    () => Math.min(100, (BREAKPOINT_MD / width) * 100),
    [width],
  );
  const lgPosPct = useMemo(
    () => Math.min(100, (BREAKPOINT_LG / width) * 100),
    [width],
  );

  return (
    <div className="pointer-events-none fixed inset-0 z-[10000]">
      <div className="fixed left-2 top-2 rounded-md border border-base-content/20 bg-base-100/90 px-3 py-2 text-sm text-base-content shadow-sm backdrop-blur">
        <div className="font-mono">
          {width}px · <span className="font-semibold">{mode}</span>
        </div>
        <div className="mt-1 flex gap-2 font-mono text-xs text-base-content/70">
          <span>md: {BREAKPOINT_MD}</span>
          <span>lg: {BREAKPOINT_LG}</span>
        </div>
        <div className="relative mt-2 h-2 w-60 rounded bg-base-content/10">
          <div
            aria-hidden="true"
            className="absolute inset-y-0 w-px bg-info/80"
            style={{ left: `${mdPosPct}%` }}
          />
          <div
            aria-hidden="true"
            className="absolute inset-y-0 w-px bg-warning/80"
            style={{ left: `${lgPosPct}%` }}
          />
        </div>
      </div>

      <div
        aria-hidden="true"
        className="fixed bottom-0 top-0 w-px bg-info/70"
        style={{ left: BREAKPOINT_MD }}
      />
      <div
        aria-hidden="true"
        className="fixed bottom-0 top-0 w-px bg-warning/70"
        style={{ left: BREAKPOINT_LG }}
      />

      <div
        aria-hidden="true"
        className="fixed top-2 -translate-x-1/2 rounded bg-info/80 px-1 py-0.5 font-mono text-[10px] leading-none text-info-content"
        style={{ left: BREAKPOINT_MD }}
      >
        {BREAKPOINT_MD}
      </div>
      <div
        aria-hidden="true"
        className="fixed top-2 -translate-x-1/2 rounded bg-warning/80 px-1 py-0.5 font-mono text-[10px] leading-none text-warning-content"
        style={{ left: BREAKPOINT_LG }}
      >
        {BREAKPOINT_LG}
      </div>
    </div>
  );
}

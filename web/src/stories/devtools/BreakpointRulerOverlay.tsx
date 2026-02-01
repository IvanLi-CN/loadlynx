import { useEffect, useMemo, useRef, useState } from "react";

const BREAKPOINT_MD = 768;
const BREAKPOINT_LG = 1024;

type BreakpointMode = "Small" | "Medium" | "Large";

type OverlayPosition = { x: number; y: number };

function computeMode(width: number): BreakpointMode {
  if (width < BREAKPOINT_MD) return "Small";
  if (width < BREAKPOINT_LG) return "Medium";
  return "Large";
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

const POS_STORAGE_KEY = "loadlynx_storybook_breakpoint_overlay_pos_v1";

export function BreakpointRulerOverlay() {
  const [width, setWidth] = useState(() => window.innerWidth);
  const cardRef = useRef<HTMLDivElement | null>(null);
  const dragStateRef = useRef<{
    active: boolean;
    pointerId: number;
    offsetX: number;
    offsetY: number;
  } | null>(null);
  const [position, setPosition] = useState<OverlayPosition>(() => {
    try {
      const raw = window.localStorage.getItem(POS_STORAGE_KEY);
      if (!raw) throw new Error("missing");
      const parsed = JSON.parse(raw) as unknown;
      if (
        parsed &&
        typeof parsed === "object" &&
        typeof (parsed as { x?: unknown }).x === "number" &&
        typeof (parsed as { y?: unknown }).y === "number"
      ) {
        return {
          x: (parsed as { x: number }).x,
          y: (parsed as { y: number }).y,
        };
      }
      throw new Error("invalid");
    } catch {
      // Default: avoid covering the small-screen hamburger button.
      return width < BREAKPOINT_MD ? { x: 64, y: 8 } : { x: 8, y: 8 };
    }
  });

  useEffect(() => {
    const onResize = () => setWidth(window.innerWidth);
    window.addEventListener("resize", onResize, { passive: true });
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useEffect(() => {
    // Keep the card on-screen when the viewport changes.
    const card = cardRef.current;
    if (!card) return;
    const rect = card.getBoundingClientRect();
    const maxX = Math.max(0, window.innerWidth - rect.width);
    const maxY = Math.max(0, window.innerHeight - rect.height);
    setPosition((value) => ({
      x: clamp(value.x, 0, maxX),
      y: clamp(value.y, 0, maxY),
    }));
  }, []);

  useEffect(() => {
    const onPointerMove = (event: PointerEvent) => {
      const dragState = dragStateRef.current;
      if (!dragState || !dragState.active) return;
      if (event.pointerId !== dragState.pointerId) return;

      const card = cardRef.current;
      if (!card) return;

      const rect = card.getBoundingClientRect();
      const maxX = Math.max(0, window.innerWidth - rect.width);
      const maxY = Math.max(0, window.innerHeight - rect.height);
      const nextX = clamp(event.clientX - dragState.offsetX, 0, maxX);
      const nextY = clamp(event.clientY - dragState.offsetY, 0, maxY);
      setPosition({ x: nextX, y: nextY });
    };

    const onPointerUp = (event: PointerEvent) => {
      const dragState = dragStateRef.current;
      if (!dragState || !dragState.active) return;
      if (event.pointerId !== dragState.pointerId) return;

      dragStateRef.current = null;
      try {
        window.localStorage.setItem(POS_STORAGE_KEY, JSON.stringify(position));
      } catch {
        // ignore
      }
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("pointercancel", onPointerUp);
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("pointercancel", onPointerUp);
    };
  }, [position]);

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
    <div
      ref={cardRef}
      className="fixed z-[10001] rounded-md border border-base-content/20 bg-base-100/90 px-3 py-2 text-left text-sm text-base-content shadow-sm backdrop-blur"
      style={{ left: position.x, top: position.y }}
    >
      <div className="flex w-full items-start gap-3 text-left">
        <button
          type="button"
          className="flex-1 cursor-move text-left"
          onPointerDown={(event) => {
            if (event.button !== 0) return;
            const card = cardRef.current;
            if (!card) return;
            const rect = card.getBoundingClientRect();
            dragStateRef.current = {
              active: true,
              pointerId: event.pointerId,
              offsetX: event.clientX - rect.x,
              offsetY: event.clientY - rect.y,
            };
            event.currentTarget.setPointerCapture(event.pointerId);
          }}
          style={{ touchAction: "none" }}
          aria-label="Drag breakpoint ruler"
          title="Drag to reposition"
        >
          <div className="font-mono select-none">
            {width}px · <span className="font-semibold">{mode}</span>
          </div>
        </button>
      </div>
      <div className="mt-1 flex gap-2 font-mono text-xs text-base-content/70 select-none">
        <span>md: {BREAKPOINT_MD}</span>
        <span>lg: {BREAKPOINT_LG}</span>
      </div>
      <div className="relative mt-2 h-2 w-52 rounded bg-base-content/10 sm:w-60">
        <div
          aria-hidden="true"
          className="absolute inset-y-0 w-0.5 bg-info/80"
          style={{ left: `${mdPosPct}%` }}
        />
        <div
          aria-hidden="true"
          className="absolute inset-y-0 w-0.5 bg-warning/80"
          style={{ left: `${lgPosPct}%` }}
        />
      </div>
    </div>
  );
}

type SparklineProps = {
  points: number[];
  min: number;
  max: number;
  height?: number;
  className?: string;
  tone?: "cyan" | "green" | "amber" | "neutral";
  variant?: "line" | "area";
  showBackground?: boolean;
};

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}

export function Sparkline({
  points,
  min,
  max,
  height = 44,
  className,
  tone = "cyan",
  variant = "line",
  showBackground = false,
}: SparklineProps) {
  const width = 240;
  const hasData =
    points.length >= 2 && Number.isFinite(min) && Number.isFinite(max);
  const range = max - min;
  const safeRange = range > 0 ? range : 1;

  const path = (() => {
    if (!hasData) {
      return "";
    }
    const step = width / Math.max(1, points.length - 1);
    return points
      .map((v, idx) => {
        const x = idx * step;
        const t = clamp01((v - min) / safeRange);
        const y = (1 - t) * (height - 6) + 3;
        return `${idx === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" ");
  })();

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      width="100%"
      height={height}
      className={className}
      role="img"
      aria-label="Trend"
      style={{
        color:
          tone === "amber"
            ? "#fdd45e"
            : tone === "green"
              ? "#83ffd2"
              : tone === "neutral"
                ? "rgba(149,168,163,0.78)"
                : "#6feaf9",
      }}
    >
      {showBackground ? (
        <rect
          x={0}
          y={0}
          width={width}
          height={height}
          rx={8}
          fill="rgba(0,0,0,0.18)"
          stroke="rgba(148,163,184,0.10)"
          strokeWidth={1}
        />
      ) : null}
      {hasData ? (
        <>
          <path
            d={path}
            fill="none"
            stroke="currentColor"
            strokeWidth={2.5}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          {variant === "area" ? (
            <path
              d={`${path} L${width},${height} L0,${height} Z`}
              fill="currentColor"
              opacity={0.12}
            />
          ) : null}
        </>
      ) : (
        <text
          x={width / 2}
          y={height / 2}
          textAnchor="middle"
          dominantBaseline="middle"
          fill="rgba(226,232,240,0.40)"
          fontSize={10}
        >
          No data
        </text>
      )}
    </svg>
  );
}

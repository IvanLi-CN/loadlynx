export function formatFixed(
  value: number | null | undefined,
  digits: number,
): string {
  if (value == null || !Number.isFinite(value)) {
    return "—";
  }
  return value.toFixed(digits);
}

export function formatWithUnit(
  value: number | null | undefined,
  digits: number,
  unit: string,
): string {
  return `${formatFixed(value, digits)} ${unit}`;
}

export function formatUptimeSeconds(
  seconds: number | null | undefined,
): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) {
    return "—";
  }

  const total = Math.floor(seconds);
  const hh = Math.floor(total / 3600);
  const mm = Math.floor((total % 3600) / 60);
  const ss = total % 60;
  return [hh, mm, ss].map((v) => String(v).padStart(2, "0")).join(":");
}

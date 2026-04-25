import type { PdFixedPdo, PdPpsPdo, PdView } from "./types.ts";

export function findFixedPdo(
  pd: PdView,
  pos: number | null | undefined,
): PdFixedPdo | null {
  if (pos == null) return null;
  return pd.fixed_pdos.find((entry) => entry.pos === pos) ?? null;
}

export function findPpsPdo(
  pd: PdView,
  pos: number | null | undefined,
): PdPpsPdo | null {
  if (pos == null) return null;
  return pd.pps_pdos.find((entry) => entry.pos === pos) ?? null;
}

export function findVisibleSavedFixedPdo(pd: PdView): PdFixedPdo | null {
  if (pd.saved.mode !== "fixed") return null;
  const byPos = findFixedPdo(pd, pd.saved.fixed_object_pos);
  if (byPos) return byPos;
  return pd.fixed_pdos.find((entry) => entry.mv === pd.saved.target_mv) ?? null;
}

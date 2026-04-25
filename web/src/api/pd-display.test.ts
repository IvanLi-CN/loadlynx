import { describe, expect, it } from "bun:test";
import { findVisibleSavedFixedPdo } from "./pd-display.ts";
import type { PdView } from "./types.ts";

const basePdView: PdView = {
  attached: true,
  contract_mv: 5000,
  contract_ma: 1200,
  fixed_pdos: [
    { pos: 1, mv: 5000, max_ma: 3000 },
    { pos: 2, mv: 9000, max_ma: 3000 },
    { pos: 3, mv: 12000, max_ma: 3000 },
  ],
  pps_pdos: [],
  saved: {
    mode: "fixed",
    fixed_object_pos: 2,
    pps_object_pos: 0,
    target_mv: 9000,
    pps_target_mv: 9000,
    i_req_ma: 1200,
  },
  apply: {
    pending: false,
    last: null,
  },
};

describe("api/pd-display", () => {
  it("returns the saved fixed PDO by object position when present", () => {
    expect(findVisibleSavedFixedPdo(basePdView)).toEqual(
      basePdView.fixed_pdos[1],
    );
  });

  it("falls back to target_mv for legacy fixed configs with object_pos=0", () => {
    const pd: PdView = {
      ...basePdView,
      saved: {
        ...basePdView.saved,
        fixed_object_pos: 0,
      },
    };

    expect(findVisibleSavedFixedPdo(pd)).toEqual(basePdView.fixed_pdos[1]);
  });

  it("keeps hidden saved fixed targets hidden when no live PDO matches", () => {
    const pd: PdView = {
      ...basePdView,
      saved: {
        ...basePdView.saved,
        fixed_object_pos: 8,
        target_mv: 28000,
      },
    };

    expect(findVisibleSavedFixedPdo(pd)).toBeNull();
  });
});

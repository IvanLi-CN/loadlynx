import { HttpApiError } from "./client-core.ts";
import { mockRequireControlReady } from "./client-mock-control.ts";
import {
  getOrCreateMockDevice,
  type MockDeviceState,
} from "./client-mock-state.ts";
import type { PdUpdateRequest, PdView } from "./types.ts";

function mockRequirePdSupported(state: MockDeviceState): PdView {
  if (!state.pd) {
    throw new HttpApiError({
      status: 404,
      code: "UNSUPPORTED_OPERATION",
      message: "USB-PD API not supported by this device",
      retryable: false,
      details: null,
    });
  }
  return state.pd;
}

function mockRequirePdReady(state: MockDeviceState): void {
  mockRequireControlReady(state);
}

export async function mockGetPd(baseUrl: string): Promise<PdView> {
  const state = getOrCreateMockDevice(baseUrl);
  mockRequirePdReady(state);
  return structuredClone(mockRequirePdSupported(state));
}

export async function mockUpdatePd(
  baseUrl: string,
  payload: PdUpdateRequest,
): Promise<PdView> {
  const state = getOrCreateMockDevice(baseUrl);
  mockRequirePdReady(state);

  const current = mockRequirePdSupported(state);
  const next = structuredClone(current);
  const nowMs = state.status.raw.uptime_ms ?? 0;

  function applyMockPdPolicy(view: PdView): void {
    if (!view.attached) {
      view.contract_mv = null;
      view.contract_ma = null;
      return;
    }

    const allowExtendedVoltage = view.allow_extended_voltage ?? true;
    if (!allowExtendedVoltage) {
      const safePdo =
        view.fixed_pdos.find((entry) => entry.mv === 5_000) ??
        view.fixed_pdos[0];
      const safeMaxMa = safePdo?.max_ma ?? view.saved.i_req_ma;
      view.contract_mv = 5_000;
      view.contract_ma = Math.min(view.saved.i_req_ma, safeMaxMa);
      return;
    }

    if (view.saved.mode === "fixed") {
      const pos = view.saved.fixed_object_pos;
      const pdo =
        view.fixed_pdos.find((entry) => entry.pos === pos) ??
        view.fixed_pdos[0];
      view.contract_mv = pdo?.mv ?? 5_000;
      view.contract_ma = view.saved.i_req_ma;
      return;
    }

    view.contract_mv = view.saved.target_mv;
    view.contract_ma = view.saved.i_req_ma;
  }

  if (!("mode" in payload)) {
    next.allow_extended_voltage = payload.allow_extended_voltage;
    applyMockPdPolicy(next);
    next.apply = {
      pending: false,
      last: { code: "ok", at_ms: nowMs },
    };
    state.pd = next;
    return structuredClone(next);
  }

  if (!current.attached) {
    throw new HttpApiError({
      status: 409,
      code: "NOT_ATTACHED",
      message: "PD is not attached",
      retryable: true,
      details: null,
    });
  }

  function limitViolation(message: string, details?: unknown): never {
    throw new HttpApiError({
      status: 422,
      code: "LIMIT_VIOLATION",
      message,
      retryable: false,
      details: details ?? null,
    });
  }

  const objectPos = payload.object_pos;
  if (payload.mode === "fixed") {
    const pdo = next.fixed_pdos.find((entry) => entry.pos === objectPos);
    if (!pdo) {
      limitViolation("Selected fixed PDO does not exist", {
        object_pos: objectPos,
      });
    }
    if (payload.i_req_ma < 0 || payload.i_req_ma > pdo.max_ma) {
      limitViolation("Ireq exceeds Imax for selected PDO", {
        i_req_ma: payload.i_req_ma,
        max_ma: pdo.max_ma,
      });
    }

    const cachedPpsTarget = next.saved.pps_target_mv ?? next.saved.target_mv;
    next.saved = {
      ...next.saved,
      mode: "fixed",
      fixed_object_pos: objectPos,
      target_mv: pdo.mv,
      pps_target_mv: cachedPpsTarget,
      i_req_ma: payload.i_req_ma,
    };
  } else {
    const apdo = next.pps_pdos.find((entry) => entry.pos === objectPos);
    if (!apdo) {
      limitViolation("Selected PPS APDO does not exist", {
        object_pos: objectPos,
      });
    }
    if (payload.target_mv < apdo.min_mv || payload.target_mv > apdo.max_mv) {
      limitViolation("Vreq is out of range for selected APDO", {
        target_mv: payload.target_mv,
        min_mv: apdo.min_mv,
        max_mv: apdo.max_mv,
      });
    }
    if (payload.i_req_ma < 0 || payload.i_req_ma > apdo.max_ma) {
      limitViolation("Ireq exceeds Imax for selected APDO", {
        i_req_ma: payload.i_req_ma,
        max_ma: apdo.max_ma,
      });
    }

    next.saved = {
      ...next.saved,
      mode: "pps",
      pps_object_pos: objectPos,
      target_mv: payload.target_mv,
      pps_target_mv: payload.target_mv,
      i_req_ma: payload.i_req_ma,
    };
  }

  if (payload.allow_extended_voltage != null) {
    next.allow_extended_voltage = payload.allow_extended_voltage;
  }
  applyMockPdPolicy(next);
  next.apply = {
    pending: false,
    last: { code: "ok", at_ms: nowMs },
  };
  state.pd = next;
  return structuredClone(next);
}

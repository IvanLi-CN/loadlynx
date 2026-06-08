import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import { isHttpApiError, postPd } from "../api/client.ts";
import {
  findFixedPdo,
  findPpsPdo,
  findVisibleSavedFixedPdo,
} from "../api/pd-display.ts";
import type {
  PdFixedUpdateRequest,
  PdPpsUpdateRequest,
  PdView,
} from "../api/types.ts";
import { PageContainer } from "../components/layout/page-container.tsx";
import { setDeviceQueryData } from "../devices/device-query-cache.ts";
import { DEVICE_QUERY_PARTS } from "../devices/device-query-key.ts";
import {
  getDevicePdQueryOptions,
  useDeviceIdentityByBaseUrl,
} from "../devices/hooks.ts";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import { requireDeviceBaseUrl } from "../lib/device-base-url.ts";
import {
  formatHttpApiErrorSummary,
  getNetworkErrorHint,
  isAnalogNotReadyError,
  isLinkUnavailableError,
  isUnsupportedOperationError,
} from "../lib/http-error.ts";
import { usePageVisibility } from "../lib/page-visibility.ts";

const PD_REFETCH_MS = 1200;
const RETRY_DELAY_MS = 500;
type PdApplyTab = "fixed" | "pps";
interface PdApplyRequest {
  tab: PdApplyTab;
}

function formatMilliAmps(ma: number | null): string {
  if (ma == null || !Number.isFinite(ma)) return "unknown";
  return `${ma} mA`;
}

function formatMilliVolts(mv: number | null): string {
  if (mv == null || !Number.isFinite(mv)) return "unknown";
  return `${(mv / 1000).toFixed(1)} V`;
}

function formatContract(pd: PdView | null | undefined): string {
  if (!pd) return "unknown";
  if (!pd.attached) return "detached";
  const v = formatMilliVolts(pd.contract_mv);
  const i = formatMilliAmps(pd.contract_ma);
  return `${v} @ ${i}`;
}

export function DevicePdRoute() {
  const { deviceId, device, baseUrl } = useDeviceContext();
  const queryClient = useQueryClient();
  const isPageVisible = usePageVisibility();
  const identityQuery = useDeviceIdentityByBaseUrl(deviceId, baseUrl);

  const pdQuery = useQuery<PdView, HttpApiError>(
    getDevicePdQueryOptions({
      deviceId,
      baseUrl,
      enabled: Boolean(baseUrl) && identityQuery.isSuccess,
      refetchInterval: isPageVisible ? PD_REFETCH_MS : false,
      retryDelay: RETRY_DELAY_MS,
    }),
  );

  const pd = pdQuery.data;

  const [tab, setTab] = useState<PdApplyTab>("fixed");

  const [selectedFixedPos, setSelectedFixedPos] = useState<number | null>(null);
  const [selectedPpsPos, setSelectedPpsPos] = useState<number | null>(null);

  const [fixedIReqMa, setFixedIReqMa] = useState<number>(0);
  const [ppsIReqMa, setPpsIReqMa] = useState<number>(0);
  const [ppsTargetMv, setPpsTargetMv] = useState<number>(0);
  const lastDraftSeedRef = useRef<string | null>(null);

  const pdDraftSeed = useMemo(() => {
    if (!pd) return null;
    return JSON.stringify({
      deviceId,
      baseUrl,
      saved: pd.saved,
      fixed_pdos: pd.fixed_pdos,
      pps_pdos: pd.pps_pdos,
    });
  }, [baseUrl, deviceId, pd]);

  useEffect(() => {
    if (!pd || pdDraftSeed == null) return;
    if (lastDraftSeedRef.current === pdDraftSeed) return;
    lastDraftSeedRef.current = pdDraftSeed;

    setTab(pd.saved.mode === "pps" ? "pps" : "fixed");
    setSelectedFixedPos(findVisibleSavedFixedPdo(pd)?.pos ?? null);
    setSelectedPpsPos(findPpsPdo(pd, pd.saved.pps_object_pos)?.pos ?? null);

    setFixedIReqMa(pd.saved.i_req_ma);
    setPpsIReqMa(pd.saved.i_req_ma);
    setPpsTargetMv(pd.saved.pps_target_mv ?? pd.saved.target_mv);
  }, [pd, pdDraftSeed]);

  const applyMutation = useMutation({
    mutationFn: async (payload: PdApplyRequest) => {
      const requiredBaseUrl = requireDeviceBaseUrl(baseUrl);

      if (!pd) {
        throw new Error("PD view is not available");
      }

      if (payload.tab === "fixed") {
        if (selectedFixedPos == null) {
          throw new Error("No Fixed PDO selected");
        }
        const request: PdFixedUpdateRequest = {
          mode: "fixed",
          object_pos: selectedFixedPos,
          i_req_ma: fixedIReqMa,
        };
        return postPd(requiredBaseUrl, request);
      }

      if (selectedPpsPos == null) {
        throw new Error("No PPS APDO selected");
      }

      const request: PdPpsUpdateRequest = {
        mode: "pps",
        object_pos: selectedPpsPos,
        target_mv: ppsTargetMv,
        i_req_ma: ppsIReqMa,
      };
      return postPd(requiredBaseUrl, request);
    },
    onSuccess: (next) => {
      setDeviceQueryData(
        queryClient,
        deviceId,
        baseUrl,
        DEVICE_QUERY_PARTS.pd,
        next,
      );
    },
  });

  const isUnsupported = (() => {
    const err = pdQuery.error;
    if (!err || !isHttpApiError(err)) return false;
    return isUnsupportedOperationError(err);
  })();

  const topError = (() => {
    const err = pdQuery.error;
    if (!err || !isHttpApiError(err) || isUnsupported) return null;

    const summary = formatHttpApiErrorSummary(err);

    if (err.status === 0 && err.code === "NETWORK_ERROR") {
      return {
        summary,
        hint: getNetworkErrorHint(baseUrl),
        kind: "error",
      } as const;
    }

    if (isLinkUnavailableError(err)) {
      return {
        summary,
        hint: "UART link is down / PD status unavailable — try again later.",
        kind: "warning",
      } as const;
    }

    if (isAnalogNotReadyError(err)) {
      return { summary, hint: null, kind: "warning" } as const;
    }

    return { summary, hint: null, kind: "error" } as const;
  })();

  const activeContractText = formatContract(pd);

  const selectedFixed = pd ? findFixedPdo(pd, selectedFixedPos) : null;
  const selectedPps = pd ? findPpsPdo(pd, selectedPpsPos) : null;
  const visibleSavedFixed = pd ? findVisibleSavedFixedPdo(pd) : null;

  const fixedValidation = useMemo(() => {
    if (!pd) return { ok: false, reason: "Loading..." } as const;
    if (!pd.attached) return { ok: false, reason: "PD not attached" } as const;
    if (pd.fixed_pdos.length === 0) {
      return { ok: false, reason: "No fixed PDOs." } as const;
    }
    if (!selectedFixed) {
      return { ok: false, reason: "Select a Fixed PDO" } as const;
    }
    if (!Number.isFinite(fixedIReqMa)) {
      return { ok: false, reason: "Invalid Ireq" } as const;
    }
    if (fixedIReqMa < 0 || fixedIReqMa > selectedFixed.max_ma) {
      return {
        ok: false,
        reason: `Ireq must be within 0..${selectedFixed.max_ma} mA`,
      } as const;
    }
    return { ok: true, reason: null } as const;
  }, [fixedIReqMa, pd, selectedFixed]);

  const ppsValidation = useMemo(() => {
    if (!pd) return { ok: false, reason: "Loading..." } as const;
    if (!pd.attached) return { ok: false, reason: "PD not attached" } as const;
    if (!selectedPps) {
      return { ok: false, reason: "Selected APDO not found" } as const;
    }
    if (!Number.isFinite(ppsTargetMv)) {
      return { ok: false, reason: "Invalid Vreq" } as const;
    }
    if (ppsTargetMv < selectedPps.min_mv || ppsTargetMv > selectedPps.max_mv) {
      return {
        ok: false,
        reason: `Vreq must be within ${selectedPps.min_mv}..${selectedPps.max_mv} mV`,
      } as const;
    }
    if (!Number.isFinite(ppsIReqMa)) {
      return { ok: false, reason: "Invalid Ireq" } as const;
    }
    if (ppsIReqMa < 0 || ppsIReqMa > selectedPps.max_ma) {
      return {
        ok: false,
        reason: `Ireq must be within 0..${selectedPps.max_ma} mA`,
      } as const;
    }
    return { ok: true, reason: null } as const;
  }, [pd, ppsIReqMa, ppsTargetMv, selectedPps]);

  const applyDisabled =
    applyMutation.isPending ||
    (tab === "fixed" ? !fixedValidation.ok : !ppsValidation.ok);

  const validationHint =
    tab === "fixed" ? fixedValidation.reason : ppsValidation.reason;

  const applyError = (() => {
    if (!applyMutation.isError) return null;
    const err = applyMutation.error;
    if (!err || !isHttpApiError(err)) return null;
    const code = err.code ?? "HTTP_ERROR";
    const summary = `Apply failed: ${code} — ${err.message}`;
    return { summary, details: err.details } as const;
  })();

  if (isUnsupported) {
    return (
      <PageContainer className="flex flex-col gap-6 font-mono tabular-nums">
        <header className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-bold">USB‑PD Settings</h2>
            <p className="mt-1 text-sm text-base-content/70">
              Device:{" "}
              <strong className="font-medium text-base-content">
                {device.name}
              </strong>
            </p>
          </div>
          <Link
            to="/$deviceId/status"
            params={{ deviceId }}
            className="ll-button ll-button-sm ll-button-outline"
          >
            Back
          </Link>
        </header>

        <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
          <div className="ll-panel-body p-8">
            <div className="ll-badge ll-badge-warning ll-badge-outline mb-4">
              UNSUPPORTED
            </div>
            <h3 className="text-xl font-bold mb-2">
              USB‑PD HTTP API not available
            </h3>
            <p className="text-sm text-base-content/70">
              This firmware does not expose <code>/api/v1/pd</code>. Upgrade the
              device firmware, or implement the endpoint per{" "}
              <code>docs/interfaces/network-http-api.md</code>.
            </p>
          </div>
        </div>
      </PageContainer>
    );
  }

  return (
    <PageContainer className="flex flex-col gap-6 font-mono tabular-nums">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-bold">USB‑PD Settings</h2>
          <p className="mt-1 text-sm text-base-content/70">
            Device:{" "}
            <strong className="font-medium text-base-content">
              {device.name}
            </strong>
            {identityQuery.data ? (
              <>
                {" "}
                · IP:{" "}
                <code className="font-mono bg-base-200 px-1 rounded text-xs">
                  {identityQuery.data.network.ip}
                </code>
              </>
            ) : null}
          </p>
        </div>
        <Link
          to="/$deviceId/status"
          params={{ deviceId }}
          className="ll-button ll-button-sm ll-button-outline"
        >
          Back
        </Link>
      </header>

      {topError ? (
        <div
          className={[
            "ll-alert shadow-sm rounded-lg text-sm",
            topError.kind === "warning" ? "ll-alert-warning" : "ll-alert-error",
          ].join(" ")}
        >
          <span className="font-bold">Error: {topError.summary}</span>
          {topError.hint && (
            <span className="text-xs opacity-80 block">{topError.hint}</span>
          )}
        </div>
      ) : null}

      <section className="rounded-lg bg-base-200/10 overflow-hidden">
        <div className="p-6 border-b border-base-content/10">
          <div className="flex flex-wrap items-center gap-3">
            <div className="text-sm font-semibold">Status</div>
            <div
              className={[
                "ll-badge",
                pd?.attached ? "ll-badge-success" : "ll-badge-ghost",
              ].join(" ")}
            >
              {pd?.attached ? "ATTACHED" : "DETACHED"}
            </div>
            <div className="text-xs text-base-content/60">Contract</div>
            <div className="text-sm">{activeContractText}</div>
            <div className="ml-auto text-xs text-base-content/60">
              Updated:{" "}
              {pdQuery.dataUpdatedAt
                ? new Date(pdQuery.dataUpdatedAt).toLocaleTimeString()
                : "..."}
            </div>
          </div>
        </div>

        <div className="p-6 grid gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <div className="text-xs text-base-content/60">Mode</div>
            <div className="ll-join">
              <button
                type="button"
                className={[
                  "ll-button ll-button-sm ll-join-item",
                  tab === "fixed"
                    ? "ll-button-primary text-info-content font-semibold"
                    : "ll-button-ghost border border-base-content/20 text-base-content/70",
                ].join(" ")}
                onClick={() => setTab("fixed")}
              >
                Fixed
              </button>
              <button
                type="button"
                className={[
                  "ll-button ll-button-sm ll-join-item",
                  tab === "pps"
                    ? "ll-button-primary text-info-content font-semibold"
                    : "ll-button-ghost border border-base-content/20 text-base-content/70",
                ].join(" ")}
                onClick={() => setTab("pps")}
              >
                PPS
              </button>
            </div>
          </div>

          {pd?.allow_extended_voltage === false ? (
            <div className="ll-alert ll-alert-info shadow-sm text-xs sm:text-sm">
              <span>
                Safe5V only (allow_extended_voltage=false) — Apply will save the
                profile, but the active contract stays at 5V until extended
                voltage is enabled on the device dashboard.
              </span>
            </div>
          ) : null}

          {applyMutation.isSuccess ? (
            <div className="ll-alert ll-alert-success shadow-sm text-xs sm:text-sm">
              <span>
                {applyMutation.data?.allow_extended_voltage === false
                  ? "Saved (Safe5V only)."
                  : "Apply succeeded."}
              </span>
            </div>
          ) : null}

          {applyError ? (
            <div className="ll-alert ll-alert-error shadow-sm text-xs sm:text-sm">
              <span className="font-bold">{applyError.summary}</span>
            </div>
          ) : null}

          <div className="grid gap-6 md:grid-cols-[minmax(0,1fr)_1px_minmax(0,1fr)] md:gap-0 items-start">
            <section className="md:pr-6">
              <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50 mb-2 h-auto min-h-0">
                {tab === "fixed" ? "Fixed PDOs" : "PPS APDOs"}
              </h3>
              <p className="text-xs text-base-content/60 mb-4">
                {tab === "fixed"
                  ? "Select one PDO (object position)."
                  : "Select one APDO (object position)."}
              </p>

              {tab === "fixed" ? (
                <div className="flex flex-col gap-2">
                  {(pd?.fixed_pdos ?? []).map((entry) => {
                    const selected = entry.pos === selectedFixedPos;
                    return (
                      <button
                        key={entry.pos}
                        type="button"
                        className={[
                          "w-full text-left rounded-box border px-4 py-3 flex items-center justify-between",
                          selected
                            ? "border-info bg-info/10"
                            : "border-base-200 bg-base-100 hover:bg-base-200/50",
                        ].join(" ")}
                        onClick={() => setSelectedFixedPos(entry.pos)}
                      >
                        <div className="flex items-center gap-3">
                          <span className="ll-badge ll-badge-ghost">
                            {entry.pos}
                          </span>
                          <span className="font-semibold">
                            {formatMilliVolts(entry.mv)}
                          </span>
                        </div>
                        <span className="text-xs text-base-content/60">
                          {entry.max_ma} mA
                        </span>
                      </button>
                    );
                  })}
                  {(pd?.fixed_pdos ?? []).length === 0 ? (
                    <div className="text-xs text-base-content/60">
                      No fixed PDOs.
                    </div>
                  ) : null}
                </div>
              ) : (
                <div className="flex flex-col gap-2">
                  {(pd?.pps_pdos ?? []).map((entry) => {
                    const selected = entry.pos === selectedPpsPos;
                    return (
                      <button
                        key={entry.pos}
                        type="button"
                        className={[
                          "w-full text-left rounded-box border px-4 py-3 flex flex-col gap-1",
                          selected
                            ? "border-info bg-info/10"
                            : "border-base-200 bg-base-100 hover:bg-base-200/50",
                        ].join(" ")}
                        onClick={() => setSelectedPpsPos(entry.pos)}
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-3">
                            <span className="ll-badge ll-badge-ghost">
                              {entry.pos}
                            </span>
                            <span className="font-semibold">
                              {(entry.min_mv / 1000).toFixed(1)}–
                              {(entry.max_mv / 1000).toFixed(1)} V
                            </span>
                          </div>
                          <span className="text-xs text-base-content/60">
                            {entry.max_ma} mA
                          </span>
                        </div>
                        <div className="text-xs text-base-content/60">
                          PPS step: 20 mV · I step: 50 mA
                        </div>
                      </button>
                    );
                  })}
                  {(pd?.pps_pdos ?? []).length === 0 ? (
                    <div className="text-xs text-base-content/60">
                      No PPS APDOs.
                    </div>
                  ) : null}
                </div>
              )}
            </section>

            <div className="hidden md:block w-px self-stretch bg-base-content/15" />

            <section className="md:pl-6">
              <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50 mb-2 h-auto min-h-0">
                Configure
              </h3>
              <p className="text-xs text-base-content/60 mb-4">
                Saved vs Active contract
              </p>

              <div className="grid gap-4">
                <div className="rounded-lg bg-base-200/20 p-4">
                  <div className="text-xs text-base-content/60 mb-1">Saved</div>
                  <div className="text-sm">
                    Mode:{" "}
                    <span className="font-semibold uppercase">
                      {pd?.saved.mode ?? "..."}
                    </span>
                    {pd?.saved.mode === "fixed" ? (
                      visibleSavedFixed ? (
                        <> · PDO #{visibleSavedFixed.pos}</>
                      ) : null
                    ) : (
                      <> · APDO #{pd?.saved.pps_object_pos}</>
                    )}
                  </div>
                  <div className="text-sm">
                    Ireq: {pd?.saved.i_req_ma ?? "..."} mA
                    {pd?.saved.mode === "pps" ? (
                      <> · Vreq: {pd?.saved.target_mv ?? "..."} mV</>
                    ) : null}
                  </div>
                </div>

                <div className="rounded-lg bg-base-200/20 p-4">
                  <div className="text-xs text-base-content/60 mb-1">
                    Active (contract)
                  </div>
                  <div className="text-sm">{activeContractText}</div>
                </div>
              </div>

              {tab === "fixed" ? (
                <div className="mt-4">
                  <div className="flex flex-wrap items-center gap-3">
                    <div className="text-xs text-base-content/60">
                      Ireq (mA)
                    </div>
                    <input
                      type="number"
                      className="ll-input ll-input-sm w-40"
                      value={fixedIReqMa}
                      min={0}
                      step={50}
                      max={selectedFixed?.max_ma ?? undefined}
                      onChange={(event) =>
                        setFixedIReqMa(Number(event.target.value))
                      }
                    />
                    <div className="text-xs text-base-content/60">
                      step: 50 mA
                    </div>
                    <button
                      type="button"
                      className="ll-button ll-button-primary ll-button-sm ml-auto"
                      disabled={applyDisabled}
                      onClick={() => {
                        applyMutation.reset();
                        applyMutation.mutate({ tab: "fixed" });
                      }}
                    >
                      Apply
                    </button>
                  </div>
                </div>
              ) : (
                <div className="mt-4">
                  <div className="flex flex-wrap items-center gap-3">
                    <div className="text-xs text-base-content/60">
                      Vreq (mV)
                    </div>
                    <input
                      type="number"
                      className="ll-input ll-input-sm w-44"
                      value={ppsTargetMv}
                      step={20}
                      min={selectedPps?.min_mv ?? undefined}
                      max={selectedPps?.max_mv ?? undefined}
                      onChange={(event) =>
                        setPpsTargetMv(Number(event.target.value))
                      }
                    />
                    <div className="text-xs text-base-content/60">
                      step: 20 mV
                    </div>
                  </div>

                  <div className="mt-3">
                    <input
                      type="range"
                      className="range range-info"
                      min={selectedPps?.min_mv ?? 0}
                      max={selectedPps?.max_mv ?? 0}
                      step={20}
                      value={ppsTargetMv}
                      disabled={!selectedPps}
                      onChange={(event) =>
                        setPpsTargetMv(Number(event.target.value))
                      }
                    />
                    <div className="flex justify-between text-xs text-base-content/60 mt-1">
                      <span>min {selectedPps?.min_mv ?? "-"}</span>
                      <span>max {selectedPps?.max_mv ?? "-"}</span>
                    </div>
                  </div>

                  <div className="mt-4 flex flex-wrap items-center gap-3">
                    <div className="text-xs text-base-content/60">
                      Ireq (mA)
                    </div>
                    <input
                      type="number"
                      className="ll-input ll-input-sm w-40"
                      value={ppsIReqMa}
                      min={0}
                      step={50}
                      max={selectedPps?.max_ma ?? undefined}
                      onChange={(event) =>
                        setPpsIReqMa(Number(event.target.value))
                      }
                    />
                    <div className="text-xs text-base-content/60">
                      step: 50 mA
                    </div>
                    {selectedPps ? (
                      <div className="text-xs text-base-content/60">
                        Imax: {selectedPps.max_ma} mA (hard limit)
                      </div>
                    ) : null}

                    <button
                      type="button"
                      className="ll-button ll-button-primary ll-button-sm ml-auto"
                      disabled={applyDisabled}
                      onClick={() => {
                        applyMutation.reset();
                        applyMutation.mutate({ tab: "pps" });
                      }}
                    >
                      Apply
                    </button>
                  </div>
                </div>
              )}

              {validationHint ? (
                <div className="mt-3 text-xs text-base-content/60">
                  {validationHint}
                </div>
              ) : null}

              {pd?.apply.last ? (
                <div className="mt-4 text-xs text-base-content/60">
                  Last apply: {pd.apply.last.code} · at {pd.apply.last.at_ms} ms
                </div>
              ) : (
                <div className="mt-4 text-xs text-base-content/60">
                  Last apply: none
                </div>
              )}
            </section>
          </div>
        </div>
      </section>
    </PageContainer>
  );
}

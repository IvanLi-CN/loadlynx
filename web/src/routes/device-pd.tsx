import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import { getIdentity, getPd, isHttpApiError, postPd } from "../api/client.ts";
import type { Identity, PdFixedPdo, PdPpsPdo, PdView } from "../api/types.ts";
import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";

const PD_REFETCH_MS = 1200;
const RETRY_DELAY_MS = 500;

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

function findFixedPdo(pd: PdView, pos: number | null): PdFixedPdo | null {
  if (pos == null) return null;
  return pd.fixed_pdos.find((entry) => entry.pos === pos) ?? null;
}

function findPpsPdo(pd: PdView, pos: number | null): PdPpsPdo | null {
  if (pos == null) return null;
  return pd.pps_pdos.find((entry) => entry.pos === pos) ?? null;
}

export function DevicePdRoute() {
  const { deviceId, device, baseUrl } = useDeviceContext();
  const queryClient = useQueryClient();

  const [isPageVisible, setIsPageVisible] = useState(() =>
    typeof document === "undefined"
      ? true
      : document.visibilityState === "visible",
  );

  useEffect(() => {
    if (typeof document === "undefined") {
      return undefined;
    }

    const handleVisibility = () => {
      setIsPageVisible(document.visibilityState === "visible");
    };

    document.addEventListener("visibilitychange", handleVisibility);
    return () => {
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, []);

  const identityQuery = useQuery<Identity, HttpApiError>({
    queryKey: ["device", deviceId, "identity"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getIdentity(baseUrl);
    },
    enabled: Boolean(baseUrl),
  });

  const pdQuery = useQuery<PdView, HttpApiError>({
    queryKey: ["device", deviceId, "pd"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getPd(baseUrl);
    },
    enabled: Boolean(baseUrl) && identityQuery.isSuccess,
    refetchInterval: isPageVisible ? PD_REFETCH_MS : false,
    refetchIntervalInBackground: false,
    retryDelay: RETRY_DELAY_MS,
  });

  const pd = pdQuery.data;

  const [tab, setTab] = useState<"fixed" | "pps">("fixed");

  const [selectedFixedPos, setSelectedFixedPos] = useState<number | null>(null);
  const [selectedPpsPos, setSelectedPpsPos] = useState<number | null>(null);

  const [fixedIReqMa, setFixedIReqMa] = useState<number>(0);
  const [ppsIReqMa, setPpsIReqMa] = useState<number>(0);
  const [ppsTargetMv, setPpsTargetMv] = useState<number>(0);

  useEffect(() => {
    if (!pd) return;
    setTab(pd.saved.mode === "pps" ? "pps" : "fixed");

    const fixedPos = pd.saved.fixed_object_pos;
    const ppsPos = pd.saved.pps_object_pos;

    const nextFixedPos = pd.fixed_pdos.some((entry) => entry.pos === fixedPos)
      ? fixedPos
      : (pd.fixed_pdos[0]?.pos ?? null);
    const nextPpsPos = pd.pps_pdos.some((entry) => entry.pos === ppsPos)
      ? ppsPos
      : (pd.pps_pdos[0]?.pos ?? null);

    setSelectedFixedPos(nextFixedPos);
    setSelectedPpsPos(nextPpsPos);

    setFixedIReqMa(pd.saved.i_req_ma);
    setPpsIReqMa(pd.saved.i_req_ma);
    setPpsTargetMv(pd.saved.target_mv);
  }, [pd]);

  const applyMutation = useMutation({
    mutationFn: async (payload: { tab: "fixed" | "pps" }) => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }

      if (!pd) {
        throw new Error("PD view is not available");
      }

      if (payload.tab === "fixed") {
        if (selectedFixedPos == null) {
          throw new Error("No Fixed PDO selected");
        }
        return postPd(baseUrl, {
          mode: "fixed",
          object_pos: selectedFixedPos,
          i_req_ma: fixedIReqMa,
        });
      }

      if (selectedPpsPos == null) {
        throw new Error("No PPS APDO selected");
      }

      return postPd(baseUrl, {
        mode: "pps",
        object_pos: selectedPpsPos,
        target_mv: ppsTargetMv,
        i_req_ma: ppsIReqMa,
      });
    },
    onSuccess: (next) => {
      queryClient.setQueryData<PdView>(["device", deviceId, "pd"], next);
    },
  });

  const isUnsupported = (() => {
    const err = pdQuery.error;
    if (!err || !isHttpApiError(err)) return false;
    return err.status === 404 && err.code === "UNSUPPORTED_OPERATION";
  })();

  const topError = (() => {
    const err = pdQuery.error;
    if (!err || !isHttpApiError(err) || isUnsupported) return null;

    const code = err.code ?? "HTTP_ERROR";
    const summary = `${code} — ${err.message}`;

    if (err.status === 0 && code === "NETWORK_ERROR") {
      const hint =
        "无法连接设备" +
        (baseUrl ? `（baseUrl=${baseUrl}）` : "") +
        "，请检查网络与 IP 设置。";
      return { summary, hint, kind: "error" } as const;
    }

    if (code === "LINK_DOWN" || code === "UNAVAILABLE") {
      return {
        summary,
        hint: "UART link is down / PD status unavailable — try again later.",
        kind: "warning",
      } as const;
    }

    if (code === "ANALOG_NOT_READY" || code === "NOT_ATTACHED") {
      return { summary, hint: null, kind: "warning" } as const;
    }

    return { summary, hint: null, kind: "error" } as const;
  })();

  const activeContractText = formatContract(pd);

  const selectedFixed = pd ? findFixedPdo(pd, selectedFixedPos) : null;
  const selectedPps = pd ? findPpsPdo(pd, selectedPpsPos) : null;

  const fixedValidation = useMemo(() => {
    if (!pd) return { ok: false, reason: "Loading..." } as const;
    if (!pd.attached) return { ok: false, reason: "PD not attached" } as const;
    if (!selectedFixed) {
      return { ok: false, reason: "Selected PDO not found" } as const;
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
            className="btn btn-sm btn-outline"
          >
            Back
          </Link>
        </header>

        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-8">
            <div className="badge badge-warning badge-outline mb-4">
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
          className="btn btn-sm btn-outline"
        >
          Back
        </Link>
      </header>

      {topError ? (
        <div
          className={[
            "alert shadow-sm rounded-lg text-sm",
            topError.kind === "warning" ? "alert-warning" : "alert-error",
          ].join(" ")}
        >
          <span className="font-bold">Error: {topError.summary}</span>
          {topError.hint && (
            <span className="text-xs opacity-80 block">{topError.hint}</span>
          )}
        </div>
      ) : null}

      <section className="rounded-box bg-base-200/10 p-6">
          <div className="flex flex-wrap items-center gap-3">
            <div className="text-sm font-semibold">Status</div>
            <div
              className={[
                "badge",
                pd?.attached ? "badge-success" : "badge-ghost",
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

          <div className="mt-4 join">
            <button
              type="button"
              className={[
                "btn btn-sm join-item",
                tab === "fixed"
                  ? "btn-info text-info-content font-semibold"
                  : "btn-ghost border border-base-content/20 text-base-content/70",
              ].join(" ")}
              onClick={() => setTab("fixed")}
            >
              Fixed
            </button>
            <button
              type="button"
              className={[
                "btn btn-sm join-item",
                tab === "pps"
                  ? "btn-info text-info-content font-semibold"
                  : "btn-ghost border border-base-content/20 text-base-content/70",
              ].join(" ")}
              onClick={() => setTab("pps")}
            >
              PPS
            </button>
          </div>
      </section>

      {applyMutation.isSuccess ? (
        <div className="alert alert-success shadow-sm text-xs sm:text-sm">
          <span>Apply succeeded.</span>
        </div>
      ) : null}

      {applyError ? (
        <div className="alert alert-error shadow-sm text-xs sm:text-sm">
          <span className="font-bold">{applyError.summary}</span>
        </div>
      ) : null}

      <div className="grid gap-6 md:grid-cols-[minmax(0,1fr)_1px_minmax(0,1fr)] md:gap-x-0 items-start">
        <section className="rounded-box bg-base-200/10 p-6 md:mr-3">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-2 h-auto min-h-0">
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
                        <span className="badge badge-ghost">{entry.pos}</span>
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
                          <span className="badge badge-ghost">{entry.pos}</span>
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

        <section className="rounded-box bg-base-200/10 p-6 md:ml-3">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-2 h-auto min-h-0">
              Configure
            </h3>
            <p className="text-xs text-base-content/60 mb-4">
              Saved vs Active contract
            </p>

            <div className="grid gap-4">
              <div className="rounded-box bg-base-200/20 p-4">
                  <div className="text-xs text-base-content/60 mb-1">Saved</div>
                  <div className="text-sm">
                    Mode:{" "}
                    <span className="font-semibold uppercase">
                      {pd?.saved.mode ?? "..."}
                    </span>
                    {pd?.saved.mode === "fixed" ? (
                      <> · PDO #{pd.saved.fixed_object_pos}</>
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

              <div className="rounded-box bg-base-200/20 p-4">
                  <div className="text-xs text-base-content/60 mb-1">
                    Active (contract)
                  </div>
                  <div className="text-sm">{activeContractText}</div>
              </div>
            </div>

            {tab === "fixed" ? (
              <div className="mt-4">
                <div className="flex flex-wrap items-center gap-3">
                  <div className="text-xs text-base-content/60">Ireq (mA)</div>
                  <input
                    type="number"
                    className="input input-bordered input-sm w-40"
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
                    className="btn btn-success btn-sm ml-auto"
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
                  <div className="text-xs text-base-content/60">Vreq (mV)</div>
                  <input
                    type="number"
                    className="input input-bordered input-sm w-44"
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
                  <div className="text-xs text-base-content/60">Ireq (mA)</div>
                  <input
                    type="number"
                    className="input input-bordered input-sm w-40"
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
                    className="btn btn-success btn-sm ml-auto"
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
    </PageContainer>
  );
}

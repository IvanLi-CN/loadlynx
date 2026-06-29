import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
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
import { BlockControlSliderRow } from "../components/ui/block-control-row.tsx";
import { setDeviceQueryData } from "../devices/device-query-cache.ts";
import { DEVICE_QUERY_PARTS } from "../devices/device-query-key.ts";
import {
  getDevicePdQueryOptions,
  useDeviceIdentityByBaseUrl,
} from "../devices/hooks.ts";
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
type PdSelection =
  | { kind: "fixed"; pos: number }
  | { kind: "pps"; pos: number }
  | null;

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

function parseClampedInt(
  raw: string,
  min: number,
  max: number,
  fallback: number,
): number {
  const parsed = Number.parseInt(raw.trim(), 10);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.min(Math.max(parsed, min), max);
}

export function PdControlPanel(props: {
  deviceId: string;
  deviceName: string;
  baseUrl: string;
  embedded?: boolean;
  onBackToDashboard?: () => void;
}) {
  const { t } = useTranslation();
  const { deviceId, deviceName, baseUrl, embedded = false } = props;
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
  const currentSelection: PdSelection = selectedPps
    ? { kind: "pps", pos: selectedPps.pos }
    : selectedFixed
      ? { kind: "fixed", pos: selectedFixed.pos }
      : null;
  const currentSelectionKind: PdApplyTab = currentSelection?.kind ?? "fixed";
  const visibleSavedFixed = pd ? findVisibleSavedFixedPdo(pd) : null;
  const updatedText = pdQuery.dataUpdatedAt
    ? new Date(pdQuery.dataUpdatedAt).toLocaleTimeString()
    : "...";
  const savedProfileSummary = (() => {
    if (!pd) return "Loading...";
    if (pd.saved.mode === "fixed") {
      if (!visibleSavedFixed) {
        return `Fixed · ${pd.saved.i_req_ma} mA`;
      }
      return `Fixed · PDO #${visibleSavedFixed.pos} · ${formatMilliVolts(
        visibleSavedFixed.mv,
      )} · ${pd.saved.i_req_ma} mA`;
    }
    return `PPS · APDO #${pd.saved.pps_object_pos} · ${pd.saved.target_mv} mV · ${pd.saved.i_req_ma} mA`;
  })();

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
    (currentSelectionKind === "fixed"
      ? !fixedValidation.ok
      : !ppsValidation.ok);

  const validationHint =
    currentSelectionKind === "fixed"
      ? fixedValidation.reason
      : ppsValidation.reason;

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
      <section
        className={embedded ? "ll-pd-panel" : "ll-pd-panel ll-pd-panel--page"}
      >
        <div className="ll-pd-panel__notice ll-pd-panel__notice--warning">
          <div className="instrument-label">
            {t("pdPanel.unsupportedBadge")}
          </div>
          <h3 className="ll-pd-panel__notice-title">
            {t("pdPanel.unsupportedTitle")}
          </h3>
          <p className="ll-pd-panel__notice-copy">
            {t("pdPanel.unsupportedBody")} <code>/api/v1/pd</code>.
          </p>
        </div>
      </section>
    );
  }

  return (
    <section
      className={
        embedded
          ? "ll-pd-panel ll-pd-panel--embedded"
          : "ll-pd-panel ll-pd-panel--page"
      }
      aria-label="USB-PD control panel"
    >
      {!embedded ? (
        <header className="ll-pd-panel__header">
          <div className="ll-pd-panel__header-copy">
            <div className="instrument-label">{t("pdPanel.title")}</div>
            <h2 className="ll-pd-panel__title">{t("pdPanel.title")}</h2>
            <p className="ll-pd-panel__subtitle">{t("pdPanel.subtitle")}</p>
            <div className="ll-pd-panel__device-line">
              {t("pdPanel.deviceContext", { deviceName })}
            </div>
          </div>
          {identityQuery.data ? (
            <div className="ll-pd-panel__identity">
              <div className="instrument-label">IP</div>
              <code>{identityQuery.data.network.ip}</code>
            </div>
          ) : null}
        </header>
      ) : null}

      <div className="ll-pd-panel__body">
        {topError ? (
          <div
            className={[
              "ll-pd-panel__notice",
              topError.kind === "warning"
                ? "ll-pd-panel__notice--warning"
                : "ll-pd-panel__notice--error",
            ].join(" ")}
          >
            <div className="instrument-label">Error</div>
            <div className="ll-pd-panel__notice-title">{topError.summary}</div>
            {topError.hint ? (
              <div className="ll-pd-panel__notice-copy">{topError.hint}</div>
            ) : null}
          </div>
        ) : null}

        <div className="ll-pd-panel__workspace">
          <section className="ll-pd-panel__surface ll-pd-panel__surface--selection">
            <div className="ll-pd-panel__selection-stack">
              <div className="ll-pd-panel__section-head">
                <div>
                  <div className="instrument-label">
                    {t("pdPanel.profileList")}
                  </div>
                  <div className="ll-pd-panel__section-copy">
                    {t("pdPanel.profileListHint")}
                  </div>
                </div>
                {currentSelection ? (
                  <div className="instrument-pill instrument-pill-cyan">
                    {currentSelection.kind === "fixed"
                      ? t("pdPanel.fixedType")
                      : t("pdPanel.ppsType")}
                  </div>
                ) : null}
              </div>
            </div>

            <div className="ll-pd-panel__surface-divider" />

            <div className="ll-pd-panel__selection-region">
              <div className="ll-pd-panel__selection-scroll">
                <div className="ll-pd-panel__group">
                  <div className="ll-pd-panel__section-head ll-pd-panel__section-head--compact">
                    <div>
                      <div className="instrument-label">
                        {t("pdPanel.fixedTitle")}
                      </div>
                      <div className="ll-pd-panel__section-copy">
                        {t("pdPanel.fixedHint")}
                      </div>
                    </div>
                  </div>
                  <div className="ll-pd-panel__option-stack ll-pd-panel__option-stack--fixed">
                    {(pd?.fixed_pdos ?? []).map((entry) => {
                      const selected =
                        currentSelection?.kind === "fixed" &&
                        entry.pos === currentSelection.pos;
                      return (
                        <button
                          key={`fixed-${entry.pos}`}
                          type="button"
                          className={[
                            "ll-pd-panel__option ll-pd-panel__option--fixed",
                            selected ? "ll-pd-panel__option--selected" : "",
                          ].join(" ")}
                          onClick={() => {
                            setSelectedFixedPos(entry.pos);
                            setSelectedPpsPos(null);
                          }}
                        >
                          <span className="ll-pd-panel__option-index">
                            #{entry.pos}
                          </span>
                          <span className="ll-pd-panel__option-main">
                            {formatMilliVolts(entry.mv)}
                          </span>
                          <span className="ll-pd-panel__option-side">
                            {entry.max_ma} mA
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>

                <div className="ll-pd-panel__surface-divider" />

                <div className="ll-pd-panel__group">
                  <div className="ll-pd-panel__section-head ll-pd-panel__section-head--compact">
                    <div>
                      <div className="instrument-label">
                        {t("pdPanel.ppsTitle")}
                      </div>
                      <div className="ll-pd-panel__section-copy">
                        {t("pdPanel.ppsHint")}
                      </div>
                    </div>
                  </div>
                  <div className="ll-pd-panel__option-stack">
                    {(pd?.pps_pdos ?? []).map((entry) => {
                      const selected =
                        currentSelection?.kind === "pps" &&
                        entry.pos === currentSelection.pos;
                      return (
                        <button
                          key={`pps-${entry.pos}`}
                          type="button"
                          className={[
                            "ll-pd-panel__option ll-pd-panel__option--dual",
                            selected ? "ll-pd-panel__option--selected" : "",
                          ].join(" ")}
                          onClick={() => {
                            setSelectedPpsPos(entry.pos);
                            setSelectedFixedPos(null);
                          }}
                        >
                          <span className="ll-pd-panel__option-index">
                            #{entry.pos}
                          </span>
                          <span className="ll-pd-panel__option-main">
                            {(entry.min_mv / 1000).toFixed(1)}–
                            {(entry.max_mv / 1000).toFixed(1)} V
                          </span>
                          <span className="ll-pd-panel__option-side">
                            {entry.max_ma} mA
                          </span>
                          <span className="ll-pd-panel__option-meta">
                            PPS step 20 mV · I step 50 mA
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section className="ll-pd-panel__surface ll-pd-panel__surface--editor">
            <section className="ll-pd-panel__summary ll-pd-panel__summary--embedded">
              <div className="ll-pd-panel__summary-main">
                <div className="ll-pd-panel__summary-block">
                  <div className="instrument-label">{t("pdPanel.status")}</div>
                  <div className="flex flex-wrap items-center gap-3">
                    <span
                      className={[
                        "instrument-pill",
                        pd?.attached ? "instrument-pill-green" : "",
                      ].join(" ")}
                    >
                      {pd?.attached
                        ? t("pdPanel.attached")
                        : t("pdPanel.detached")}
                    </span>
                    <div className="ll-pd-panel__contract">
                      <span className="ll-pd-panel__inline-label">
                        {t("pdPanel.contract")}
                      </span>
                      <span>{activeContractText}</span>
                    </div>
                  </div>
                </div>

                <div className="ll-pd-panel__summary-grid">
                  <div className="ll-pd-panel__summary-card">
                    <div className="instrument-label">
                      {t("pdPanel.savedProfile")}
                    </div>
                    <div className="ll-pd-panel__summary-value">
                      {savedProfileSummary}
                    </div>
                  </div>
                  <div className="ll-pd-panel__summary-card ll-pd-panel__summary-card--compact">
                    <div className="instrument-label">
                      {t("pdPanel.updated")}
                    </div>
                    <div className="ll-pd-panel__summary-value">
                      {updatedText}
                    </div>
                  </div>
                </div>
              </div>
              {pd?.allow_extended_voltage === false ? (
                <div className="ll-pd-panel__notice ll-pd-panel__notice--info">
                  <div className="instrument-label">Safe5V</div>
                  <div className="ll-pd-panel__notice-copy">
                    {t("demo.pd.safe5v")}
                  </div>
                </div>
              ) : null}
            </section>

            <div className="ll-pd-panel__section-head">
              <div>
                <div className="instrument-label">
                  {t("pdPanel.requestEditor")}
                </div>
                <div className="ll-pd-panel__section-copy">
                  {currentSelectionKind === "fixed"
                    ? selectedFixed
                      ? `${t("pdPanel.selectedObject")} · ${t(
                          "pdPanel.fixedPdo",
                        )} #${selectedFixed.pos}`
                      : t("pdPanel.noSelection")
                    : selectedPps
                      ? `${t("pdPanel.selectedObject")} · ${t(
                          "pdPanel.ppsApdo",
                        )} #${selectedPps.pos}`
                      : t("pdPanel.noSelection")}
                </div>
              </div>
              <div className="instrument-pill">
                {currentSelectionKind === "fixed"
                  ? t("pdPanel.fixedType")
                  : t("pdPanel.ppsType")}
              </div>
            </div>

            {currentSelectionKind === "fixed" ? (
              <div className="ll-pd-panel__editor-stack">
                <BlockControlSliderRow
                  id="pd-fixed-ireq"
                  label={t("pdPanel.fixedCurrent")}
                  value={fixedIReqMa}
                  min={0}
                  max={selectedFixed?.max_ma ?? 0}
                  step={50}
                  displayValue={String(fixedIReqMa)}
                  disabled={!selectedFixed}
                  className="ll-pd-panel__slider-row"
                  onValueChange={setFixedIReqMa}
                  onDisplayValueCommit={(raw) =>
                    setFixedIReqMa(
                      parseClampedInt(
                        raw,
                        0,
                        selectedFixed?.max_ma ?? 0,
                        fixedIReqMa,
                      ),
                    )
                  }
                />
              </div>
            ) : (
              <div className="ll-pd-panel__editor-stack">
                <BlockControlSliderRow
                  id="pd-pps-vreq"
                  label={t("pdPanel.ppsVoltage")}
                  value={ppsTargetMv}
                  min={selectedPps?.min_mv ?? 0}
                  max={selectedPps?.max_mv ?? 0}
                  step={20}
                  displayValue={String(ppsTargetMv)}
                  disabled={!selectedPps}
                  className="ll-pd-panel__slider-row"
                  onValueChange={setPpsTargetMv}
                  onDisplayValueCommit={(raw) =>
                    setPpsTargetMv(
                      parseClampedInt(
                        raw,
                        selectedPps?.min_mv ?? 0,
                        selectedPps?.max_mv ?? 0,
                        ppsTargetMv,
                      ),
                    )
                  }
                />
                <BlockControlSliderRow
                  id="pd-pps-ireq"
                  label={t("pdPanel.ppsCurrent")}
                  value={ppsIReqMa}
                  min={0}
                  max={selectedPps?.max_ma ?? 0}
                  step={50}
                  displayValue={String(ppsIReqMa)}
                  disabled={!selectedPps}
                  className="ll-pd-panel__slider-row"
                  onValueChange={setPpsIReqMa}
                  onDisplayValueCommit={(raw) =>
                    setPpsIReqMa(
                      parseClampedInt(
                        raw,
                        0,
                        selectedPps?.max_ma ?? 0,
                        ppsIReqMa,
                      ),
                    )
                  }
                />
              </div>
            )}

            {validationHint ? (
              <div className="ll-pd-panel__hint">{validationHint}</div>
            ) : null}

            {applyMutation.isSuccess ? (
              <div className="ll-pd-panel__notice ll-pd-panel__notice--success">
                <div className="ll-pd-panel__notice-copy">
                  {applyMutation.data?.allow_extended_voltage === false
                    ? t("pdPanel.applySuccessSafe5v")
                    : t("pdPanel.applySuccess")}
                </div>
              </div>
            ) : null}

            {applyError ? (
              <div className="ll-pd-panel__notice ll-pd-panel__notice--error">
                <div className="ll-pd-panel__notice-title">
                  {applyError.summary}
                </div>
              </div>
            ) : null}

            <div className="ll-pd-panel__footer">
              <div className="ll-pd-panel__footer-meta">
                {t("pdPanel.lastApply")}:{" "}
                {pd?.apply.last
                  ? `${pd.apply.last.code} · ${pd.apply.last.at_ms} ms`
                  : t("pdPanel.none")}
              </div>
              <button
                type="button"
                className="ll-button ll-button-primary"
                disabled={applyDisabled}
                onClick={() => {
                  applyMutation.reset();
                  applyMutation.mutate({ tab: currentSelectionKind });
                }}
              >
                {t("pdPanel.applyAction")}
              </button>
            </div>
          </section>
        </div>
      </div>
    </section>
  );
}

import { useNavigate, useRouterState } from "@tanstack/react-router";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ENABLE_MOCK_DEVTOOLS,
  isHttpApiError,
  isMockBaseUrl,
} from "../api/client.ts";
import { DashboardTrendPanel } from "../components/instrument/dashboard-trend-panel.tsx";
import { formatUptimeSeconds } from "../components/instrument/format.ts";
import { InstrumentStatusBar } from "../components/instrument/instrument-status-bar.tsx";
import { LiveControlPanel } from "../components/instrument/live-control-panel.tsx";
import { PresetsPanel } from "../components/instrument/presets-panel.tsx";
import { ThermalPanel } from "../components/instrument/thermal-panel.tsx";
import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import { usePageVisibility } from "../lib/page-visibility.ts";
import { AdvancedControlsPanel } from "./device-cc/advanced-controls-panel.tsx";
import { useDeviceCcState } from "./device-cc/use-device-cc-state.ts";
import { PdControlPanel } from "./device-pd-panel.tsx";

type DashboardToolDrawerKind = "presets" | "pd" | "advanced";

function DashboardToolDrawer(props: {
  open: boolean;
  title: string;
  description: string;
  panelWidth?: "default" | "wide";
  onClose: () => void;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  useEffect(() => {
    if (!props.open) return undefined;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        props.onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = previousOverflow;
    };
  }, [props.open, props.onClose]);

  if (!props.open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50">
      <button
        type="button"
        aria-label={t("dashboard.drawer.close")}
        className="absolute inset-0 bg-slate-950/68 backdrop-blur-[2px]"
        onClick={props.onClose}
      />
      <aside
        role="dialog"
        aria-modal="true"
        aria-label={props.title}
        className={[
          "absolute inset-y-0 right-0 flex h-full w-full flex-col border-l border-cyan-400/25 bg-[linear-gradient(180deg,oklch(0.16_0.04_262/.98),oklch(0.09_0.03_262/.99))] shadow-[-18px_0_48px_oklch(0.82_0.17_210/.12)]",
          props.panelWidth === "wide"
            ? "max-w-none lg:w-[min(84vw,54rem)]"
            : "max-w-[480px]",
        ].join(" ")}
      >
        <div className="border-b border-base-300/80 px-4 py-3">
          <div className="flex items-start justify-between gap-4">
            <div>
              <div className="instrument-label">
                {t("dashboard.drawer.tools")}
              </div>
              <h2 className="mt-2 text-lg font-bold text-slate-50">
                {props.title}
              </h2>
              <p className="mt-2 text-sm text-slate-200/60">
                {props.description}
              </p>
            </div>
            <button
              type="button"
              className="ll-button ll-button-ghost ll-button-square ll-button-square-lg ll-drawer-close-button"
              onClick={props.onClose}
              aria-label={t("dashboard.drawer.close")}
            >
              <span aria-hidden="true" className="ll-drawer-close-button__icon">
                ×
              </span>
            </button>
          </div>
        </div>

        <div className="flex-1 min-h-0 overflow-hidden px-3 py-3">
          {props.children}
        </div>
      </aside>
    </div>
  );
}

export function DeviceCcRoute() {
  const { t } = useTranslation();
  const { deviceId, device, baseUrl } = useDeviceContext();
  const isPageVisible = usePageVisibility();
  const [advancedCollapsed, setAdvancedCollapsed] = useState(true);
  const [activeToolDrawer, setActiveToolDrawer] =
    useState<DashboardToolDrawerKind | null>(null);
  const navigate = useNavigate();
  const requestedPanel = useRouterState({
    select: (state) =>
      new URLSearchParams(state.location.searchStr).get("panel"),
  });

  const { view, mutation } = useDeviceCcState(deviceId, baseUrl, isPageVisible);

  useEffect(() => {
    if (requestedPanel === "pd") {
      setActiveToolDrawer("pd");
      return;
    }

    setActiveToolDrawer((current) => (current === "pd" ? null : current));
  }, [requestedPanel]);

  function openToolDrawer(kind: DashboardToolDrawerKind) {
    setActiveToolDrawer(kind);

    if (kind === "pd") {
      navigate({
        to: "/$deviceId/cc",
        params: { deviceId },
        search: { panel: "pd" },
        replace: true,
      });
      return;
    }

    if (requestedPanel === "pd") {
      navigate({
        to: "/$deviceId/cc",
        params: { deviceId },
        search: {},
        replace: true,
      });
    }
  }

  function closeToolDrawer() {
    setActiveToolDrawer(null);

    if (requestedPanel === "pd") {
      navigate({
        to: "/$deviceId/cc",
        params: { deviceId },
        search: {},
        replace: true,
      });
    }
  }

  const toolDrawerTitle =
    activeToolDrawer === "pd"
      ? t("dashboard.drawer.pdTitle")
      : activeToolDrawer === "presets"
        ? t("dashboard.drawer.presetsTitle")
        : activeToolDrawer === "advanced"
          ? t("dashboard.drawer.advancedTitle")
          : "";
  const toolDrawerDescription =
    activeToolDrawer === "pd"
      ? t("dashboard.drawer.pdDescription")
      : activeToolDrawer === "presets"
        ? t("dashboard.drawer.presetsDescription")
        : activeToolDrawer === "advanced"
          ? t("dashboard.drawer.advancedDescription")
          : "";

  return (
    <PageContainer variant="full" className="font-mono tabular-nums">
      <div className="instrument-viewport rounded-[28px] p-4 sm:p-6 md:p-8 xl:h-[calc(100dvh-4.5rem)] xl:overflow-hidden">
        <div className="mx-auto flex h-full max-w-[1600px] flex-col">
          <InstrumentStatusBar
            modeLabel={view.activeLoadModeBadge}
            linkState={view.linkState}
            outputState={{
              enabled: view.control?.output_enabled ?? false,
              setpointLabel: view.activeSetpointLabel,
            }}
            protectionState={view.protectionState}
            faultSummary={view.faultSummary}
            stale={view.telemetryStale}
          />

          {view.topError ? (
            <section
              aria-label={t("dashboard.errors.http")}
              className="mt-5 rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200"
            >
              <div className="font-semibold">
                {t("dashboard.errors.http")}: {view.topError.summary}
              </div>
              {view.topError.hint ? (
                <div className="mt-1 text-red-200/80">{view.topError.hint}</div>
              ) : null}
              {view.isLinkDownLike ? (
                <div className="mt-1 text-red-200/70">
                  {t("dashboard.errors.linkDownStale")}
                </div>
              ) : null}
            </section>
          ) : null}

          <DashboardToolDrawer
            open={activeToolDrawer !== null}
            title={toolDrawerTitle}
            description={toolDrawerDescription}
            panelWidth={activeToolDrawer === "pd" ? "wide" : "default"}
            onClose={closeToolDrawer}
          >
            {activeToolDrawer === "pd" ? (
              <PdControlPanel
                deviceId={deviceId}
                deviceName={device.name}
                baseUrl={baseUrl}
                embedded
                onBackToDashboard={closeToolDrawer}
              />
            ) : null}

            {activeToolDrawer === "presets" ? (
              <PresetsPanel
                presets={view.presetsButtons}
                selectedPresetId={mutation.selectedPresetId}
                activePresetId={view.control?.active_preset_id ?? null}
                onPresetSelect={(id) => {
                  if (id >= 1 && id <= 5) {
                    mutation.setSelectedPresetId(id as 1 | 2 | 3 | 4 | 5);
                  }
                }}
                onApply={mutation.handleApplyPreset}
                onSave={mutation.handleSavePreset}
                applyDisabled={
                  !baseUrl || mutation.applyPresetMutation.isPending
                }
                saveDisabled={view.savePresetDisabled}
                applying={mutation.applyPresetMutation.isPending}
                saving={mutation.updatePresetMutation.isPending}
                cpSupported={view.cpSupported}
                cpDraftOutOfRange={view.cpDraftOutOfRange}
                draftPresetMode={mutation.draftPresetMode}
                draftPresetTargetIMa={mutation.draftPresetTargetIMa}
                draftPresetTargetVMv={mutation.draftPresetTargetVMv}
                draftPresetTargetPMw={mutation.draftPresetTargetPMw}
                draftPresetMinVMv={mutation.draftPresetMinVMv}
                draftPresetMaxIMaTotal={mutation.draftPresetMaxIMaTotal}
                draftPresetMaxPMw={mutation.draftPresetMaxPMw}
                getDisplayValue={mutation.getPresetDisplayValue}
                setDisplayDraft={mutation.setPresetDisplayDraft}
                commitDisplayDraft={mutation.commitPresetDisplayDraft}
                fieldError={mutation.presetDisplayError}
                onModeChange={mutation.setDraftPresetMode}
                onTargetCurrentChange={mutation.setDraftPresetTargetIMa}
                onTargetVoltageChange={mutation.setDraftPresetTargetVMv}
                onTargetPowerChange={mutation.setDraftPresetTargetPMw}
                onMinVoltageChange={mutation.setDraftPresetMinVMv}
                onMaxCurrentChange={mutation.setDraftPresetMaxIMaTotal}
                onMaxPowerChange={mutation.setDraftPresetMaxPMw}
                saveError={
                  mutation.updatePresetMutation.isError &&
                  mutation.updatePresetMutation.error
                    ? isHttpApiError(mutation.updatePresetMutation.error)
                      ? `${mutation.updatePresetMutation.error.code ?? "HTTP_ERROR"} — ${mutation.updatePresetMutation.error.message}${mutation.explainHttpError(mutation.updatePresetMutation.error) ? `\n${mutation.explainHttpError(mutation.updatePresetMutation.error)}` : ""}`
                      : mutation.updatePresetMutation.error instanceof Error
                        ? mutation.updatePresetMutation.error.message
                        : t("dashboard.errors.unknown")
                    : null
                }
                applyError={
                  mutation.applyPresetMutation.isError &&
                  mutation.applyPresetMutation.error
                    ? isHttpApiError(mutation.applyPresetMutation.error)
                      ? `${mutation.applyPresetMutation.error.code ?? "HTTP_ERROR"} — ${mutation.applyPresetMutation.error.message}${mutation.explainHttpError(mutation.applyPresetMutation.error) ? `\n${mutation.explainHttpError(mutation.applyPresetMutation.error)}` : ""}`
                      : mutation.applyPresetMutation.error instanceof Error
                        ? mutation.applyPresetMutation.error.message
                        : t("dashboard.errors.unknown")
                    : null
                }
                actionNotice={view.presetActionNotice}
              />
            ) : null}

            {activeToolDrawer === "advanced" ? (
              <AdvancedControlsPanel
                collapsed={advancedCollapsed}
                selectedPresetId={mutation.selectedPresetId}
                activePresetId={view.control?.active_preset_id ?? null}
                cpSupported={view.cpSupported}
                cpDraftOutOfRange={view.cpDraftOutOfRange}
                display={{
                  remoteVoltageV: view.remoteVoltageV ?? 0,
                  localVoltageV: view.localVoltageV ?? 0,
                  localCurrentA: view.localCurrentA ?? 0,
                  remoteCurrentA: view.remoteCurrentA ?? 0,
                  totalCurrentA: view.totalCurrentA ?? 0,
                  totalPowerW: view.totalPowerW ?? 0,
                  controlMode: view.controlMode,
                  controlTargetMilli: view.controlTargetMilli,
                  controlTargetUnit: view.controlTargetUnit,
                  uptimeSeconds: view.uptimeSeconds ?? 0,
                  tempCoreC: view.tempCoreC ?? undefined,
                  tempSinkC: view.tempSinkC ?? undefined,
                  tempMcuC: view.tempMcuC ?? undefined,
                  remoteActive: view.remoteActive,
                  analogState: view.analogState,
                  faultFlags: view.faultFlags,
                }}
                draftPresetMode={mutation.draftPresetMode}
                draftPresetTargetIMa={mutation.draftPresetTargetIMa}
                draftPresetTargetVMv={mutation.draftPresetTargetVMv}
                draftPresetTargetPMw={mutation.draftPresetTargetPMw}
                draftPresetMinVMv={mutation.draftPresetMinVMv}
                draftPresetMaxIMaTotal={mutation.draftPresetMaxIMaTotal}
                draftPresetMaxPMw={mutation.draftPresetMaxPMw}
                getDisplayValue={mutation.getPresetDisplayValue}
                setDisplayDraft={mutation.setPresetDisplayDraft}
                commitDisplayDraft={mutation.commitPresetDisplayDraft}
                fieldError={mutation.presetDisplayError}
                baseUrl={baseUrl}
                mockDevtoolsEnabled={
                  ENABLE_MOCK_DEVTOOLS &&
                  Boolean(baseUrl) &&
                  isMockBaseUrl(baseUrl)
                }
                saveDisabled={view.savePresetDisabled}
                saveError={
                  mutation.updatePresetMutation.isError &&
                  mutation.updatePresetMutation.error
                    ? isHttpApiError(mutation.updatePresetMutation.error)
                      ? `${mutation.updatePresetMutation.error.code ?? "HTTP_ERROR"} — ${mutation.updatePresetMutation.error.message}${mutation.explainHttpError(mutation.updatePresetMutation.error) ? `\n${mutation.explainHttpError(mutation.updatePresetMutation.error)}` : ""}`
                      : mutation.updatePresetMutation.error instanceof Error
                        ? mutation.updatePresetMutation.error.message
                        : t("dashboard.errors.unknown")
                    : null
                }
                applyError={
                  mutation.applyPresetMutation.isError &&
                  mutation.applyPresetMutation.error
                    ? isHttpApiError(mutation.applyPresetMutation.error)
                      ? `${mutation.applyPresetMutation.error.code ?? "HTTP_ERROR"} — ${mutation.applyPresetMutation.error.message}${mutation.explainHttpError(mutation.applyPresetMutation.error) ? `\n${mutation.explainHttpError(mutation.applyPresetMutation.error)}` : ""}`
                      : mutation.applyPresetMutation.error instanceof Error
                        ? mutation.applyPresetMutation.error.message
                        : t("dashboard.errors.unknown")
                    : null
                }
                savePending={mutation.updatePresetMutation.isPending}
                applyPending={mutation.applyPresetMutation.isPending}
                onApplyPreset={mutation.handleApplyPreset}
                onSavePreset={mutation.handleSavePreset}
                onSetCollapsed={setAdvancedCollapsed}
                onModeChange={mutation.setDraftPresetMode}
                onTargetCurrentChange={mutation.setDraftPresetTargetIMa}
                onTargetVoltageChange={mutation.setDraftPresetTargetVMv}
                onTargetPowerChange={mutation.setDraftPresetTargetPMw}
                onMinVoltageChange={mutation.setDraftPresetMinVMv}
                onMaxCurrentChange={mutation.setDraftPresetMaxIMaTotal}
                onMaxPowerChange={mutation.setDraftPresetMaxPMw}
                onToggleMockUvLatch={() => {
                  mutation.debugUvMutation.mutate(
                    !(view.control?.uv_latched ?? false),
                  );
                }}
              />
            ) : null}
          </DashboardToolDrawer>

          <div className="mt-4 flex-1 min-h-0 xl:mt-5 xl:overflow-hidden">
            <div className="grid h-full grid-cols-1 gap-6 xl:min-h-0 xl:grid-cols-[minmax(0,1.45fr)_minmax(22rem,0.95fr)] xl:items-start">
              <div className="flex min-w-0 flex-col gap-4 xl:min-h-0 xl:overflow-y-auto xl:pr-1">
                <DashboardTrendPanel
                  headline={view.headline}
                  modeLabel={view.activeLoadModeBadge}
                  setpointLabel={view.activeSetpointLabel ?? "—"}
                  uptimeLabel={formatUptimeSeconds(view.uptimeSeconds)}
                  stale={view.telemetryStale}
                  metrics={view.primaryMetrics}
                  trendSeries={view.trendSeries}
                />

                <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)]">
                  <ThermalPanel
                    sinkCoreC={view.tempCoreC}
                    sinkExhaustC={view.tempSinkC}
                    mcuC={view.tempMcuC}
                    faults={view.faultList}
                    trend={{
                      points: view.thermalTrendPoints,
                      min: view.thermalTrendMin - view.thermalTrendPad,
                      max: view.thermalTrendMax + view.thermalTrendPad,
                    }}
                  />
                  <section
                    aria-label={t("dashboard.secondaryStatus.title")}
                    className="instrument-card p-5"
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div>
                        <div className="instrument-label">
                          {t("dashboard.secondaryStatus.title")}
                        </div>
                        <div className="mt-2 text-sm font-semibold text-slate-100">
                          {t("dashboard.secondaryStatus.subtitle")}
                        </div>
                      </div>
                      <button
                        type="button"
                        className="ll-button ll-button-sm ll-button-ghost"
                        onClick={() => {
                          navigate({
                            to: "/$deviceId/status",
                            params: { deviceId },
                          });
                        }}
                      >
                        {t("dashboard.secondaryStatus.openStatusPage")}
                      </button>
                    </div>

                    <div className="mt-4 grid gap-3">
                      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                        <div className="rounded-xl border border-slate-400/10 bg-black/16 px-4 py-3">
                          <div className="instrument-label">
                            {t("dashboard.secondaryStatus.analog")}
                          </div>
                          <div className="mt-2 text-sm font-semibold text-slate-100">
                            {view.analogState === "ready"
                              ? t("dashboard.secondaryStatus.analogOnline")
                              : view.analogState === "offline"
                                ? t("dashboard.secondaryStatus.analogOffline")
                                : view.analogState === "cal_missing"
                                  ? t(
                                      "dashboard.secondaryStatus.analogCalMissing",
                                    )
                                  : view.analogState === "faulted"
                                    ? t(
                                        "dashboard.secondaryStatus.analogFaulted",
                                      )
                                    : view.analogState}
                          </div>
                          <div className="mt-1 text-[11px] text-slate-200/46">
                            {view.diagnostics.analogLinkText}
                          </div>
                        </div>

                        <div className="rounded-xl border border-slate-400/10 bg-black/16 px-4 py-3">
                          <div className="instrument-label">
                            {t("dashboard.secondaryStatus.faults")}
                          </div>
                          <div className="mt-2 text-sm font-semibold text-slate-100">
                            {view.faultList.length > 0
                              ? view.faultList[0]
                              : view.control?.uv_latched
                                ? t("dashboard.secondaryStatus.faultUvLatch")
                                : t("dashboard.secondaryStatus.faultNone")}
                          </div>
                          <div className="mt-1 text-[11px] text-slate-200/46">
                            {view.faultSummary ??
                              t("dashboard.secondaryStatus.protectionReady")}
                          </div>
                        </div>
                      </div>

                      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                        <div className="rounded-xl border border-slate-400/10 bg-black/16 px-4 py-3">
                          <div className="instrument-label">
                            {t("dashboard.secondaryStatus.usbPd")}
                          </div>
                          <div className="mt-2 text-sm font-semibold text-slate-100">
                            {view.pdPanel.contractText ??
                              t("dashboard.secondaryStatus.contractFallback")}
                          </div>
                          <div className="mt-1 text-[11px] text-slate-200/46">
                            {view.pdPanel.savedText ??
                              view.pdPanel.ppsText ??
                              t("dashboard.secondaryStatus.savedFallback")}
                          </div>
                        </div>

                        <div className="rounded-xl border border-slate-400/10 bg-black/16 px-4 py-3">
                          <div className="instrument-label">
                            {t("dashboard.secondaryStatus.apply")}
                          </div>
                          <div className="mt-2 text-sm font-semibold text-slate-100">
                            {view.diagnostics.lastApplyText}
                          </div>
                          <div className="mt-1 text-[11px] text-slate-200/46">
                            {view.diagnostics.loopText}
                          </div>
                        </div>
                      </div>
                    </div>
                  </section>
                </div>
              </div>

              <div className="flex min-w-0 flex-col gap-4 pt-1 xl:h-full xl:min-h-0 xl:overflow-y-auto xl:pl-1">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <div className="instrument-label">
                      {t("dashboard.controls.label")}
                    </div>
                    <div className="mt-2 text-lg font-bold tracking-tight text-slate-50">
                      {t("dashboard.controls.title")}
                    </div>
                    <div className="mt-1 text-[11px] text-slate-200/48">
                      Setpoint: {view.activeSetpointLabel ?? "—"} · Uptime:{" "}
                      {formatUptimeSeconds(view.uptimeSeconds)}
                    </div>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      className="ll-button ll-button-sm ll-button-outline"
                      onClick={() => openToolDrawer("presets")}
                    >
                      {t("dashboard.controls.presetsButton")}
                    </button>
                    <button
                      type="button"
                      className="ll-button ll-button-sm ll-button-outline"
                      onClick={() => openToolDrawer("pd")}
                    >
                      USB-PD
                    </button>
                    <button
                      type="button"
                      className="ll-button ll-button-sm ll-button-outline"
                      onClick={() => openToolDrawer("advanced")}
                    >
                      {t("dashboard.controls.advancedButton")}
                    </button>
                  </div>
                </div>

                <LiveControlPanel
                  deviceId={deviceId}
                  baseUrl={baseUrl}
                  activePresetId={view.control?.active_preset_id ?? null}
                  preset={view.activePresetDraft}
                  availableModes={view.availableModes}
                  cpSupported={view.cpSupported}
                  outputEnabled={view.control?.output_enabled ?? false}
                  outputToggleDisabled={view.outputToggleDisabled}
                  showOutputReenableHint={mutation.showOutputReenableHint}
                  savePending={mutation.updatePresetMutation.isPending}
                  saveError={
                    mutation.updatePresetMutation.isError &&
                    mutation.updatePresetMutation.error
                      ? isHttpApiError(mutation.updatePresetMutation.error)
                        ? `${mutation.updatePresetMutation.error.code ?? "HTTP_ERROR"} — ${mutation.updatePresetMutation.error.message}${mutation.explainHttpError(mutation.updatePresetMutation.error) ? `\n${mutation.explainHttpError(mutation.updatePresetMutation.error)}` : ""}`
                        : mutation.updatePresetMutation.error instanceof Error
                          ? mutation.updatePresetMutation.error.message
                          : t("dashboard.errors.unknown")
                      : null
                  }
                  actionNotice={view.presetActionNotice}
                  onOutputToggle={(nextEnabled) => {
                    if (nextEnabled) {
                      mutation.setShowOutputReenableHint(false);
                    }
                    mutation.updateControlMutation.mutate({
                      output_enabled: nextEnabled,
                    });
                  }}
                  onSaveDraft={(presetId, draft) =>
                    mutation.savePresetDraft(presetId, draft)
                  }
                />

                {mutation.updateControlMutation.isError &&
                mutation.updateControlMutation.error ? (
                  <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
                    {isHttpApiError(mutation.updateControlMutation.error)
                      ? `${mutation.updateControlMutation.error.code ?? "HTTP_ERROR"} — ${mutation.updateControlMutation.error.message}`
                      : mutation.updateControlMutation.error instanceof Error
                        ? mutation.updateControlMutation.error.message
                        : t("dashboard.errors.unknown")}
                  </div>
                ) : null}

                <div className="sr-only">
                  <div data-testid="control-active-preset">
                    Active preset: {view.control?.active_preset_id ?? "—"}
                  </div>
                  <div data-testid="control-active-mode">
                    Active mode: {view.control?.preset.mode ?? "—"}
                  </div>
                  <div data-testid="control-output-enabled">
                    Output enabled:{" "}
                    {view.control?.output_enabled ? "true" : "false"}
                  </div>
                  <div data-testid="control-uv-latched">
                    UV latched: {view.control?.uv_latched ? "true" : "false"}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </PageContainer>
  );
}

export default DeviceCcRoute;

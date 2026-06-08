import { useState } from "react";
import {
  ENABLE_MOCK_DEVTOOLS,
  isHttpApiError,
  isMockBaseUrl,
} from "../api/client.ts";
import { ControlModePanel } from "../components/instrument/control-mode-panel.tsx";
import { DiagnosticsPanel } from "../components/instrument/diagnostics-panel.tsx";
import { formatUptimeSeconds } from "../components/instrument/format.ts";
import { HealthTiles } from "../components/instrument/health-tiles.tsx";
import { InstrumentStatusBar } from "../components/instrument/instrument-status-bar.tsx";
import { LimitsPanel } from "../components/instrument/limits-panel.tsx";
import { MainDisplayPanel } from "../components/instrument/main-display-panel.tsx";
import { MonitorReadouts } from "../components/instrument/monitor-readouts.tsx";
import { PdSummaryPanel } from "../components/instrument/pd-summary-panel.tsx";
import { PresetsPanel } from "../components/instrument/presets-panel.tsx";
import { SetpointsPanel } from "../components/instrument/setpoints-panel.tsx";
import { ThermalPanel } from "../components/instrument/thermal-panel.tsx";
import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import { usePageVisibility } from "../lib/page-visibility.ts";
import { AdvancedControlsPanel } from "./device-cc/advanced-controls-panel.tsx";
import { useDeviceCcState } from "./device-cc/use-device-cc-state.ts";

export function DeviceCcRoute() {
  const { deviceId, device, baseUrl } = useDeviceContext();
  const isPageVisible = usePageVisibility();
  const [advancedCollapsed, setAdvancedCollapsed] = useState(true);

  const { view, mutation } = useDeviceCcState(deviceId, baseUrl, isPageVisible);

  return (
    <PageContainer variant="full" className="font-mono tabular-nums">
      <div className="instrument-viewport rounded-[28px] p-4 sm:p-6 md:p-8">
        <div className="mx-auto max-w-[1600px]">
          <div className="instrument-frame p-3 sm:p-4 md:p-5">
            <div className="instrument-frame-inner p-4 sm:p-5 md:p-6">
              <InstrumentStatusBar
                deviceName={device.name}
                deviceIp={view.identity?.network.ip ?? null}
                firmwareVersion={view.identity?.digital_fw_version ?? null}
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
                  aria-label="HTTP error"
                  className="mt-5 rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200"
                >
                  <div className="font-semibold">
                    HTTP error: {view.topError.summary}
                  </div>
                  {view.topError.hint ? (
                    <div className="mt-1 text-red-200/80">
                      {view.topError.hint}
                    </div>
                  ) : null}
                  {view.isLinkDownLike ? (
                    <div className="mt-1 text-red-200/70">
                      Link down / Wi-Fi unavailable - telemetry and control
                      updates may be stale until connectivity recovers.
                    </div>
                  ) : null}
                </section>
              ) : null}

              <div className="mt-6 grid grid-cols-1 gap-6 xl:grid-cols-[3fr_2fr] xl:items-start">
                <div className="flex min-w-0 flex-col gap-6">
                  <MonitorReadouts
                    voltage={{
                      read: view.localVoltageV,
                      local: view.localVoltageV,
                      remote: view.remoteVoltageV,
                    }}
                    current={{
                      read: view.totalCurrentA,
                      local: view.localCurrentA,
                      remote: view.remoteCurrentA,
                    }}
                    power={{ read: view.totalPowerW }}
                    resistance={{ read: view.resistanceOhms }}
                  />

                  <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <MainDisplayPanel
                      headline={view.headline}
                      modeLabel={view.activeLoadModeBadge}
                      setpointLabel={view.activeSetpointLabel ?? "—"}
                      uptimeLabel={formatUptimeSeconds(view.uptimeSeconds)}
                      trend={{
                        points: view.activeTrendPoints,
                        min: view.trendMin - view.trendPad,
                        max: view.trendMax + view.trendPad,
                      }}
                      stale={view.telemetryStale}
                    />
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
                  </div>

                  <HealthTiles
                    analogState={view.analogState}
                    faultLabel={
                      view.faultList.length > 0
                        ? "FAULT"
                        : view.control?.uv_latched
                          ? "UV_LATCH"
                          : view.remoteActive
                            ? "OK"
                            : "LINK_DOWN"
                    }
                    linkLatencyMs={null}
                  />

                  <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <PdSummaryPanel
                      visible={view.pdPanel.visible}
                      contractText={view.pdPanel.contractText}
                      ppsText={view.pdPanel.ppsText}
                      savedText={view.pdPanel.savedText}
                    />
                    <DiagnosticsPanel
                      analogLinkText={view.diagnostics.analogLinkText}
                      loopText={view.diagnostics.loopText}
                      lastApplyText={view.diagnostics.lastApplyText}
                      to={{ to: "/$deviceId/status", params: { deviceId } }}
                    />
                  </div>
                </div>

                <div className="flex min-w-0 flex-col gap-6">
                  <ControlModePanel
                    availableModes={view.availableModes}
                    activeMode={view.draftModeLabel}
                    onModeChange={(mode) =>
                      mutation.setDraftPresetMode(
                        mode.toLowerCase() as "cc" | "cv" | "cp",
                      )
                    }
                    outputEnabled={view.control?.output_enabled ?? false}
                    outputToggleDisabled={view.outputToggleDisabled}
                    onOutputToggle={(nextEnabled) => {
                      if (nextEnabled) {
                        mutation.setShowOutputReenableHint(false);
                      }
                      mutation.updateControlMutation.mutate({
                        output_enabled: nextEnabled,
                      });
                    }}
                    outputHint={
                      view.control?.output_enabled
                        ? "Apply preset turns output off"
                        : "Toggle on to start the load"
                    }
                    showOutputReenableHint={mutation.showOutputReenableHint}
                  />

                  {mutation.updateControlMutation.isError &&
                  mutation.updateControlMutation.error ? (
                    <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
                      {isHttpApiError(mutation.updateControlMutation.error)
                        ? `${mutation.updateControlMutation.error.code ?? "HTTP_ERROR"} — ${mutation.updateControlMutation.error.message}`
                        : mutation.updateControlMutation.error instanceof Error
                          ? mutation.updateControlMutation.error.message
                          : "Unknown error"}
                    </div>
                  ) : null}

                  <SetpointsPanel setpoints={view.setpoints} />
                  <LimitsPanel limits={view.limits} />

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

                  <PresetsPanel
                    presets={view.presetsButtons}
                    selectedPresetId={mutation.selectedPresetId}
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
                  />

                  <AdvancedControlsPanel
                    collapsed={advancedCollapsed}
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
                            : "Unknown error"
                        : null
                    }
                    applyError={
                      mutation.applyPresetMutation.isError &&
                      mutation.applyPresetMutation.error
                        ? isHttpApiError(mutation.applyPresetMutation.error)
                          ? `${mutation.applyPresetMutation.error.code ?? "HTTP_ERROR"} — ${mutation.applyPresetMutation.error.message}${mutation.explainHttpError(mutation.applyPresetMutation.error) ? `\n${mutation.explainHttpError(mutation.applyPresetMutation.error)}` : ""}`
                          : mutation.applyPresetMutation.error instanceof Error
                            ? mutation.applyPresetMutation.error.message
                            : "Unknown error"
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

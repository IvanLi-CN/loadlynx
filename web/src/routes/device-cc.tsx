import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import {
  __debugSetUvLatched,
  applyPreset,
  ENABLE_MOCK_DEVTOOLS,
  getControl,
  getIdentity,
  getPd,
  getPresets,
  getStatus,
  isHttpApiError,
  isMockBaseUrl,
  subscribeStatusStream,
  updateControl,
  updatePreset,
} from "../api/client.ts";
import type {
  AnalogState,
  ControlView,
  FastStatusView,
  Identity,
  LoadMode,
  PdView,
  Preset,
  PresetId,
} from "../api/types.ts";
import {
  formatUptimeSeconds,
  formatWithUnit,
} from "../components/instrument/format.ts";
import { AdvancedPanel } from "../components/instrument/advanced-panel.tsx";
import { ControlModePanel } from "../components/instrument/control-mode-panel.tsx";
import { DiagnosticsPanel } from "../components/instrument/diagnostics-panel.tsx";
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
import {
  isSevenSegPixel,
  sevenSegFontCharCount,
  sevenSegFontFirstChar,
  sevenSegFontHeight,
  sevenSegFontWidth,
} from "../fonts/sevenSegFont.ts";
import {
  isSmallFontPixel,
  smallFontHeight,
  smallFontWidth,
} from "../fonts/smallFont.ts";
import { useDeviceContext } from "../layouts/device-layout.tsx";

const FAST_STATUS_REFETCH_MS = 400;
const PD_REFETCH_MS = 1500;
const RETRY_DELAY_MS = 500;
const jitterRetryDelay = () => 200 + Math.random() * 300;
const TREND_MAX_POINTS = 96;

function pushTrendPoint(prev: number[], value: number | null): number[] {
  if (value == null || !Number.isFinite(value)) {
    return prev;
  }
  const next = prev.length >= TREND_MAX_POINTS ? prev.slice(1) : prev.slice();
  next.push(value);
  return next;
}

export function DeviceCcRoute() {
  const { deviceId, device, baseUrl } = useDeviceContext();

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

  const [streamStatus, setStreamStatus] = useState<FastStatusView | null>(null);

  useEffect(() => {
    // Reset SSE state when switching devices or URLs.
    if (baseUrl === undefined) {
      setStreamStatus(null);
      return;
    }
    setStreamStatus(null);
  }, [baseUrl]);

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

  const controlQuery = useQuery<ControlView, HttpApiError>({
    queryKey: ["device", deviceId, "control"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getControl(baseUrl);
    },
    enabled: Boolean(baseUrl) && identityQuery.isSuccess,
    retryDelay: RETRY_DELAY_MS,
  });

  const presetsQuery = useQuery<Preset[], HttpApiError>({
    queryKey: ["device", deviceId, "presets"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getPresets(baseUrl);
    },
    enabled: Boolean(baseUrl) && identityQuery.isSuccess,
    retryDelay: RETRY_DELAY_MS,
  });

  const queryClient = useQueryClient();

  const [selectedPresetId, setSelectedPresetId] = useState<PresetId>(1);
  const selectedPresetInitializedRef = useRef(false);
  const applyOutputWasEnabledRef = useRef(false);
  const [showOutputReenableHint, setShowOutputReenableHint] = useState(false);
  const [advancedCollapsed, setAdvancedCollapsed] = useState(true);

  const [draftPresetMode, setDraftPresetMode] = useState<LoadMode>("cc");
  const [draftPresetTargetIMa, setDraftPresetTargetIMa] = useState(0);
  const [draftPresetTargetVMv, setDraftPresetTargetVMv] = useState(0);
  const [draftPresetTargetPMw, setDraftPresetTargetPMw] = useState(0);
  const [draftPresetMinVMv, setDraftPresetMinVMv] = useState(0);
  const [draftPresetMaxIMaTotal, setDraftPresetMaxIMaTotal] = useState(0);
  const [draftPresetMaxPMw, setDraftPresetMaxPMw] = useState(0);

  useEffect(() => {
    const control = controlQuery.data;
    if (!control || selectedPresetInitializedRef.current) {
      return;
    }
    selectedPresetInitializedRef.current = true;
    setSelectedPresetId(control.active_preset_id);
  }, [controlQuery.data]);

  useEffect(() => {
    const presets = presetsQuery.data;
    if (!presets) {
      return;
    }
    const preset = presets.find((p) => p.preset_id === selectedPresetId);
    if (!preset) {
      return;
    }
    setDraftPresetMode(preset.mode);
    setDraftPresetTargetIMa(preset.target_i_ma);
    setDraftPresetTargetVMv(preset.target_v_mv);
    setDraftPresetTargetPMw(preset.target_p_mw);
    setDraftPresetMinVMv(preset.min_v_mv);
    setDraftPresetMaxIMaTotal(preset.max_i_ma_total);
    setDraftPresetMaxPMw(preset.max_p_mw);
  }, [presetsQuery.data, selectedPresetId]);

  const updatePresetMutation = useMutation({
    mutationFn: async (payload: Preset) => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return updatePreset(baseUrl, payload);
    },
    onSuccess: (nextPreset) => {
      queryClient.setQueryData<Preset[]>(
        ["device", deviceId, "presets"],
        (prev) => {
          const prevList = prev ?? [];
          const next = prevList.slice();
          const idx = next.findIndex(
            (preset) => preset.preset_id === nextPreset.preset_id,
          );
          if (idx >= 0) {
            next[idx] = nextPreset;
          } else {
            next.push(nextPreset);
            next.sort((a, b) => a.preset_id - b.preset_id);
          }
          return next;
        },
      );
      queryClient.invalidateQueries({
        queryKey: ["device", deviceId, "control"],
      });
      queryClient.invalidateQueries({
        queryKey: ["device", deviceId, "status"],
      });
    },
  });

  const applyPresetMutation = useMutation({
    mutationFn: async (presetId: PresetId) => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return applyPreset(baseUrl, presetId);
    },
    onSuccess: (nextControl) => {
      queryClient.setQueryData<ControlView>(
        ["device", deviceId, "control"],
        nextControl,
      );
      setSelectedPresetId(nextControl.active_preset_id);
      if (applyOutputWasEnabledRef.current && !nextControl.output_enabled) {
        setShowOutputReenableHint(true);
      } else {
        setShowOutputReenableHint(false);
      }
      queryClient.invalidateQueries({
        queryKey: ["device", deviceId, "status"],
      });
    },
  });

  const updateControlMutation = useMutation({
    mutationFn: async (payload: { output_enabled: boolean }) => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return updateControl(baseUrl, payload);
    },
    onSuccess: (nextControl) => {
      queryClient.setQueryData<ControlView>(
        ["device", deviceId, "control"],
        nextControl,
      );
      queryClient.invalidateQueries({
        queryKey: ["device", deviceId, "status"],
      });
    },
  });

  const writesInFlight =
    updatePresetMutation.isPending ||
    applyPresetMutation.isPending ||
    updateControlMutation.isPending;

  const pdQuery = useQuery<PdView, HttpApiError>({
    queryKey: ["device", deviceId, "pd"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getPd(baseUrl);
    },
    enabled: Boolean(baseUrl) && identityQuery.isSuccess && !writesInFlight,
    refetchInterval: isPageVisible ? PD_REFETCH_MS : false,
    refetchIntervalInBackground: false,
    retryDelay: RETRY_DELAY_MS,
  });

  const debugUvMutation = useMutation({
    mutationFn: async (uv_latched: boolean) => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return __debugSetUvLatched(baseUrl, uv_latched);
    },
    onSuccess: (nextControl) => {
      queryClient.setQueryData<ControlView>(
        ["device", deviceId, "control"],
        nextControl,
      );
    },
  });

  const statusQuery = useQuery<FastStatusView, HttpApiError>({
    queryKey: ["device", deviceId, "status"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getStatus(baseUrl);
    },
    // Pause polling while a write is in flight to avoid exhausting tiny
    // connection limits on the device HTTP stack.
    enabled:
      Boolean(baseUrl) &&
      identityQuery.isSuccess &&
      !writesInFlight &&
      streamStatus === null,
    refetchInterval: isPageVisible ? FAST_STATUS_REFETCH_MS : false,
    refetchIntervalInBackground: false,
    retry: 2,
    retryDelay: jitterRetryDelay,
  });

  useEffect(() => {
    if (!baseUrl || !identityQuery.isSuccess) {
      return undefined;
    }

    let cancelled = false;
    const unsubscribe = subscribeStatusStream(
      baseUrl,
      (view) => {
        if (!cancelled) {
          setStreamStatus(view);
        }
      },
      () => {
        if (!cancelled) {
          // Allow EventSource to auto-retry; keep polling alive until we
          // receive the next message.
          setStreamStatus(null);
        }
      },
    );

    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [baseUrl, identityQuery.isSuccess]);

  const firstHttpError: HttpApiError | null = (() => {
    const errors: Array<unknown> = [
      identityQuery.error,
      statusQuery.error,
      controlQuery.error,
      presetsQuery.error,
    ];
    for (const err of errors) {
      if (isHttpApiError(err)) {
        return err;
      }
    }
    return null;
  })();

  const topError = (() => {
    if (!firstHttpError) {
      return null;
    }
    const code = firstHttpError.code ?? "HTTP_ERROR";
    const summary = `${code} — ${firstHttpError.message}`;

    if (firstHttpError.status === 0 && code === "NETWORK_ERROR") {
      const hint =
        "可能是短暂的网络抖动，已自动重试" +
        (baseUrl ? `（baseUrl=${baseUrl}）` : "") +
        "；若仍无法连接，请检查网络与 IP 设置。";
      return { summary, hint } as const;
    }

    if (firstHttpError.status === 404 && code === "UNSUPPORTED_OPERATION") {
      const hint = "固件版本不支持该 API，请升级固件后重试。";
      return { summary, hint } as const;
    }

    return { summary, hint: null } as const;
  })();

  const isLinkDownLike =
    firstHttpError &&
    (firstHttpError.code === "LINK_DOWN" ||
      firstHttpError.code === "UNAVAILABLE");

  const identity = identityQuery.data;
  const status = streamStatus ?? statusQuery.data;
  const control = controlQuery.data;
  const pd = pdQuery.data ?? null;

  const activeLoadModeBadge =
    control?.preset.mode === "cv"
      ? "CV"
      : control?.preset.mode === "cp"
        ? "CP"
        : "CC";

  const statusLocalMa = status?.raw.i_local_ma ?? null;
  const statusRemoteMa = status?.raw.i_remote_ma ?? null;

  const remoteVoltageV =
    status?.raw.v_remote_mv != null ? status.raw.v_remote_mv / 1_000 : null;
  const localVoltageV =
    status?.raw.v_local_mv != null
      ? status.raw.v_local_mv / 1_000
      : remoteVoltageV;
  const localCurrentA = statusLocalMa != null ? statusLocalMa / 1_000 : null;
  const remoteCurrentA = statusRemoteMa != null ? statusRemoteMa / 1_000 : null;
  const totalCurrentA =
    statusLocalMa != null && statusRemoteMa != null
      ? (statusLocalMa + statusRemoteMa) / 1_000
      : null;
  const totalPowerW =
    status?.raw.calc_p_mw != null ? status.raw.calc_p_mw / 1_000 : null;

  const uptimeSeconds =
    status?.raw.uptime_ms != null
      ? Math.floor(status.raw.uptime_ms / 1_000)
      : null;
  const tempCoreC =
    status?.raw.sink_core_temp_mc != null
      ? status.raw.sink_core_temp_mc / 1_000
      : null;
  const tempSinkC =
    status?.raw.sink_exhaust_temp_mc != null
      ? status.raw.sink_exhaust_temp_mc / 1_000
      : null;
  const tempMcuC =
    status?.raw.mcu_temp_mc != null ? status.raw.mcu_temp_mc / 1_000 : null;

  const controlMode: LoadMode = control?.preset.mode ?? "cc";
  const controlTargetMilli =
    controlMode === "cv"
      ? (control?.preset.target_v_mv ?? 0)
      : controlMode === "cp"
        ? (control?.preset.target_p_mw ?? 0)
        : (control?.preset.target_i_ma ?? 0);
  const controlTargetUnit =
    controlMode === "cv" ? "V" : controlMode === "cp" ? "W" : "A";
  const remoteActive = status?.link_up === true;
  const analogState = status?.analog_state ?? "offline";
  const faultFlags = status?.raw.fault_flags ?? 0;

  const resistanceOhms =
    localVoltageV != null && totalCurrentA != null && totalCurrentA > 0.0001
      ? localVoltageV / totalCurrentA
      : null;

  const lastTrendUptimeMsRef = useRef<number | null>(null);
  const [trend, setTrend] = useState<{
    v: number[];
    i: number[];
    p: number[];
    t: number[];
  }>({ v: [], i: [], p: [], t: [] });

  useEffect(() => {
    const uptimeMs = status?.raw.uptime_ms ?? null;
    if (uptimeMs == null) {
      return;
    }
    if (lastTrendUptimeMsRef.current === uptimeMs) {
      return;
    }
    lastTrendUptimeMsRef.current = uptimeMs;

    setTrend((prev) => ({
      v: pushTrendPoint(prev.v, localVoltageV),
      i: pushTrendPoint(prev.i, totalCurrentA),
      p: pushTrendPoint(prev.p, totalPowerW),
      t: pushTrendPoint(prev.t, tempCoreC),
    }));
  }, [
    localVoltageV,
    status?.raw.uptime_ms,
    totalCurrentA,
    totalPowerW,
    tempCoreC,
  ]);

  const handleSavePreset = () => {
    const payload: Preset = {
      preset_id: selectedPresetId,
      mode: draftPresetMode,
      target_i_ma: draftPresetTargetIMa,
      target_v_mv: draftPresetTargetVMv,
      target_p_mw: draftPresetTargetPMw,
      min_v_mv: draftPresetMinVMv,
      max_i_ma_total: draftPresetMaxIMaTotal,
      max_p_mw: draftPresetMaxPMw,
    };

    updatePresetMutation.mutate(payload);
  };

  const handleApplyPreset = () => {
    applyOutputWasEnabledRef.current = control?.output_enabled ?? false;
    setShowOutputReenableHint(false);
    applyPresetMutation.mutate(selectedPresetId);
  };

  const cpSupported = identity?.capabilities.cp_supported ?? false;

  const cpDraftOutOfRange =
    draftPresetMode === "cp" && draftPresetTargetPMw > draftPresetMaxPMw;

  const savePresetDisabled =
    !baseUrl || updatePresetMutation.isPending || cpDraftOutOfRange;

  const explainHttpError = (error: HttpApiError): string | null => {
    switch (error.code) {
      case "LINK_DOWN":
        return "UART 链路掉线：请检查 analog↔digital 连接与供电。";
      case "ANALOG_NOT_READY":
        return "Analog 未就绪：常见原因是校准缺失或仍在初始化。";
      case "ANALOG_FAULTED":
        return "Analog 处于故障态：请先排查过流/过温/硬件异常。";
      case "LIMIT_VIOLATION":
        return "输入超出限值：例如 CP 模式需要 target_p_mw <= max_p_mw。";
      default:
        return null;
    }
  };

  const telemetryStale = Boolean(firstHttpError);

  const faultList = status?.fault_flags_decoded ?? [];
  const faultSummary = faultList.length > 0 ? faultList.join(", ") : null;

  const linkState: "up" | "down" | "unknown" = status
    ? status.link_up
      ? "up"
      : "down"
    : "unknown";

  const protectionState = (() => {
    if (faultList.length > 0 || analogState === "faulted") {
      return { summary: "FAULT", level: "danger" } as const;
    }
    if (analogState === "cal_missing") {
      return { summary: "CAL_MISSING", level: "warn" } as const;
    }
    if (control?.uv_latched) {
      return { summary: "UV_LATCH", level: "warn" } as const;
    }
    if (!remoteActive) {
      return { summary: "LINK_DOWN", level: "warn" } as const;
    }
    return { summary: "OK", level: "ok" } as const;
  })();

  const activeSetpointLabel = (() => {
    if (!control) {
      return null;
    }
    if (controlMode === "cv") {
      return `${(control.preset.target_v_mv / 1000).toFixed(3)} V`;
    }
    if (controlMode === "cp") {
      return `${(control.preset.target_p_mw / 1000).toFixed(2)} W`;
    }
    return `${(control.preset.target_i_ma / 1000).toFixed(3)} A`;
  })();

  const headline = (() => {
    if (controlMode === "cv") {
      return { value: localVoltageV, unit: "V" as const };
    }
    if (controlMode === "cp") {
      return { value: totalPowerW, unit: "W" as const };
    }
    return { value: totalCurrentA, unit: "A" as const };
  })();

  const activeTrendPoints =
    controlMode === "cv" ? trend.v : controlMode === "cp" ? trend.p : trend.i;
  const trendMin =
    activeTrendPoints.length > 0 ? Math.min(...activeTrendPoints) : 0;
  const trendMax =
    activeTrendPoints.length > 0 ? Math.max(...activeTrendPoints) : 1;
  const trendPad = trendMax > trendMin ? (trendMax - trendMin) * 0.05 : 1;

  const thermalTrendPoints = trend.t;
  const thermalTrendMin =
    thermalTrendPoints.length > 0 ? Math.min(...thermalTrendPoints) : 0;
  const thermalTrendMax =
    thermalTrendPoints.length > 0 ? Math.max(...thermalTrendPoints) : 1;
  const thermalTrendPad =
    thermalTrendMax > thermalTrendMin
      ? (thermalTrendMax - thermalTrendMin) * 0.05
      : 1;

  const pdPanel = (() => {
    if (pd) {
      const attached = pd.attached;
      const contractText =
        attached && pd.contract_mv != null && pd.contract_ma != null
          ? `Contract: ${(pd.contract_mv / 1000).toFixed(1)} V @ ${pd.contract_ma} mA`
          : attached
            ? "Contract: unknown"
            : "Contract: detached";

      const ppsText = (() => {
        const first = pd.pps_pdos[0];
        if (!first) {
          return "PPS: —";
        }
        return `PPS: ${(first.min_mv / 1000).toFixed(1)}–${(first.max_mv / 1000).toFixed(1)}V (${pd.pps_pdos.length} APDO)`;
      })();

      const savedText =
        pd.saved.mode === "fixed"
          ? `Saved: Fixed · PDO #${pd.saved.fixed_object_pos} · ${pd.saved.i_req_ma} mA`
          : `Saved: PPS · APDO #${pd.saved.pps_object_pos} · ${pd.saved.target_mv} mV · ${pd.saved.i_req_ma} mA`;

      return {
        visible: true,
        contractText,
        ppsText,
        savedText,
      } as const;
    }

    const err = pdQuery.error;
    if (
      err &&
      isHttpApiError(err) &&
      err.status === 404 &&
      err.code === "UNSUPPORTED_OPERATION"
    ) {
      return {
        visible: true,
        contractText: "PD: unsupported",
        ppsText: null,
        savedText: null,
      } as const;
    }

    return {
      visible: true,
      contractText: "Contract: —",
      ppsText: "PPS: —",
      savedText: "Saved: —",
    } as const;
  })();

  const availableModes: Array<"CC" | "CV" | "CP" | "CR"> = ["CC", "CV"];
  if (cpSupported) {
    availableModes.push("CP");
  }

  const draftModeLabel: "CC" | "CV" | "CP" | "CR" =
    draftPresetMode === "cc" ? "CC" : draftPresetMode === "cv" ? "CV" : "CP";

  const presetsButtons = Array.from({ length: 8 }, (_, idx) => {
    const id = idx + 1;
    const supported = id >= 1 && id <= 5;
    return {
      id,
      label: `#${id}`,
      active: supported ? control?.active_preset_id === id : false,
      disabled: !supported,
    };
  });

  const outputToggleDisabled =
    !control || updateControlMutation.isPending || applyPresetMutation.isPending;

  const setpoints = [
    {
      label: "Target Current",
      value: formatWithUnit(draftPresetTargetIMa / 1000, 3, "A"),
      readback: `Read: ${formatWithUnit(totalCurrentA, 3, "A")}`,
      active: draftPresetMode === "cc",
    },
    {
      label: "Target Voltage",
      value: formatWithUnit(draftPresetTargetVMv / 1000, 3, "V"),
      readback: `Read: ${formatWithUnit(localVoltageV, 3, "V")}`,
      active: draftPresetMode === "cv",
    },
  ];

  const limits = [
    {
      label: "Min Voltage",
      value: control ? formatWithUnit(draftPresetMinVMv / 1000, 3, "V") : "— V",
      tone: control?.uv_latched ? ("warn" as const) : ("ok" as const),
    },
    {
      label: "Max Current",
      value: control
        ? formatWithUnit(draftPresetMaxIMaTotal / 1000, 3, "A")
        : "— A",
    },
    {
      label: "Max Power",
      value: control ? formatWithUnit(draftPresetMaxPMw / 1000, 2, "W") : "— W",
    },
    {
      label: "UV Latch",
      value: control ? (control.uv_latched ? "Latched" : "Ready") : "—",
      tone: control?.uv_latched ? ("warn" as const) : ("ok" as const),
    },
  ];

  const diagnostics = {
    analogLinkText: `Analog Link: ${remoteActive ? "Stable" : "Down"}`,
    loopText: "Loop: —",
    lastApplyText: "Last Apply: —",
  } as const;

  return (
    <PageContainer variant="full" className="font-mono tabular-nums">
      <div className="instrument-viewport rounded-[28px] p-4 sm:p-6 md:p-8">
        <div className="mx-auto max-w-[1600px]">
          <div className="instrument-frame p-3 sm:p-4 md:p-5">
            <div className="instrument-frame-inner p-4 sm:p-5 md:p-6">
              <InstrumentStatusBar
                deviceName={device.name}
                deviceIp={identity?.network.ip ?? null}
                firmwareVersion={identity?.digital_fw_version ?? null}
                modeLabel={
                  activeLoadModeBadge as "CC" | "CV" | "CP" | "CR" | "UNKNOWN"
                }
                linkState={linkState}
                outputState={{
                  enabled: control?.output_enabled ?? false,
                  setpointLabel: activeSetpointLabel,
                }}
                protectionState={protectionState}
                faultSummary={faultSummary}
                stale={telemetryStale}
              />

              {topError ? (
                <section
                  aria-label="HTTP error"
                  className="mt-5 rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200"
                >
                  <div className="font-semibold">
                    HTTP error: {topError.summary}
                  </div>
                  {topError.hint ? (
                    <div className="mt-1 text-red-200/80">{topError.hint}</div>
                  ) : null}
                  {isLinkDownLike ? (
                    <div className="mt-1 text-red-200/70">
                      Link down / Wi‑Fi unavailable — telemetry and control
                      updates may be stale until connectivity recovers.
                    </div>
                  ) : null}
                </section>
              ) : null}

              <div className="mt-6 grid grid-cols-1 gap-6 xl:grid-cols-[3fr_2fr] xl:items-start">
                <div className="flex min-w-0 flex-col gap-6">
                  <MonitorReadouts
                    voltage={{
                      read: localVoltageV,
                      local: localVoltageV,
                      remote: remoteVoltageV,
                    }}
                    current={{
                      read: totalCurrentA,
                      local: localCurrentA,
                      remote: remoteCurrentA,
                    }}
                    power={{ read: totalPowerW }}
                    resistance={{ read: resistanceOhms }}
                  />

                  <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <MainDisplayPanel
                      headline={headline}
                      modeLabel={activeLoadModeBadge as "CC" | "CV" | "CP" | "CR"}
                      setpointLabel={activeSetpointLabel ?? "—"}
                      uptimeLabel={formatUptimeSeconds(uptimeSeconds)}
                      trend={{
                        points: activeTrendPoints,
                        min: trendMin - trendPad,
                        max: trendMax + trendPad,
                      }}
                      stale={telemetryStale}
                    />
                    <ThermalPanel
                      sinkCoreC={tempCoreC}
                      sinkExhaustC={tempSinkC}
                      mcuC={tempMcuC}
                      faults={faultList}
                      trend={{
                        points: thermalTrendPoints,
                        min: thermalTrendMin - thermalTrendPad,
                        max: thermalTrendMax + thermalTrendPad,
                      }}
                    />
                  </div>

                  <HealthTiles
                    analogState={analogState}
                    faultLabel={
                      faultList.length > 0
                        ? "FAULT"
                        : control?.uv_latched
                          ? "UV_LATCH"
                          : remoteActive
                            ? "OK"
                            : "LINK_DOWN"
                    }
                    linkLatencyMs={null}
                  />

                  <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <PdSummaryPanel
                      visible={pdPanel.visible}
                      contractText={pdPanel.contractText}
                      ppsText={pdPanel.ppsText}
                      savedText={pdPanel.savedText}
                    />
                    <DiagnosticsPanel
                      analogLinkText={diagnostics.analogLinkText}
                      loopText={diagnostics.loopText}
                      lastApplyText={diagnostics.lastApplyText}
                      to={{ to: "/$deviceId/status", params: { deviceId } }}
                    />
                  </div>
                </div>

                <div className="flex min-w-0 flex-col gap-6">
                  <ControlModePanel
                    availableModes={availableModes}
                    activeMode={draftModeLabel}
                    onModeChange={(mode) => {
                      if (mode === "CR") {
                        return;
                      }
                      setDraftPresetMode(mode.toLowerCase() as LoadMode);
                    }}
                    outputEnabled={control?.output_enabled ?? false}
                    outputToggleDisabled={outputToggleDisabled}
                    onOutputToggle={(nextEnabled) => {
                      if (nextEnabled) {
                        setShowOutputReenableHint(false);
                      }
                      updateControlMutation.mutate({ output_enabled: nextEnabled });
                    }}
                    outputHint={
                      control?.output_enabled
                        ? "Apply preset turns output off"
                        : "Toggle on to start the load"
                    }
                    showOutputReenableHint={showOutputReenableHint}
                  />

                  {updateControlMutation.isError && updateControlMutation.error ? (
                    <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
                      {isHttpApiError(updateControlMutation.error)
                        ? `${updateControlMutation.error.code ?? "HTTP_ERROR"} — ${updateControlMutation.error.message}`
                        : updateControlMutation.error instanceof Error
                          ? updateControlMutation.error.message
                          : "Unknown error"}
                    </div>
                  ) : null}

                  <SetpointsPanel setpoints={setpoints} />
                  <LimitsPanel limits={limits} />

                  <div className="sr-only">
                    <div data-testid="control-active-preset">
                      Active preset: {control?.active_preset_id ?? "—"}
                    </div>
                    <div data-testid="control-active-mode">
                      Active mode: {control?.preset.mode ?? "—"}
                    </div>
                    <div data-testid="control-output-enabled">
                      Output enabled: {control?.output_enabled ? "true" : "false"}
                    </div>
                    <div data-testid="control-uv-latched">
                      UV latched: {control?.uv_latched ? "true" : "false"}
                    </div>
                  </div>

                  <PresetsPanel
                    presets={presetsButtons}
                    selectedPresetId={selectedPresetId}
                    onPresetSelect={(id) => {
                      if (id >= 1 && id <= 5) {
                        setSelectedPresetId(id as PresetId);
                      }
                    }}
                    onApply={handleApplyPreset}
                    onSave={handleSavePreset}
                    applyDisabled={!baseUrl || applyPresetMutation.isPending}
                    saveDisabled={savePresetDisabled}
                    applying={applyPresetMutation.isPending}
                    saving={updatePresetMutation.isPending}
                  />

                  <AdvancedPanel
                    summary="Transient · List · Battery · Trigger"
                    collapsed={advancedCollapsed}
                    onToggle={setAdvancedCollapsed}
                  >
                    <div className="grid gap-4">
                      <div>
                        <div className="instrument-label">Preset editor</div>
                        <div className="mt-3 grid gap-3">
                          <div>
                            <label
                              htmlFor="preset-mode"
                              className="block text-[11px] text-slate-200/60"
                            >
                              Mode
                            </label>
                            <select
                              id="preset-mode"
                              className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                              value={draftPresetMode}
                              onChange={(event) =>
                                setDraftPresetMode(event.target.value as LoadMode)
                              }
                            >
                              <option value="cc">cc</option>
                              <option value="cv">cv</option>
                              {cpSupported ? <option value="cp">cp</option> : null}
                            </select>
                            {identityQuery.isSuccess && !cpSupported ? (
                              <div className="mt-2 text-[11px] text-slate-200/55">
                                CP: 固件不支持（identity.capabilities.cp_supported=false）
                              </div>
                            ) : null}
                          </div>

                          {draftPresetMode === "cc" ? (
                            <div>
                              <label
                                htmlFor="preset-target-i"
                                className="block text-[11px] text-slate-200/60"
                              >
                                Target current (mA)
                              </label>
                              <input
                                id="preset-target-i"
                                type="number"
                                className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                                value={draftPresetTargetIMa}
                                onChange={(event) =>
                                  setDraftPresetTargetIMa(
                                    Number.parseInt(event.target.value || "0", 10),
                                  )
                                }
                              />
                            </div>
                          ) : draftPresetMode === "cv" ? (
                            <div>
                              <label
                                htmlFor="preset-target-v"
                                className="block text-[11px] text-slate-200/60"
                              >
                                Target voltage (mV)
                              </label>
                              <input
                                id="preset-target-v"
                                type="number"
                                className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                                value={draftPresetTargetVMv}
                                onChange={(event) =>
                                  setDraftPresetTargetVMv(
                                    Number.parseInt(event.target.value || "0", 10),
                                  )
                                }
                              />
                            </div>
                          ) : (
                            <div>
                              <label
                                htmlFor="preset-target-p"
                                className="block text-[11px] text-slate-200/60"
                              >
                                Target power (mW)
                              </label>
                              <input
                                id="preset-target-p"
                                type="number"
                                className={[
                                  "mt-1 w-full rounded-lg border bg-black/20 px-3 py-2 text-[12px] text-slate-100",
                                  cpDraftOutOfRange
                                    ? "border-red-400/25"
                                    : "border-slate-400/10",
                                ].join(" ")}
                                value={draftPresetTargetPMw}
                                onChange={(event) =>
                                  setDraftPresetTargetPMw(
                                    Number.parseInt(event.target.value || "0", 10),
                                  )
                                }
                              />
                              {cpDraftOutOfRange ? (
                                <div className="mt-2 text-[11px] text-red-200/85">
                                  target_p_mw must be ≤ max_p_mw
                                </div>
                              ) : null}
                            </div>
                          )}

                          <div>
                            <label
                              htmlFor="preset-min-v"
                              className="block text-[11px] text-slate-200/60"
                            >
                              Min voltage (mV)
                            </label>
                            <input
                              id="preset-min-v"
                              type="number"
                              className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                              value={draftPresetMinVMv}
                              onChange={(event) =>
                                setDraftPresetMinVMv(
                                  Number.parseInt(event.target.value || "0", 10),
                                )
                              }
                            />
                          </div>

                          <div>
                            <label
                              htmlFor="preset-max-i"
                              className="block text-[11px] text-slate-200/60"
                            >
                              Max current total (mA)
                            </label>
                            <input
                              id="preset-max-i"
                              type="number"
                              className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                              value={draftPresetMaxIMaTotal}
                              onChange={(event) =>
                                setDraftPresetMaxIMaTotal(
                                  Number.parseInt(event.target.value || "0", 10),
                                )
                              }
                            />
                          </div>

                          <div>
                            <label
                              htmlFor="preset-max-p"
                              className="block text-[11px] text-slate-200/60"
                            >
                              Max power (mW)
                            </label>
                            <input
                              id="preset-max-p"
                              type="number"
                              className="mt-1 w-full rounded-lg border border-slate-400/10 bg-black/20 px-3 py-2 text-[12px] text-slate-100"
                              value={draftPresetMaxPMw}
                              onChange={(event) =>
                                setDraftPresetMaxPMw(
                                  Number.parseInt(event.target.value || "0", 10),
                                )
                              }
                            />
                          </div>

                          <div className="mt-2 flex flex-col gap-2">
                            <button
                              type="button"
                              className="h-9 rounded-lg border border-amber-400/25 bg-amber-500/10 px-3 text-xs font-semibold tracking-[0.14em] text-amber-100 uppercase disabled:opacity-50"
                              disabled={savePresetDisabled}
                              onClick={handleSavePreset}
                            >
                              {updatePresetMutation.isPending ? "Saving…" : "Save Draft"}
                            </button>
                            <button
                              type="button"
                              className="h-9 rounded-lg border border-sky-400/25 bg-sky-500/10 px-3 text-xs font-semibold tracking-[0.14em] text-sky-100 uppercase disabled:opacity-50"
                              disabled={!baseUrl || applyPresetMutation.isPending}
                              onClick={handleApplyPreset}
                            >
                              {applyPresetMutation.isPending ? "Applying…" : "Apply Preset"}
                            </button>
                          </div>
                        </div>
                      </div>

                      {ENABLE_MOCK_DEVTOOLS && baseUrl && isMockBaseUrl(baseUrl) ? (
                        <button
                          type="button"
                          className="h-9 rounded-lg border border-slate-400/10 bg-black/20 px-3 text-xs font-semibold text-slate-200/70 disabled:opacity-50"
                          disabled={!control || debugUvMutation.isPending}
                          onClick={() => {
                            debugUvMutation.mutate(!(control?.uv_latched ?? false));
                          }}
                        >
                          Toggle UV latch (mock)
                        </button>
                      ) : null}

                      {updatePresetMutation.isError && updatePresetMutation.error ? (
                        <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
                          {isHttpApiError(updatePresetMutation.error)
                            ? `${updatePresetMutation.error.code ?? "HTTP_ERROR"} — ${updatePresetMutation.error.message}`
                            : updatePresetMutation.error instanceof Error
                              ? updatePresetMutation.error.message
                              : "Unknown error"}
                          {isHttpApiError(updatePresetMutation.error) &&
                          explainHttpError(updatePresetMutation.error) ? (
                            <div className="mt-1 text-red-200/70">
                              {explainHttpError(updatePresetMutation.error)}
                            </div>
                          ) : null}
                        </div>
                      ) : null}

                      {applyPresetMutation.isError && applyPresetMutation.error ? (
                        <div className="rounded-2xl border border-red-400/15 bg-red-500/10 px-4 py-3 text-[12px] text-red-200">
                          {isHttpApiError(applyPresetMutation.error)
                            ? `${applyPresetMutation.error.code ?? "HTTP_ERROR"} — ${applyPresetMutation.error.message}`
                            : applyPresetMutation.error instanceof Error
                              ? applyPresetMutation.error.message
                              : "Unknown error"}
                          {isHttpApiError(applyPresetMutation.error) &&
                          explainHttpError(applyPresetMutation.error) ? (
                            <div className="mt-1 text-red-200/70">
                              {explainHttpError(applyPresetMutation.error)}
                            </div>
                          ) : null}
                        </div>
                      ) : null}

                      <section aria-label="Hardware main display">
                        <div className="instrument-label">Hardware display</div>
                        <div className="mt-3 flex justify-center">
                          <MainDisplayCanvas
                            remoteVoltageV={remoteVoltageV ?? 0}
                            localVoltageV={localVoltageV ?? 0}
                            localCurrentA={localCurrentA ?? 0}
                            remoteCurrentA={remoteCurrentA ?? 0}
                            totalCurrentA={totalCurrentA ?? 0}
                            totalPowerW={totalPowerW ?? 0}
                            controlMode={controlMode}
                            controlTargetMilli={controlTargetMilli}
                            controlTargetUnit={controlTargetUnit}
                            uptimeSeconds={uptimeSeconds ?? 0}
                            tempCoreC={tempCoreC ?? undefined}
                            tempSinkC={tempSinkC ?? undefined}
                            tempMcuC={tempMcuC ?? undefined}
                            remoteActive={remoteActive}
                            analogState={analogState}
                            faultFlags={faultFlags}
                          />
                        </div>
                      </section>
                    </div>
                  </AdvancedPanel>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </PageContainer>
  );
}

export interface MainDisplayCanvasProps {
  remoteVoltageV: number;
  localVoltageV: number;
  localCurrentA: number;
  remoteCurrentA: number;
  totalCurrentA: number;
  totalPowerW: number;
  controlMode: LoadMode;
  controlTargetMilli: number;
  controlTargetUnit: "A" | "V" | "W";
  uptimeSeconds: number;
  tempCoreC: number | undefined;
  tempSinkC: number | undefined;
  tempMcuC: number | undefined;
  remoteActive: boolean;
  analogState: AnalogState;
  faultFlags: number;
}

export function MainDisplayCanvas({
  remoteVoltageV,
  localVoltageV,
  localCurrentA,
  remoteCurrentA,
  totalCurrentA,
  totalPowerW,
  controlMode,
  controlTargetMilli,
  controlTargetUnit,
  uptimeSeconds,
  tempCoreC,
  tempSinkC,
  tempMcuC,
  remoteActive,
  analogState,
  faultFlags,
}: MainDisplayCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [canvasSize, setCanvasSize] = useState<{
    width: number;
    height: number;
  }>({
    width: 0,
    height: 0,
  });

  // Track the rendered size of the canvas in CSS pixels so that we can
  // render at the corresponding device-pixel resolution.
  useEffect(() => {
    const handleResize = () => {
      const canvas = canvasRef.current;
      if (!canvas) {
        return;
      }
      const rect = canvas.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        setCanvasSize({
          width: rect.width,
          height: rect.height,
        });
      }
    };

    handleResize();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || canvasSize.width === 0 || canvasSize.height === 0) {
      return;
    }
    const dpr = window.devicePixelRatio ?? 1;
    const baseWidth = 320;
    const baseHeight = 240;

    const logicalWidth = Math.round(canvasSize.width * dpr);
    const logicalHeight = Math.round(canvasSize.height * dpr);
    if (canvas.width !== logicalWidth || canvas.height !== logicalHeight) {
      canvas.width = logicalWidth;
      canvas.height = logicalHeight;
    }

    const context = canvas.getContext("2d");
    if (!context) {
      return;
    }

    const ctx = context;
    const scaleX = canvas.width / baseWidth;
    const scaleY = canvas.height / baseHeight;
    ctx.setTransform(scaleX, 0, 0, scaleY, 0, 0);
    ctx.imageSmoothingEnabled = false;

    const width = baseWidth;
    const height = baseHeight;

    type Rect = { left: number; top: number; right: number; bottom: number };

    const COLOR_CANVAS = "#05070D";
    const COLOR_LEFT_BASE = "#101829";
    const COLOR_RIGHT_BASE = "#080F19";
    const CARD_TINTS = ["#171F33", "#141D2F", "#111828"] as const;

    const COLOR_CAPTION = "#9AB0D8";
    const COLOR_VOLTAGE = "#FFB347";
    const COLOR_CURRENT = "#FF5252";
    const COLOR_POWER = "#6EF58C";
    const COLOR_RIGHT_LABEL = "#6D7FA4";
    const COLOR_RIGHT_VALUE = "#DFE7FF";
    const COLOR_BAR_TRACK = "#1C2638";
    const COLOR_BAR_FILL = "#4CC9F0";

    const fillRect = (rect: Rect, color: string) => {
      const w = rect.right - rect.left;
      const h = rect.bottom - rect.top;
      if (w <= 0 || h <= 0) {
        return;
      }
      ctx.fillStyle = color;
      ctx.fillRect(rect.left, rect.top, w, h);
    };

    const drawSmallChar = (
      ch: string,
      x0: number,
      y0: number,
      color: string,
    ) => {
      if (!ch) {
        return;
      }
      const code = ch.charCodeAt(0);
      ctx.fillStyle = color;
      for (let y = 0; y < smallFontHeight; y += 1) {
        for (let x = 0; x < smallFontWidth; x += 1) {
          if (isSmallFontPixel(code, x, y)) {
            ctx.fillRect(x0 + x, y0 + y, 1, 1);
          }
        }
      }
    };

    const drawSmallText = (
      text: string,
      x: number,
      y: number,
      color: string,
      spacing: number = 0,
    ) => {
      let cursorX = x;
      for (const ch of text) {
        if (ch === " ") {
          cursorX += smallFontWidth + spacing;
          continue;
        }
        drawSmallChar(ch, cursorX, y, color);
        cursorX += smallFontWidth + spacing;
      }
    };

    const drawSevenSegDigit = (
      code: number,
      x0: number,
      y0: number,
      color: string,
    ) => {
      if (
        code < sevenSegFontFirstChar ||
        code >= sevenSegFontFirstChar + sevenSegFontCharCount
      ) {
        return;
      }
      ctx.fillStyle = color;
      for (let y = 0; y < sevenSegFontHeight; y += 1) {
        for (let x = 0; x < sevenSegFontWidth; x += 1) {
          if (isSevenSegPixel(code, x, y)) {
            ctx.fillRect(x0 + x, y0 + y, 1, 1);
          }
        }
      }
    };

    const drawSevenSegValue = (text: string, area: Rect, color: string) => {
      const spacing = 4;
      let totalWidth = 0;
      for (const ch of text) {
        totalWidth += (ch === "." ? 8 : sevenSegFontWidth) + spacing;
      }
      if (text.length > 0) {
        totalWidth -= spacing;
      }

      let cursorX = area.right - totalWidth;
      for (const ch of text) {
        if (ch === ".") {
          ctx.fillStyle = color;
          ctx.fillRect(cursorX, area.bottom - 10, 6, 6);
          cursorX += 8 + spacing;
          continue;
        }
        drawSevenSegDigit(ch.charCodeAt(0), cursorX, area.top, color);
        cursorX += sevenSegFontWidth + spacing;
      }
    };

    const fillRoundRect = (rect: Rect, radius: number, color: string) => {
      const w = rect.right - rect.left;
      const h = rect.bottom - rect.top;
      if (w <= 0 || h <= 0) {
        return;
      }
      const r = Math.max(0, Math.min(radius, w / 2, h / 2));
      ctx.fillStyle = color;
      if (r === 0) {
        ctx.fillRect(rect.left, rect.top, w, h);
        return;
      }
      ctx.beginPath();
      ctx.moveTo(rect.left + r, rect.top);
      ctx.lineTo(rect.right - r, rect.top);
      ctx.quadraticCurveTo(rect.right, rect.top, rect.right, rect.top + r);
      ctx.lineTo(rect.right, rect.bottom - r);
      ctx.quadraticCurveTo(
        rect.right,
        rect.bottom,
        rect.right - r,
        rect.bottom,
      );
      ctx.lineTo(rect.left + r, rect.bottom);
      ctx.quadraticCurveTo(rect.left, rect.bottom, rect.left, rect.bottom - r);
      ctx.lineTo(rect.left, rect.top + r);
      ctx.quadraticCurveTo(rect.left, rect.top, rect.left + r, rect.top);
      ctx.closePath();
      ctx.fill();
    };

    const clamp01 = (value: number) => Math.max(0, Math.min(1, value));

    const drawMirrorBar = (
      top: number,
      left: number,
      right: number,
      leftRatio: number,
      rightRatio: number,
    ) => {
      const barHeight = 8;
      const center = Math.floor((left + right) / 2);
      fillRect({ left, top, right, bottom: top + barHeight }, COLOR_BAR_TRACK);
      fillRect(
        {
          left: center,
          top: top - 2,
          right: center + 1,
          bottom: top + barHeight + 2,
        },
        COLOR_RIGHT_LABEL,
      );

      const halfWidth = Math.floor((right - left) / 2);
      const leftFill = Math.round(halfWidth * clamp01(leftRatio));
      const rightFill = Math.round(halfWidth * clamp01(rightRatio));
      if (leftFill > 0) {
        fillRect(
          {
            left: center - leftFill,
            top,
            right: center,
            bottom: top + barHeight,
          },
          COLOR_BAR_FILL,
        );
      }
      if (rightFill > 0) {
        fillRect(
          {
            left: center,
            top,
            right: center + rightFill,
            bottom: top + barHeight,
          },
          COLOR_BAR_FILL,
        );
      }
    };

    const formatFixed2dp = (value: number) => {
      if (!Number.isFinite(value)) {
        return "99.99";
      }
      const v = Math.abs(value);
      const scaled = Math.floor(v * 100 + 0.5);
      if (scaled > 9_999) {
        return "99.99";
      }
      const intPart = Math.floor(scaled / 100);
      const fracPart = scaled % 100;
      return `${String(intPart).padStart(2, "0")}.${String(fracPart).padStart(2, "0")}`;
    };

    const formatFixed1dp3i = (value: number) => {
      if (!Number.isFinite(value)) {
        return "999.9";
      }
      const v = Math.abs(value);
      const scaled = Math.floor(v * 10 + 0.5);
      if (scaled > 9_999) {
        return "999.9";
      }
      const intPart = Math.floor(scaled / 10);
      const fracPart = scaled % 10;
      return `${String(intPart).padStart(3, "0")}.${fracPart}`;
    };

    const formatPairValue = (value: number, unit: "V" | "A") =>
      `${formatFixed2dp(value)}${unit}`;

    const formatSetpointMilli = (valueMilli: number, unit: "V" | "A" | "W") => {
      let v = Math.max(0, Math.trunc(valueMilli));
      v = Math.floor((v + 5) / 10) * 10;
      const centi = Math.floor(v / 10);
      if (centi > 9_999) {
        return `--.--${unit}`;
      }
      const intPart = Math.floor(centi / 100);
      const fracPart = centi % 100;
      return `${String(intPart).padStart(2, "0")}.${String(fracPart).padStart(2, "0")}${unit}`;
    };

    const formatRunTime = (secs: number) => {
      const hours = Math.floor(secs / 3_600);
      const minutes = Math.floor((secs % 3_600) / 60);
      const seconds = Math.floor(secs % 60);
      return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
    };

    const formatTemp1dp = (temp: number | undefined) => {
      if (typeof temp !== "number" || !Number.isFinite(temp)) {
        return "--.-";
      }
      return temp.toFixed(1);
    };

    const formatStatusLine5 = () => {
      if (faultFlags !== 0) {
        const hex = (faultFlags >>> 0)
          .toString(16)
          .toUpperCase()
          .padStart(8, "0");
        return `FLT 0x${hex}`;
      }
      switch (analogState) {
        case "ready":
          return "RDY";
        case "cal_missing":
          return "CAL";
        case "faulted":
          return "FLT";
        default:
          return "OFF";
      }
    };

    // Background blocks.
    fillRect({ left: 0, top: 0, right: width, bottom: height }, COLOR_CANVAS);
    fillRect({ left: 0, top: 0, right: 190, bottom: height }, COLOR_LEFT_BASE);
    fillRect(
      { left: 190, top: 0, right: width, bottom: height },
      COLOR_RIGHT_BASE,
    );

    // Slab backgrounds.
    const cardTops = [0, 80, 160] as const;
    for (let idx = 0; idx < cardTops.length; idx += 1) {
      const top = cardTops[idx];
      fillRect(
        { left: 8, top: top + 6, right: 182, bottom: top + 80 },
        CARD_TINTS[idx],
      );
    }

    // Left metrics.
    drawSmallText("VOLTAGE", 16, 10, COLOR_CAPTION);
    drawSevenSegValue(
      formatFixed2dp(remoteVoltageV),
      { left: 24, top: 28, right: 170, bottom: 72 },
      COLOR_VOLTAGE,
    );
    drawSmallText("V", 170, 56, COLOR_CAPTION, 1);

    drawSmallText("CURRENT", 16, 90, COLOR_CAPTION);
    drawMirrorBar(92, 76, 180, localCurrentA / 5, remoteCurrentA / 5);
    drawSevenSegValue(
      formatFixed2dp(totalCurrentA),
      { left: 24, top: 108, right: 170, bottom: 152 },
      COLOR_CURRENT,
    );
    drawSmallText("A", 170, 136, COLOR_CAPTION, 1);

    drawSmallText("POWER", 16, 170, COLOR_CAPTION);
    drawSevenSegValue(
      formatFixed1dp3i(totalPowerW),
      { left: 24, top: 188, right: 170, bottom: 232 },
      COLOR_POWER,
    );
    drawSmallText("W", 170, 216, COLOR_CAPTION, 1);

    // Right column: control row.
    fillRoundRect(
      { left: 198, top: 10, right: 252, bottom: 38 },
      6,
      COLOR_BAR_TRACK,
    );
    fillRoundRect(
      { left: 256, top: 10, right: 314, bottom: 38 },
      6,
      COLOR_BAR_TRACK,
    );
    fillRect({ left: 225, top: 12, right: 226, bottom: 36 }, COLOR_RIGHT_BASE);

    if (controlMode === "cp") {
      drawSmallText("CP", 204, 18, COLOR_POWER);
    } else {
      const ccColor = controlMode === "cc" ? COLOR_CURRENT : COLOR_RIGHT_LABEL;
      const cvColor = controlMode === "cv" ? COLOR_VOLTAGE : COLOR_RIGHT_LABEL;
      drawSmallText("CC", 204, 18, ccColor);
      drawSmallText("CV", 230, 18, cvColor);
    }

    const targetText = formatSetpointMilli(
      controlTargetMilli,
      controlTargetUnit,
    );
    const valueX = 314 - 4 - targetText.length * smallFontWidth;
    const valueY = 18;
    const selectedIdx = 3; // tenths (0.1) by default
    const cellX = valueX + selectedIdx * smallFontWidth;
    fillRect(
      { left: cellX - 1, top: valueY, right: cellX + 6, bottom: valueY + 12 },
      COLOR_BAR_FILL,
    );
    drawSmallText(targetText, valueX, valueY, COLOR_RIGHT_VALUE);
    if (selectedIdx >= 0 && selectedIdx < targetText.length) {
      drawSmallChar(targetText[selectedIdx], cellX, valueY, COLOR_RIGHT_BASE);
    }

    // Right column: remote/local voltage pair + mirror bar.
    drawSmallText("REMOTE", 198, 50, COLOR_RIGHT_LABEL);
    const remoteText = remoteActive
      ? formatPairValue(remoteVoltageV, "V")
      : "--.--";
    drawSmallText(remoteText, 198, 62, COLOR_RIGHT_VALUE);
    drawSmallText("LOCAL", 258, 50, COLOR_RIGHT_LABEL);
    drawSmallText(
      formatPairValue(localVoltageV, "V"),
      258,
      62,
      COLOR_RIGHT_VALUE,
    );

    const remoteBar = remoteActive ? remoteVoltageV / 40 : 0;
    drawMirrorBar(84, 198, 314, remoteBar, localVoltageV / 40);

    // Status lines.
    const runText = `RUN ${formatRunTime(uptimeSeconds)}`;
    const coreText = `CORE ${formatTemp1dp(tempCoreC)}C`;
    const sinkText = `SINK ${formatTemp1dp(tempSinkC)}C`;
    const mcuText = `MCU  ${formatTemp1dp(tempMcuC)}C`;
    const statusText = formatStatusLine5();
    const statusLines = [
      runText,
      coreText,
      sinkText,
      mcuText,
      statusText,
    ] as const;
    for (let idx = 0; idx < statusLines.length; idx += 1) {
      drawSmallText(statusLines[idx], 198, 172 + idx * 12, COLOR_RIGHT_VALUE);
    }

    // Wi‑Fi status overlay (not yet wired to API; render disabled state).
    fillRect({ left: 288, top: 0, right: 320, bottom: 10 }, COLOR_RIGHT_BASE);
    drawSmallText("W:--", 290, 1, COLOR_RIGHT_LABEL);
  }, [
    canvasSize.height,
    canvasSize.width,
    remoteVoltageV,
    localVoltageV,
    localCurrentA,
    remoteCurrentA,
    totalCurrentA,
    totalPowerW,
    controlMode,
    controlTargetMilli,
    controlTargetUnit,
    uptimeSeconds,
    tempCoreC,
    tempSinkC,
    tempMcuC,
    remoteActive,
    analogState,
    faultFlags,
  ]);

  return (
    <div className="card w-full max-w-[640px] aspect-[4/3] rounded-2xl bg-[#05070D] shadow-2xl overflow-hidden border border-[#1f2937]">
      <canvas
        ref={canvasRef}
        width={320}
        height={240}
        className="w-full h-full block"
        style={{
          imageRendering: "pixelated",
        }}
      />
    </div>
  );
}

export default DeviceCcRoute;

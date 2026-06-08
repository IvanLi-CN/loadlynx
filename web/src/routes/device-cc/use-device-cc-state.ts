import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import type { HttpApiError } from "../../api/client.ts";
import {
  __debugSetUvLatched,
  applyPreset,
  isHttpApiError,
  subscribeStatusStream,
  updateControl,
  updatePreset,
} from "../../api/client.ts";
import { findVisibleSavedFixedPdo } from "../../api/pd-display.ts";
import type {
  ControlUpdateRequest,
  ControlView,
  FastStatusView,
  Identity,
  LoadMode,
  PdView,
  Preset,
  PresetId,
} from "../../api/types.ts";
import { formatWithUnit } from "../../components/instrument/format.ts";
import {
  invalidateDeviceQuery,
  setDeviceQueryData,
} from "../../devices/device-query-cache.ts";
import { DEVICE_QUERY_PARTS } from "../../devices/device-query-key.ts";
import {
  getDeviceControlQueryOptions,
  getDevicePdQueryOptions,
  getDevicePresetsQueryOptions,
  getDeviceStatusQueryOptions,
  useDeviceIdentityByBaseUrl,
} from "../../devices/hooks.ts";
import { requireDeviceBaseUrl } from "../../lib/device-base-url.ts";
import {
  formatHttpApiErrorSummary,
  getNetworkErrorHint,
  isUnsupportedOperationError,
} from "../../lib/http-error.ts";

type EditableLoadMode = Exclude<LoadMode, "cr">;
type UpdateControlMutation = ReturnType<
  typeof useMutation<ControlView, Error, ControlUpdateRequest>
>;

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

export interface DeviceCcViewState {
  identity: Identity | undefined;
  status: FastStatusView | undefined | null;
  control: ControlView | undefined;
  pd: PdView | null;
  topError: { summary: string; hint: string | null } | null;
  isLinkDownLike: boolean | null;
  activeLoadModeBadge: "CC" | "CV" | "CP" | "CR";
  remoteVoltageV: number | null;
  localVoltageV: number | null;
  localCurrentA: number | null;
  remoteCurrentA: number | null;
  totalCurrentA: number | null;
  totalPowerW: number | null;
  uptimeSeconds: number | null;
  tempCoreC: number | null;
  tempSinkC: number | null;
  tempMcuC: number | null;
  controlMode: LoadMode;
  remoteActive: boolean;
  analogState: FastStatusView["analog_state"];
  faultFlags: number;
  resistanceOhms: number | null;
  controlTargetMilli: number;
  controlTargetUnit: "A" | "V" | "W" | "Ω";
  cpSupported: boolean;
  cpDraftOutOfRange: boolean;
  savePresetDisabled: boolean;
  telemetryStale: boolean;
  faultList: string[];
  faultSummary: string | null;
  linkState: "up" | "down" | "unknown";
  protectionState: { summary: string; level: "danger" | "warn" | "ok" };
  activeSetpointLabel: string | null;
  headline: { value: number | null; unit: "A" | "V" | "W" | "Ω" };
  activeTrendPoints: number[];
  trendMin: number;
  trendMax: number;
  trendPad: number;
  thermalTrendPoints: number[];
  thermalTrendMin: number;
  thermalTrendMax: number;
  thermalTrendPad: number;
  pdPanel: {
    visible: boolean;
    contractText: string;
    ppsText: string | null;
    savedText: string | null;
  };
  availableModes: Array<"CC" | "CV" | "CP">;
  draftModeLabel: "CC" | "CV" | "CP" | "CR";
  presetsButtons: Array<{
    id: number;
    label: string;
    active: boolean;
    disabled: boolean;
  }>;
  outputToggleDisabled: boolean;
  setpoints: Array<{
    label: string;
    value: string;
    readback: string;
    active: boolean;
  }>;
  limits: Array<{
    label: string;
    value: string;
    tone?: "warn" | "ok";
  }>;
  diagnostics: {
    analogLinkText: string;
    loopText: string;
    lastApplyText: string;
  };
}

export interface DeviceCcMutationState {
  showOutputReenableHint: boolean;
  setShowOutputReenableHint: (next: boolean) => void;
  selectedPresetId: PresetId;
  setSelectedPresetId: (id: PresetId) => void;
  draftPresetMode: LoadMode;
  setDraftPresetMode: (mode: EditableLoadMode) => void;
  draftPresetTargetIMa: number;
  setDraftPresetTargetIMa: (value: number) => void;
  draftPresetTargetVMv: number;
  setDraftPresetTargetVMv: (value: number) => void;
  draftPresetTargetPMw: number;
  setDraftPresetTargetPMw: (value: number) => void;
  draftPresetMinVMv: number;
  setDraftPresetMinVMv: (value: number) => void;
  draftPresetMaxIMaTotal: number;
  setDraftPresetMaxIMaTotal: (value: number) => void;
  draftPresetMaxPMw: number;
  setDraftPresetMaxPMw: (value: number) => void;
  updateControlMutation: UpdateControlMutation;
  updatePresetMutation: ReturnType<typeof useMutation<Preset, Error, Preset>>;
  applyPresetMutation: ReturnType<
    typeof useMutation<ControlView, Error, PresetId>
  >;
  debugUvMutation: ReturnType<typeof useMutation<ControlView, Error, boolean>>;
  handleSavePreset: () => void;
  handleApplyPreset: () => void;
  explainHttpError: (error: HttpApiError) => string | null;
}

export function useDeviceCcState(
  deviceId: string,
  baseUrl: string | undefined,
  isPageVisible: boolean,
): {
  view: DeviceCcViewState;
  mutation: DeviceCcMutationState;
} {
  const queryClient = useQueryClient();
  const [streamStatus, setStreamStatus] = useState<FastStatusView | null>(null);

  useEffect(() => {
    if (baseUrl === undefined) {
      setStreamStatus(null);
      return;
    }
    setStreamStatus(null);
  }, [baseUrl]);

  const identityQuery = useDeviceIdentityByBaseUrl(deviceId, baseUrl);

  const controlQuery = useQuery<ControlView, HttpApiError>(
    getDeviceControlQueryOptions({
      deviceId,
      baseUrl,
      enabled: Boolean(baseUrl) && identityQuery.isSuccess,
      retryDelay: RETRY_DELAY_MS,
    }),
  );

  const presetsQuery = useQuery<Preset[], HttpApiError>(
    getDevicePresetsQueryOptions({
      deviceId,
      baseUrl,
      enabled: Boolean(baseUrl) && identityQuery.isSuccess,
      retryDelay: RETRY_DELAY_MS,
    }),
  );

  const [selectedPresetId, setSelectedPresetId] = useState<PresetId>(1);
  const selectedPresetInitializedRef = useRef(false);
  const applyOutputWasEnabledRef = useRef(false);
  const [showOutputReenableHint, setShowOutputReenableHint] = useState(false);
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
    const preset = presets.find(
      (entry) => entry.preset_id === selectedPresetId,
    );
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
      return updatePreset(requireDeviceBaseUrl(baseUrl), payload);
    },
    onSuccess: (nextPreset) => {
      setDeviceQueryData<Preset[]>(
        queryClient,
        deviceId,
        baseUrl,
        DEVICE_QUERY_PARTS.presets,
        (prev) => {
          const prevList = prev ?? [];
          const next = prevList.slice();
          const idx = next.findIndex(
            (preset) => preset.preset_id === nextPreset.preset_id,
          );
          if (idx >= 0) next[idx] = nextPreset;
          else {
            next.push(nextPreset);
            next.sort((a, b) => a.preset_id - b.preset_id);
          }
          return next;
        },
      );
      void invalidateDeviceQuery(
        queryClient,
        deviceId,
        baseUrl,
        ...DEVICE_QUERY_PARTS.control,
      );
      void invalidateDeviceQuery(
        queryClient,
        deviceId,
        baseUrl,
        ...DEVICE_QUERY_PARTS.status,
      );
    },
  });

  const applyPresetMutation = useMutation({
    mutationFn: async (presetId: PresetId) => {
      return applyPreset(requireDeviceBaseUrl(baseUrl), presetId);
    },
    onSuccess: (nextControl) => {
      setDeviceQueryData(
        queryClient,
        deviceId,
        baseUrl,
        DEVICE_QUERY_PARTS.control,
        nextControl,
      );
      setSelectedPresetId(nextControl.active_preset_id);
      setShowOutputReenableHint(
        Boolean(
          applyOutputWasEnabledRef.current && !nextControl.output_enabled,
        ),
      );
      void invalidateDeviceQuery(
        queryClient,
        deviceId,
        baseUrl,
        ...DEVICE_QUERY_PARTS.status,
      );
    },
  });

  const updateControlMutation = useMutation({
    mutationFn: async (payload: ControlUpdateRequest) => {
      return updateControl(requireDeviceBaseUrl(baseUrl), payload);
    },
    onSuccess: (nextControl) => {
      setDeviceQueryData(
        queryClient,
        deviceId,
        baseUrl,
        DEVICE_QUERY_PARTS.control,
        nextControl,
      );
      void invalidateDeviceQuery(
        queryClient,
        deviceId,
        baseUrl,
        ...DEVICE_QUERY_PARTS.status,
      );
    },
  });

  const writesInFlight =
    updatePresetMutation.isPending ||
    applyPresetMutation.isPending ||
    updateControlMutation.isPending;

  const pdQuery = useQuery<PdView, HttpApiError>(
    getDevicePdQueryOptions({
      deviceId,
      baseUrl,
      enabled: Boolean(baseUrl) && identityQuery.isSuccess && !writesInFlight,
      refetchInterval: isPageVisible ? PD_REFETCH_MS : false,
      retryDelay: RETRY_DELAY_MS,
    }),
  );

  const debugUvMutation = useMutation({
    mutationFn: async (uv_latched: boolean) => {
      return __debugSetUvLatched(requireDeviceBaseUrl(baseUrl), uv_latched);
    },
    onSuccess: (nextControl) => {
      setDeviceQueryData(
        queryClient,
        deviceId,
        baseUrl,
        DEVICE_QUERY_PARTS.control,
        nextControl,
      );
    },
  });

  const statusQuery = useQuery<FastStatusView, HttpApiError>({
    ...getDeviceStatusQueryOptions({
      deviceId,
      baseUrl,
      enabled:
        Boolean(baseUrl) &&
        identityQuery.isSuccess &&
        !writesInFlight &&
        streamStatus === null,
      refetchInterval: isPageVisible ? FAST_STATUS_REFETCH_MS : false,
    }),
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
    for (const error of errors) {
      if (isHttpApiError(error)) {
        return error;
      }
    }
    return null;
  })();

  const topError = (() => {
    if (!firstHttpError) return null;
    const summary = formatHttpApiErrorSummary(firstHttpError);

    if (
      firstHttpError.status === 0 &&
      firstHttpError.code === "NETWORK_ERROR"
    ) {
      return {
        summary,
        hint: `可能是短暂的网络抖动，已自动重试；若仍无法连接，请检查网络与 IP 设置。${baseUrl ? `（${getNetworkErrorHint(baseUrl)}）` : ""}`,
      } as const;
    }

    if (isUnsupportedOperationError(firstHttpError)) {
      return {
        summary,
        hint: "固件版本不支持该 API，请升级固件后重试。",
      } as const;
    }

    return { summary, hint: null } as const;
  })();

  const identity = identityQuery.data;
  const status = streamStatus ?? statusQuery.data;
  const control = controlQuery.data;
  const pd = pdQuery.data ?? null;
  const activeLoadModeBadge: "CC" | "CV" | "CP" | "CR" =
    control?.preset.mode === "cv"
      ? "CV"
      : control?.preset.mode === "cp"
        ? "CP"
        : control?.preset.mode === "cr"
          ? "CR"
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
  const remoteActive = status?.link_up === true;
  const analogState = status?.analog_state ?? "offline";
  const faultFlags = status?.raw.fault_flags ?? 0;
  const resistanceOhms =
    localVoltageV != null && totalCurrentA != null && totalCurrentA > 0.0001
      ? localVoltageV / totalCurrentA
      : null;
  const controlTargetMilli =
    controlMode === "cv"
      ? (control?.preset.target_v_mv ?? 0)
      : controlMode === "cp"
        ? (control?.preset.target_p_mw ?? 0)
        : controlMode === "cr"
          ? resistanceOhms == null
            ? -1
            : Math.round(resistanceOhms * 1_000)
          : (control?.preset.target_i_ma ?? 0);
  const controlTargetUnit =
    controlMode === "cv"
      ? "V"
      : controlMode === "cp"
        ? "W"
        : controlMode === "cr"
          ? "Ω"
          : "A";

  const lastTrendUptimeMsRef = useRef<number | null>(null);
  const [trend, setTrend] = useState<{
    v: number[];
    i: number[];
    p: number[];
    r: number[];
    t: number[];
  }>({ v: [], i: [], p: [], r: [], t: [] });

  useEffect(() => {
    const uptimeMs = status?.raw.uptime_ms ?? null;
    if (uptimeMs == null || lastTrendUptimeMsRef.current === uptimeMs) {
      return;
    }
    lastTrendUptimeMsRef.current = uptimeMs;

    setTrend((prev) => ({
      v: pushTrendPoint(prev.v, localVoltageV),
      i: pushTrendPoint(prev.i, totalCurrentA),
      p: pushTrendPoint(prev.p, totalPowerW),
      r: pushTrendPoint(prev.r, resistanceOhms),
      t: pushTrendPoint(prev.t, tempCoreC),
    }));
  }, [
    localVoltageV,
    resistanceOhms,
    status?.raw.uptime_ms,
    tempCoreC,
    totalCurrentA,
    totalPowerW,
  ]);

  const handleSavePreset = () => {
    updatePresetMutation.mutate({
      preset_id: selectedPresetId,
      mode: draftPresetMode,
      target_i_ma: draftPresetTargetIMa,
      target_v_mv: draftPresetTargetVMv,
      target_p_mw: draftPresetTargetPMw,
      min_v_mv: draftPresetMinVMv,
      max_i_ma_total: draftPresetMaxIMaTotal,
      max_p_mw: draftPresetMaxPMw,
    });
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
    !baseUrl ||
    updatePresetMutation.isPending ||
    cpDraftOutOfRange ||
    draftPresetMode === "cr";

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
    if (!control) return null;
    if (controlMode === "cv") {
      return `${(control.preset.target_v_mv / 1000).toFixed(3)} V`;
    }
    if (controlMode === "cp") {
      return `${(control.preset.target_p_mw / 1000).toFixed(2)} W`;
    }
    if (controlMode === "cr") {
      return resistanceOhms == null
        ? "CR read-only"
        : `${resistanceOhms.toFixed(2)} Ω`;
    }
    return `${(control.preset.target_i_ma / 1000).toFixed(3)} A`;
  })();

  const headline =
    controlMode === "cv"
      ? { value: localVoltageV, unit: "V" as const }
      : controlMode === "cp"
        ? { value: totalPowerW, unit: "W" as const }
        : controlMode === "cr"
          ? { value: resistanceOhms, unit: "Ω" as const }
          : { value: totalCurrentA, unit: "A" as const };

  const activeTrendPoints =
    controlMode === "cv"
      ? trend.v
      : controlMode === "cp"
        ? trend.p
        : controlMode === "cr"
          ? trend.r
          : trend.i;
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
      const contractText =
        pd.attached && pd.contract_mv != null && pd.contract_ma != null
          ? `Contract: ${(pd.contract_mv / 1000).toFixed(1)} V @ ${pd.contract_ma} mA`
          : pd.attached
            ? "Contract: unknown"
            : "Contract: detached";

      const ppsText = (() => {
        const first = pd.pps_pdos[0];
        if (!first) return "PPS: —";
        return `PPS: ${(first.min_mv / 1000).toFixed(1)}–${(first.max_mv / 1000).toFixed(1)}V (${pd.pps_pdos.length} APDO)`;
      })();

      const savedText =
        pd.saved.mode === "fixed"
          ? (() => {
              const visibleSavedFixed = findVisibleSavedFixedPdo(pd);
              return [
                "Saved: Fixed",
                visibleSavedFixed ? `PDO #${visibleSavedFixed.pos}` : null,
                `${pd.saved.i_req_ma} mA`,
              ]
                .filter(Boolean)
                .join(" · ");
            })()
          : `Saved: PPS · APDO #${pd.saved.pps_object_pos} · ${pd.saved.target_mv} mV · ${pd.saved.i_req_ma} mA`;

      return { visible: true, contractText, ppsText, savedText } as const;
    }

    const error = pdQuery.error;
    if (
      error &&
      isHttpApiError(error) &&
      error.status === 404 &&
      error.code === "UNSUPPORTED_OPERATION"
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

  const availableModes: Array<"CC" | "CV" | "CP"> = ["CC", "CV"];
  if (cpSupported) {
    availableModes.push("CP");
  }

  const draftModeLabel: "CC" | "CV" | "CP" | "CR" =
    draftPresetMode === "cc"
      ? "CC"
      : draftPresetMode === "cv"
        ? "CV"
        : draftPresetMode === "cr"
          ? "CR"
          : "CP";

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
    !control ||
    updateControlMutation.isPending ||
    applyPresetMutation.isPending;

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
  };

  return {
    view: {
      identity,
      status,
      control,
      pd,
      topError,
      isLinkDownLike: Boolean(
        firstHttpError &&
          (firstHttpError.code === "LINK_DOWN" ||
            firstHttpError.code === "UNAVAILABLE"),
      ),
      activeLoadModeBadge,
      remoteVoltageV,
      localVoltageV,
      localCurrentA,
      remoteCurrentA,
      totalCurrentA,
      totalPowerW,
      uptimeSeconds,
      tempCoreC,
      tempSinkC,
      tempMcuC,
      controlMode,
      remoteActive,
      analogState,
      faultFlags,
      resistanceOhms,
      controlTargetMilli,
      controlTargetUnit,
      cpSupported,
      cpDraftOutOfRange,
      savePresetDisabled,
      telemetryStale,
      faultList,
      faultSummary,
      linkState,
      protectionState,
      activeSetpointLabel,
      headline,
      activeTrendPoints,
      trendMin,
      trendMax,
      trendPad,
      thermalTrendPoints,
      thermalTrendMin,
      thermalTrendMax,
      thermalTrendPad,
      pdPanel,
      availableModes,
      draftModeLabel,
      presetsButtons,
      outputToggleDisabled,
      setpoints,
      limits,
      diagnostics,
    },
    mutation: {
      showOutputReenableHint,
      setShowOutputReenableHint,
      selectedPresetId,
      setSelectedPresetId,
      draftPresetMode,
      setDraftPresetMode: (mode) => setDraftPresetMode(mode),
      draftPresetTargetIMa,
      setDraftPresetTargetIMa,
      draftPresetTargetVMv,
      setDraftPresetTargetVMv,
      draftPresetTargetPMw,
      setDraftPresetTargetPMw,
      draftPresetMinVMv,
      setDraftPresetMinVMv,
      draftPresetMaxIMaTotal,
      setDraftPresetMaxIMaTotal,
      draftPresetMaxPMw,
      setDraftPresetMaxPMw,
      updateControlMutation,
      updatePresetMutation,
      applyPresetMutation,
      debugUvMutation,
      handleSavePreset,
      handleApplyPreset,
      explainHttpError,
    },
  };
}

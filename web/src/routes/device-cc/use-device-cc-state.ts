import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
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
import { clampPresetDraft, type PresetDraft } from "./preset-constraints.ts";
import {
  formatPresetRawValue,
  makePresetInputMemoryKey,
  type PresetEditableField,
  type PresetInputMemoryStore,
  type PresetInputUnitKind,
  parsePresetInputValue,
  readPresetInputMemory,
  reconcilePresetInputMemory,
  writePresetInputMemory,
} from "./preset-input-memory.ts";
import {
  getStatusRenderDelay,
  STREAM_UI_INTERVAL_MS,
} from "./status-stream-gate.ts";
import {
  buildTrendSeries,
  ELECTRICAL_TREND_SAMPLE_INTERVAL_MS,
  shouldAppendTrendSample,
  snapshotTrendSeriesDomains,
  stabilizeTrendSeriesDomains,
  THERMAL_TREND_SAMPLE_INTERVAL_MS,
  TREND_WINDOW_SECONDS,
  type TrendDomainMemoryMap,
  type TrendSample,
  trimTrendSamplesToWindow,
} from "./trend-domain.ts";

type EditableLoadMode = Exclude<LoadMode, "cr">;
type UpdateControlMutation = ReturnType<
  typeof useMutation<ControlView, Error, ControlUpdateRequest>
>;
export type DashboardMetricKey = "voltage" | "current" | "power" | "resistance";

const FAST_STATUS_REFETCH_MS = 400;
const PD_REFETCH_MS = 1500;
const RETRY_DELAY_MS = 500;
const jitterRetryDelay = () => 200 + Math.random() * 300;
const TREND_MAX_POINTS = 96;
const EMPTY_PRESET: PresetDraft = {
  mode: "cc",
  target_i_ma: 0,
  target_v_mv: 0,
  target_p_mw: 0,
  min_v_mv: 0,
  max_i_ma_total: 0,
  max_p_mw: 0,
};

function appendTrendSampleWindow(
  prev: TrendSample[],
  nextSample: TrendSample,
  shouldResetWindow: boolean,
): TrendSample[] {
  const base = shouldResetWindow ? [] : prev;
  const next = base.length >= TREND_MAX_POINTS ? base.slice(1) : base.slice();
  next.push(nextSample);
  return trimTrendSamplesToWindow(next, TREND_WINDOW_SECONDS);
}

function computeTrendBounds(points: number[]): {
  min: number;
  max: number;
  pad: number;
} {
  const min = points.length > 0 ? Math.min(...points) : 0;
  const max = points.length > 0 ? Math.max(...points) : 1;
  const pad = max > min ? (max - min) * 0.05 : 1;
  return { min, max, pad };
}

export interface DeviceCcViewState {
  identity: Identity | undefined;
  status: FastStatusView | undefined | null;
  control: ControlView | undefined;
  activePresetDraft: PresetDraft | null;
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
  defaultChartMetric: DashboardMetricKey;
  primaryMetrics: Array<{
    key: DashboardMetricKey;
    label: string;
    value: number | null;
    unit: "V" | "A" | "W" | "Ω";
    digits: number;
    detail: string | null;
    emphasized: boolean;
  }>;
  trendSeries: ReturnType<typeof buildTrendSeries>;
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
  presetActionNotice: string | null;
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
  getPresetDisplayValue: (
    field: PresetEditableField,
    rawValue: number,
  ) => string;
  setPresetDisplayDraft: (field: PresetEditableField, text: string) => void;
  commitPresetDisplayDraft: (
    field: PresetEditableField,
    unitKind: PresetInputUnitKind,
    fallbackValue: number,
    setValue: (value: number) => void,
  ) => void;
  presetDisplayError: (
    field: PresetEditableField,
    fallbackError?: string | null,
  ) => string | null;
  updateControlMutation: UpdateControlMutation;
  updatePresetMutation: ReturnType<typeof useMutation<Preset, Error, Preset>>;
  applyPresetMutation: ReturnType<
    typeof useMutation<ControlView, Error, PresetId>
  >;
  debugUvMutation: ReturnType<typeof useMutation<ControlView, Error, boolean>>;
  handleSavePreset: () => void;
  savePresetDraft: (presetId: PresetId, draft: PresetDraft) => void;
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
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [streamStatus, setStreamStatus] = useState<FastStatusView | null>(null);
  const pendingStreamStatusRef = useRef<FastStatusView | null>(null);
  const streamCommitAtMsRef = useRef<number | null>(null);
  const streamFlushTimerRef = useRef<number | null>(null);
  const [presetInputMemory, setPresetInputMemory] =
    useState<PresetInputMemoryStore>(() =>
      readPresetInputMemory(window.localStorage),
    );
  const [presetInputDrafts, setPresetInputDrafts] = useState<
    Partial<Record<PresetEditableField, string>>
  >({});
  const [presetInputErrors, setPresetInputErrors] = useState<
    Partial<Record<PresetEditableField, string | null>>
  >({});

  useEffect(() => {
    if (baseUrl === undefined) {
      setStreamStatus(null);
      pendingStreamStatusRef.current = null;
      streamCommitAtMsRef.current = null;
      if (streamFlushTimerRef.current != null) {
        window.clearTimeout(streamFlushTimerRef.current);
        streamFlushTimerRef.current = null;
      }
      return;
    }
    setStreamStatus(null);
  }, [baseUrl]);

  const clearStreamFlushTimer = useCallback(() => {
    if (streamFlushTimerRef.current != null) {
      window.clearTimeout(streamFlushTimerRef.current);
      streamFlushTimerRef.current = null;
    }
  }, []);

  const commitStreamStatus = useCallback(
    (nextStatus: FastStatusView | null) => {
      streamCommitAtMsRef.current = window.performance.now();
      startTransition(() => {
        setStreamStatus(nextStatus);
      });
    },
    [],
  );

  useEffect(() => {
    return () => {
      clearStreamFlushTimer();
    };
  }, [clearStreamFlushTimer]);

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
  const [presetActionNotice, setPresetActionNotice] = useState<string | null>(
    null,
  );
  const presetActionNoticeTimerRef = useRef<number | null>(null);

  const showPresetActionNotice = useCallback((message: string) => {
    if (presetActionNoticeTimerRef.current != null) {
      window.clearTimeout(presetActionNoticeTimerRef.current);
    }
    setPresetActionNotice(message);
    presetActionNoticeTimerRef.current = window.setTimeout(() => {
      setPresetActionNotice(null);
      presetActionNoticeTimerRef.current = null;
    }, 2400);
  }, []);

  useEffect(() => {
    return () => {
      if (presetActionNoticeTimerRef.current != null) {
        window.clearTimeout(presetActionNoticeTimerRef.current);
      }
    };
  }, []);

  const persistPresetInputMemory = useCallback(
    (updater: (prev: PresetInputMemoryStore) => PresetInputMemoryStore) => {
      setPresetInputMemory((prev) => {
        const next = updater(prev);
        writePresetInputMemory(window.localStorage, next);
        return next;
      });
    },
    [],
  );

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

    persistPresetInputMemory((prev) =>
      reconcilePresetInputMemory({
        store: prev,
        deviceId,
        baseUrl,
        presets,
      }),
    );

    const preset = presets.find(
      (entry) => entry.preset_id === selectedPresetId,
    );
    if (!preset) {
      return;
    }
    const nextPreset = clampPresetDraft(preset);
    setDraftPresetMode(nextPreset.mode);
    setDraftPresetTargetIMa(nextPreset.target_i_ma);
    setDraftPresetTargetVMv(nextPreset.target_v_mv);
    setDraftPresetTargetPMw(nextPreset.target_p_mw);
    setDraftPresetMinVMv(nextPreset.min_v_mv);
    setDraftPresetMaxIMaTotal(nextPreset.max_i_ma_total);
    setDraftPresetMaxPMw(nextPreset.max_p_mw);
  }, [
    baseUrl,
    deviceId,
    persistPresetInputMemory,
    presetsQuery.data,
    selectedPresetId,
  ]);

  const getPresetMemoryKey = (field: PresetEditableField) =>
    makePresetInputMemoryKey({
      deviceId,
      baseUrl,
      presetId: selectedPresetId,
      field,
    });

  const getPresetDisplayValue = (
    field: PresetEditableField,
    rawValue: number,
  ): string => {
    const draft = presetInputDrafts[field];
    if (draft !== undefined) {
      return draft;
    }

    const memory = presetInputMemory[getPresetMemoryKey(field)];
    if (memory && memory.value === rawValue) {
      return memory.text;
    }

    return formatPresetRawValue(rawValue);
  };

  const setPresetDisplayDraft = (field: PresetEditableField, text: string) => {
    setPresetInputDrafts((prev) => ({
      ...prev,
      [field]: text,
    }));
    setPresetInputErrors((prev) => ({
      ...prev,
      [field]: null,
    }));
  };

  const currentDraftPreset = (): PresetDraft => ({
    mode: draftPresetMode,
    target_i_ma: draftPresetTargetIMa,
    target_v_mv: draftPresetTargetVMv,
    target_p_mw: draftPresetTargetPMw,
    min_v_mv: draftPresetMinVMv,
    max_i_ma_total: draftPresetMaxIMaTotal,
    max_p_mw: draftPresetMaxPMw,
  });

  const applyDraftPreset = (preset: PresetDraft) => {
    setDraftPresetMode(preset.mode);
    setDraftPresetTargetIMa(preset.target_i_ma);
    setDraftPresetTargetVMv(preset.target_v_mv);
    setDraftPresetTargetPMw(preset.target_p_mw);
    setDraftPresetMinVMv(preset.min_v_mv);
    setDraftPresetMaxIMaTotal(preset.max_i_ma_total);
    setDraftPresetMaxPMw(preset.max_p_mw);
  };

  const computeNextDraftPreset = (patch: Partial<PresetDraft>): PresetDraft =>
    clampPresetDraft({
      ...currentDraftPreset(),
      ...patch,
    });

  const commitDraftPresetPatch = (patch: Partial<PresetDraft>): PresetDraft => {
    const next = computeNextDraftPreset(patch);
    applyDraftPreset(next);
    return next;
  };

  const commitPresetDisplayDraft = (
    field: PresetEditableField,
    unitKind: PresetInputUnitKind,
    fallbackValue: number,
    setValue: (value: number) => void,
  ) => {
    const currentRaw =
      presetInputDrafts[field] ?? getPresetDisplayValue(field, fallbackValue);
    const parsed = parsePresetInputValue(currentRaw, unitKind);
    if (!parsed.ok) {
      setPresetInputErrors((prev) => ({
        ...prev,
        [field]: parsed.error,
      }));
      setPresetInputDrafts((prev) => ({
        ...prev,
        [field]: currentRaw,
      }));
      return;
    }

    const next = computeNextDraftPreset({
      [field]: parsed.value,
    } as Partial<PresetDraft>);
    const nextValue = next[field];
    setValue(nextValue);
    setPresetInputErrors((prev) => ({
      ...prev,
      [field]: null,
    }));
    setPresetInputDrafts((prev) => {
      const next = { ...prev };
      delete next[field];
      return next;
    });

    const memoryKey = getPresetMemoryKey(field);
    if (nextValue === parsed.value) {
      persistPresetInputMemory((prev) => ({
        ...prev,
        [memoryKey]: {
          value: nextValue,
          text: parsed.displayText,
        },
      }));
      return;
    }

    persistPresetInputMemory((prev) => {
      const nextStore = { ...prev };
      delete nextStore[memoryKey];
      return nextStore;
    });
  };

  const presetDisplayError = (
    field: PresetEditableField,
    fallbackError: string | null = null,
  ) => presetInputErrors[field] ?? fallbackError;

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
      const savedActivePresetId = controlQuery.data?.active_preset_id ?? null;
      showPresetActionNotice(
        savedActivePresetId === nextPreset.preset_id
          ? t("dashboard.presets.savedActiveNotice", {
              slot: nextPreset.preset_id,
            })
          : t("dashboard.presets.savedNotice", {
              slot: nextPreset.preset_id,
            }),
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
      showPresetActionNotice(
        t("dashboard.presets.appliedNotice", {
          slot: nextControl.active_preset_id,
        }),
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
    if (!baseUrl || !identityQuery.isSuccess || !isPageVisible) {
      clearStreamFlushTimer();
      pendingStreamStatusRef.current = null;
      return undefined;
    }

    let cancelled = false;
    const unsubscribe = subscribeStatusStream(
      baseUrl,
      (view) => {
        if (cancelled) {
          return;
        }

        const nowMs = window.performance.now();
        const delayMs = getStatusRenderDelay(
          streamCommitAtMsRef.current,
          nowMs,
          STREAM_UI_INTERVAL_MS,
        );

        if (delayMs === 0) {
          clearStreamFlushTimer();
          pendingStreamStatusRef.current = null;
          commitStreamStatus(view);
          return;
        }

        pendingStreamStatusRef.current = view;
        if (streamFlushTimerRef.current != null) {
          return;
        }

        streamFlushTimerRef.current = window.setTimeout(() => {
          streamFlushTimerRef.current = null;
          if (cancelled || pendingStreamStatusRef.current == null) {
            return;
          }
          const nextPending = pendingStreamStatusRef.current;
          pendingStreamStatusRef.current = null;
          commitStreamStatus(nextPending);
        }, delayMs);
      },
      () => {
        if (cancelled) {
          return;
        }
        clearStreamFlushTimer();
        pendingStreamStatusRef.current = null;
        startTransition(() => {
          setStreamStatus(null);
        });
      },
    );

    return () => {
      cancelled = true;
      clearStreamFlushTimer();
      pendingStreamStatusRef.current = null;
      unsubscribe();
    };
  }, [
    baseUrl,
    clearStreamFlushTimer,
    commitStreamStatus,
    identityQuery.isSuccess,
    isPageVisible,
  ]);

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
        hint: t("dashboard.errors.networkRetry", {
          hint: baseUrl ? `（${getNetworkErrorHint(baseUrl)}）` : "",
        }),
      } as const;
    }

    if (isUnsupportedOperationError(firstHttpError)) {
      return {
        summary,
        hint: t("dashboard.errors.unsupportedApi"),
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

  const lastElectricalTrendUptimeMsRef = useRef<number | null>(null);
  const lastThermalTrendUptimeMsRef = useRef<number | null>(null);
  const [electricalTrendSamples, setElectricalTrendSamples] = useState<
    TrendSample[]
  >([]);
  const [thermalTrendSamples, setThermalTrendSamples] = useState<TrendSample[]>(
    [],
  );
  const trendDomainMemoryRef = useRef<TrendDomainMemoryMap>({});

  useEffect(() => {
    const uptimeMs = status?.raw.uptime_ms ?? null;
    const previousUptimeMs = lastElectricalTrendUptimeMsRef.current;
    if (
      uptimeMs == null ||
      !shouldAppendTrendSample(
        previousUptimeMs,
        uptimeMs,
        ELECTRICAL_TREND_SAMPLE_INTERVAL_MS,
      )
    ) {
      return;
    }
    lastElectricalTrendUptimeMsRef.current = uptimeMs;

    setElectricalTrendSamples((prev) => {
      const shouldResetWindow =
        previousUptimeMs != null &&
        uptimeMs != null &&
        uptimeMs < previousUptimeMs;
      return appendTrendSampleWindow(
        prev,
        {
          time: uptimeMs / 1_000,
          voltage: localVoltageV,
          current: totalCurrentA,
          power: totalPowerW,
          resistance: resistanceOhms,
        },
        shouldResetWindow,
      );
    });
  }, [
    localVoltageV,
    resistanceOhms,
    status?.raw.uptime_ms,
    totalCurrentA,
    totalPowerW,
  ]);

  useEffect(() => {
    const uptimeMs = status?.raw.uptime_ms ?? null;
    const previousUptimeMs = lastThermalTrendUptimeMsRef.current;
    if (
      uptimeMs == null ||
      !shouldAppendTrendSample(
        previousUptimeMs,
        uptimeMs,
        THERMAL_TREND_SAMPLE_INTERVAL_MS,
      )
    ) {
      return;
    }
    lastThermalTrendUptimeMsRef.current = uptimeMs;

    setThermalTrendSamples((prev) => {
      const shouldResetWindow =
        previousUptimeMs != null &&
        uptimeMs != null &&
        uptimeMs < previousUptimeMs;
      return appendTrendSampleWindow(
        prev,
        {
          time: uptimeMs / 1_000,
          voltage: null,
          current: null,
          power: null,
          thermal: tempCoreC,
        },
        shouldResetWindow,
      );
    });
  }, [status?.raw.uptime_ms, tempCoreC]);

  const handleSavePreset = () => {
    savePresetDraft(selectedPresetId, {
      mode: draftPresetMode,
      target_i_ma: draftPresetTargetIMa,
      target_v_mv: draftPresetTargetVMv,
      target_p_mw: draftPresetTargetPMw,
      min_v_mv: draftPresetMinVMv,
      max_i_ma_total: draftPresetMaxIMaTotal,
      max_p_mw: draftPresetMaxPMw,
    });
  };

  const savePresetDraft = (presetId: PresetId, draft: PresetDraft) => {
    updatePresetMutation.mutate({
      preset_id: presetId,
      mode: draft.mode,
      target_i_ma: draft.target_i_ma,
      target_v_mv: draft.target_v_mv,
      target_p_mw: draft.target_p_mw,
      min_v_mv: draft.min_v_mv,
      max_i_ma_total: draft.max_i_ma_total,
      max_p_mw: draft.max_p_mw,
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
        return t("dashboard.errors.linkDownExplain");
      case "ANALOG_NOT_READY":
        return t("dashboard.errors.analogNotReadyExplain");
      case "ANALOG_FAULTED":
        return t("dashboard.errors.analogFaultedExplain");
      case "LIMIT_VIOLATION":
        return t("dashboard.errors.limitViolationExplain");
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
      return resistanceOhms == null ? "CR" : `${resistanceOhms.toFixed(2)} Ω`;
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

  const defaultChartMetric: DashboardMetricKey =
    controlMode === "cv"
      ? "voltage"
      : controlMode === "cp"
        ? "power"
        : controlMode === "cr"
          ? "resistance"
          : "current";

  const primaryMetrics: DeviceCcViewState["primaryMetrics"] = [
    {
      key: "voltage",
      label: "Voltage",
      value: localVoltageV,
      unit: "V",
      digits: 3,
      detail:
        remoteVoltageV != null
          ? `Remote ${formatWithUnit(remoteVoltageV, 3, "V")}`
          : null,
      emphasized: defaultChartMetric === "voltage",
    },
    {
      key: "current",
      label: "Current",
      value: totalCurrentA,
      unit: "A",
      digits: 3,
      detail:
        remoteCurrentA != null
          ? `Remote ${formatWithUnit(remoteCurrentA, 3, "A")}`
          : null,
      emphasized: defaultChartMetric === "current",
    },
    {
      key: "power",
      label: "Power",
      value: totalPowerW,
      unit: "W",
      digits: 2,
      detail: "Calculated load power",
      emphasized: defaultChartMetric === "power",
    },
    {
      key: "resistance",
      label: "Resistance",
      value: resistanceOhms,
      unit: "Ω",
      digits: 2,
      detail: "Calculated from V / I",
      emphasized: defaultChartMetric === "resistance",
    },
  ];

  const thermalTrendPoints = thermalTrendSamples
    .map((sample) => sample.thermal)
    .filter(
      (value): value is number => value != null && Number.isFinite(value),
    );
  const thermalTrendBounds = computeTrendBounds(thermalTrendPoints);
  const thermalTrendMin = thermalTrendBounds.min;
  const thermalTrendMax = thermalTrendBounds.max;
  const thermalTrendPad = thermalTrendBounds.pad;
  const trendSeries = useMemo(
    () =>
      stabilizeTrendSeriesDomains(
        buildTrendSeries({
          samples: electricalTrendSamples,
          mode: controlMode,
          targetVoltageV: draftPresetTargetVMv / 1_000,
          minVoltageV: draftPresetMinVMv / 1_000,
          targetCurrentA: draftPresetTargetIMa / 1_000,
          maxCurrentA: draftPresetMaxIMaTotal / 1_000,
          targetPowerW: draftPresetTargetPMw / 1_000,
          maxPowerW: draftPresetMaxPMw / 1_000,
        }),
        trendDomainMemoryRef.current,
      ),
    [
      controlMode,
      draftPresetMaxIMaTotal,
      draftPresetMaxPMw,
      draftPresetMinVMv,
      draftPresetTargetIMa,
      draftPresetTargetPMw,
      draftPresetTargetVMv,
      electricalTrendSamples,
    ],
  );

  useEffect(() => {
    trendDomainMemoryRef.current = snapshotTrendSeriesDomains(trendSeries);
  }, [trendSeries]);

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

  const activePresetDraft = (() => {
    const activePresetId = control?.active_preset_id;
    if (activePresetId == null) {
      return control?.preset ? clampPresetDraft(control.preset) : null;
    }
    const presetFromList = presetsQuery.data?.find(
      (entry) => entry.preset_id === activePresetId,
    );
    return clampPresetDraft(presetFromList ?? control?.preset ?? EMPTY_PRESET);
  })();

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
      activePresetDraft,
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
      defaultChartMetric,
      primaryMetrics,
      trendSeries,
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
      presetActionNotice,
    },
    mutation: {
      showOutputReenableHint,
      setShowOutputReenableHint,
      selectedPresetId,
      setSelectedPresetId,
      draftPresetMode,
      setDraftPresetMode: (mode) => {
        commitDraftPresetPatch({
          mode,
        });
      },
      draftPresetTargetIMa,
      setDraftPresetTargetIMa: (value) => {
        commitDraftPresetPatch({
          target_i_ma: value,
        });
      },
      draftPresetTargetVMv,
      setDraftPresetTargetVMv: (value) => {
        commitDraftPresetPatch({
          target_v_mv: value,
        });
      },
      draftPresetTargetPMw,
      setDraftPresetTargetPMw: (value) => {
        commitDraftPresetPatch({
          target_p_mw: value,
        });
      },
      draftPresetMinVMv,
      setDraftPresetMinVMv: (value) => {
        commitDraftPresetPatch({
          min_v_mv: value,
        });
      },
      draftPresetMaxIMaTotal,
      setDraftPresetMaxIMaTotal: (value) => {
        commitDraftPresetPatch({
          max_i_ma_total: value,
        });
      },
      draftPresetMaxPMw,
      setDraftPresetMaxPMw: (value) => {
        commitDraftPresetPatch({
          max_p_mw: value,
        });
      },
      getPresetDisplayValue,
      setPresetDisplayDraft,
      commitPresetDisplayDraft,
      presetDisplayError,
      updateControlMutation,
      updatePresetMutation,
      applyPresetMutation,
      debugUvMutation,
      handleSavePreset,
      savePresetDraft,
      handleApplyPreset,
      explainHttpError,
    },
  };
}

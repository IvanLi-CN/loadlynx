import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import {
  __debugSetUvLatched,
  applyPreset,
  ENABLE_MOCK_DEVTOOLS,
  getControl,
  getIdentity,
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
  Preset,
  PresetId,
} from "../api/types.ts";
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
const RETRY_DELAY_MS = 500;
const jitterRetryDelay = () => 200 + Math.random() * 300;

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
      !(
        updatePresetMutation.isPending ||
        applyPresetMutation.isPending ||
        updateControlMutation.isPending
      ) &&
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
  const presets = presetsQuery.data ?? null;

  const activeLoadModeBadge =
    control?.preset.mode === "cv"
      ? "CV"
      : control?.preset.mode === "cp"
        ? "CP"
        : "CC";

  const statusLocalMa = status?.raw.i_local_ma ?? null;
  const statusRemoteMa = status?.raw.i_remote_ma ?? null;

  const remoteVoltageV =
    status?.raw.v_remote_mv != null ? status.raw.v_remote_mv / 1_000 : 0;
  const localVoltageV =
    status?.raw.v_local_mv != null
      ? status.raw.v_local_mv / 1_000
      : remoteVoltageV;
  const localCurrentA = statusLocalMa != null ? statusLocalMa / 1_000 : 0;
  const remoteCurrentA = statusRemoteMa != null ? statusRemoteMa / 1_000 : 0;
  const totalCurrentA =
    statusLocalMa != null && statusRemoteMa != null
      ? (statusLocalMa + statusRemoteMa) / 1_000
      : 0;
  const totalPowerW =
    status?.raw.calc_p_mw != null ? status.raw.calc_p_mw / 1_000 : 0;

  const uptimeSeconds =
    status?.raw.uptime_ms != null
      ? Math.floor(status.raw.uptime_ms / 1_000)
      : 0;
  const tempCoreC =
    status?.raw.sink_core_temp_mc != null
      ? status.raw.sink_core_temp_mc / 1_000
      : undefined;
  const tempSinkC =
    status?.raw.sink_exhaust_temp_mc != null
      ? status.raw.sink_exhaust_temp_mc / 1_000
      : undefined;
  const tempMcuC =
    status?.raw.mcu_temp_mc != null
      ? status.raw.mcu_temp_mc / 1_000
      : undefined;

  const controlMode: LoadMode = control?.preset.mode ?? "cc";
  const controlTargetMilli =
    controlMode === "cv"
      ? (control?.preset.target_v_mv ?? 0)
      : controlMode === "cp"
        ? (control?.preset.target_p_mw ?? 0)
        : (control?.preset.target_i_ma ?? 0);
  const controlTargetUnit =
    controlMode === "cv" ? "V" : controlMode === "cp" ? "W" : "A";
  const remoteActive = Boolean(status?.link_up);
  const analogState = status?.analog_state ?? "offline";
  const faultFlags = status?.raw.fault_flags ?? 0;

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

  return (
    <PageContainer className="flex flex-col gap-6 font-mono tabular-nums">
      {/* Top context: device + mode strip + link status */}
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-bold">Device control</h2>
          <p className="mt-1 text-sm text-base-content/70">
            Shared control surface for all operating modes. This view focuses on
            CC behaviour while keeping the layout aligned with the hardware main
            display.
          </p>
          <p className="mt-1 text-xs text-base-content/60">
            Device name:{" "}
            <strong className="font-medium text-base-content">
              {device.name}
            </strong>
            {identity ? (
              <>
                {" "}
                · IP:{" "}
                <code className="font-mono bg-base-200 px-1 rounded text-xs">
                  {identity.network.ip}
                </code>
              </>
            ) : null}
          </p>
        </div>
        <div className="flex flex-col items-end gap-2 text-xs text-base-content/70">
          <div className="flex items-center gap-2">
            <span className="uppercase tracking-wider text-base-content/50 text-[10px]">
              Mode
            </span>
            <div className="flex gap-1">
              {["CC", "CV", "CP", "CR"].map((mode) => {
                const isActive = mode === activeLoadModeBadge;
                return (
                  <span
                    key={mode}
                    className={`badge badge-sm ${isActive ? "badge-primary" : "badge-ghost opacity-50"}`}
                  >
                    {mode}
                  </span>
                );
              })}
            </div>
          </div>

          <div className="flex flex-col gap-1 text-right">
            <div>
              Link:{" "}
              <span
                className={
                  status?.link_up
                    ? "text-success font-medium"
                    : "text-error font-medium"
                }
              >
                {status?.link_up ? "up" : "down"}
              </span>
            </div>
            <div>
              Analog state: <span>{status?.analog_state ?? "unknown"}</span>
            </div>
            {identity ? (
              <div>
                API version:{" "}
                <span className="font-mono">
                  {identity.capabilities.api_version}
                </span>
              </div>
            ) : null}
            {status && status.fault_flags_decoded.length > 0 ? (
              <div className="text-error font-medium">
                Faults: {status.fault_flags_decoded.join(", ")}
              </div>
            ) : null}
          </div>
        </div>
      </header>

      {topError ? (
        <section
          aria-label="HTTP error"
          className="alert alert-error shadow-lg rounded-lg"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="stroke-current shrink-0 h-6 w-6"
            fill="none"
            viewBox="0 0 24 24"
            role="img"
            aria-label="Error icon"
          >
            <title>Error</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="2"
              d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <div className="text-sm">
            <div className="font-bold">HTTP error: {topError.summary}</div>
            {topError.hint ? (
              <div className="mt-1 opacity-90">{topError.hint}</div>
            ) : null}
            {isLinkDownLike ? (
              <div className="mt-1 opacity-90">
                Link down / Wi‑Fi unavailable — telemetry and control updates
                may be stale until connectivity recovers.
              </div>
            ) : null}
          </div>
        </section>
      ) : null}

      <section
        aria-label="Presets and control"
        className="grid grid-cols-1 lg:grid-cols-3 gap-6 items-start"
      >
        <div className="card bg-base-100 shadow-sm border border-base-200 lg:col-span-2">
          <div className="card-body p-6 space-y-4">
            <div className="flex flex-wrap items-start justify-between gap-4">
              <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 h-auto min-h-0">
                Presets
              </h3>
              <div className="text-xs text-base-content/60 space-y-1 text-right">
                <div data-testid="control-active-preset">
                  Active preset:{" "}
                  <span className="font-mono">
                    {control?.active_preset_id ?? "—"}
                  </span>
                </div>
                <div data-testid="control-active-mode">
                  Active mode:{" "}
                  <span className="font-mono">
                    {control?.preset.mode ?? "—"}
                  </span>
                </div>
                <div data-testid="control-output-enabled">
                  Output enabled:{" "}
                  <span className="font-mono">
                    {control?.output_enabled ? "true" : "false"}
                  </span>
                </div>
                <div data-testid="control-uv-latched">
                  UV latched:{" "}
                  <span className="font-mono">
                    {control?.uv_latched ? "true" : "false"}
                  </span>
                </div>
                <div className="text-[10px]">
                  Hint: Toggle output off → on to clear UV latch.
                </div>
              </div>
            </div>

            <div className="flex flex-wrap gap-2 items-center">
              <label className="label cursor-pointer justify-start gap-3 p-0">
                <input
                  type="checkbox"
                  className="toggle toggle-primary"
                  checked={control?.output_enabled ?? false}
                  disabled={
                    !control ||
                    updateControlMutation.isPending ||
                    applyPresetMutation.isPending
                  }
                  onChange={(event) => {
                    updateControlMutation.mutate({
                      output_enabled: event.target.checked,
                    });
                  }}
                />
                <div className="flex flex-col">
                  <span className="label-text font-semibold">
                    Output enabled
                  </span>
                  <span className="label-text-alt text-base-content/60">
                    Preset apply forces this off; toggle on to start the load.
                  </span>
                </div>
              </label>

              {ENABLE_MOCK_DEVTOOLS && baseUrl && isMockBaseUrl(baseUrl) ? (
                <button
                  type="button"
                  className="btn btn-xs"
                  disabled={!control || debugUvMutation.isPending}
                  onClick={() => {
                    debugUvMutation.mutate(!(control?.uv_latched ?? false));
                  }}
                >
                  Toggle UV latch (mock)
                </button>
              ) : null}
            </div>

            {updateControlMutation.isError && updateControlMutation.error ? (
              <div className="alert alert-error text-xs p-2">
                <span>
                  {isHttpApiError(updateControlMutation.error)
                    ? `${updateControlMutation.error.code ?? "HTTP_ERROR"} — ${updateControlMutation.error.message}`
                    : updateControlMutation.error instanceof Error
                      ? updateControlMutation.error.message
                      : "Unknown error"}
                </span>
              </div>
            ) : null}

            <div className="overflow-x-auto">
              <table className="table table-zebra table-sm">
                <thead>
                  <tr>
                    <th>Preset</th>
                    <th>Mode</th>
                    <th>Target</th>
                    <th>Limits</th>
                  </tr>
                </thead>
                <tbody>
                  {presetsQuery.isLoading ? (
                    <tr>
                      <td colSpan={4} className="text-xs text-base-content/60">
                        Loading presets…
                      </td>
                    </tr>
                  ) : (presets ?? []).length === 0 ? (
                    <tr>
                      <td colSpan={4} className="text-xs text-base-content/60">
                        No presets yet.
                      </td>
                    </tr>
                  ) : (
                    (presets ?? []).map((preset) => (
                      <tr
                        key={preset.preset_id}
                        data-testid="preset-row"
                        className={
                          preset.preset_id === selectedPresetId
                            ? "bg-base-200/60"
                            : undefined
                        }
                      >
                        <td className="whitespace-nowrap">
                          <button
                            type="button"
                            className={[
                              "btn btn-xs",
                              preset.preset_id === selectedPresetId
                                ? "btn-primary"
                                : "btn-ghost",
                            ].join(" ")}
                            onClick={() =>
                              setSelectedPresetId(preset.preset_id)
                            }
                          >
                            #{preset.preset_id}
                          </button>
                          {preset.preset_id === control?.active_preset_id ? (
                            <span className="ml-2 badge badge-xs badge-outline">
                              active
                            </span>
                          ) : null}
                        </td>
                        <td className="font-mono text-xs">{preset.mode}</td>
                        <td className="font-mono text-xs">
                          {preset.mode === "cc"
                            ? `${preset.target_i_ma} mA`
                            : preset.mode === "cp"
                              ? `${(preset.target_p_mw / 1_000).toFixed(2)} W`
                              : `${preset.target_v_mv} mV`}
                        </td>
                        <td className="font-mono text-xs">
                          min_v={preset.min_v_mv} · max_i=
                          {preset.max_i_ma_total} · max_p={preset.max_p_mw}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6 space-y-4">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 h-auto min-h-0">
              Preset editor
            </h3>

            <div className="form-control">
              <label className="label" htmlFor="preset-mode">
                <span className="label-text">Mode</span>
              </label>
              <select
                id="preset-mode"
                className="select select-bordered select-sm"
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
                <div className="mt-2 text-xs text-base-content/60">
                  CP: 固件不支持（identity.capabilities.cp_supported=false）
                </div>
              ) : null}
            </div>

            {draftPresetMode === "cc" ? (
              <div className="form-control">
                <label className="label" htmlFor="preset-target-i">
                  <span className="label-text">Target current (mA)</span>
                </label>
                <input
                  id="preset-target-i"
                  type="number"
                  className="input input-bordered input-sm"
                  value={draftPresetTargetIMa}
                  onChange={(event) =>
                    setDraftPresetTargetIMa(
                      Number.parseInt(event.target.value || "0", 10),
                    )
                  }
                />
              </div>
            ) : draftPresetMode === "cv" ? (
              <div className="form-control">
                <label className="label" htmlFor="preset-target-v">
                  <span className="label-text">Target voltage (mV)</span>
                </label>
                <input
                  id="preset-target-v"
                  type="number"
                  className="input input-bordered input-sm"
                  value={draftPresetTargetVMv}
                  onChange={(event) =>
                    setDraftPresetTargetVMv(
                      Number.parseInt(event.target.value || "0", 10),
                    )
                  }
                />
              </div>
            ) : (
              <div className="form-control">
                <label className="label" htmlFor="preset-target-p">
                  <span className="label-text">Target power (mW)</span>
                </label>
                <input
                  id="preset-target-p"
                  type="number"
                  className={[
                    "input input-bordered input-sm",
                    cpDraftOutOfRange ? "input-error" : "",
                  ].join(" ")}
                  value={draftPresetTargetPMw}
                  onChange={(event) =>
                    setDraftPresetTargetPMw(
                      Number.parseInt(event.target.value || "0", 10),
                    )
                  }
                />
                {cpDraftOutOfRange ? (
                  <div className="mt-2 text-xs text-error">
                    target_p_mw must be ≤ max_p_mw
                  </div>
                ) : null}
              </div>
            )}

            <div className="form-control">
              <label className="label" htmlFor="preset-min-v">
                <span className="label-text">Min voltage (mV)</span>
              </label>
              <input
                id="preset-min-v"
                type="number"
                className="input input-bordered input-sm"
                value={draftPresetMinVMv}
                onChange={(event) =>
                  setDraftPresetMinVMv(
                    Number.parseInt(event.target.value || "0", 10),
                  )
                }
              />
            </div>

            <div className="form-control">
              <label className="label" htmlFor="preset-max-i">
                <span className="label-text">Max current total (mA)</span>
              </label>
              <input
                id="preset-max-i"
                type="number"
                className="input input-bordered input-sm"
                value={draftPresetMaxIMaTotal}
                onChange={(event) =>
                  setDraftPresetMaxIMaTotal(
                    Number.parseInt(event.target.value || "0", 10),
                  )
                }
              />
            </div>

            <div className="form-control">
              <label className="label" htmlFor="preset-max-p">
                <span className="label-text">Max power (mW)</span>
              </label>
              <input
                id="preset-max-p"
                type="number"
                className="input input-bordered input-sm"
                value={draftPresetMaxPMw}
                onChange={(event) =>
                  setDraftPresetMaxPMw(
                    Number.parseInt(event.target.value || "0", 10),
                  )
                }
              />
            </div>

            <div className="flex flex-col gap-2 pt-2">
              <button
                type="button"
                className="btn btn-sm"
                disabled={savePresetDisabled}
                onClick={handleSavePreset}
              >
                {updatePresetMutation.isPending ? (
                  <span className="loading loading-spinner loading-xs" />
                ) : null}
                Save preset
              </button>
              <button
                type="button"
                className="btn btn-sm btn-primary"
                disabled={!baseUrl || applyPresetMutation.isPending}
                onClick={handleApplyPreset}
              >
                {applyPresetMutation.isPending ? (
                  <span className="loading loading-spinner loading-xs" />
                ) : null}
                Apply preset (forces output off)
              </button>
            </div>

            {updatePresetMutation.isError && updatePresetMutation.error ? (
              <div className="alert alert-error text-xs p-2">
                <span>
                  {isHttpApiError(updatePresetMutation.error)
                    ? `${updatePresetMutation.error.code ?? "HTTP_ERROR"} — ${updatePresetMutation.error.message}`
                    : updatePresetMutation.error instanceof Error
                      ? updatePresetMutation.error.message
                      : "Unknown error"}
                </span>
                {isHttpApiError(updatePresetMutation.error) &&
                explainHttpError(updatePresetMutation.error) ? (
                  <span className="opacity-90">
                    {explainHttpError(updatePresetMutation.error)}
                  </span>
                ) : null}
              </div>
            ) : null}

            {applyPresetMutation.isError && applyPresetMutation.error ? (
              <div className="alert alert-error text-xs p-2">
                <span>
                  {isHttpApiError(applyPresetMutation.error)
                    ? `${applyPresetMutation.error.code ?? "HTTP_ERROR"} — ${applyPresetMutation.error.message}`
                    : applyPresetMutation.error instanceof Error
                      ? applyPresetMutation.error.message
                      : "Unknown error"}
                </span>
                {isHttpApiError(applyPresetMutation.error) &&
                explainHttpError(applyPresetMutation.error) ? (
                  <span className="opacity-90">
                    {explainHttpError(applyPresetMutation.error)}
                  </span>
                ) : null}
              </div>
            ) : null}
          </div>
        </div>
      </section>

      <section aria-label="Main display" className="flex justify-center">
        <MainDisplayCanvas
          remoteVoltageV={remoteVoltageV}
          localVoltageV={localVoltageV}
          localCurrentA={localCurrentA}
          remoteCurrentA={remoteCurrentA}
          totalCurrentA={totalCurrentA}
          totalPowerW={totalPowerW}
          controlMode={controlMode}
          controlTargetMilli={controlTargetMilli}
          controlTargetUnit={controlTargetUnit}
          uptimeSeconds={uptimeSeconds}
          tempCoreC={tempCoreC}
          tempSinkC={tempSinkC}
          tempMcuC={tempMcuC}
          remoteActive={remoteActive}
          analogState={analogState}
          faultFlags={faultFlags}
        />
      </section>
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

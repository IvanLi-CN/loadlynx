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
import { useDeviceContext } from "../layouts/device-layout.tsx";

const MONO_FONT_FAMILY =
  'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace';

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

  const activeLoadModeBadge = control?.preset.mode === "cv" ? "CV" : "CC";

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
  const tempC =
    status?.raw.sink_core_temp_mc != null
      ? status.raw.sink_core_temp_mc / 1_000
      : undefined;

  const handleSavePreset = () => {
    const payload: Preset = {
      preset_id: selectedPresetId,
      mode: draftPresetMode,
      target_i_ma: draftPresetTargetIMa,
      target_v_mv: draftPresetTargetVMv,
      min_v_mv: draftPresetMinVMv,
      max_i_ma_total: draftPresetMaxIMaTotal,
      max_p_mw: draftPresetMaxPMw,
    };

    updatePresetMutation.mutate(payload);
  };

  const handleApplyPreset = () => {
    applyPresetMutation.mutate(selectedPresetId);
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

              <button
                type="button"
                className="btn btn-sm btn-primary"
                disabled={
                  !control ||
                  control.output_enabled ||
                  updateControlMutation.isPending ||
                  applyPresetMutation.isPending
                }
                onClick={() => {
                  updateControlMutation.mutate({ output_enabled: true });
                }}
              >
                Start load
              </button>

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
              </select>
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
            ) : (
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
                disabled={!baseUrl || updatePresetMutation.isPending}
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
          setTargetA={(status?.raw.target_value ?? 0) / 1_000}
          uptimeSeconds={uptimeSeconds}
          tempC={tempC}
        />
      </section>
    </PageContainer>
  );
}

interface MainDisplayCanvasProps {
  remoteVoltageV: number;
  localVoltageV: number;
  localCurrentA: number;
  remoteCurrentA: number;
  totalCurrentA: number;
  totalPowerW: number;
  setTargetA: number;
  uptimeSeconds: number;
  tempC: number | undefined;
}

function MainDisplayCanvas({
  remoteVoltageV,
  localVoltageV,
  localCurrentA,
  remoteCurrentA,
  totalCurrentA,
  totalPowerW,
  setTargetA,
  uptimeSeconds,
  tempC,
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

    // Base background.
    ctx.fillStyle = "#05070D";
    ctx.fillRect(0, 0, width, height);

    // Left primary block background.
    ctx.fillStyle = "#101829";
    ctx.fillRect(0, 0, 190, height);

    // Right block background.
    ctx.fillStyle = "#05070D";
    ctx.fillRect(190, 0, width - 190, height);

    const labelColor = "#9AB0D8";
    const voltageColor = "#FFB347";
    const currentColor = "#FF5252";
    const powerColor = "#6EF58C";
    const rightLabel = "#6D7FA4";
    const rightValue = "#DFE7FF";

    const drawSevenSegmentNumber = (
      value: number,
      precision: number,
      xRight: number,
      baselineY: number,
      color: string,
    ) => {
      const text =
        Number.isFinite(value) && !Number.isNaN(value)
          ? value.toFixed(precision)
          : "----";

      const glyphWidth = sevenSegFontWidth;
      const glyphHeight = sevenSegFontHeight;
      const digitSpacing = 4;
      const dotSize = 6;
      const dotBaselineOffset = 8; // dot center is 8 px above baseline in the reference mock

      ctx.fillStyle = color;

      const drawDigitGlyph = (
        code: number,
        leftX: number,
        baseline: number,
      ) => {
        if (
          code < sevenSegFontFirstChar ||
          code >= sevenSegFontFirstChar + sevenSegFontCharCount
        ) {
          return;
        }
        const topY = baseline - glyphHeight;
        for (let gy = 0; gy < glyphHeight; gy += 1) {
          for (let gx = 0; gx < glyphWidth; gx += 1) {
            if (isSevenSegPixel(code, gx, gy)) {
              const px = leftX + gx;
              const py = topY + gy;
              ctx.fillRect(px, py, 1, 1);
            }
          }
        }
      };

      let cursorX = xRight;
      for (let index = text.length - 1; index >= 0; index -= 1) {
        const ch = text[index];

        if (ch >= "0" && ch <= "9") {
          const code = ch.charCodeAt(0);
          const glyphLeft = cursorX - glyphWidth;
          drawDigitGlyph(code, glyphLeft, baselineY);
          cursorX = glyphLeft - digitSpacing;
        } else if (ch === ".") {
          const dotLeft = cursorX - dotSize;
          const dotTop = baselineY - dotBaselineOffset - dotSize;
          ctx.fillRect(dotLeft, dotTop, dotSize, dotSize);
          cursorX = dotLeft - digitSpacing;
        } else if (ch === " ") {
          cursorX -= glyphWidth / 2;
        } else if (ch === "-") {
          const dashWidth = glyphWidth * 0.6;
          const dashHeight = 4;
          const dashLeft = cursorX - dashWidth;
          const dashY = baselineY - glyphHeight / 2;
          ctx.fillRect(dashLeft, dashY, dashWidth, dashHeight);
          cursorX = dashLeft - digitSpacing;
        }
      }
    };

    const drawMetric = (
      y: number,
      label: string,
      value: number,
      unit: string,
      color: string,
      precision: number,
    ) => {
      const slabTop = y + 4;
      const slabHeight = 72;
      ctx.fillStyle = "#171F33";
      ctx.beginPath();
      const radius = 6;
      const x = 8;
      const w = 174;
      ctx.moveTo(x + radius, slabTop);
      ctx.lineTo(x + w - radius, slabTop);
      ctx.quadraticCurveTo(x + w, slabTop, x + w, slabTop + radius);
      ctx.lineTo(x + w, slabTop + slabHeight - radius);
      ctx.quadraticCurveTo(
        x + w,
        slabTop + slabHeight,
        x + w - radius,
        slabTop + slabHeight,
      );
      ctx.lineTo(x + radius, slabTop + slabHeight);
      ctx.quadraticCurveTo(
        x,
        slabTop + slabHeight,
        x,
        slabTop + slabHeight - radius,
      );
      ctx.lineTo(x, slabTop + radius);
      ctx.quadraticCurveTo(x, slabTop, x + radius, slabTop);
      ctx.closePath();
      ctx.fill();

      // Label.
      ctx.fillStyle = labelColor;
      ctx.font = `8px ${MONO_FONT_FAMILY}`;
      ctx.textBaseline = "top";
      ctx.fillText(label, 16, y + 6);

      // Main digits: SevenSegNumFont rendering aligned to the right edge
      // of the slab area. Baseline matches the hardware mock at y = 72
      // for the top card, hence y + 72 for subsequent cards.
      const digitsBaselineY = y + 72;

      // Reserve space for the unit on the far right and push the numeric
      // value slightly left so glyphs and unit never collide.
      ctx.font = `10px ${MONO_FONT_FAMILY}`;
      ctx.textBaseline = "alphabetic";
      const unitRightX = x + w - 6;
      const unitWidth = ctx.measureText(unit).width;
      const unitLeftX = unitRightX - unitWidth;

      const digitsRightX = unitLeftX - 6;
      drawSevenSegmentNumber(
        value,
        precision,
        digitsRightX,
        digitsBaselineY,
        color,
      );

      // Unit.
      ctx.fillStyle = labelColor;
      ctx.font = `10px ${MONO_FONT_FAMILY}`;
      ctx.textBaseline = "alphabetic";
      ctx.fillText(unit, unitLeftX, digitsBaselineY);
    };

    drawMetric(0, "VOLTAGE", remoteVoltageV, "V", voltageColor, 2);
    drawMetric(80, "CURRENT", totalCurrentA, "A", currentColor, 2);
    drawMetric(160, "POWER", totalPowerW, "W", powerColor, 1);

    // Right column: REMOTE/LOCAL voltages.
    ctx.font = `8px ${MONO_FONT_FAMILY}`;
    ctx.textBaseline = "top";
    ctx.fillStyle = rightLabel;
    ctx.fillText("REMOTE", 198, 8);
    ctx.fillText("LOCAL", 258, 8);

    ctx.fillStyle = rightValue;
    const remoteText = `${remoteVoltageV.toFixed(2)}V`;
    const localText = `${localVoltageV.toFixed(2)}V`;
    const voltageValueY = 18;
    ctx.fillText(remoteText, 198, voltageValueY);
    ctx.fillText(localText, 258, voltageValueY);

    // Voltage mirror bar.
    const barLeft = 198;
    const barRight = 314;
    const barHeight = 6;
    const barOffsetY = 12;
    const barTop = voltageValueY + barOffsetY;
    const barCenter = (barLeft + barRight) / 2;
    ctx.fillStyle = "#1C2638";
    ctx.fillRect(barLeft, barTop, barRight - barLeft, barHeight);

    const drawMirrorFill = (
      value: number,
      max: number,
      side: "left" | "right",
    ) => {
      const ratio = Math.max(0, Math.min(1, value / max));
      ctx.fillStyle = "#4CC9F0";
      if (side === "left") {
        const widthHalf = (barRight - barLeft) / 2;
        const widthVal = widthHalf * ratio;
        ctx.fillRect(barCenter - widthVal, barTop, widthVal, barHeight);
      } else {
        const widthHalf = (barRight - barLeft) / 2;
        const widthVal = widthHalf * ratio;
        ctx.fillRect(barCenter, barTop, widthVal, barHeight);
      }
    };

    drawMirrorFill(remoteVoltageV, 40, "left");
    drawMirrorFill(localVoltageV, 40, "right");

    // Current pair (CH1 / CH2).
    ctx.fillStyle = rightLabel;
    ctx.fillText("CH1", 198, 80);
    ctx.fillText("CH2", 258, 80);

    ctx.fillStyle = rightValue;
    const iLocalText = `${localCurrentA.toFixed(2)}A`;
    const iRemoteText = `${remoteCurrentA.toFixed(2)}A`;
    const currentValueY = 90;
    ctx.fillText(iLocalText, 198, currentValueY);
    ctx.fillText(iRemoteText, 258, currentValueY);

    // Second mirror bar: same offset from the CURRENT numeric values as the
    // first bar has from the voltage numeric values, to keep symmetry.
    const bar2Top = currentValueY + barOffsetY;
    ctx.fillStyle = "#1C2638";
    ctx.fillRect(barLeft, bar2Top, barRight - barLeft, barHeight);
    drawMirrorFill(localCurrentA, 5, "left");
    drawMirrorFill(remoteCurrentA, 5, "right");

    // SET line.
    ctx.fillStyle = rightLabel;
    const setBaselineY = bar2Top + barHeight + 8;
    ctx.fillText("SET", 198, setBaselineY);
    ctx.fillStyle = rightValue;
    const ccText = `${setTargetA.toFixed(1)}A`;
    const ccTextWidth = ctx.measureText(ccText).width;
    ctx.fillText(ccText, barRight - ccTextWidth, setBaselineY);

    // RUN / TEMP / ENERGY.
    const timeText = `${String(Math.floor(uptimeSeconds / 3_600)).padStart(
      2,
      "0",
    )}:${String(Math.floor((uptimeSeconds % 3_600) / 60)).padStart(
      2,
      "0",
    )}:${String(uptimeSeconds % 60).padStart(2, "0")}`;

    let infoY = 168;
    ctx.fillStyle = rightValue;
    ctx.fillText(`RUN ${timeText}`, 198, infoY);
    infoY += 12;

    if (typeof tempC === "number") {
      const tempText = `TEMP ${tempC.toFixed(1)}°C`;
      ctx.fillText(tempText, 198, infoY);
      infoY += 12;
    }

    const energyWh =
      totalPowerW > 0 && uptimeSeconds > 0
        ? (totalPowerW * uptimeSeconds) / 3_600
        : 0;
    const energyText = `ENERGY ${energyWh.toFixed(1)}Wh`;
    ctx.fillText(energyText, 198, infoY);
  }, [
    canvasSize.height,
    canvasSize.width,
    remoteVoltageV,
    localVoltageV,
    localCurrentA,
    remoteCurrentA,
    totalCurrentA,
    totalPowerW,
    setTargetA,
    uptimeSeconds,
    tempC,
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

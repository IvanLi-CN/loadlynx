import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import {
  getCc,
  getIdentity,
  getStatus,
  isHttpApiError,
  subscribeStatusStream,
  updateCc,
} from "../api/client.ts";
import type {
  CcControlView,
  CcUpdateRequest,
  FastStatusView,
  Identity,
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

  const ccQuery = useQuery<CcControlView, HttpApiError>({
    queryKey: ["device", deviceId, "cc"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getCc(baseUrl);
    },
    enabled: Boolean(baseUrl) && identityQuery.isSuccess,
    retryDelay: RETRY_DELAY_MS,
  });

  const queryClient = useQueryClient();

  const [draftEnable, setDraftEnable] = useState(false);
  const [draftTargetIMa, setDraftTargetIMa] = useState(0);
  const [draftMaxPMw, setDraftMaxPMw] = useState(0);

  useEffect(() => {
    const cc = ccQuery.data;
    if (!cc) {
      return;
    }
    setDraftEnable(cc.enable);
    setDraftTargetIMa(cc.target_i_ma);
    setDraftMaxPMw(cc.limit_profile.max_p_mw);
  }, [ccQuery.data]);

  const updateCcMutation = useMutation({
    mutationFn: async (payload: CcUpdateRequest) => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return updateCc(baseUrl, payload);
    },
    onSuccess: (nextCc) => {
      queryClient.setQueryData<CcControlView>(
        ["device", deviceId, "cc"],
        nextCc,
      );
      queryClient.invalidateQueries({
        queryKey: ["device", deviceId, "status"],
      });
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
      !updateCcMutation.isPending &&
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
      ccQuery.error,
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
  const cc = ccQuery.data;

  const statusLocalMa = status?.raw.i_local_ma ?? null;
  const statusRemoteMa = status?.raw.i_remote_ma ?? null;

  const remoteVoltageV =
    status?.raw.v_remote_mv != null
      ? status.raw.v_remote_mv / 1_000
      : cc?.v_main_mv != null
        ? cc.v_main_mv / 1_000
        : 0;
  const localVoltageV =
    status?.raw.v_local_mv != null
      ? status.raw.v_local_mv / 1_000
      : remoteVoltageV;
  const localCurrentA =
    statusLocalMa != null
      ? statusLocalMa / 1_000
      : (cc?.i_total_ma ?? 0) / 1_000 / 2;
  const remoteCurrentA = statusRemoteMa != null ? statusRemoteMa / 1_000 : 0;
  const totalCurrentA =
    statusLocalMa != null && statusRemoteMa != null
      ? (statusLocalMa + statusRemoteMa) / 1_000
      : (cc?.i_total_ma ?? 0) / 1_000;
  const totalPowerW =
    status?.raw.calc_p_mw != null
      ? status.raw.calc_p_mw / 1_000
      : (cc?.p_main_mw ?? 0) / 1_000;

  const maxIMa =
    cc?.limit_profile.max_i_ma != null ? cc.limit_profile.max_i_ma : 5_000;
  const maxPMw =
    cc?.limit_profile.max_p_mw != null ? cc.limit_profile.max_p_mw : 60_000;

  const uptimeSeconds =
    status?.raw.uptime_ms != null
      ? Math.floor(status.raw.uptime_ms / 1_000)
      : 0;
  const tempC =
    status?.raw.sink_core_temp_mc != null
      ? status.raw.sink_core_temp_mc / 1_000
      : undefined;

  const handleApply = () => {
    if (!cc) {
      return;
    }

    const payload: CcUpdateRequest = {
      enable: draftEnable,
      target_i_ma: draftTargetIMa,
      max_p_mw: draftMaxPMw,
    };

    updateCcMutation.mutate(payload);
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
                const isActive = mode === "CC";
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

      <section aria-label="Main display" className="flex justify-center">
        <MainDisplayCanvas
          remoteVoltageV={remoteVoltageV}
          localVoltageV={localVoltageV}
          localCurrentA={localCurrentA}
          remoteCurrentA={remoteCurrentA}
          totalCurrentA={totalCurrentA}
          totalPowerW={totalPowerW}
          ccTargetA={(cc?.target_i_ma ?? 0) / 1_000}
          uptimeSeconds={uptimeSeconds}
          tempC={tempC}
        />
      </section>

      <section
        aria-label="CC control"
        className="grid grid-cols-1 lg:grid-cols-3 gap-6 items-start"
      >
        <div className="card bg-base-100 shadow-sm border border-base-200 lg:col-span-2">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Setpoint
            </h3>

            <div className="flex flex-col gap-6">
              <div className="form-control">
                <label className="label" htmlFor="input-target-current">
                  <span className="label-text">Target current</span>
                  <span className="label-text-alt font-mono">
                    {(draftTargetIMa / 1_000).toFixed(2)} A · {draftTargetIMa}{" "}
                    mA
                  </span>
                </label>
                <input
                  id="input-target-current"
                  type="range"
                  min={0}
                  max={maxIMa}
                  step={50}
                  value={draftTargetIMa}
                  onChange={(event) => {
                    setDraftTargetIMa(Number.parseInt(event.target.value, 10));
                  }}
                  className="range range-primary range-sm"
                />
              </div>

              <div className="form-control">
                <label className="label" htmlFor="input-max-power">
                  <span className="label-text">Max power limit</span>
                  <span className="label-text-alt font-mono">
                    {(draftMaxPMw / 1_000).toFixed(1)} W · {draftMaxPMw} mW
                  </span>
                </label>
                <input
                  id="input-max-power"
                  type="range"
                  min={10_000}
                  max={maxPMw}
                  step={1_000}
                  value={draftMaxPMw}
                  onChange={(event) => {
                    setDraftMaxPMw(Number.parseInt(event.target.value, 10));
                  }}
                  className="range range-sm"
                />
              </div>
            </div>
          </div>
        </div>

        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Output &amp; limits
            </h3>

            <div className="flex flex-col gap-4">
              <div className="form-control">
                <label className="label cursor-pointer justify-start gap-4">
                  <input
                    type="checkbox"
                    className="toggle toggle-primary"
                    checked={draftEnable}
                    onChange={(event) => {
                      setDraftEnable(event.target.checked);
                    }}
                  />
                  <div className="flex flex-col">
                    <span className="label-text font-semibold">
                      Enable output
                    </span>
                    <span className="label-text-alt text-base-content/60">
                      zero setpoint if disabled
                    </span>
                  </div>
                </label>
              </div>

              <div className="text-xs text-base-content/60 space-y-1 bg-base-200/50 p-3 rounded-lg">
                <div>
                  Status:{" "}
                  <span
                    className={
                      cc?.enable
                        ? "text-success font-bold"
                        : "text-base-content font-bold"
                    }
                  >
                    {cc?.enable ? "enabled" : "disabled"}
                  </span>
                </div>
                <div>
                  Target:{" "}
                  <span className="font-mono">
                    {(cc?.target_i_ma ?? 0) / 1_000} A
                  </span>
                </div>
                <div>
                  Limit:{" "}
                  <span className="font-mono">
                    {(cc?.limit_profile.max_p_mw ?? 0) / 1_000} W
                  </span>
                </div>
              </div>

              {updateCcMutation.isError && updateCcMutation.error ? (
                <div className="alert alert-error text-xs p-2">
                  <span>
                    {(() => {
                      const error = updateCcMutation.error;
                      if (isHttpApiError(error)) {
                        const code = error.code ?? "HTTP_ERROR";
                        if (error.status === 0 && code === "NETWORK_ERROR") {
                          return `Network error — unable to reach device${
                            baseUrl ? ` (${baseUrl})` : ""
                          }. Check network/IP.`;
                        }
                        if (
                          error.status === 404 &&
                          code === "UNSUPPORTED_OPERATION"
                        ) {
                          return "API unsupported by device firmware — please upgrade and retry.";
                        }
                        if (error.status >= 400 && error.status < 500 && code) {
                          return `Device rejected request: ${code} — ${error.message}`;
                        }
                        return `HTTP error: ${code} — ${error.message}`;
                      }
                      if (error instanceof Error) {
                        return `Error: ${error.message}`;
                      }
                      return "Error: unknown HTTP failure";
                    })()}
                  </span>
                </div>
              ) : null}

              <button
                type="button"
                onClick={handleApply}
                disabled={updateCcMutation.isPending || !cc}
                className="btn btn-primary w-full mt-2"
              >
                {updateCcMutation.isPending ? (
                  <span className="loading loading-spinner text-primary-content"></span>
                ) : null}
                {updateCcMutation.isPending ? "Applying..." : "Apply changes"}
              </button>
            </div>
          </div>
        </div>
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
  ccTargetA: number;
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
  ccTargetA,
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
    const ccText = `${ccTargetA.toFixed(1)}A`;
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
    ccTargetA,
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

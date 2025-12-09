import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import {
  getCc,
  getIdentity,
  getStatus,
  isHttpApiError,
  updateCc,
} from "../api/client.ts";
import type {
  CcControlView,
  CcUpdateRequest,
  FastStatusView,
  Identity,
} from "../api/types.ts";
import { useDevicesQuery } from "../devices/hooks.ts";
import {
  isSevenSegPixel,
  sevenSegFontCharCount,
  sevenSegFontFirstChar,
  sevenSegFontHeight,
  sevenSegFontWidth,
} from "../fonts/sevenSegFont.ts";

const MONO_FONT_FAMILY =
  'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace';

const FAST_STATUS_REFETCH_MS = 400;
const RETRY_DELAY_MS = 500;

export function DeviceCcRoute() {
  const { deviceId } = useParams({
    from: "/$deviceId/cc",
  }) as {
    deviceId: string;
  };

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

  const devicesQuery = useDevicesQuery();
  const device = useMemo(
    () => devicesQuery.data?.find((entry) => entry.id === deviceId),
    [devicesQuery.data, deviceId],
  );

  const baseUrl = device?.baseUrl;

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
      !updateCcMutation.isPending,
    refetchInterval: isPageVisible ? FAST_STATUS_REFETCH_MS : false,
    refetchIntervalInBackground: false,
    retryDelay: RETRY_DELAY_MS,
  });

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
        "无法连接设备" +
        (baseUrl ? `（baseUrl=${baseUrl}）` : "") +
        "，请检查网络与 IP 设置。";
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

  if (devicesQuery.isLoading) {
    return (
      <p
        style={{
          fontSize: "0.9rem",
          color: "#9ca3af",
        }}
      >
        Loading devices...
      </p>
    );
  }

  if (!device) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "0.75rem",
          maxWidth: "640px",
        }}
      >
        <h2
          style={{
            margin: 0,
            fontSize: "1.1rem",
          }}
        >
          Device not found
        </h2>
        <p
          style={{
            margin: 0,
            fontSize: "0.9rem",
            color: "#9ca3af",
          }}
        >
          The requested device ID <code>{deviceId}</code> does not exist in the
          local registry. Please return to the device list and add or select a
          device.
        </p>
        <div>
          <Link
            to="/devices"
            style={{
              display: "inline-flex",
              alignItems: "center",
              padding: "0.4rem 0.8rem",
              borderRadius: "0.375rem",
              border: "1px solid #4b5563",
              color: "#e5e7eb",
              textDecoration: "none",
              fontSize: "0.9rem",
            }}
          >
            Back to devices
          </Link>
        </div>
      </div>
    );
  }

  const identity = identityQuery.data;
  const status = statusQuery.data;
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
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "0.9rem",
        maxWidth: "960px",
        fontFamily: MONO_FONT_FAMILY,
        fontVariantNumeric: "tabular-nums",
      }}
    >
      {/* Top context: device + mode strip + link status */}
      <header
        style={{
          display: "flex",
          justifyContent: "space-between",
          gap: "0.8rem",
          alignItems: "flex-start",
        }}
      >
        <div>
          <h2
            style={{
              margin: 0,
              fontSize: "1.05rem",
            }}
          >
            Device control
          </h2>
          <p
            style={{
              margin: "0.25rem 0 0",
              fontSize: "0.85rem",
              color: "#9ca3af",
            }}
          >
            Shared control surface for all operating modes. This view focuses on
            CC behaviour while keeping the layout aligned with the hardware main
            display.
          </p>
          <p
            style={{
              margin: "0.25rem 0 0",
              fontSize: "0.75rem",
              color: "#6b7280",
            }}
          >
            Device name:{" "}
            <strong style={{ fontWeight: 500 }}>{device.name}</strong>
            {identity ? (
              <>
                {" "}
                · IP: <code>{identity.network.ip}</code>
              </>
            ) : null}
          </p>
        </div>
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "flex-end",
            gap: "0.4rem",
            fontSize: "0.78rem",
            color: "#9ca3af",
          }}
        >
          <div
            style={{
              display: "flex",
              gap: "0.4rem",
              alignItems: "center",
            }}
          >
            <span
              style={{
                fontSize: "0.75rem",
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                color: "#6b7280",
              }}
            >
              Mode
            </span>
            <div
              style={{
                display: "inline-flex",
                gap: "0.25rem",
              }}
            >
              {["CC", "CV", "CP", "CR"].map((mode) => {
                const isActive = mode === "CC";
                return (
                  <span
                    key={mode}
                    style={{
                      padding: "0.15rem 0.5rem",
                      borderRadius: "999px",
                      border: isActive
                        ? "1px solid #38bdf8"
                        : "1px solid #374151",
                      backgroundColor: isActive ? "#0f172a" : "#020617",
                      fontSize: "0.75rem",
                      color: isActive ? "#e0f2fe" : "#9ca3af",
                      opacity: mode === "CC" ? 1 : 0.55,
                    }}
                  >
                    {mode}
                  </span>
                );
              })}
            </div>
          </div>

          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "0.2rem",
              textAlign: "right",
            }}
          >
            <div>
              Link:{" "}
              <span
                style={{
                  color: status?.link_up ? "#22c55e" : "#f97316",
                  fontWeight: 500,
                }}
              >
                {status?.link_up ? "up" : "down"}
              </span>
            </div>
            <div>
              Analog state: <span>{status?.analog_state ?? "unknown"}</span>
            </div>
            {identity ? (
              <div>
                API version: <code>{identity.capabilities.api_version}</code>
              </div>
            ) : null}
            {status && status.fault_flags_decoded.length > 0 ? (
              <div
                style={{
                  color: "#f97316",
                  fontSize: "0.75rem",
                }}
              >
                Faults: {status.fault_flags_decoded.join(", ")}
              </div>
            ) : null}
          </div>
        </div>
      </header>

      {topError ? (
        <section
          aria-label="HTTP error"
          style={{
            marginTop: "0.25rem",
            padding: "0.5rem 0.75rem",
            borderRadius: "0.5rem",
            border: "1px solid #7f1d1d",
            background:
              "linear-gradient(135deg, rgba(127,29,29,0.35), rgba(15,23,42,0.9))",
            color: "#fecaca",
            fontSize: "0.8rem",
          }}
        >
          <div>
            <strong style={{ fontWeight: 600 }}>HTTP error:</strong>{" "}
            {topError.summary}
          </div>
          {topError.hint ? (
            <div
              style={{
                marginTop: "0.15rem",
                color: "#fed7d7",
              }}
            >
              {topError.hint}
            </div>
          ) : null}
          {isLinkDownLike ? (
            <div
              style={{
                marginTop: "0.15rem",
                color: "#fed7d7",
              }}
            >
              Link down / Wi‑Fi unavailable — telemetry and control updates may
              be stale until connectivity recovers.
            </div>
          ) : null}
        </section>
      ) : null}

      <section
        aria-label="Main display"
        style={{
          padding: "0.4rem 0.25rem 0.4rem",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "center",
          }}
        >
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
        </div>
      </section>

      <section
        aria-label="CC control"
        style={{
          display: "grid",
          gridTemplateColumns: "minmax(0, 1.8fr) minmax(0, 1.1fr)",
          gap: "0.75rem",
          alignItems: "flex-start",
          marginTop: "0.2rem",
        }}
      >
        <div
          style={{
            padding: "0.7rem 0.9rem",
            borderRadius: "0.6rem",
            border: "1px solid #111827",
            backgroundColor: "#020617",
          }}
        >
          <h3
            style={{
              margin: 0,
              fontSize: "0.9rem",
              marginBottom: "0.6rem",
            }}
          >
            Setpoint
          </h3>

          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "0.9rem",
            }}
          >
            <label
              style={{
                display: "flex",
                flexDirection: "column",
                gap: "0.2rem",
                fontSize: "0.8rem",
              }}
            >
              <span>
                Target current{" "}
                <span
                  style={{
                    color: "#9ca3af",
                    fontSize: "0.78rem",
                  }}
                >
                  ({(draftTargetIMa / 1_000).toFixed(2)} A · {draftTargetIMa}{" "}
                  mA)
                </span>
              </span>
              <input
                type="range"
                min={0}
                max={maxIMa}
                step={50}
                value={draftTargetIMa}
                onChange={(event) => {
                  setDraftTargetIMa(Number.parseInt(event.target.value, 10));
                }}
              />
            </label>

            <label
              style={{
                display: "flex",
                flexDirection: "column",
                gap: "0.2rem",
                fontSize: "0.8rem",
              }}
            >
              <span>
                Max power limit{" "}
                <span
                  style={{
                    color: "#9ca3af",
                    fontSize: "0.8rem",
                  }}
                >
                  ({(draftMaxPMw / 1_000).toFixed(1)} W · {draftMaxPMw} mW)
                </span>
              </span>
              <input
                type="range"
                min={10_000}
                max={maxPMw}
                step={1_000}
                value={draftMaxPMw}
                onChange={(event) => {
                  setDraftMaxPMw(Number.parseInt(event.target.value, 10));
                }}
              />
            </label>
          </div>
        </div>

        <div
          style={{
            padding: "0.7rem 0.9rem",
            borderRadius: "0.6rem",
            border: "1px solid #111827",
            backgroundColor: "#020617",
          }}
        >
          <h3
            style={{
              margin: 0,
              fontSize: "0.9rem",
              marginBottom: "0.6rem",
            }}
          >
            Output &amp; limits
          </h3>

          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "0.55rem",
              fontSize: "0.8rem",
            }}
          >
            <label
              style={{
                display: "flex",
                alignItems: "center",
                gap: "0.5rem",
              }}
            >
              <input
                type="checkbox"
                checked={draftEnable}
                onChange={(event) => {
                  setDraftEnable(event.target.checked);
                }}
              />
              <span>
                Enable output{" "}
                <span
                  style={{
                    color: "#9ca3af",
                    fontSize: "0.78rem",
                  }}
                >
                  (current firmware maps disable to a zero setpoint)
                </span>
              </span>
            </label>

            <div
              style={{
                marginTop: "0.25rem",
                fontSize: "0.78rem",
                color: "#9ca3af",
              }}
            >
              Current state:{" "}
              <strong style={{ fontWeight: 500 }}>
                {cc?.enable ? "enabled" : "disabled"}
              </strong>{" "}
              · target {(cc?.target_i_ma ?? 0) / 1_000} A · limit{" "}
              {(cc?.limit_profile.max_p_mw ?? 0) / 1_000} W
            </div>

            {updateCcMutation.isError && updateCcMutation.error ? (
              <div
                style={{
                  marginTop: "0.35rem",
                  padding: "0.35rem 0.55rem",
                  borderRadius: "0.4rem",
                  border: "1px solid #7f1d1d",
                  backgroundColor: "#450a0a",
                  color: "#fecaca",
                  fontSize: "0.78rem",
                }}
              >
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
              </div>
            ) : null}

            <button
              type="button"
              onClick={handleApply}
              disabled={updateCcMutation.isPending || !cc}
              style={{
                marginTop: "0.4rem",
                padding: "0.35rem 0.8rem",
                borderRadius: "0.375rem",
                border: "1px solid #4b5563",
                backgroundColor: "#111827",
                color: "#e5e7eb",
                fontSize: "0.85rem",
                cursor: updateCcMutation.isPending ? "wait" : "pointer",
              }}
            >
              {updateCcMutation.isPending ? "Applying..." : "Apply changes"}
            </button>
          </div>
        </div>
      </section>
    </div>
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
    <div
      style={{
        width: "100%",
        maxWidth: 640,
        aspectRatio: "4 / 3",
        borderRadius: "1rem",
        backgroundColor: "#05070D",
        boxShadow:
          "0 18px 40px rgba(15,23,42,0.85), 0 0 0 1px rgba(15,23,42,0.9)",
        boxSizing: "border-box",
        overflow: "hidden",
      }}
    >
      <canvas
        ref={canvasRef}
        width={320}
        height={240}
        style={{
          width: "100%",
          height: "100%",
          imageRendering: "pixelated",
          display: "block",
        }}
      />
    </div>
  );
}

export default DeviceCcRoute;

import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import { getIdentity, getStatus, isHttpApiError } from "../api/client.ts";
import type { FastStatusView, Identity } from "../api/types.ts";
import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";

const FAST_STATUS_REFETCH_MS = 1000;
const RETRY_DELAY_MS = 500;

export function DeviceStatusRoute() {
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

  const statusQuery = useQuery<FastStatusView, HttpApiError>({
    queryKey: ["device", deviceId, "status"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getStatus(baseUrl);
    },
    enabled: Boolean(baseUrl) && identityQuery.isSuccess,
    refetchInterval: isPageVisible ? FAST_STATUS_REFETCH_MS : false,
    refetchIntervalInBackground: false,
    retryDelay: RETRY_DELAY_MS,
  });

  const firstHttpError: HttpApiError | null = (() => {
    const errors: Array<unknown> = [identityQuery.error, statusQuery.error];
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

  const identity = identityQuery.data;
  const status = statusQuery.data;

  // Derived values
  const statusLocalMa = status?.raw.i_local_ma ?? null;
  const statusRemoteMa = status?.raw.i_remote_ma ?? null;
  const totalCurrentA =
    statusLocalMa != null && statusRemoteMa != null
      ? (statusLocalMa + statusRemoteMa) / 1_000
      : 0;
  const totalPowerW =
    status?.raw.calc_p_mw != null ? status.raw.calc_p_mw / 1_000 : 0;
  const localVoltageV =
    status?.raw.v_local_mv != null ? status.raw.v_local_mv / 1_000 : 0;

  // Temps
  const tempSinkCore =
    status?.raw.sink_core_temp_mc != null
      ? status.raw.sink_core_temp_mc / 1000
      : null;
  const tempSinkExhaust =
    status?.raw.sink_exhaust_temp_mc != null
      ? status.raw.sink_exhaust_temp_mc / 1000
      : null;
  const tempMcu =
    status?.raw.mcu_temp_mc != null ? status.raw.mcu_temp_mc / 1000 : null;

  return (
    <PageContainer className="flex flex-col gap-6 font-mono tabular-nums">
      {/* Top Header */}
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-bold">Device Status</h2>
          <p className="mt-1 text-sm text-base-content/70">
            Real-time operating status and telemetry.
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
          </div>
        </div>
      </header>

      {/* Error Alert */}
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
                Link down / Wi‑Fi unavailable — telemetry may be stale.
              </div>
            ) : null}
          </div>
        </section>
      ) : null}

      <div className="grid gap-6 md:grid-cols-2">
        {/* Card 1: Overview */}
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Overview
            </h3>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <div className="text-xs text-base-content/60">Voltage</div>
                <div className="text-xl font-medium">
                  {localVoltageV.toFixed(3)} V
                </div>
              </div>
              <div>
                <div className="text-xs text-base-content/60">
                  Total Current
                </div>
                <div className="text-xl font-medium">
                  {totalCurrentA.toFixed(3)} A
                </div>
              </div>
              <div>
                <div className="text-xs text-base-content/60">Total Power</div>
                <div className="text-xl font-medium">
                  {totalPowerW.toFixed(2)} W
                </div>
              </div>
              <div>
                <div className="text-xs text-base-content/60">Uptime</div>
                <div className="text-xl font-medium">
                  {status?.raw.uptime_ms
                    ? `${(status.raw.uptime_ms / 1000).toFixed(0)} s`
                    : "-"}
                </div>
              </div>
              <div className="col-span-2 border-t border-base-200 pt-2 mt-2">
                <div className="flex justify-between text-xs">
                  <span className="text-base-content/60">Protocol V</span>
                  <span>{identity?.protocol_version ?? "-"}</span>
                </div>
                <div className="flex justify-between text-xs mt-1">
                  <span className="text-base-content/60">API V</span>
                  <span>{identity?.capabilities.api_version ?? "-"}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Card 2: Temperature & Faults */}
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Temperature & Faults
            </h3>

            <div className="flex flex-col gap-4">
              <div className="grid grid-cols-3 gap-2 text-center">
                <div className="bg-base-200/50 p-2 rounded">
                  <div className="text-xs text-base-content/60 mb-1">
                    Sink Core
                  </div>
                  <div className="font-medium">
                    {tempSinkCore?.toFixed(1) ?? "-"} °C
                  </div>
                </div>
                <div className="bg-base-200/50 p-2 rounded">
                  <div className="text-xs text-base-content/60 mb-1">
                    Sink Exhaust
                  </div>
                  <div className="font-medium">
                    {tempSinkExhaust?.toFixed(1) ?? "-"} °C
                  </div>
                </div>
                <div className="bg-base-200/50 p-2 rounded">
                  <div className="text-xs text-base-content/60 mb-1">MCU</div>
                  <div className="font-medium">
                    {tempMcu?.toFixed(1) ?? "-"} °C
                  </div>
                </div>
              </div>

              <div className="mt-2">
                <div className="text-xs text-base-content/60 mb-2">
                  Active Faults
                </div>
                <div className="flex flex-wrap gap-2">
                  {status?.fault_flags_decoded &&
                  status.fault_flags_decoded.length > 0 ? (
                    status.fault_flags_decoded.map((fault) => (
                      <div key={fault} className="badge badge-error gap-1">
                        <svg
                          xmlns="http://www.w3.org/2000/svg"
                          fill="none"
                          viewBox="0 0 24 24"
                          className="inline-block w-3 h-3 stroke-current"
                          role="img"
                          aria-label="Error icon"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth="2"
                            d="M6 18L18 6M6 6l12 12"
                          />
                        </svg>
                        {fault}
                      </div>
                    ))
                  ) : (
                    <div className="badge badge-ghost text-base-content/60">
                      No active faults
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      {/* Aux Card: Raw JSON */}
      <div className="collapse collapse-arrow border border-base-200 bg-base-100 rounded-box">
        <input type="checkbox" />
        <div className="collapse-title text-sm font-medium text-base-content/70">
          Raw Status JSON
        </div>
        <div className="collapse-content">
          <pre className="text-xs bg-base-200 p-4 rounded overflow-x-auto">
            {status ? JSON.stringify(status.raw, null, 2) : "No data"}
          </pre>
        </div>
      </div>
    </PageContainer>
  );
}

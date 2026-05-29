import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import {
  deleteWifiConfig,
  exportDiagnostics,
  getIdentity,
  getWifiStatus,
  isDevdCompatBaseUrl,
  isHttpApiError,
  isMockBaseUrl,
  postSoftReset,
  postWifiConfig,
} from "../api/client.ts";
import type { Identity } from "../api/types.ts";
import { ConfirmDialog } from "../components/common/confirm-dialog.tsx";
import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";

export function DeviceSettingsRoute() {
  const { deviceId, baseUrl } = useDeviceContext();
  const [confirmSoftResetOpen, setConfirmSoftResetOpen] = useState(false);
  const [confirmWifiAction, setConfirmWifiAction] = useState<
    "save" | "clear" | null
  >(null);
  const [wifiSsid, setWifiSsid] = useState("");
  const [wifiPsk, setWifiPsk] = useState("");

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

  const wifiQuery = useQuery({
    queryKey: ["device", deviceId, "wifi"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getWifiStatus(baseUrl);
    },
    enabled: Boolean(baseUrl),
  });

  const topError = (() => {
    const err = identityQuery.error;
    if (!err || !isHttpApiError(err)) return null;

    const code = err.code ?? "HTTP_ERROR";
    const summary = `${code} — ${err.message}`;

    if (err.status === 0 && code === "NETWORK_ERROR") {
      return { summary, hint: "Check device connectivity." } as const;
    }
    return { summary, hint: null } as const;
  })();

  const softResetMutation = useMutation({
    mutationFn: async () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return postSoftReset(baseUrl, "manual");
    },
  });

  const wifiMutation = useMutation({
    mutationFn: async () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return postWifiConfig(baseUrl, {
        ssid: wifiSsid.trim(),
        psk: wifiPsk,
        wait: false,
      });
    },
    onSuccess: () => {
      setWifiPsk("");
      void wifiQuery.refetch();
    },
  });

  const wifiClearMutation = useMutation({
    mutationFn: async () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return deleteWifiConfig(baseUrl);
    },
    onSuccess: () => {
      void wifiQuery.refetch();
    },
  });

  const diagnosticsMutation = useMutation({
    mutationFn: async () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return exportDiagnostics(baseUrl);
    },
  });

  const softResetError = (() => {
    const err = softResetMutation.error;
    if (!err || !isHttpApiError(err)) return null;

    const code = err.code ?? "HTTP_ERROR";
    const summary = `Soft reset failed: ${code} — ${err.message}`;

    let hint: string | null = null;
    if (code === "NETWORK_ERROR") {
      hint = "Network error: check device network/IP.";
    } else if (code === "LINK_DOWN" || code === "UNAVAILABLE") {
      hint = "Link is not ready; soft reset is temporarily unavailable.";
    }

    return { summary, hint } as const;
  })();

  const identity = identityQuery.data;
  const wifi = wifiQuery.data;
  const wifiLanConfirmationRequired = baseUrl
    ? !isMockBaseUrl(baseUrl) && !isDevdCompatBaseUrl(baseUrl)
    : false;

  const handleSoftReset = () => {
    if (!baseUrl) {
      return;
    }
    setConfirmSoftResetOpen(true);
  };

  const handleWifiSave = () => {
    if (wifiLanConfirmationRequired) {
      setConfirmWifiAction("save");
      return;
    }
    wifiMutation.mutate();
  };

  const handleWifiClear = () => {
    if (wifiLanConfirmationRequired) {
      setConfirmWifiAction("clear");
      return;
    }
    wifiClearMutation.mutate();
  };

  return (
    <PageContainer className="flex flex-col gap-6 font-mono tabular-nums">
      <ConfirmDialog
        open={confirmSoftResetOpen}
        title="Soft Reset"
        body="确定要进行 Soft Reset 吗？当前输出会被重置。"
        details={["Writes device: Yes.", "May interrupt ongoing output."]}
        confirmLabel="Soft Reset"
        destructive
        confirmDisabled={softResetMutation.isPending}
        onCancel={() => setConfirmSoftResetOpen(false)}
        onConfirm={() => {
          setConfirmSoftResetOpen(false);
          softResetMutation.reset();
          softResetMutation.mutate();
        }}
      />
      <ConfirmDialog
        open={confirmWifiAction !== null}
        title={confirmWifiAction === "clear" ? "Clear User WiFi" : "Save WiFi"}
        body="This LAN request writes WiFi settings over the network."
        details={[
          "Use the local USB/devd path when available.",
          "The PSK will not be shown in diagnostics or traces.",
        ]}
        confirmLabel={
          confirmWifiAction === "clear" ? "Clear User WiFi" : "Save WiFi"
        }
        destructive={confirmWifiAction === "clear"}
        confirmDisabled={
          wifiMutation.isPending || wifiClearMutation.isPending || !baseUrl
        }
        onCancel={() => setConfirmWifiAction(null)}
        onConfirm={() => {
          const action = confirmWifiAction;
          setConfirmWifiAction(null);
          if (action === "clear") {
            wifiClearMutation.mutate();
          } else if (action === "save") {
            wifiMutation.mutate();
          }
        }}
      />
      <header>
        <h2 className="text-lg font-bold">Device Settings</h2>
        <p className="mt-1 text-sm text-base-content/70">
          Device information and configuration.
        </p>
      </header>

      {topError ? (
        <div className="alert alert-error shadow-sm rounded-lg text-sm">
          <span className="font-bold">Error: {topError.summary}</span>
          {topError.hint && (
            <span className="text-xs opacity-80 block">{topError.hint}</span>
          )}
        </div>
      ) : null}

      <div className="grid gap-6 md:grid-cols-2">
        {/* 1. Device Identity */}
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Device Identity
            </h3>
            <div className="overflow-x-auto">
              <table className="table table-xs">
                <tbody>
                  <tr>
                    <td className="text-base-content/60">Device ID</td>
                    <td>
                      <code className="bg-base-200 px-1 rounded">
                        {identity?.device_id ?? "..."}
                      </code>
                    </td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">Digital FW</td>
                    <td>{identity?.digital_fw_version ?? "..."}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">Analog FW</td>
                    <td>{identity?.analog_fw_version ?? "..."}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">Protocol</td>
                    <td>v{identity?.protocol_version ?? "..."}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">Uptime</td>
                    <td>
                      {identity?.uptime_ms
                        ? `${(identity.uptime_ms / 1000).toFixed(0)} s`
                        : "..."}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* 2. Network */}
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Network
            </h3>
            <div className="overflow-x-auto">
              <table className="table table-xs">
                <tbody>
                  <tr>
                    <td className="text-base-content/60">Hostname</td>
                    <td>{identity?.network.hostname ?? "..."}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">IP Address</td>
                    <td>
                      <code className="bg-base-200 px-1 rounded">
                        {identity?.network.ip ?? "..."}
                      </code>
                    </td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">MAC Address</td>
                    <td className="uppercase">
                      {identity?.network.mac ?? "..."}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* 3. Capabilities */}
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Capabilities
            </h3>
            <div className="flex flex-wrap gap-2">
              {identity ? (
                <>
                  <div
                    className={`badge ${identity.capabilities.cc_supported ? "badge-neutral" : "badge-ghost opacity-50"}`}
                  >
                    CC
                  </div>
                  <div
                    className={`badge ${identity.capabilities.cv_supported ? "badge-neutral" : "badge-ghost opacity-50"}`}
                  >
                    CV
                  </div>
                  <div
                    className={`badge ${identity.capabilities.cp_supported ? "badge-neutral" : "badge-ghost opacity-50"}`}
                  >
                    CP
                  </div>
                  <div className="badge badge-ghost">
                    API v{identity.capabilities.api_version}
                  </div>
                </>
              ) : (
                <span className="text-xs text-base-content/50">Loading...</span>
              )}
            </div>
          </div>
        </div>

        {/* 4. WiFi */}
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              WiFi
            </h3>
            <div className="grid gap-3">
              <div className="grid grid-cols-2 gap-3 text-xs">
                <span className="text-base-content/60">SSID</span>
                <span className="truncate">{wifi?.ssid ?? "..."}</span>
                <span className="text-base-content/60">Source</span>
                <span>{wifi?.source ?? "..."}</span>
                <span className="text-base-content/60">State</span>
                <span>{wifi?.state ?? "..."}</span>
                <span className="text-base-content/60">IP</span>
                <span>{wifi?.ip ?? "..."}</span>
              </div>
              <input
                className="input input-bordered input-sm w-full"
                placeholder="SSID"
                value={wifiSsid}
                onChange={(event) => setWifiSsid(event.target.value)}
              />
              <input
                className="input input-bordered input-sm w-full"
                placeholder="PSK"
                type="password"
                value={wifiPsk}
                onChange={(event) => setWifiPsk(event.target.value)}
              />
              {wifiMutation.error && isHttpApiError(wifiMutation.error) ? (
                <div className="alert alert-error shadow-sm text-xs">
                  <span>
                    WiFi update failed: {wifiMutation.error.code ?? "HTTP"} —{" "}
                    {wifiMutation.error.message}
                  </span>
                </div>
              ) : null}
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  className="btn btn-neutral btn-sm"
                  disabled={
                    !wifiSsid.trim() ||
                    !wifiPsk ||
                    wifiMutation.isPending ||
                    !baseUrl
                  }
                  onClick={handleWifiSave}
                >
                  Save WiFi
                </button>
                <button
                  type="button"
                  className="btn btn-outline btn-sm"
                  disabled={wifiClearMutation.isPending || !baseUrl}
                  onClick={handleWifiClear}
                >
                  Clear User WiFi
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* 5. Actions */}
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Actions
            </h3>
            <div className="flex flex-col gap-3">
              {softResetMutation.isSuccess ? (
                <div className="alert alert-success shadow-sm text-xs sm:text-sm">
                  <span>
                    Soft reset requested (reason:{" "}
                    {softResetMutation.data?.reason ?? "manual"}).
                  </span>
                </div>
              ) : null}
              {softResetError ? (
                <div className="alert alert-error shadow-sm text-xs sm:text-sm">
                  <span className="font-bold">{softResetError.summary}</span>
                  {softResetError.hint && (
                    <span className="text-xs opacity-80 block">
                      {softResetError.hint}
                    </span>
                  )}
                </div>
              ) : null}
              <button
                type="button"
                className="btn btn-outline btn-sm text-error hover:bg-error hover:text-white"
                onClick={handleSoftReset}
              >
                Soft Reset
              </button>
              <button
                type="button"
                className="btn btn-outline btn-sm"
                disabled={diagnosticsMutation.isPending || !baseUrl}
                onClick={() => diagnosticsMutation.mutate()}
              >
                Export Diagnostics
              </button>
              {diagnosticsMutation.isSuccess ? (
                <pre className="max-h-48 overflow-auto rounded bg-base-200 p-3 text-[11px] leading-relaxed">
                  {JSON.stringify(diagnosticsMutation.data, null, 2)}
                </pre>
              ) : null}
            </div>
          </div>
        </div>
      </div>
    </PageContainer>
  );
}

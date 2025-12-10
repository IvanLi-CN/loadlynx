import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { useMemo } from "react";
import type { HttpApiError } from "../api/client.ts";
import { getIdentity, isHttpApiError } from "../api/client.ts";
import type { Identity } from "../api/types.ts";
import { useDevicesQuery } from "../devices/hooks.ts";

export function DeviceSettingsRoute() {
  const { deviceId } = useParams({
    from: "/$deviceId/settings",
  }) as {
    deviceId: string;
  };

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

  if (devicesQuery.isLoading) {
    return <p className="text-sm text-base-content/60">Loading devices...</p>;
  }

  if (!device) {
    return (
      <div className="flex flex-col gap-4 max-w-xl">
        <h2 className="text-xl font-bold">Device not found</h2>
        <p className="text-sm text-base-content/70">
          The requested device ID{" "}
          <code className="font-mono bg-base-200 px-1 rounded">{deviceId}</code>{" "}
          does not exist in the local registry.
        </p>
        <div>
          <Link to="/devices" className="btn btn-sm btn-outline">
            Back to devices
          </Link>
        </div>
      </div>
    );
  }

  const identity = identityQuery.data;

  const handleSoftReset = () => {
    // Placeholder
    alert("Soft reset API is not yet implemented.");
  };

  return (
    <div className="flex flex-col gap-6 max-w-5xl font-mono tabular-nums">
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

        {/* 4. Actions */}
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-6">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
              Actions
            </h3>
            <div>
              <button
                type="button"
                className="btn btn-outline btn-sm text-error hover:bg-error hover:text-white"
                onClick={handleSoftReset}
              >
                Soft Reset
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

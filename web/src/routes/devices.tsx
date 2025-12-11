import { useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { ENABLE_MOCK, isHttpApiError } from "../api/client.ts";
import type { StoredDevice } from "../devices/device-store.ts";
import {
  type DiscoveredDevice,
  type ScanProgress,
  useAddDeviceMutation,
  useAddRealDeviceMutation,
  useDeviceIdentity,
  useDevicesQuery,
  useSubnetScanMutation,
} from "../devices/hooks.ts";

export function DevicesRoute() {
  const queryClient = useQueryClient();
  const devicesQuery = useDevicesQuery();
  const addDeviceMutation = useAddDeviceMutation();
  const addRealDeviceMutation = useAddRealDeviceMutation();

  const [newDeviceName, setNewDeviceName] = useState("");
  const [newDeviceBaseUrl, setNewDeviceBaseUrl] = useState("");
  const [addDeviceError, setAddDeviceError] = useState<string | null>(null);

  const devices: StoredDevice[] = useMemo(
    () => devicesQuery.data ?? [],
    [devicesQuery.data],
  );

  const isMutating = addDeviceMutation.isPending;
  const isAddingReal = addRealDeviceMutation.isPending;

  // Scanning state
  const scanMutation = useSubnetScanMutation();
  const [isScanPanelOpen, setIsScanPanelOpen] = useState(false);
  const [seedIp, setSeedIp] = useState("192.168.1.100"); // Default/Example
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [scanResults, setScanResults] = useState<DiscoveredDevice[]>([]);
  const [scanError, setScanError] = useState<string | null>(null);

  const startScan = () => {
    setScanProgress({ scannedCount: 0, totalCount: 0, foundCount: 0 });
    setScanResults([]);
    setScanError(null);

    scanMutation.mutate(
      {
        options: { seedIp: seedIp.trim() },
        onProgress: (p: ScanProgress) => setScanProgress(p),
      },
      {
        onSuccess: (data: DiscoveredDevice[]) => {
          setScanResults(data);
        },
        onError: (err: Error) => {
          setScanError(err instanceof Error ? err.message : "Unknown error");
        },
      },
    );
  };

  const isScanning = scanMutation.isPending;

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      <header>
        <h2 className="text-2xl font-bold">Devices</h2>
        <p className="mt-1 text-sm text-base-content/70">
          Manage known devices for the LoadLynx network console. Each device is
          probed via <code className="code">/api/v1/identity</code> to show live
          status.
        </p>
      </header>

      <div className="card bg-base-100 shadow-sm border border-base-200">
        <div className="card-body p-4">
          <form
            onSubmit={(event) => {
              event.preventDefault();
              const name = newDeviceName.trim();
              const baseUrl = newDeviceBaseUrl.trim();

              if (!name || !baseUrl) {
                setAddDeviceError("Name and base URL are required.");
                return;
              }

              const lowerBaseUrl = baseUrl.toLowerCase();
              if (
                !lowerBaseUrl.startsWith("http://") &&
                !lowerBaseUrl.startsWith("https://")
              ) {
                setAddDeviceError(
                  "Base URL must start with http:// or https://.",
                );
                return;
              }

              setAddDeviceError(null);
              addRealDeviceMutation.mutate(
                { name, baseUrl },
                {
                  onSuccess: () => {
                    setNewDeviceName("");
                    setNewDeviceBaseUrl("");
                    // Keep any stale query instances (e.g. from other tabs) in sync.
                    queryClient.invalidateQueries({ queryKey: ["devices"] });
                  },
                },
              );
            }}
            className="flex flex-col gap-4"
          >
            <div className="flex flex-wrap gap-4 items-end">
              <label className="form-control flex-1 min-w-[200px]">
                <div className="label pb-1">
                  <span className="label-text">Device name</span>
                </div>
                <input
                  type="text"
                  value={newDeviceName}
                  onChange={(event) => setNewDeviceName(event.target.value)}
                  placeholder="My LoadLynx"
                  className="input input-bordered w-full"
                />
              </label>
              <label className="form-control flex-[2] min-w-[250px]">
                <div className="label pb-1">
                  <span className="label-text">Base URL</span>
                </div>
                <input
                  type="text"
                  value={newDeviceBaseUrl}
                  onChange={(event) => setNewDeviceBaseUrl(event.target.value)}
                  placeholder="http://loadlynx-a1b2c3.local"
                  className="input input-bordered w-full"
                />
              </label>
              <button
                type="submit"
                disabled={isAddingReal}
                className="btn btn-primary"
              >
                {isAddingReal ? (
                  <span className="loading loading-spinner loading-xs"></span>
                ) : null}
                {isAddingReal ? "Adding..." : "Add device"}
              </button>
            </div>
            {addDeviceError ? (
              <div role="alert" className="alert alert-error text-sm py-2">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  className="stroke-current shrink-0 h-4 w-4"
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
                <span>{addDeviceError}</span>
              </div>
            ) : (
              <div className="text-xs text-base-content/60">
                Enter the device <strong>Hostname</strong> (recommended) or IP
                address.
                <br />
                Example:{" "}
                <code className="code">http://loadlynx-d68638.local</code> (uses
                the Short ID from the device screen).
              </div>
            )}
          </form>
        </div>
      </div>

      {/* Scan Panel Toggle */}
      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => setIsScanPanelOpen(!isScanPanelOpen)}
          className="btn btn-sm btn-ghost"
        >
          {isScanPanelOpen ? "Hide Network Scanner" : "Scan current network..."}
        </button>
      </div>

      {/* Scan Panel */}
      {isScanPanelOpen && (
        <div className="card bg-base-100 shadow-sm border border-base-200">
          <div className="card-body p-4">
            <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50">
              LAN Scanner
            </h3>

            <div className="alert alert-warning text-xs">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="stroke-current shrink-0 h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                role="img"
                aria-label="Warning icon"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                />
              </svg>
              <span>
                <b>Risk Warning:</b> This scan sends short-lived HTTP probes to
                the seed IP’s entire /24 subnet. It is intended for small lab
                networks only and may be blocked or discouraged on
                managed/corporate networks.
              </span>
            </div>

            <div className="flex flex-wrap gap-4 items-end mt-2">
              <label className="form-control flex-1 min-w-[200px]">
                <div className="label pb-1">
                  <span className="label-text">Seed IP (Example Device)</span>
                </div>
                <input
                  type="text"
                  value={seedIp}
                  onChange={(e) => setSeedIp(e.target.value)}
                  placeholder="e.g. 192.168.1.100"
                  disabled={isScanning}
                  className="input input-bordered w-full input-sm"
                />
              </label>
              <button
                type="button"
                onClick={startScan}
                disabled={isScanning || !seedIp}
                className="btn btn-primary btn-sm"
              >
                {isScanning ? (
                  <span className="loading loading-spinner loading-xs"></span>
                ) : null}
                {isScanning ? "Scanning..." : "Start scan"}
              </button>
              {isScanning && (
                <button
                  type="button"
                  className="btn btn-ghost btn-sm"
                  onClick={() => scanMutation.reset()}
                >
                  Cancel
                </button>
              )}
            </div>

            {scanError && (
              <div className="text-error text-sm mt-2">
                Scan failed: {scanError}
              </div>
            )}

            {isScanning && scanProgress && (
              <div className="mt-4 space-y-2">
                <div className="flex justify-between text-xs text-base-content/70">
                  <span>
                    Scanned {scanProgress.scannedCount} /{" "}
                    {scanProgress.totalCount} hosts
                  </span>
                  <span>Found {scanProgress.foundCount} devices</span>
                </div>
                <progress
                  className="progress progress-primary w-full"
                  value={scanProgress.scannedCount}
                  max={scanProgress.totalCount}
                ></progress>
              </div>
            )}

            {/* Scan Results */}
            {scanResults.length > 0 && (
              <div className="mt-4">
                <h4 className="font-bold text-sm mb-2">
                  Discovered Devices ({scanResults.length})
                </h4>
                <div className="overflow-x-auto">
                  <table className="table table-xs">
                    <thead>
                      <tr>
                        <th>Hostname</th>
                        <th>IP</th>
                        <th>Device ID</th>
                        <th>Action</th>
                      </tr>
                    </thead>
                    <tbody>
                      {scanResults.map((d) => (
                        <tr key={d.ip}>
                          <td>{d.hostname || "-"}</td>
                          <td className="font-mono">{d.ip}</td>
                          <td className="font-mono">{d.identity.device_id}</td>
                          <td>
                            <button
                              type="button"
                              className="btn btn-xs btn-outline btn-primary"
                              onClick={() => {
                                addRealDeviceMutation.mutate({
                                  name:
                                    d.hostname ||
                                    d.identity.device_id ||
                                    `Device ${d.ip}`,
                                  baseUrl: `http://${d.ip}`,
                                });
                              }}
                              disabled={isAddingReal}
                            >
                              Add
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
            {scanMutation.isSuccess && scanResults.length === 0 && (
              <div className="text-sm text-base-content/60 mt-4 italic">
                No LoadLynx devices discovered in this subnet.
              </div>
            )}
          </div>
        </div>
      )}

      {ENABLE_MOCK ? (
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => {
              addDeviceMutation.mutate(undefined, {
                onSuccess: () => {
                  // Keep any stale query instances (e.g. from other tabs) in sync.
                  queryClient.invalidateQueries({ queryKey: ["devices"] });
                },
              });
            }}
            disabled={isMutating}
            className="btn btn-secondary btn-sm"
          >
            {isMutating ? "Adding device..." : "Add demo device"}
          </button>
          <span className="text-xs text-base-content/60">
            Adds a built-in demo device backed by an in-memory backend (for
            development).
          </span>
        </div>
      ) : null}

      <div className="card bg-base-100 shadow-sm border border-base-200">
        <div className="card-body p-0 overflow-x-auto">
          {devicesQuery.isLoading ? (
            <div className="p-8 text-center text-base-content/60">
              Loading devices...
            </div>
          ) : devices.length === 0 ? (
            <div className="p-8 text-center text-base-content/60">
              {ENABLE_MOCK ? (
                <>
                  No devices yet. Use the{" "}
                  <strong className="font-medium text-base-content">
                    Add demo device
                  </strong>{" "}
                  action above to seed a demo entry.
                </>
              ) : (
                <>No devices yet. Add one or more real devices to begin.</>
              )}
            </div>
          ) : (
            <table className="table table-zebra table-sm">
              <thead className="bg-base-200">
                <tr>
                  <th>Name</th>
                  <th>Device ID</th>
                  <th>Base URL</th>
                  <th>Status</th>
                  <th className="text-right">Actions</th>
                </tr>
              </thead>
              <tbody>
                {devices.map((device) => (
                  <DeviceRow key={device.id} device={device} />
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}

export default DevicesRoute;

function DeviceRow(props: { device: StoredDevice }) {
  const { device } = props;
  const identityQuery = useDeviceIdentity(device);

  const identity = identityQuery.data;
  const error: unknown = identityQuery.error;

  let statusBadgeClass = "badge badge-ghost";
  let statusLabel = "Checking...";
  let statusDetail: string | null = null;

  if (identityQuery.isLoading || identityQuery.isFetching) {
    statusBadgeClass = "badge badge-ghost";
    statusLabel = "Checking...";
  } else if (identityQuery.isSuccess && identity) {
    statusBadgeClass = "badge badge-success";
    statusLabel = "Online";

    const primaryHost = identity.hostname ?? identity.network?.hostname;
    if (primaryHost) {
      statusDetail = `${primaryHost} (${identity.network.ip})`;
    } else {
      statusDetail = identity.network.ip;
    }
  } else if (identityQuery.isError) {
    statusBadgeClass = "badge badge-error";
    statusLabel = "Offline";

    const formatSnippet = (message: string) =>
      message.length > 80 ? `${message.slice(0, 77)}…` : message;

    if (isHttpApiError(error)) {
      const code = error.code ?? "HTTP_ERROR";
      if (error.status === 0 && code === "NETWORK_ERROR") {
        statusDetail = "网络异常，已自动重试；如仍失败请检查设备 IP 或网络";
      } else if (error.status === 404 && code === "UNSUPPORTED_OPERATION") {
        statusBadgeClass = "badge badge-warning";
        statusLabel = "Online (HTTP)";
        statusDetail = "Unsupported API on current firmware";
      } else {
        statusDetail = `${code}: ${formatSnippet(error.message)}`;
      }
    } else if (error instanceof Error) {
      statusDetail = formatSnippet(error.message);
    } else {
      statusDetail = "Unknown error";
    }
  }

  return (
    <tr>
      <td className="font-medium">{device.name}</td>
      <td className="font-mono text-xs">{device.id}</td>
      <td className="font-mono text-xs text-base-content/70">
        {device.baseUrl}
      </td>
      <td>
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <div className={`badge badge-sm gap-2 ${statusBadgeClass}`}>
              {statusLabel}
            </div>
            <button
              type="button"
              onClick={() => {
                void identityQuery.refetch();
              }}
              disabled={identityQuery.isFetching}
              className="btn btn-ghost btn-xs btn-circle"
              title="Test connectivity"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                strokeWidth={1.5}
                stroke="currentColor"
                className={`w-4 h-4 ${identityQuery.isFetching ? "animate-spin" : ""}`}
                role="img"
                aria-label="Refresh icon"
              >
                <title>Refresh</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99"
                />
              </svg>
            </button>
          </div>
          {statusDetail ? (
            <span
              className="text-xs text-base-content/60 max-w-[200px] truncate"
              title={statusDetail}
            >
              {statusDetail}
            </span>
          ) : null}
        </div>
      </td>
      <td className="text-right">
        <Link
          to="/$deviceId/cc"
          params={{ deviceId: device.id }}
          className="btn btn-sm btn-outline"
        >
          Open CC Control
        </Link>
      </td>
    </tr>
  );
}

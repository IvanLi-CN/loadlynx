import { useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { ENABLE_MOCK_DEVTOOLS, isHttpApiError } from "../api/client.ts";
import { PageContainer } from "../components/layout/page-container.tsx";
import {
  buildDevdCompatBaseUrl,
  DEFAULT_DEVD_BASE_URL,
} from "../devd/client.ts";
import { useCreateDevdLease, useDevdScan } from "../devd/hooks.ts";
import type { DevdDevice } from "../devd/types.ts";
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
import { readStoredDemoMode } from "../lib/demo-mode.ts";

export function DevicesRoute() {
  const queryClient = useQueryClient();
  const devicesQuery = useDevicesQuery();
  const addDeviceMutation = useAddDeviceMutation();
  const addRealDeviceMutation = useAddRealDeviceMutation();
  const isDemoMode =
    typeof window !== "undefined" &&
    readStoredDemoMode(window.localStorage) === true;

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
  const devdScan = useDevdScan();
  const createDevdLease = useCreateDevdLease();
  const [devdDevices, setDevdDevices] = useState<DevdDevice[]>([]);
  const [selectedDevdDeviceId, setSelectedDevdDeviceId] = useState("");
  const [devdError, setDevdError] = useState<string | null>(null);

  const startScan = () => {
    if (isDemoMode) return;

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
    <PageContainer className="space-y-6">
      <header>
        <h2 className="text-2xl font-bold">Devices</h2>
        <p className="mt-1 text-sm text-base-content/70">
          Manage known devices for the LoadLynx network console. Each device is
          probed via <code className="code">/api/v1/identity</code> to show live
          status.
        </p>
      </header>

      <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
        <div className="ll-panel-body p-4">
          <form
            onSubmit={(event) => {
              event.preventDefault();

              if (isDemoMode) {
                setAddDeviceError(
                  "Demo mode only uses built-in mock:// devices. Switch demo=false to add real devices.",
                );
                return;
              }

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
            {isDemoMode ? (
              <output className="ll-alert ll-alert-info text-sm">
                Demo mode is mock-only. Real HTTP devices are disabled here, and
                the device table is limited to mock:// entries.
              </output>
            ) : null}
            <div className="flex flex-wrap gap-4 items-end">
              <label className="ll-form-control flex-1 min-w-[200px]">
                <div className="ll-label-row pb-1">
                  <span className="ll-label-text">Device name</span>
                </div>
                <input
                  id="device-name"
                  name="device_name"
                  type="text"
                  value={newDeviceName}
                  onChange={(event) => setNewDeviceName(event.target.value)}
                  placeholder="My LoadLynx"
                  disabled={isDemoMode}
                  className="ll-input w-full"
                />
              </label>
              <label className="ll-form-control flex-[2] min-w-[250px]">
                <div className="ll-label-row pb-1">
                  <span className="ll-label-text">Base URL</span>
                </div>
                <input
                  id="device-base-url"
                  name="device_base_url"
                  type="text"
                  value={newDeviceBaseUrl}
                  onChange={(event) => setNewDeviceBaseUrl(event.target.value)}
                  placeholder="http://loadlynx-a1b2c3.local"
                  disabled={isDemoMode}
                  className="ll-input w-full"
                />
              </label>
              <button
                type="submit"
                disabled={isAddingReal || isDemoMode}
                className="ll-button ll-button-primary"
              >
                {isAddingReal ? (
                  <span className="ll-loading ll-loading-spinner ll-loading-xs"></span>
                ) : null}
                {isAddingReal ? "Adding..." : "Add device"}
              </button>
            </div>
            {addDeviceError ? (
              <div
                role="alert"
                className="ll-alert ll-alert-error text-sm py-2"
              >
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

      <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
        <div className="ll-panel-body p-4 gap-4">
          <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h3 className="ll-panel-title text-base">Local devd bridge</h3>
              <p className="text-xs text-base-content/60">
                Discover USB/probe candidates through{" "}
                <code className="code">{DEFAULT_DEVD_BASE_URL}</code>. A device
                is only connected after you choose a candidate and create a
                lease.
              </p>
            </div>
            <button
              type="button"
              className="ll-button ll-button-sm ll-button-outline"
              disabled={isDemoMode || devdScan.isPending}
              onClick={() => {
                if (isDemoMode) return;
                setDevdError(null);
                devdScan.mutate(undefined, {
                  onSuccess: (payload) => {
                    setDevdDevices(payload.devices);
                    setSelectedDevdDeviceId(payload.devices[0]?.id ?? "");
                  },
                  onError: (error) => {
                    setDevdError(
                      error instanceof Error
                        ? error.message
                        : "devd scan failed",
                    );
                  },
                });
              }}
            >
              {devdScan.isPending ? (
                <span className="ll-loading ll-loading-spinner ll-loading-xs"></span>
              ) : null}
              Scan devd
            </button>
          </div>

          {devdError ? (
            <div role="alert" className="ll-alert ll-alert-error py-2 text-sm">
              <span>{devdError}</span>
            </div>
          ) : null}

          {devdDevices.length > 0 ? (
            <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
              <label className="ll-form-control">
                <div className="ll-label-row pb-1">
                  <span className="ll-label-text">devd candidate</span>
                </div>
                <select
                  id="devd-candidate"
                  name="devd_candidate"
                  className="ll-select ll-select-sm w-full"
                  value={selectedDevdDeviceId}
                  onChange={(event) =>
                    setSelectedDevdDeviceId(event.target.value)
                  }
                >
                  {devdDevices.map((device) => (
                    <option key={device.id} value={device.id}>
                      {device.display_name} · {device.id}
                    </option>
                  ))}
                </select>
              </label>
              <button
                type="button"
                className="ll-button ll-button-primary ll-button-sm"
                disabled={
                  isDemoMode ||
                  !selectedDevdDeviceId ||
                  createDevdLease.isPending
                }
                onClick={() => {
                  if (isDemoMode) return;
                  const candidate = devdDevices.find(
                    (device) => device.id === selectedDevdDeviceId,
                  );
                  if (!candidate) return;
                  createDevdLease.mutate(candidate.id, {
                    onSuccess: (lease) => {
                      const connectionMarks: StoredDevice["connectionMarks"] = [
                        "usb",
                      ];
                      if (candidate.lan_endpoint) {
                        connectionMarks.push("lan");
                      }
                      const devd = {
                        baseUrl: DEFAULT_DEVD_BASE_URL,
                        deviceId: candidate.id,
                        leaseId: lease.lease_id,
                      };
                      const baseUrl = buildDevdCompatBaseUrl(devd);
                      addRealDeviceMutation.mutate(
                        {
                          name: candidate.display_name,
                          baseUrl,
                          connectionMarks,
                          devd,
                        },
                        {
                          onSuccess: () => {
                            queryClient.invalidateQueries({
                              queryKey: ["devices"],
                            });
                          },
                        },
                      );
                    },
                    onError: (error) => {
                      setDevdError(
                        error instanceof Error
                          ? error.message
                          : "failed to create devd lease",
                      );
                    },
                  });
                }}
              >
                {createDevdLease.isPending ? (
                  <span className="ll-loading ll-loading-spinner ll-loading-xs"></span>
                ) : null}
                Create USB lease
              </button>
            </div>
          ) : (
            <div className="text-xs text-base-content/60">
              Scan returns mock, cached selector and USB serial candidates
              without auto-connecting to hardware.
            </div>
          )}

          {devdDevices.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="ll-table ll-table-xs">
                <thead>
                  <tr>
                    <th>Candidate</th>
                    <th>Digital</th>
                    <th>Analog</th>
                    <th>LAN</th>
                    <th>State</th>
                  </tr>
                </thead>
                <tbody>
                  {devdDevices.map((device) => (
                    <tr key={device.id}>
                      <td>
                        <div className="font-medium">{device.display_name}</div>
                        <div className="font-mono text-[11px] opacity-60">
                          {device.id}
                        </div>
                      </td>
                      <td className="font-mono text-[11px]">
                        {device.digital_target?.port_path ?? "-"}
                      </td>
                      <td className="font-mono text-[11px]">
                        {device.analog_target?.probe_selector ?? "-"}
                      </td>
                      <td className="font-mono text-[11px]">
                        {device.lan_endpoint ?? "-"}
                      </td>
                      <td>{device.connection}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : null}
        </div>
      </div>

      {/* Scan Panel Toggle */}
      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => {
            if (isDemoMode) return;
            setIsScanPanelOpen(!isScanPanelOpen);
          }}
          disabled={isDemoMode}
          className="ll-button ll-button-sm ll-button-ghost"
        >
          {isScanPanelOpen ? "Hide Network Scanner" : "Scan current network..."}
        </button>
      </div>

      {/* Scan Panel */}
      {isScanPanelOpen && (
        <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
          <div className="ll-panel-body p-4">
            <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50">
              LAN Scanner
            </h3>

            <div className="ll-alert ll-alert-warning text-xs">
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
              <label className="ll-form-control flex-1 min-w-[200px]">
                <div className="ll-label-row pb-1">
                  <span className="ll-label-text">
                    Seed IP (Example Device)
                  </span>
                </div>
                <input
                  id="lan-scan-seed-ip"
                  name="lan_scan_seed_ip"
                  type="text"
                  value={seedIp}
                  onChange={(e) => setSeedIp(e.target.value)}
                  placeholder="e.g. 192.168.1.100"
                  disabled={isDemoMode || isScanning}
                  className="ll-input w-full ll-input-sm"
                />
              </label>
              <button
                type="button"
                onClick={startScan}
                disabled={isDemoMode || isScanning || !seedIp}
                className="ll-button ll-button-primary ll-button-sm"
              >
                {isScanning ? (
                  <span className="ll-loading ll-loading-spinner ll-loading-xs"></span>
                ) : null}
                {isScanning ? "Scanning..." : "Start scan"}
              </button>
              {isScanning && (
                <button
                  type="button"
                  className="ll-button ll-button-ghost ll-button-sm"
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
                  <table className="ll-table ll-table-xs">
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
                              className="ll-button ll-button-xs ll-button-outline ll-button-primary"
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

      {ENABLE_MOCK_DEVTOOLS ? (
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
            className="ll-button ll-button-secondary ll-button-sm"
          >
            {isMutating ? "Adding device..." : "Add demo device"}
          </button>
          <span className="text-xs text-base-content/60">
            Adds a built-in demo device backed by an in-memory backend (for
            development / testing).
          </span>
        </div>
      ) : null}

      <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
        <div className="ll-panel-body p-0 overflow-x-auto">
          {devicesQuery.isLoading ? (
            <div className="p-8 text-center text-base-content/60">
              Loading devices...
            </div>
          ) : devices.length === 0 ? (
            <div className="p-8 text-center space-y-4">
              <p className="text-base-content/80 text-lg">
                No devices yet. Add a LoadLynx device to begin monitoring and
                control.
              </p>

              <div className="divider text-base-content/30 text-xs">OR</div>

              <div className="flex flex-col items-center gap-2">
                <p className="text-sm text-base-content/70">
                  No hardware on hand? You can add a simulation device to try
                  the console UI before connecting a real LoadLynx.
                </p>
                <button
                  type="button"
                  className="ll-button ll-button-secondary ll-button-sm"
                  onClick={() => {
                    addDeviceMutation.mutate(undefined, {
                      onSuccess: () => {
                        queryClient.invalidateQueries({
                          queryKey: ["devices"],
                        });
                      },
                    });
                  }}
                  disabled={isMutating}
                >
                  {isMutating
                    ? "Adding simulation..."
                    : "Add simulation device"}
                </button>
              </div>
            </div>
          ) : (
            <table className="ll-table ll-table-sm">
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
    </PageContainer>
  );
}

export default DevicesRoute;

function DeviceRow(props: { device: StoredDevice }) {
  const { device } = props;
  const identityQuery = useDeviceIdentity(device);

  const identity = identityQuery.data;
  const error: unknown = identityQuery.error;

  let statusBadgeClass = "ll-badge ll-badge-ghost";
  let statusLabel = "Checking...";
  let statusDetail: string | null = null;

  if (identityQuery.isLoading || identityQuery.isFetching) {
    statusBadgeClass = "ll-badge ll-badge-ghost";
    statusLabel = "Checking...";
  } else if (identityQuery.isSuccess && identity) {
    statusBadgeClass = "ll-badge ll-badge-success";
    statusLabel = "Online";

    const primaryHost = identity.hostname ?? identity.network?.hostname;
    if (primaryHost) {
      statusDetail = `${primaryHost} (${identity.network.ip})`;
    } else {
      statusDetail = identity.network.ip;
    }
  } else if (identityQuery.isError) {
    statusBadgeClass = "ll-badge ll-badge-error";
    statusLabel = "Offline";

    const formatSnippet = (message: string) =>
      message.length > 80 ? `${message.slice(0, 77)}…` : message;

    if (isHttpApiError(error)) {
      const code = error.code ?? "HTTP_ERROR";
      if (error.status === 0 && code === "NETWORK_ERROR") {
        statusDetail = "网络异常，已自动重试；如仍失败请检查设备 IP 或网络";
      } else if (error.status === 404 && code === "UNSUPPORTED_OPERATION") {
        statusBadgeClass = "ll-badge ll-badge-warning";
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
        {device.connectionMarks?.length ? (
          <div className="mt-1 flex flex-wrap gap-1">
            {device.connectionMarks.map((mark) => (
              <span
                key={mark}
                className="ll-badge ll-badge-xs ll-badge-outline"
              >
                {mark}
              </span>
            ))}
          </div>
        ) : null}
      </td>
      <td>
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <div className={`ll-badge ll-badge-sm gap-2 ${statusBadgeClass}`}>
              {statusLabel}
            </div>
            <button
              type="button"
              onClick={() => {
                void identityQuery.refetch();
              }}
              disabled={identityQuery.isFetching}
              className="ll-button ll-button-ghost ll-button-xs ll-button-circle"
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
          className="ll-button ll-button-sm ll-button-outline"
        >
          Open CC Control
        </Link>
        <Link
          to="/$deviceId/firmware"
          params={{ deviceId: device.id }}
          className="ll-button ll-button-sm ll-button-ghost ml-2"
        >
          Firmware
        </Link>
      </td>
    </tr>
  );
}

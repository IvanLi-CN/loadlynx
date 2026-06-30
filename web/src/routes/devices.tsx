import { useQuery } from "@tanstack/react-query";
import { Link, useRouterState } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { HttpApiError } from "../api/client.ts";
import { ENABLE_MOCK_DEVTOOLS, isHttpApiError } from "../api/client.ts";
import type { ControlView, FastStatusView } from "../api/types.ts";
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
  getDeviceControlQueryOptions,
  getDeviceStatusQueryOptions,
  type ScanProgress,
  useAddDeviceMutation,
  useAddRealDeviceMutation,
  useDeviceIdentity,
  useDevicesQuery,
  useSubnetScanMutation,
} from "../devices/hooks.ts";
import {
  beginManagedScan,
  cancelManagedScan,
  clearManagedScan,
  isManagedScanCurrent,
} from "../devices/scan-controller.ts";
import { useDeviceStore } from "../devices/store-context.tsx";
import {
  getConnectionLabels,
  getDeviceRouteIntentFromHref,
} from "../layouts/console-navigation.ts";
import { readStoredDemoMode } from "../lib/demo-mode.ts";
import { isUnsupportedOperationError } from "../lib/http-error.ts";

type OverviewIntent = ReturnType<typeof getDeviceRouteIntentFromHref>;
type OverviewTranslator = ReturnType<typeof useTranslation>["t"];

type OverviewTopRightDetailParams = {
  t: OverviewTranslator;
  status: FastStatusView | undefined;
  statusError: HttpApiError | null;
  identityError: HttpApiError | null;
  identityLoading: boolean;
  linkValue: string;
  protectionValue: string;
  fallbackDetail: string;
};

function getOverviewIntent(searchStr: string): OverviewIntent {
  if (!searchStr) {
    return { route: "cc" };
  }

  const params = new URLSearchParams(searchStr);
  return getDeviceRouteIntentFromHref(params.get("returnTo"));
}

export function getOverviewTopRightDetail({
  t,
  status,
  statusError,
  identityError,
  identityLoading,
  linkValue,
  protectionValue,
  fallbackDetail,
}: OverviewTopRightDetailParams): string {
  if (status || statusError) {
    return `${linkValue} · ${protectionValue}`;
  }

  if (identityLoading) {
    return t("overview.checkingRequest");
  }

  if (identityError) {
    if (
      identityError.status === 0 &&
      (identityError.code ?? "NETWORK_ERROR") === "NETWORK_ERROR"
    ) {
      return t("overview.networkRetry");
    }
    return identityError.code ?? t("overview.offline");
  }

  return fallbackDetail;
}

export function DevicesRoute() {
  const { t } = useTranslation();
  const deviceStore = useDeviceStore();
  const devicesQuery = useDevicesQuery();
  const addDeviceMutation = useAddDeviceMutation();
  const addRealDeviceMutation = useAddRealDeviceMutation();
  const isDemoMode =
    typeof window !== "undefined" &&
    readStoredDemoMode(window.localStorage) === true;
  const searchStr = useRouterState({
    select: (state) => state.location.searchStr,
  });
  const selectionMode = useMemo(
    () => new URLSearchParams(searchStr).has("returnTo"),
    [searchStr],
  );
  const selectionIntent = useMemo(
    () => getOverviewIntent(searchStr),
    [searchStr],
  );

  const [newDeviceName, setNewDeviceName] = useState("");
  const [newDeviceBaseUrl, setNewDeviceBaseUrl] = useState("");
  const [addDeviceError, setAddDeviceError] = useState<string | null>(null);

  const devices: StoredDevice[] = useMemo(
    () => devicesQuery.data ?? [],
    [devicesQuery.data],
  );
  const lastActiveDeviceId = deviceStore.getLastActiveDeviceId();

  const isMutating = addDeviceMutation.isPending;
  const isAddingReal = addRealDeviceMutation.isPending;

  const scanMutation = useSubnetScanMutation();
  const activeScanControllerRef = useRef<AbortController | null>(null);
  const [isScanPanelOpen, setIsScanPanelOpen] = useState(false);
  const [seedIp, setSeedIp] = useState("192.168.1.100");
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [scanResults, setScanResults] = useState<DiscoveredDevice[]>([]);
  const [scanError, setScanError] = useState<string | null>(null);
  const devdScan = useDevdScan();
  const createDevdLease = useCreateDevdLease();
  const [devdDevices, setDevdDevices] = useState<DevdDevice[]>([]);
  const [selectedDevdDeviceId, setSelectedDevdDeviceId] = useState("");
  const [devdError, setDevdError] = useState<string | null>(null);

  useEffect(() => {
    return () => {
      activeScanControllerRef.current = cancelManagedScan(
        activeScanControllerRef.current,
      );
    };
  }, []);

  const stopScan = () => {
    activeScanControllerRef.current = cancelManagedScan(
      activeScanControllerRef.current,
    );
    scanMutation.reset();
    setScanProgress(null);
  };

  const startScan = () => {
    if (isDemoMode) return;

    const controller = beginManagedScan(activeScanControllerRef.current);
    activeScanControllerRef.current = controller;

    setScanProgress({ scannedCount: 0, totalCount: 0, foundCount: 0 });
    setScanResults([]);
    setScanError(null);

    scanMutation.mutate(
      {
        options: { seedIp: seedIp.trim(), signal: controller.signal },
        onProgress: (p: ScanProgress) => {
          if (
            isManagedScanCurrent(activeScanControllerRef.current, controller)
          ) {
            setScanProgress(p);
          }
        },
      },
      {
        onSuccess: (data: DiscoveredDevice[]) => {
          if (
            isManagedScanCurrent(activeScanControllerRef.current, controller)
          ) {
            setScanResults(data);
          }
        },
        onError: (err: Error) => {
          if (
            isManagedScanCurrent(activeScanControllerRef.current, controller)
          ) {
            setScanError(err instanceof Error ? err.message : "Unknown error");
          }
        },
        onSettled: () => {
          activeScanControllerRef.current = clearManagedScan(
            activeScanControllerRef.current,
            controller,
          );
        },
      },
    );
  };

  const isScanning = scanMutation.isPending;

  return (
    <PageContainer variant="workspace" className="space-y-8">
      <header className="space-y-3">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <h2 className="text-2xl font-bold">
              {selectionMode ? "选择设备" : "总览"}
            </h2>
            <p className="mt-1 max-w-3xl text-sm text-base-content/70">
              {selectionMode
                ? "选择要进入的设备。"
                : "查看所有设备的核心状态、连接方式与入口。"}
            </p>
          </div>
          <div className="rounded-2xl border border-base-300/70 bg-base-200/30 px-4 py-3 text-xs text-base-content/60">
            <div className="font-mono uppercase tracking-[0.14em] text-base-content/45">
              {selectionMode ? "Selected View" : "Devices"}
            </div>
            <div className="mt-2">
              {selectionMode
                ? `${selectionIntent.route.toUpperCase()}${selectionIntent.panel ? " · PD Panel" : ""}`
                : `${devices.length} device${devices.length === 1 ? "" : "s"}`}
            </div>
          </div>
        </div>
      </header>

      <section className="space-y-4">
        <div className="flex items-end justify-between gap-4">
          <div>
            <div className="text-xs font-mono uppercase tracking-[0.14em] text-base-content/45">
              Device List
            </div>
            <h3 className="mt-1 text-lg font-bold">当前已知设备</h3>
          </div>
        </div>

        {devicesQuery.isLoading ? (
          <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
            <div className="ll-panel-body p-8 text-center text-base-content/60">
              Loading devices...
            </div>
          </div>
        ) : devices.length === 0 ? (
          <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
            <div className="ll-panel-body p-8 text-center space-y-4">
              <p className="text-base-content/80 text-lg">
                No devices yet. Add a LoadLynx device to get started.
              </p>
              <div className="border-t border-base-300/70 pt-4">
                <p className="text-sm text-base-content/70">
                  You can also add a sample device to preview the interface.
                </p>
                <button
                  type="button"
                  className="ll-button ll-button-secondary ll-button-sm mt-4"
                  onClick={() => {
                    addDeviceMutation.mutate(undefined);
                  }}
                  disabled={isMutating}
                >
                  {isMutating ? "Adding sample..." : "Add sample device"}
                </button>
              </div>
            </div>
          </div>
        ) : (
          <div className="grid gap-4 xl:grid-cols-2">
            {devices.map((device) => (
              <OverviewDeviceCard
                key={device.id}
                device={device}
                isCurrentDevice={device.id === lastActiveDeviceId}
                selectionMode={selectionMode}
                selectionIntent={selectionIntent}
              />
            ))}
          </div>
        )}
      </section>

      <section className="space-y-5">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <div className="text-xs font-mono uppercase tracking-[0.14em] text-base-content/45">
              Device Tools
            </div>
            <h3 className="mt-1 text-lg font-bold">注册、桥接与发现</h3>
          </div>
          {ENABLE_MOCK_DEVTOOLS ? (
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={() => {
                  addDeviceMutation.mutate(undefined);
                }}
                disabled={isMutating}
                className="ll-button ll-button-secondary ll-button-sm"
              >
                {isMutating ? "Adding device..." : "Add sample device"}
              </button>
            </div>
          ) : null}
        </div>

        <div className="grid gap-6 xl:grid-cols-[1.2fr_1fr]">
          <div className="space-y-6">
            <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
              <div className="ll-panel-body p-4">
                <form
                  onSubmit={(event) => {
                    event.preventDefault();

                    if (isDemoMode) {
                      setAddDeviceError(
                        "Sample devices are enabled on this page. Disable demo mode to add a network device.",
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
                        },
                      },
                    );
                  }}
                  className="flex flex-col gap-4"
                >
                  <div>
                    <h4 className="ll-panel-title text-base">
                      Register device
                    </h4>
                    <p className="mt-1 text-xs text-base-content/60">
                      Add a network device by hostname or base URL.
                    </p>
                  </div>

                  {isDemoMode ? (
                    <output className="ll-alert ll-alert-info text-sm">
                      Sample devices are active right now. Switch demo mode off
                      to add a network device here.
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
                        onChange={(event) =>
                          setNewDeviceName(event.target.value)
                        }
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
                        onChange={(event) =>
                          setNewDeviceBaseUrl(event.target.value)
                        }
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
                      {isAddingReal ? "Adding..." : "Add device"}
                    </button>
                  </div>

                  {addDeviceError ? (
                    <div
                      role="alert"
                      className="ll-alert ll-alert-error text-sm py-2"
                    >
                      <span>{addDeviceError}</span>
                    </div>
                  ) : (
                    <div className="text-xs text-base-content/60">
                      Enter the hostname or IP. Example:{" "}
                      <code className="code">http://loadlynx-d68638.local</code>
                    </div>
                  )}
                </form>
              </div>
            </div>

            <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
              <div className="ll-panel-body p-4 gap-4">
                <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                  <div>
                    <h4 className="ll-panel-title text-base">Local devd</h4>
                    <p className="text-xs text-base-content/60">
                      Available devices from{" "}
                      <code className="code">{DEFAULT_DEVD_BASE_URL}</code>.
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
                    Refresh
                  </button>
                </div>

                {devdError ? (
                  <div
                    role="alert"
                    className="ll-alert ll-alert-error py-2 text-sm"
                  >
                    <span>{devdError}</span>
                  </div>
                ) : null}

                {devdDevices.length > 0 ? (
                  <>
                    <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
                      <label className="ll-form-control">
                        <div className="ll-label-row pb-1">
                          <span className="ll-label-text">
                            Available device
                          </span>
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
                              const connectionMarks: StoredDevice["connectionMarks"] =
                                ["usb"];
                              if (candidate.lan_endpoint) {
                                connectionMarks.push("lan");
                              }
                              const devd = {
                                baseUrl: DEFAULT_DEVD_BASE_URL,
                                deviceId: candidate.id,
                                leaseId: lease.lease_id,
                              };
                              const baseUrl = buildDevdCompatBaseUrl(devd);
                              addRealDeviceMutation.mutate({
                                name: candidate.display_name,
                                baseUrl,
                                connectionMarks,
                                devd,
                              });
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
                        Add from devd
                      </button>
                    </div>

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
                                <div className="font-medium">
                                  {device.display_name}
                                </div>
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
                  </>
                ) : (
                  <div className="text-xs text-base-content/60">
                    No devd devices found yet.
                  </div>
                )}
              </div>
            </div>
          </div>

          <div className="space-y-6">
            <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
              <div className="ll-panel-body p-4">
                <div className="flex justify-between items-center">
                  <div>
                    <h4 className="ll-panel-title text-base">LAN scan</h4>
                    <p className="mt-1 text-xs text-base-content/60">
                      Search the current network for LoadLynx devices.
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      if (isDemoMode) return;
                      if (isScanPanelOpen) {
                        stopScan();
                      }
                      setIsScanPanelOpen(!isScanPanelOpen);
                    }}
                    disabled={isDemoMode}
                    className="ll-button ll-button-sm ll-button-ghost"
                  >
                    {isScanPanelOpen ? "Hide scan" : "Scan network..."}
                  </button>
                </div>

                {isScanPanelOpen ? (
                  <div className="mt-4 space-y-4">
                    <div className="ll-alert ll-alert-warning text-xs">
                      <span>
                        This scans the selected subnet with short HTTP probes.
                      </span>
                    </div>

                    <div className="flex flex-wrap gap-4 items-end">
                      <label className="ll-form-control flex-1 min-w-[200px]">
                        <div className="ll-label-row pb-1">
                          <span className="ll-label-text">Seed IP</span>
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
                        {isScanning ? "Scanning..." : "Start scan"}
                      </button>
                      {isScanning ? (
                        <button
                          type="button"
                          className="ll-button ll-button-ghost ll-button-sm"
                          onClick={stopScan}
                        >
                          Cancel
                        </button>
                      ) : null}
                    </div>

                    {scanError ? (
                      <div className="text-error text-sm">
                        Scan failed: {scanError}
                      </div>
                    ) : null}

                    {isScanning && scanProgress ? (
                      <div className="space-y-2">
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
                    ) : null}

                    {scanResults.length > 0 ? (
                      <div>
                        <h5 className="font-bold text-sm mb-2">
                          Discovered devices ({scanResults.length})
                        </h5>
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
                                  <td className="font-mono">
                                    {d.identity.device_id}
                                  </td>
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
                    ) : null}

                    {scanMutation.isSuccess && scanResults.length === 0 ? (
                      <div className="text-sm text-base-content/60 italic">
                        {t("overview.managementEmpty")}
                      </div>
                    ) : null}
                  </div>
                ) : null}
              </div>
            </div>
          </div>
        </div>
      </section>
    </PageContainer>
  );
}

function OverviewDeviceCard(props: {
  device: StoredDevice;
  isCurrentDevice: boolean;
  selectionMode: boolean;
  selectionIntent: OverviewIntent;
}) {
  const { t } = useTranslation();
  const { device, isCurrentDevice, selectionMode, selectionIntent } = props;
  const identityQuery = useDeviceIdentity(device);
  const statusQuery = useQuery<FastStatusView, HttpApiError>({
    ...getDeviceStatusQueryOptions({
      deviceId: device.id,
      baseUrl: device.baseUrl,
      enabled: Boolean(device.baseUrl),
      refetchInterval: false,
      retry: 1,
      retryDelay: 250,
    }),
  });
  const controlQuery = useQuery<ControlView, HttpApiError>({
    ...getDeviceControlQueryOptions({
      deviceId: device.id,
      baseUrl: device.baseUrl,
      enabled: Boolean(device.baseUrl),
      retryDelay: 250,
    }),
    retry: 1,
  });
  const identity = identityQuery.data;
  const status = statusQuery.data;
  const control = controlQuery.data;
  const connectionLabels = getConnectionLabels(device);
  let statusTone = "ok" as "ok" | "warn" | "danger";
  let statusLabel = t("overview.online");
  let statusDetail = identity?.network.ip ?? t("overview.noLiveIdentity");
  const dashboardLabel = selectionMode
    ? t("overview.selectionAction")
    : t("overview.dashboardAction");
  const modeLabel = getOverviewModeLabel(control, status);
  const identitySummary = identity
    ? [
        identity.short_id ? `#${identity.short_id}` : null,
        identity.network.ip,
        `API ${identity.capabilities.api_version}`,
      ]
        .filter(Boolean)
        .join(" · ")
    : t("overview.noLiveIdentity");
  const identityDetail = identity
    ? `${identity.hostname ?? identity.network.hostname}`
    : statusDetail;
  const voltageSummary = getOverviewVoltageSummary(t, status);
  const currentSummary = getOverviewCurrentSummary(t, status);
  const powerSummary = getOverviewPowerSummary(t, status);
  const resistanceSummary = getOverviewResistanceSummary(t, status);
  const modeSummary = getOverviewModeSummary(t, control, modeLabel);
  const linkSummary = getOverviewLinkSummary(t, status, statusQuery.error);
  const protectionSummary = getOverviewProtectionSummary(t, status, control);

  if (identityQuery.isLoading || identityQuery.isFetching) {
    statusTone = "warn";
    statusLabel = t("overview.checking");
    statusDetail = t("overview.checkingRequest");
  } else if (
    status?.fault_flags_decoded.length ||
    status?.analog_state === "faulted"
  ) {
    statusTone = "danger";
    statusLabel = t("overview.protectionFault");
    statusDetail =
      status.fault_flags_decoded.length > 0
        ? formatOverviewToken(status.fault_flags_decoded[0])
        : formatOverviewToken(status.analog_state);
  } else if (
    status?.analog_state === "cal_missing" ||
    control?.uv_latched ||
    status?.link_up === false
  ) {
    statusTone = "warn";
    statusLabel =
      status?.link_up === false
        ? t("overview.linkDown")
        : t("overview.statusAttention");
    statusDetail = control?.uv_latched
      ? t("overview.protectionUvLatch")
      : status?.analog_state === "cal_missing"
        ? t("overview.protectionCalMissing")
        : statusDetail;
  } else if (identityQuery.isError) {
    statusTone = "danger";
    statusLabel = t("overview.offline");
    if (isHttpApiError(identityQuery.error)) {
      const code = identityQuery.error.code ?? "HTTP_ERROR";
      statusDetail =
        identityQuery.error.status === 0 && code === "NETWORK_ERROR"
          ? t("overview.networkRetry")
          : `${code}: ${identityQuery.error.message}`;
      if (isUnsupportedOperationError(identityQuery.error)) {
        statusTone = "warn";
        statusLabel = t("overview.httpPartial");
      }
    }
  }
  const topRightDetail = getOverviewTopRightDetail({
    t,
    status,
    statusError: statusQuery.error,
    identityError: identityQuery.error,
    identityLoading: identityQuery.isLoading || identityQuery.isFetching,
    linkValue: linkSummary.value,
    protectionValue: protectionSummary.value,
    fallbackDetail: statusDetail,
  });

  return (
    <article
      className={`ll-panel bg-base-100 shadow-sm border border-base-200 ${isCurrentDevice ? "ring-1 ring-cyan-400/35" : ""}`}
    >
      <div className="ll-panel-body p-5">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <h4 className="truncate text-lg font-bold">{device.name}</h4>
            <p className="mt-1 font-mono text-xs text-base-content/55">
              {device.id}
            </p>
            <div className="mt-3 flex flex-wrap gap-2">
              {connectionLabels.map((label) => (
                <span
                  key={label}
                  className="rounded-full border border-base-300/75 bg-base-200/35 px-2.5 py-1 text-[10px] font-mono uppercase tracking-[0.12em] text-base-content/70"
                >
                  {label}
                </span>
              ))}
            </div>
          </div>
          <div className="flex min-w-[9rem] flex-col items-end gap-2 text-right">
            <div className="flex flex-wrap justify-end gap-2">
              {isCurrentDevice ? (
                <StatusPill tone="ok" label={t("overview.currentDevice")} />
              ) : null}
              <StatusPill tone={statusTone} label={statusLabel} />
            </div>
            <div className="max-w-full text-[11px] text-base-content/55">
              {topRightDetail}
            </div>
          </div>
        </div>

        <div className="grid gap-x-5 gap-y-4 border-y border-base-300/65 py-5 sm:grid-cols-2 xl:grid-cols-5">
          <OverviewStat
            label={t("overview.voltage")}
            value={voltageSummary.value}
            detail={voltageSummary.detail}
            tone={voltageSummary.tone}
            emphasize
          />
          <OverviewStat
            label={t("overview.current")}
            value={currentSummary.value}
            detail={currentSummary.detail}
            tone={currentSummary.tone}
            emphasize
          />
          <OverviewStat
            label={t("overview.power")}
            value={powerSummary.value}
            detail={powerSummary.detail}
            tone={powerSummary.tone}
            emphasize
          />
          <OverviewStat
            label={t("overview.resistance")}
            value={resistanceSummary.value}
            detail={resistanceSummary.detail}
            tone={resistanceSummary.tone}
            emphasize
          />
          <OverviewStat
            label={t("overview.mode")}
            value={modeSummary.value}
            detail={modeSummary.detail}
            tone={modeSummary.tone}
            emphasize
          />
        </div>

        <div className="flex flex-wrap items-end justify-between gap-4 pt-1">
          <div className="min-w-0 flex-1 text-xs text-base-content/60">
            <div className="truncate text-sm font-semibold text-base-content/85">
              {identitySummary}
            </div>
            <div className="mt-1 truncate">{identityDetail}</div>
          </div>

          <div className="flex flex-wrap justify-end gap-3">
            <PrimaryDeviceLink
              deviceId={device.id}
              selectionMode={selectionMode}
              selectionIntent={selectionIntent}
              label={dashboardLabel}
            />
            {!selectionMode ? (
              <Link
                to="/$deviceId/settings"
                params={{ deviceId: device.id }}
                className="ll-button ll-button-sm ll-button-outline"
              >
                {t("overview.systemAction")}
              </Link>
            ) : null}
          </div>
        </div>
      </div>
    </article>
  );
}

function OverviewStat(props: {
  label: string;
  value: string;
  detail: string | null;
  tone: "ok" | "warn" | "danger" | "neutral";
  emphasize?: boolean;
}) {
  const valueClass =
    props.tone === "danger"
      ? "text-red-100"
      : props.tone === "warn"
        ? "text-amber-100"
        : props.tone === "ok"
          ? "text-cyan-100"
          : "text-base-content/90";

  const valueSizeClass = props.emphasize
    ? "text-[1.45rem] leading-none sm:text-[1.7rem]"
    : "text-sm";

  return (
    <div className="min-w-0">
      <div className="text-[10px] font-mono uppercase tracking-[0.14em] text-base-content/45">
        {props.label}
      </div>
      <div className={`mt-2 font-semibold ${valueSizeClass} ${valueClass}`}>
        {props.value}
      </div>
      {props.detail ? (
        <div className="mt-1 truncate text-xs text-base-content/58">
          {props.detail}
        </div>
      ) : null}
    </div>
  );
}

function PrimaryDeviceLink(props: {
  deviceId: string;
  selectionMode: boolean;
  selectionIntent: OverviewIntent;
  label: string;
}) {
  const { deviceId, selectionMode, selectionIntent, label } = props;

  if (!selectionMode) {
    return (
      <Link
        to="/$deviceId/cc"
        params={{ deviceId }}
        className="ll-button ll-button-primary ll-button-sm"
      >
        {label}
      </Link>
    );
  }

  if (selectionIntent.route === "settings") {
    return (
      <Link
        to="/$deviceId/settings"
        params={{ deviceId }}
        className="ll-button ll-button-primary ll-button-sm"
      >
        {label}
      </Link>
    );
  }

  if (selectionIntent.route === "status") {
    return (
      <Link
        to="/$deviceId/status"
        params={{ deviceId }}
        className="ll-button ll-button-primary ll-button-sm"
      >
        {label}
      </Link>
    );
  }

  if (selectionIntent.route === "calibration") {
    return (
      <Link
        to="/$deviceId/calibration"
        params={{ deviceId }}
        className="ll-button ll-button-primary ll-button-sm"
      >
        {label}
      </Link>
    );
  }

  if (selectionIntent.route === "firmware") {
    return (
      <Link
        to="/$deviceId/firmware"
        params={{ deviceId }}
        className="ll-button ll-button-primary ll-button-sm"
      >
        {label}
      </Link>
    );
  }

  if (selectionIntent.route === "about") {
    return (
      <Link
        to="/$deviceId/about"
        params={{ deviceId }}
        className="ll-button ll-button-primary ll-button-sm"
      >
        {label}
      </Link>
    );
  }

  return (
    <Link
      to="/$deviceId/cc"
      params={{ deviceId }}
      search={selectionIntent.panel ? { panel: "pd" } : undefined}
      className="ll-button ll-button-primary ll-button-sm"
    >
      {label}
    </Link>
  );
}

function StatusPill(props: { tone: "ok" | "warn" | "danger"; label: string }) {
  const className =
    props.tone === "danger"
      ? "border-red-300/35 bg-red-400/12 text-red-100"
      : props.tone === "warn"
        ? "border-amber-300/35 bg-amber-400/12 text-amber-100"
        : "border-emerald-300/35 bg-emerald-400/12 text-emerald-100";

  return (
    <span
      className={`inline-flex items-center rounded-full border px-3 py-1 text-[10px] font-mono uppercase tracking-[0.14em] ${className}`}
    >
      {props.label}
    </span>
  );
}

function getOverviewModeLabel(
  control: ControlView | undefined,
  status: FastStatusView | undefined,
): "CC" | "CV" | "CP" | "CR" | null {
  const controlMode = control?.preset.mode;
  if (controlMode === "cc") return "CC";
  if (controlMode === "cv") return "CV";
  if (controlMode === "cp") return "CP";
  if (controlMode === "cr") return "CR";

  if (!status) return null;
  if (status.raw.mode === 2) return "CV";
  if (status.raw.mode === 3) return "CP";
  return "CC";
}

function getOverviewVoltageSummary(
  t: ReturnType<typeof useTranslation>["t"],
  status: FastStatusView | undefined,
) {
  if (!status) {
    return {
      value: t("overview.readingUnavailable"),
      detail: t("overview.readingPending"),
      tone: "neutral" as const,
    };
  }

  const voltageV = status.raw.v_remote_mv / 1_000;
  return {
    value: `${voltageV.toFixed(3)} V`,
    detail: null,
    tone: "ok" as const,
  };
}

function getOverviewCurrentSummary(
  t: ReturnType<typeof useTranslation>["t"],
  status: FastStatusView | undefined,
) {
  if (!status) {
    return {
      value: t("overview.readingUnavailable"),
      detail: t("overview.readingPending"),
      tone: "neutral" as const,
    };
  }

  const totalCurrentA =
    (status.raw.i_local_ma + status.raw.i_remote_ma) / 1_000;

  return {
    value: `${totalCurrentA.toFixed(3)} A`,
    detail: null,
    tone: "ok" as const,
  };
}

function getOverviewPowerSummary(
  t: ReturnType<typeof useTranslation>["t"],
  status: FastStatusView | undefined,
) {
  if (!status) {
    return {
      value: t("overview.readingUnavailable"),
      detail: t("overview.readingPending"),
      tone: "neutral" as const,
    };
  }

  const powerW = status.raw.calc_p_mw / 1_000;

  return {
    value: `${powerW.toFixed(2)} W`,
    detail: null,
    tone: "ok" as const,
  };
}

function getOverviewResistanceSummary(
  t: ReturnType<typeof useTranslation>["t"],
  status: FastStatusView | undefined,
) {
  if (!status) {
    return {
      value: t("overview.readingUnavailable"),
      detail: t("overview.readingPending"),
      tone: "neutral" as const,
    };
  }

  const totalCurrentMa = status.raw.i_local_ma + status.raw.i_remote_ma;
  if (totalCurrentMa <= 0) {
    return {
      value: t("overview.readingUnavailable"),
      detail: null,
      tone: "neutral" as const,
    };
  }

  const resistanceOhms = status.raw.v_remote_mv / totalCurrentMa;

  return {
    value: `${resistanceOhms.toFixed(2)} Ω`,
    detail: null,
    tone: "ok" as const,
  };
}

function getOverviewModeSummary(
  t: ReturnType<typeof useTranslation>["t"],
  control: ControlView | undefined,
  modeLabel: "CC" | "CV" | "CP" | "CR" | null,
) {
  if (!control && !modeLabel) {
    return {
      value: t("overview.readingUnavailable"),
      detail: t("overview.modePending"),
      tone: "neutral" as const,
    };
  }

  if (!modeLabel) {
    return {
      value: control?.output_enabled
        ? t("overview.readingUnavailable")
        : t("overview.modeOff"),
      detail: control?.output_enabled ? t("overview.modePending") : null,
      tone: control?.output_enabled ? ("neutral" as const) : ("warn" as const),
    };
  }

  return {
    value: modeLabel,
    detail: control?.output_enabled
      ? t("overview.outputOn")
      : t("overview.outputOff"),
    tone: control?.output_enabled ? ("ok" as const) : ("warn" as const),
  };
}

function getOverviewLinkSummary(
  t: ReturnType<typeof useTranslation>["t"],
  status: FastStatusView | undefined,
  error: HttpApiError | null,
) {
  if (status) {
    return {
      value: status.link_up ? t("overview.linkUp") : t("overview.linkDown"),
      detail: formatOverviewToken(status.analog_state),
      tone: status.link_up ? ("ok" as const) : ("warn" as const),
    };
  }

  if (error) {
    return {
      value: t("overview.offline"),
      detail: error.code ?? "HTTP_ERROR",
      tone: "danger" as const,
    };
  }

  return {
    value: t("overview.checking"),
    detail: t("overview.checkingRequest"),
    tone: "neutral" as const,
  };
}

function getOverviewProtectionSummary(
  t: ReturnType<typeof useTranslation>["t"],
  status: FastStatusView | undefined,
  control: ControlView | undefined,
) {
  if (!status) {
    return {
      value: t("overview.protectionPending"),
      detail: t("overview.modePending"),
      tone: "neutral" as const,
    };
  }

  const firstFault = status.fault_flags_decoded[0];
  if (firstFault || status.analog_state === "faulted") {
    return {
      value: t("overview.protectionFault"),
      detail: formatOverviewToken(firstFault ?? status.analog_state),
      tone: "danger" as const,
    };
  }
  if (control?.uv_latched) {
    return {
      value: t("overview.protectionUvLatch"),
      detail: t("overview.outputOff"),
      tone: "warn" as const,
    };
  }
  if (status.analog_state === "cal_missing") {
    return {
      value: t("overview.protectionCalMissing"),
      detail: formatOverviewToken(status.analog_state),
      tone: "warn" as const,
    };
  }
  if (!status.link_up) {
    return {
      value: t("overview.protectionLinkDown"),
      detail: formatOverviewToken(status.analog_state),
      tone: "warn" as const,
    };
  }

  return {
    value: t("overview.protectionOk"),
    detail:
      status.state_flags_decoded.length > 0
        ? formatOverviewToken(status.state_flags_decoded[0])
        : t("overview.online"),
    tone: "ok" as const,
  };
}

function formatOverviewToken(token: string): string {
  return token.replaceAll("_", " ");
}

export default DevicesRoute;

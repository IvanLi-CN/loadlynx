import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import {
  BACKUP_SECTION_KEYS,
  deleteWifiConfig,
  exportDeviceBackup,
  exportDiagnostics,
  getBackupUnknownWarnings,
  getIdentity,
  getPd,
  getSupportedBackupSections,
  getWifiStatus,
  isDevdCompatBaseUrl,
  isHttpApiError,
  isMockBaseUrl,
  postSoftReset,
  postWifiConfig,
  restoreDeviceBackup,
  supportsBackupWifiCredentials,
  validateBackupEnvelope,
} from "../api/client.ts";
import type {
  BackupRestoreResult,
  BackupSectionKey,
  Identity,
  LoadLynxBackup,
} from "../api/types.ts";
import { ConfirmDialog } from "../components/common/confirm-dialog.tsx";
import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";

const BACKUP_SECTION_LABELS: Record<BackupSectionKey, string> = {
  presets: "Presets",
  calibration: "Calibration",
  "settings.wifi": "WiFi",
  "settings.pd": "USB-PD",
};

function toggleSection(
  current: BackupSectionKey[],
  section: BackupSectionKey,
): BackupSectionKey[] {
  if (current.includes(section)) {
    return current.filter((entry) => entry !== section);
  }
  return [...current, section];
}

function downloadBackupJson(backup: LoadLynxBackup) {
  const blob = new Blob([`${JSON.stringify(backup, null, 2)}\n`], {
    type: "application/json",
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `loadlynx-backup-${new Date().toISOString().replace(/[:.]/g, "-")}.json`;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function DeviceSettingsRoute() {
  const { deviceId, baseUrl } = useDeviceContext();
  const [confirmSoftResetOpen, setConfirmSoftResetOpen] = useState(false);
  const [confirmWifiAction, setConfirmWifiAction] = useState<
    "save" | "clear" | null
  >(null);
  const [confirmBackupRestoreOpen, setConfirmBackupRestoreOpen] =
    useState(false);
  const [wifiSsid, setWifiSsid] = useState("");
  const [wifiPsk, setWifiPsk] = useState("");
  const [backupExportSelection, setBackupExportSelection] =
    useState<BackupSectionKey[]>(BACKUP_SECTION_KEYS);
  const [backupImport, setBackupImport] = useState<LoadLynxBackup | null>(null);
  const [backupImportName, setBackupImportName] = useState("");
  const [backupImportError, setBackupImportError] = useState<string | null>(
    null,
  );
  const [backupRestoreSelection, setBackupRestoreSelection] = useState<
    BackupSectionKey[]
  >([]);

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

  const backupPdQuery = useQuery({
    queryKey: ["device", deviceId, "backup-pd"],
    queryFn: () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return getPd(baseUrl);
    },
    enabled: Boolean(baseUrl),
    retry: false,
  });
  const backupPdUnsupported = Boolean(
    backupPdQuery.error &&
      isHttpApiError(backupPdQuery.error) &&
      backupPdQuery.error.code === "UNSUPPORTED_OPERATION",
  );
  const backupWifiCredentialsSupported = baseUrl
    ? supportsBackupWifiCredentials(baseUrl)
    : false;
  const effectiveBackupExportSelection = backupExportSelection.filter(
    (section) =>
      (section !== "settings.wifi" || backupWifiCredentialsSupported) &&
      (section !== "settings.pd" || !backupPdUnsupported),
  );

  const backupExportMutation = useMutation({
    mutationFn: async () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      return exportDeviceBackup(baseUrl, effectiveBackupExportSelection);
    },
    onSuccess: (backup) => {
      downloadBackupJson(backup);
    },
  });

  const backupRestoreMutation = useMutation<BackupRestoreResult, Error, void>({
    mutationFn: async () => {
      if (!baseUrl) {
        throw new Error("Device base URL is not available");
      }
      if (!backupImport) {
        throw new Error("No backup file selected.");
      }
      return restoreDeviceBackup(baseUrl, backupImport, backupRestoreSelection);
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
  const supportedImportSections = getSupportedBackupSections(backupImport);
  const backupWarnings = getBackupUnknownWarnings(backupImport);
  const backupSafetyBlocked =
    backupRestoreMutation.error &&
    isHttpApiError(backupRestoreMutation.error) &&
    backupRestoreMutation.error.code === "SAFETY_BLOCKED";
  const backupWifiLanConfirmationRequired =
    wifiLanConfirmationRequired &&
    backupRestoreSelection.includes("settings.wifi");

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

  const handleBackupRestore = () => {
    if (backupWifiLanConfirmationRequired) {
      setConfirmBackupRestoreOpen(true);
      return;
    }
    backupRestoreMutation.mutate();
  };

  const handleBackupFile = async (file: File | null) => {
    backupRestoreMutation.reset();
    setBackupImport(null);
    setBackupImportName("");
    setBackupImportError(null);
    setBackupRestoreSelection([]);
    if (!file) {
      return;
    }
    try {
      const parsed = validateBackupEnvelope(JSON.parse(await file.text()));
      const supported = getSupportedBackupSections(parsed);
      setBackupImport(parsed);
      setBackupImportName(file.name);
      setBackupRestoreSelection(supported);
    } catch (error) {
      setBackupImportError(
        error instanceof Error ? error.message : "Invalid backup file.",
      );
    }
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
      <ConfirmDialog
        open={confirmBackupRestoreOpen}
        title="Restore WiFi Backup"
        body="This restore will write WiFi credentials over the LAN connection."
        details={[
          "Use the local USB/devd path when available.",
          "The backup file contains plaintext PSK.",
          "Output will be disabled before any restore writes.",
        ]}
        confirmLabel="Restore Selected"
        destructive
        confirmDisabled={backupRestoreMutation.isPending || !baseUrl}
        onCancel={() => setConfirmBackupRestoreOpen(false)}
        onConfirm={() => {
          setConfirmBackupRestoreOpen(false);
          backupRestoreMutation.mutate();
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

      <div className="grid gap-6 md:grid-cols-2 md:items-start">
        <div className="grid gap-6">
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
                  <span className="text-xs text-base-content/50">
                    Loading...
                  </span>
                )}
              </div>
            </div>
          </div>

          {/* 5. Backup & Restore */}
          <div className="card bg-base-100 shadow-sm border border-base-200">
            <div className="card-body p-6">
              <h3 className="card-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
                Backup & Restore
              </h3>
              <div className="grid gap-5 text-xs">
                <div className="grid gap-3">
                  <div className="font-bold text-base-content/70">Export</div>
                  <div className="grid grid-cols-2 gap-2">
                    {BACKUP_SECTION_KEYS.map((section) => (
                      <label
                        key={`export-${section}`}
                        className="flex items-center gap-2 rounded border border-base-200 px-2 py-1"
                      >
                        <input
                          type="checkbox"
                          className="checkbox checkbox-xs"
                          checked={
                            (section === "settings.wifi" &&
                              !backupWifiCredentialsSupported) ||
                            (section === "settings.pd" && backupPdUnsupported)
                              ? false
                              : backupExportSelection.includes(section)
                          }
                          disabled={
                            (section === "settings.wifi" &&
                              !backupWifiCredentialsSupported) ||
                            (section === "settings.pd" && backupPdUnsupported)
                          }
                          onChange={() =>
                            setBackupExportSelection((current) =>
                              toggleSection(current, section),
                            )
                          }
                        />
                        <span>{BACKUP_SECTION_LABELS[section]}</span>
                      </label>
                    ))}
                  </div>
                  {!backupWifiCredentialsSupported ? (
                    <div className="alert alert-warning shadow-sm text-xs">
                      <span>WiFi credentials require local USB/devd.</span>
                    </div>
                  ) : null}
                  {backupPdUnsupported ? (
                    <div className="alert alert-warning shadow-sm text-xs">
                      <span>
                        USB-PD settings are not supported by this device.
                      </span>
                    </div>
                  ) : null}
                  <button
                    type="button"
                    className="btn btn-neutral btn-sm"
                    disabled={
                      !baseUrl ||
                      effectiveBackupExportSelection.length === 0 ||
                      backupExportMutation.isPending
                    }
                    onClick={() => backupExportMutation.mutate()}
                  >
                    Export Backup
                  </button>
                  {backupExportMutation.error ? (
                    <div className="alert alert-error shadow-sm text-xs">
                      <span>
                        Export failed:{" "}
                        {backupExportMutation.error instanceof Error
                          ? backupExportMutation.error.message
                          : "unknown error"}
                      </span>
                    </div>
                  ) : null}
                </div>

                <div className="grid gap-3 border-t border-base-200 pt-4">
                  <div className="font-bold text-base-content/70">Import</div>
                  <input
                    aria-label="Import backup file"
                    type="file"
                    accept="application/json,.json"
                    className="file-input file-input-bordered file-input-sm w-full"
                    onChange={(event) => {
                      void handleBackupFile(event.target.files?.[0] ?? null);
                    }}
                  />
                  {backupImportError ? (
                    <div className="alert alert-error shadow-sm text-xs">
                      <span>{backupImportError}</span>
                    </div>
                  ) : null}
                  {backupImport ? (
                    <div className="grid gap-3 rounded border border-base-200 bg-base-200/40 p-3">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-bold truncate">
                          {backupImportName || "backup.json"}
                        </span>
                        <span className="badge badge-ghost">
                          schema v{backupImport.schema_version}
                        </span>
                      </div>
                      <div className="grid grid-cols-2 gap-2">
                        {supportedImportSections.map((section) => (
                          <label
                            key={`restore-${section}`}
                            className="flex items-center gap-2 rounded border border-base-300 bg-base-100 px-2 py-1"
                          >
                            <input
                              type="checkbox"
                              className="checkbox checkbox-xs"
                              checked={backupRestoreSelection.includes(section)}
                              onChange={() =>
                                setBackupRestoreSelection((current) =>
                                  toggleSection(current, section),
                                )
                              }
                            />
                            <span>{BACKUP_SECTION_LABELS[section]}</span>
                          </label>
                        ))}
                      </div>
                      {backupWarnings.length > 0 ? (
                        <div className="alert alert-warning shadow-sm text-xs">
                          <span>{backupWarnings.join(" · ")}</span>
                        </div>
                      ) : null}
                      <button
                        type="button"
                        className="btn btn-outline btn-sm"
                        disabled={
                          !baseUrl ||
                          backupRestoreSelection.length === 0 ||
                          backupRestoreMutation.isPending
                        }
                        onClick={handleBackupRestore}
                      >
                        Restore Selected
                      </button>
                    </div>
                  ) : null}
                  {backupSafetyBlocked ? (
                    <div className="alert alert-error shadow-sm text-xs">
                      <span>
                        Restore safety-blocked:{" "}
                        {backupRestoreMutation.error?.message}
                      </span>
                    </div>
                  ) : backupRestoreMutation.error ? (
                    <div className="alert alert-error shadow-sm text-xs">
                      <span>
                        Restore failed: {backupRestoreMutation.error.message}
                      </span>
                    </div>
                  ) : null}
                  {backupRestoreMutation.data ? (
                    <div
                      className={`alert shadow-sm text-xs ${
                        backupRestoreMutation.data.ok
                          ? "alert-success"
                          : "alert-warning"
                      }`}
                    >
                      <span>
                        {backupRestoreMutation.data.restored
                          .map((entry) =>
                            entry.ok
                              ? `${BACKUP_SECTION_LABELS[entry.section]} OK`
                              : `${BACKUP_SECTION_LABELS[entry.section]} failed`,
                          )
                          .join(" · ")}
                      </span>
                    </div>
                  ) : null}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="grid gap-6">
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

          {/* 6. Actions */}
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
      </div>
    </PageContainer>
  );
}

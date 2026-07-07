import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Cable } from "lucide-react";
import { useEffect, useState } from "react";
import {
  BACKUP_SECTION_KEYS,
  deleteWifiConfig,
  exportDeviceBackup,
  exportDiagnostics,
  getBackupUnknownWarnings,
  getSupportedBackupSections,
  getWifiStatus,
  isDevdCompatBaseUrl,
  isHttpApiError,
  makeManualSoftResetRequest,
  makeWifiSetRequest,
  postSoftReset,
  postWifiConfig,
  restoreDeviceBackup,
  supportsBackupWifiCredentials,
} from "../api/client.ts";
import type {
  BackupRestoreResult,
  BackupSectionKey,
  LoadLynxBackup,
  SoftResetRequest,
  WifiSetRequest,
  WifiStatus,
} from "../api/types.ts";
import { ConfirmDialog } from "../components/common/confirm-dialog.tsx";
import { PageContainer } from "../components/layout/page-container.tsx";
import { buildDevdCompatBaseUrl, createDevdLease } from "../devd/client.ts";
import {
  DEVICE_QUERY_PARTS,
  makeDeviceQueryKey,
} from "../devices/device-query-key.ts";
import type { StoredDevice } from "../devices/device-store.ts";
import {
  getDevicePdQueryOptions,
  getDeviceWifiQueryOptions,
  upsertRealDevice,
  useDeviceIdentityByBaseUrl,
} from "../devices/hooks.ts";
import {
  getManagementTransportLabel,
  isWifiTransportBaseUrl,
  isWifiWriteTransportVerified,
} from "../devices/management-transport.ts";
import { syncDevicesQueryCache } from "../devices/query-cache.ts";
import { useDeviceStore } from "../devices/store-context.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import { requireDeviceBaseUrl } from "../lib/device-base-url.ts";
import { downloadJsonFile } from "../lib/download.ts";
import {
  formatHttpApiErrorSummary,
  getNetworkErrorHint,
  getUsbSerialErrorHint,
  isLinkUnavailableError,
  isUsbSerialUnavailableError,
} from "../lib/http-error.ts";
import { parseBackupImportText } from "./device-settings/import-backup.ts";

const BACKUP_SECTION_LABELS: Record<BackupSectionKey, string> = {
  presets: "Presets",
  calibration: "Calibration",
  "settings.wifi": "WiFi",
  "settings.pd": "USB-PD",
};

function formatWifiFailureReason(error: string | null | undefined): string {
  if (!error) {
    return "device reported WiFi error";
  }
  return error.replace(/_/g, " ");
}

function formatWifiWriteErrorMessage(error: Error): string {
  if (!isHttpApiError(error)) {
    return `WiFi 更新失败：${error.message}`;
  }

  if (
    error.code === "UNAVAILABLE" &&
    /EEPROM write failed/i.test(error.message)
  ) {
    return "WiFi 更新失败：设备存储写入失败。请重试；如果反复出现，需要检查固件的 EEPROM/I2C 写入。";
  }

  if (
    error.code === "serial_response_timeout" ||
    error.code === "serial_response_mismatch"
  ) {
    return "WiFi 更新状态未确认：USB 管理通道没有收到匹配响应。页面会读回设备状态；如果设置未变化，请重新连接 USB 后重试。";
  }

  const summary = `${error.code ?? "HTTP"} — ${error.message}`;
  return `WiFi 更新失败：${summary}`;
}

function toggleSection(
  current: BackupSectionKey[],
  section: BackupSectionKey,
): BackupSectionKey[] {
  if (current.includes(section)) {
    return current.filter((entry) => entry !== section);
  }
  return [...current, section];
}

type WifiWriteAction = "save" | "clear" | "restore";
type WifiSwitchDialogAction = WifiWriteAction | "switch";

function wifiWriteActionLabel(action: WifiSwitchDialogAction | null): string {
  if (action === "clear") {
    return "Clear User WiFi";
  }
  if (action === "restore") {
    return "Restore WiFi Backup";
  }
  if (action === "switch") {
    return "WiFi Settings";
  }
  return "Save WiFi";
}

type WifiConnectionOption = {
  id: "usb-devd" | "web-serial" | "usb-unavailable";
  label: string;
  detail: string;
  disabled: boolean;
};

type WifiWriteTarget = {
  baseUrl: string;
  identityVerified: boolean;
};

const WIFI_STATUS_REFETCH_INTERVAL_MS = 2_000;
const WIFI_WRITE_REFRESH_DELAY_MS = 1_000;
const WIFI_WRITE_REFRESH_ATTEMPTS = 8;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function shouldRefreshWifiStatusAfterWrite(wifi: WifiStatus): boolean {
  return (
    !isCompleteWifiStatus(wifi) ||
    wifi.state === "configured" ||
    wifi.state === "connecting"
  );
}

function hydrateWifiStatusAfterSet(
  wifi: WifiStatus,
  expectedSsid: string,
): WifiStatus {
  if (!expectedSsid || wifi.state === "error" || wifi.last_error) {
    return wifi;
  }

  return {
    ...wifi,
    ssid: wifi.ssid ?? expectedSsid,
    source: wifi.source ?? "user",
  };
}

function getWifiClearApplyError(wifi: WifiStatus): string | null {
  if (wifi.source !== "user") {
    return null;
  }

  return "WiFi clear did not take effect: device still reports user WiFi.";
}

function isCompleteWifiStatus(wifi: WifiStatus): boolean {
  return (
    (wifi.source === "factory" ||
      wifi.source === "user" ||
      wifi.source === "none") &&
    (typeof wifi.ssid === "string" || wifi.ssid === null)
  );
}

function withConnectionMark(
  marks: StoredDevice["connectionMarks"],
  mark: NonNullable<StoredDevice["connectionMarks"]>[number],
): StoredDevice["connectionMarks"] {
  return marks?.includes(mark) ? marks : [...(marks ?? []), mark];
}

function getWifiConnectionOptions(
  device: StoredDevice,
): WifiConnectionOption[] {
  const options: WifiConnectionOption[] = [];

  if (device.devd?.baseUrl && device.devd.deviceId) {
    options.push({
      id: "usb-devd",
      label: "USB / local devd",
      detail: `${device.devd.deviceId} via ${device.devd.baseUrl}`,
      disabled: false,
    });
  } else if (device.connectionMarks?.includes("usb")) {
    options.push({
      id: "usb-unavailable",
      label: "USB",
      detail: "USB was seen before, but no local devd lease is stored.",
      disabled: true,
    });
  }

  if (device.webSerial) {
    options.push({
      id: "web-serial",
      label: "Web Serial",
      detail: "Known device profile; settings writes are not routed here yet.",
      disabled: true,
    });
  }

  return options;
}

export function DeviceSettingsRoute() {
  const { deviceId, device, baseUrl } = useDeviceContext();
  const queryClient = useQueryClient();
  const deviceStore = useDeviceStore();
  const [confirmSoftResetOpen, setConfirmSoftResetOpen] = useState(false);
  const [wifiSwitchAction, setWifiSwitchAction] =
    useState<WifiSwitchDialogAction | null>(null);
  const [selectedWifiConnection, setSelectedWifiConnection] =
    useState("usb-devd");
  const [wifiSwitchError, setWifiSwitchError] = useState<string | null>(null);
  const [wifiSwitchPending, setWifiSwitchPending] = useState(false);
  const [confirmBackupExportOpen, setConfirmBackupExportOpen] = useState(false);
  const [wifiSsid, setWifiSsid] = useState("");
  const [wifiPsk, setWifiPsk] = useState("");
  const [wifiClearApplyError, setWifiClearApplyError] = useState<string | null>(
    null,
  );
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
  const isUsbDevdTransport = baseUrl ? isDevdCompatBaseUrl(baseUrl) : false;
  const identityQuery = useDeviceIdentityByBaseUrl(deviceId, baseUrl);

  const wifiQueryOptions = getDeviceWifiQueryOptions({
    deviceId,
    baseUrl,
    enabled: Boolean(baseUrl),
    refetchInterval: isUsbDevdTransport
      ? false
      : WIFI_STATUS_REFETCH_INTERVAL_MS,
    refetchOnWindowFocus: true,
  });
  const wifiQuery = useQuery(wifiQueryOptions);

  const cacheWifiStatus = (targetBaseUrl: string, wifi: WifiStatus) => {
    queryClient.setQueryData(
      makeDeviceQueryKey(deviceId, targetBaseUrl, ...DEVICE_QUERY_PARTS.wifi),
      wifi,
    );
  };

  const refreshWifiStatusAfterWrite = (
    initialWifi: WifiStatus,
    writeTarget?: WifiWriteTarget,
    expectedSsid = "",
  ) => {
    const targetBaseUrl = writeTarget?.baseUrl ?? baseUrl;
    if (!targetBaseUrl) {
      return;
    }

    const initial = hydrateWifiStatusAfterSet(initialWifi, expectedSsid);
    if (isCompleteWifiStatus(initial)) {
      cacheWifiStatus(targetBaseUrl, initial);
    }
    if (!shouldRefreshWifiStatusAfterWrite(initial)) {
      return;
    }

    void (async () => {
      let latest = initial;
      for (
        let attempt = 0;
        attempt < WIFI_WRITE_REFRESH_ATTEMPTS &&
        shouldRefreshWifiStatusAfterWrite(latest);
        attempt += 1
      ) {
        await delay(WIFI_WRITE_REFRESH_DELAY_MS);
        try {
          latest = hydrateWifiStatusAfterSet(
            await getWifiStatus(targetBaseUrl),
            expectedSsid,
          );
          cacheWifiStatus(targetBaseUrl, latest);
        } catch {
          return;
        }
      }
    })();
  };

  const refreshWifiStatusAfterClear = (
    initialWifi: WifiStatus,
    writeTarget?: WifiWriteTarget,
  ) => {
    const targetBaseUrl = writeTarget?.baseUrl ?? baseUrl;
    if (!targetBaseUrl) {
      return;
    }

    if (isCompleteWifiStatus(initialWifi)) {
      cacheWifiStatus(targetBaseUrl, initialWifi);
    }
    const initialError = getWifiClearApplyError(initialWifi);
    if (initialError) {
      setWifiClearApplyError(initialError);
      return;
    }
    if (!shouldRefreshWifiStatusAfterWrite(initialWifi)) {
      return;
    }

    void (async () => {
      let latest = initialWifi;
      for (
        let attempt = 0;
        attempt < WIFI_WRITE_REFRESH_ATTEMPTS &&
        shouldRefreshWifiStatusAfterWrite(latest);
        attempt += 1
      ) {
        await delay(WIFI_WRITE_REFRESH_DELAY_MS);
        try {
          latest = await getWifiStatus(targetBaseUrl);
          cacheWifiStatus(targetBaseUrl, latest);
        } catch {
          return;
        }

        const latestError = getWifiClearApplyError(latest);
        if (latestError) {
          setWifiClearApplyError(latestError);
          return;
        }
      }
    })();
  };

  const recoverWifiClearAfterError = async (
    writeTarget?: WifiWriteTarget,
  ): Promise<boolean> => {
    const targetBaseUrl = writeTarget?.baseUrl ?? baseUrl;
    if (!targetBaseUrl) {
      return false;
    }

    let lastApplyError: string | null = null;
    for (let attempt = 0; attempt < WIFI_WRITE_REFRESH_ATTEMPTS; attempt += 1) {
      if (attempt > 0) {
        await delay(WIFI_WRITE_REFRESH_DELAY_MS);
      }

      try {
        const latest = await getWifiStatus(targetBaseUrl);
        cacheWifiStatus(targetBaseUrl, latest);
        lastApplyError = getWifiClearApplyError(latest);
        if (!lastApplyError) {
          return true;
        }
      } catch {}
    }

    if (lastApplyError) {
      setWifiClearApplyError(lastApplyError);
    }
    return false;
  };

  const topError = (() => {
    const err = [identityQuery.error, wifiQuery.error].find(isHttpApiError);
    if (!err) return null;

    const summary = formatHttpApiErrorSummary(err);

    if (err.status === 0 && err.code === "NETWORK_ERROR") {
      return { summary, hint: getNetworkErrorHint(baseUrl) } as const;
    }
    if (isUsbSerialUnavailableError(err)) {
      return { summary, hint: getUsbSerialErrorHint(baseUrl) } as const;
    }
    return { summary, hint: null } as const;
  })();

  const softResetMutation = useMutation({
    mutationFn: async () => {
      const requiredBaseUrl = requireDeviceBaseUrl(baseUrl);
      const payload: SoftResetRequest = makeManualSoftResetRequest();
      return postSoftReset(requiredBaseUrl, payload.reason);
    },
  });

  const wifiMutation = useMutation<
    WifiStatus,
    Error,
    WifiWriteTarget | undefined
  >({
    mutationFn: async (writeTarget) => {
      setWifiClearApplyError(null);
      const requiredBaseUrl = requireDeviceBaseUrl(
        writeTarget?.baseUrl ?? baseUrl,
      );
      const payload: WifiSetRequest = makeWifiSetRequest(
        wifiSsid.trim(),
        wifiPsk,
      );
      return postWifiConfig(requiredBaseUrl, payload, {
        identityVerified:
          writeTarget?.identityVerified ?? identityQuery.isSuccess,
      });
    },
    onSuccess: (wifi, writeTarget) => {
      refreshWifiStatusAfterWrite(wifi, writeTarget, wifiSsid.trim());
      setWifiPsk("");
    },
  });

  const wifiClearMutation = useMutation<
    WifiStatus,
    Error,
    WifiWriteTarget | undefined
  >({
    mutationFn: async (writeTarget) => {
      setWifiClearApplyError(null);
      return deleteWifiConfig(
        requireDeviceBaseUrl(writeTarget?.baseUrl ?? baseUrl),
        {
          identityVerified:
            writeTarget?.identityVerified ?? identityQuery.isSuccess,
        },
      );
    },
    onSuccess: (wifi, writeTarget) => {
      refreshWifiStatusAfterClear(wifi, writeTarget);
      setWifiSsid("");
      setWifiPsk("");
    },
    onError: (_error, writeTarget) => {
      void (async () => {
        if (await recoverWifiClearAfterError(writeTarget)) {
          wifiClearMutation.reset();
          setWifiSsid("");
          setWifiPsk("");
        }
      })();
    },
  });

  const diagnosticsMutation = useMutation({
    mutationFn: async () => {
      return exportDiagnostics(requireDeviceBaseUrl(baseUrl));
    },
  });

  const backupPdQuery = useQuery(
    getDevicePdQueryOptions({
      deviceId,
      baseUrl,
      enabled: Boolean(baseUrl),
      refetchInterval: false,
      parts: DEVICE_QUERY_PARTS.backupPd,
      retry: false,
      retryDelay: 0,
    }),
  );
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
      return exportDeviceBackup(
        requireDeviceBaseUrl(baseUrl),
        effectiveBackupExportSelection,
      );
    },
    onSuccess: (backup) => {
      downloadJsonFile(
        `loadlynx-backup-${new Date().toISOString().replace(/[:.]/g, "-")}.json`,
        backup,
      );
    },
  });

  const backupRestoreMutation = useMutation<
    BackupRestoreResult,
    Error,
    WifiWriteTarget | undefined
  >({
    mutationFn: async (writeTarget) => {
      const requiredBaseUrl = requireDeviceBaseUrl(
        writeTarget?.baseUrl ?? baseUrl,
      );
      if (!backupImport) {
        throw new Error("No backup file selected.");
      }
      return restoreDeviceBackup(
        requiredBaseUrl,
        backupImport,
        backupRestoreSelection,
        {
          identityVerified:
            writeTarget?.identityVerified ?? identityQuery.isSuccess,
        },
      );
    },
  });

  const softResetError = (() => {
    const err = softResetMutation.error;
    if (!err || !isHttpApiError(err)) return null;

    const summary = `Soft reset failed: ${formatHttpApiErrorSummary(err)}`;

    let hint: string | null = null;
    if (err.code === "NETWORK_ERROR") {
      hint = "Network error: check device network/IP.";
    } else if (isLinkUnavailableError(err)) {
      hint = "Link is not ready; soft reset is temporarily unavailable.";
    }

    return { summary, hint } as const;
  })();

  const identity = identityQuery.data;
  const wifi = wifiQuery.data;
  const hasUserWifiOverride = wifi?.source === "user";

  useEffect(() => {
    if (wifi?.state !== "connected" || !wifi.ip) {
      return;
    }

    const identityDeviceId =
      identity?.device_id?.trim() || device.identityDeviceId;
    if (!identityDeviceId) {
      return;
    }

    const current = deviceStore.getDevices();
    const next = upsertRealDevice(current, {
      name: device.name,
      baseUrl: device.baseUrl,
      identityDeviceId,
      connectionMarks: device.connectionMarks,
      lan: device.lan,
      devd: device.devd,
      webSerial: device.webSerial,
      identity,
      wifi,
    });
    if (JSON.stringify(current) === JSON.stringify(next)) {
      return;
    }

    deviceStore.setDevices(next);
    syncDevicesQueryCache(queryClient, deviceStore.getDevices());
  }, [device, deviceStore, identity, queryClient, wifi]);

  const wifiFailureReason =
    wifi?.last_error || wifi?.state === "error"
      ? formatWifiFailureReason(wifi?.last_error)
      : null;
  const wifiWriteErrorMessage = (() => {
    if (wifiClearApplyError) {
      return wifiClearApplyError;
    }

    const clearError =
      wifiClearMutation.error && wifi && !getWifiClearApplyError(wifi)
        ? null
        : wifiClearMutation.error;
    const error = clearError ?? wifiMutation.error;
    if (!error) {
      return null;
    }

    return formatWifiWriteErrorMessage(error);
  })();
  const wifiConnectionSwitchRequired = isWifiTransportBaseUrl(baseUrl);
  const wifiWriteTransportVerified = isWifiWriteTransportVerified(
    baseUrl,
    identityQuery.isSuccess,
  );
  const wifiWriteReadinessMessage = wifiWriteTransportVerified
    ? null
    : "需要先切换到已验证的 USB/devd 管理连接，才可修改 WiFi。";
  const wifiWriteLocked = Boolean(wifiWriteReadinessMessage);
  const wifiConnectionOptions = getWifiConnectionOptions(device);
  const availableWifiConnectionOptions = wifiConnectionOptions.filter(
    (option) => !option.disabled,
  );
  const hasKnownWifiConnectionOptions = wifiConnectionOptions.length > 0;
  const selectedWifiConnectionOption = wifiConnectionOptions.find(
    (option) => option.id === selectedWifiConnection,
  );
  const supportedImportSections = getSupportedBackupSections(backupImport);
  const backupWarnings = getBackupUnknownWarnings(backupImport);
  const backupSafetyBlocked =
    backupRestoreMutation.error &&
    isHttpApiError(backupRestoreMutation.error) &&
    backupRestoreMutation.error.code === "SAFETY_BLOCKED";
  const backupWifiWriteConnectionRequired =
    !wifiWriteTransportVerified &&
    backupRestoreSelection.includes("settings.wifi");
  const backupWifiExportLanConfirmationRequired =
    wifiConnectionSwitchRequired &&
    effectiveBackupExportSelection.includes("settings.wifi");

  const openWifiSwitchDialog = (action: WifiSwitchDialogAction) => {
    setWifiSwitchError(null);
    setSelectedWifiConnection(
      wifiConnectionOptions.find((option) => !option.disabled)?.id ??
        wifiConnectionOptions[0]?.id ??
        "usb-devd",
    );
    setWifiSwitchAction(action);
  };

  const requestWifiWrite = (action: WifiWriteAction) => {
    if (!wifiWriteTransportVerified) {
      openWifiSwitchDialog(action);
      return;
    }

    if (action === "clear") {
      wifiClearMutation.mutate(undefined);
    } else if (action === "restore") {
      backupRestoreMutation.mutate(undefined);
    } else {
      wifiMutation.mutate(undefined);
    }
  };

  const switchWifiConnection = async (
    option: WifiConnectionOption | undefined = selectedWifiConnectionOption,
  ): Promise<WifiWriteTarget> => {
    if (option?.id !== "usb-devd") {
      throw new Error("Select an available management connection.");
    }
    const devd = device.devd;
    if (!devd?.baseUrl || !devd.deviceId) {
      throw new Error("This device has no stored USB/devd connection.");
    }

    const lease = await createDevdLease(devd.deviceId, devd.baseUrl);
    const nextDevd = {
      ...devd,
      leaseId: lease.lease_id,
    };
    const nextBaseUrl = buildDevdCompatBaseUrl(nextDevd);
    const current = deviceStore.getDevices();
    const next = current.map((entry) =>
      entry.id === deviceId
        ? {
            ...entry,
            baseUrl: nextBaseUrl,
            devd: nextDevd,
            connectionMarks: withConnectionMark(entry.connectionMarks, "usb"),
          }
        : entry,
    );
    deviceStore.setDevices(next);
    syncDevicesQueryCache(queryClient, next);
    return { baseUrl: nextBaseUrl, identityVerified: true };
  };

  const continueWifiWriteVia = (option: WifiConnectionOption | undefined) => {
    if (!option || option.disabled || wifiSwitchPending) {
      return;
    }
    const action = wifiSwitchAction;
    setSelectedWifiConnection(option.id);
    setWifiSwitchPending(true);
    setWifiSwitchError(null);
    switchWifiConnection(option)
      .then((writeTarget) => {
        setWifiSwitchAction(null);
        if (!action || action === "switch") {
          return;
        }
        if (action === "clear") {
          wifiClearMutation.mutate(writeTarget);
        } else if (action === "restore") {
          backupRestoreMutation.mutate(writeTarget);
        } else {
          wifiMutation.mutate(writeTarget);
        }
      })
      .catch((error) => {
        setWifiSwitchError(
          error instanceof Error
            ? error.message
            : "Failed to switch connection.",
        );
      })
      .finally(() => {
        setWifiSwitchPending(false);
      });
  };

  const handleSoftReset = () => {
    if (!baseUrl) {
      return;
    }
    setConfirmSoftResetOpen(true);
  };

  const handleWifiSave = () => {
    requestWifiWrite("save");
  };

  const handleWifiClear = () => {
    requestWifiWrite("clear");
  };

  const handleWifiSwitchConnection = () => {
    openWifiSwitchDialog("switch");
  };

  const handleBackupRestore = () => {
    if (backupWifiWriteConnectionRequired) {
      requestWifiWrite("restore");
      return;
    }
    backupRestoreMutation.mutate(undefined);
  };

  const handleBackupExport = () => {
    if (backupWifiExportLanConfirmationRequired) {
      setConfirmBackupExportOpen(true);
      return;
    }
    backupExportMutation.mutate();
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
    const result = parseBackupImportText(await file.text());
    if (result.ok) {
      setBackupImport(result.backup);
      setBackupImportName(file.name);
      setBackupRestoreSelection(result.supportedSections);
      return;
    }
    setBackupImportError(result.error);
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
      {wifiSwitchAction ? (
        <div className="ll-modal" role="dialog" aria-modal="true">
          <div className="ll-modal-box">
            <h3 className="font-bold text-lg">
              {hasKnownWifiConnectionOptions ? "Switch" : "Bind"} Connection for{" "}
              {wifiWriteActionLabel(wifiSwitchAction)}
            </h3>
            {hasKnownWifiConnectionOptions ? (
              <p className="py-3 text-sm">
                WiFi writes run only after an independent management connection
                is verified. Continue through USB/devd; the write will not be
                sent if the switch fails.
              </p>
            ) : (
              <p className="py-3 text-sm">
                This device is only known through the current WiFi/HTTP path.
                Bind USB/devd before changing WiFi.
              </p>
            )}
            {hasKnownWifiConnectionOptions ? (
              <div className="grid gap-2">
                {wifiConnectionOptions.map((option) => (
                  <label
                    key={option.id}
                    className={`flex items-start gap-3 rounded border border-base-200 p-3 text-sm ${
                      option.disabled
                        ? "cursor-not-allowed opacity-60"
                        : "cursor-pointer hover:bg-base-200/50"
                    }`}
                  >
                    <input
                      type="radio"
                      name="wifi-connection"
                      className="ll-radio ll-radio-sm mt-0.5"
                      value={option.id}
                      checked={selectedWifiConnection === option.id}
                      disabled={wifiSwitchPending || option.disabled}
                      onChange={() => setSelectedWifiConnection(option.id)}
                    />
                    <span className="min-w-0">
                      <span className="block font-bold">{option.label}</span>
                      <span className="block break-all text-xs text-base-content/60">
                        {option.detail}
                      </span>
                    </span>
                  </label>
                ))}
              </div>
            ) : null}
            {wifiSwitchError ? (
              <div className="ll-alert ll-alert-error mt-3 text-xs">
                <span>{wifiSwitchError}</span>
              </div>
            ) : null}
            <div className="ll-modal-action">
              {availableWifiConnectionOptions.length > 0 ? (
                <button
                  type="button"
                  className="ll-button ll-button-primary min-w-32"
                  disabled={
                    wifiSwitchPending ||
                    !selectedWifiConnectionOption ||
                    selectedWifiConnectionOption.disabled
                  }
                  onClick={() =>
                    continueWifiWriteVia(selectedWifiConnectionOption)
                  }
                >
                  {wifiSwitchPending ? (
                    <span className="ll-loading ll-loading-spinner ll-loading-xs"></span>
                  ) : (
                    <Cable size={16} strokeWidth={2.4} aria-hidden="true" />
                  )}
                  Continue
                </button>
              ) : (
                <Link
                  to="/devices"
                  className="ll-button ll-button-primary min-w-40"
                >
                  <Cable size={16} strokeWidth={2.4} aria-hidden="true" />
                  Bind connection
                </Link>
              )}
              <button
                type="button"
                className="ll-button min-w-24"
                disabled={wifiSwitchPending}
                onClick={() => {
                  setWifiSwitchAction(null);
                  setWifiSwitchError(null);
                }}
              >
                Cancel
              </button>
            </div>
          </div>
          <button
            type="button"
            className="ll-modal-backdrop"
            aria-label="Close"
            disabled={wifiSwitchPending}
            onClick={() => {
              if (wifiSwitchPending) {
                return;
              }
              setWifiSwitchAction(null);
              setWifiSwitchError(null);
            }}
          />
        </div>
      ) : null}
      <ConfirmDialog
        open={confirmBackupExportOpen}
        title="Export WiFi Backup"
        body="This export will read WiFi credentials over the LAN connection."
        details={[
          "The downloaded backup file contains plaintext PSK.",
          "Use the local USB/devd path when available.",
          "Status, diagnostics and traces still redact PSK.",
        ]}
        confirmLabel="Export Backup"
        confirmDisabled={backupExportMutation.isPending || !baseUrl}
        onCancel={() => setConfirmBackupExportOpen(false)}
        onConfirm={() => {
          setConfirmBackupExportOpen(false);
          backupExportMutation.mutate();
        }}
      />
      <header>
        <h2 className="text-lg font-bold">Device Settings</h2>
        <p className="mt-1 text-sm text-base-content/70">
          Device information and configuration.
        </p>
      </header>

      {topError ? (
        <div className="ll-alert ll-alert-error shadow-sm rounded-lg text-sm">
          <span className="font-bold">Error: {topError.summary}</span>
          {topError.hint && (
            <span className="text-xs opacity-80 block">{topError.hint}</span>
          )}
        </div>
      ) : null}

      <div className="grid gap-6 md:grid-cols-2 md:items-start">
        <div className="grid gap-6">
          {/* 1. Device Identity */}
          <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
            <div className="ll-panel-body p-6">
              <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
                Device Identity
              </h3>
              <div className="overflow-x-auto">
                <table className="ll-table ll-table-xs">
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
          <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
            <div className="ll-panel-body p-6">
              <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
                Capabilities
              </h3>
              <div className="flex flex-wrap gap-2">
                {identity ? (
                  <>
                    <div
                      className={`ll-badge ${identity.capabilities.cc_supported ? "ll-badge-neutral" : "ll-badge-ghost opacity-50"}`}
                    >
                      CC
                    </div>
                    <div
                      className={`ll-badge ${identity.capabilities.cv_supported ? "ll-badge-neutral" : "ll-badge-ghost opacity-50"}`}
                    >
                      CV
                    </div>
                    <div
                      className={`ll-badge ${identity.capabilities.cp_supported ? "ll-badge-neutral" : "ll-badge-ghost opacity-50"}`}
                    >
                      CP
                    </div>
                    <div className="ll-badge ll-badge-ghost">
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
          <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
            <div className="ll-panel-body p-6">
              <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
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
                          className="ll-checkbox ll-checkbox-xs"
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
                    <div className="ll-alert ll-alert-warning shadow-sm text-xs">
                      <span>WiFi credentials require local USB/devd.</span>
                    </div>
                  ) : null}
                  {backupPdUnsupported ? (
                    <div className="ll-alert ll-alert-warning shadow-sm text-xs">
                      <span>
                        USB-PD settings are not supported by this device.
                      </span>
                    </div>
                  ) : null}
                  <button
                    type="button"
                    className="ll-button ll-button-neutral ll-button-sm"
                    disabled={
                      !baseUrl ||
                      effectiveBackupExportSelection.length === 0 ||
                      backupExportMutation.isPending
                    }
                    onClick={handleBackupExport}
                  >
                    Export Backup
                  </button>
                  {backupExportMutation.error ? (
                    <div className="ll-alert ll-alert-error shadow-sm text-xs">
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
                    className="ll-file-input ll-file-input-sm w-full"
                    onChange={(event) => {
                      void handleBackupFile(event.target.files?.[0] ?? null);
                    }}
                  />
                  {backupImportError ? (
                    <div className="ll-alert ll-alert-error shadow-sm text-xs">
                      <span>{backupImportError}</span>
                    </div>
                  ) : null}
                  {backupImport ? (
                    <div className="grid gap-3 rounded border border-base-200 bg-base-200/40 p-3">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-bold truncate">
                          {backupImportName || "backup.json"}
                        </span>
                        <span className="ll-badge ll-badge-ghost">
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
                              className="ll-checkbox ll-checkbox-xs"
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
                        <div className="ll-alert ll-alert-warning shadow-sm text-xs">
                          <span>{backupWarnings.join(" · ")}</span>
                        </div>
                      ) : null}
                      <button
                        type="button"
                        className="ll-button ll-button-outline ll-button-sm"
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
                    <div className="ll-alert ll-alert-error shadow-sm text-xs">
                      <span>
                        Restore safety-blocked:{" "}
                        {backupRestoreMutation.error?.message}
                      </span>
                    </div>
                  ) : backupRestoreMutation.error ? (
                    <div className="ll-alert ll-alert-error shadow-sm text-xs">
                      <span>
                        Restore failed: {backupRestoreMutation.error.message}
                      </span>
                    </div>
                  ) : null}
                  {backupRestoreMutation.data ? (
                    <div
                      className={`ll-alert shadow-sm text-xs ${backupRestoreMutation.data.ok ? "ll-alert-success" : "ll-alert-warning"}`}
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
          <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
            <div className="ll-panel-body p-6">
              <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
                Network
              </h3>
              <div className="overflow-x-auto">
                <table className="ll-table ll-table-xs">
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
                    <tr>
                      <td className="text-base-content/60">Management path</td>
                      <td data-testid="management-transport">
                        {getManagementTransportLabel(baseUrl)}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          {/* 4. WiFi */}
          <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
            <div className="ll-panel-body p-6">
              <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
                WiFi
              </h3>
              <div className="grid gap-3">
                <div className="grid grid-cols-2 gap-3 text-xs">
                  <span className="text-base-content/60">SSID</span>
                  <span className="truncate" data-testid="wifi-status-ssid">
                    {wifi?.ssid ?? "..."}
                  </span>
                  <span className="text-base-content/60">Source</span>
                  <span data-testid="wifi-status-source">
                    {wifi?.source ?? "..."}
                  </span>
                  <span className="text-base-content/60">State</span>
                  <span data-testid="wifi-status-state">
                    {wifi?.state ?? "..."}
                  </span>
                  <span className="text-base-content/60">IP</span>
                  <span data-testid="wifi-status-ip">{wifi?.ip ?? "..."}</span>
                  <span className="text-base-content/60">Last error</span>
                  <span
                    className={
                      wifiFailureReason ? "text-error font-bold" : undefined
                    }
                  >
                    {wifiFailureReason ?? wifi?.last_error ?? "none"}
                  </span>
                </div>
                {wifiFailureReason ? (
                  <div className="ll-alert ll-alert-error shadow-sm text-xs">
                    <span>WiFi connection failed: {wifiFailureReason}</span>
                  </div>
                ) : null}
                <div
                  className={`relative rounded-lg ${
                    wifiWriteLocked ? "overflow-hidden" : "overflow-visible"
                  }`}
                >
                  <div
                    aria-hidden={wifiWriteLocked ? true : undefined}
                    className={`grid gap-3 transition duration-200 ${
                      wifiWriteLocked
                        ? "pointer-events-none select-none blur-[2px] opacity-45"
                        : ""
                    }`}
                  >
                    <input
                      className="ll-input ll-input-sm w-full"
                      placeholder="SSID"
                      value={wifiSsid}
                      disabled={wifiWriteLocked}
                      onChange={(event) => setWifiSsid(event.target.value)}
                    />
                    <input
                      className="ll-input ll-input-sm w-full"
                      placeholder="PSK"
                      type="password"
                      value={wifiPsk}
                      disabled={wifiWriteLocked}
                      onChange={(event) => setWifiPsk(event.target.value)}
                    />
                    {wifiWriteErrorMessage ? (
                      <div className="ll-alert ll-alert-error shadow-sm text-xs">
                        <span>{wifiWriteErrorMessage}</span>
                      </div>
                    ) : null}
                    <div className="flex flex-wrap gap-2">
                      <button
                        type="button"
                        className="ll-button ll-button-neutral ll-button-sm"
                        disabled={
                          wifiWriteLocked ||
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
                        className="ll-button ll-button-outline ll-button-sm"
                        title={
                          hasUserWifiOverride
                            ? undefined
                            : "No user WiFi override is saved."
                        }
                        disabled={
                          wifiWriteLocked ||
                          !hasUserWifiOverride ||
                          wifiClearMutation.isPending ||
                          !baseUrl
                        }
                        onClick={handleWifiClear}
                      >
                        Clear User WiFi
                      </button>
                    </div>
                  </div>
                  {wifiWriteReadinessMessage ? (
                    <div className="absolute inset-0 z-10 flex items-center justify-center rounded-lg border border-warning/50 bg-base-100/45 px-4 py-6 text-center backdrop-blur-md">
                      <div className="flex max-w-md flex-col items-center gap-3">
                        <p className="text-sm font-semibold leading-relaxed text-base-content">
                          {wifiWriteReadinessMessage}
                        </p>
                        <button
                          type="button"
                          className="ll-button ll-button-primary ll-button-sm min-w-44"
                          onClick={handleWifiSwitchConnection}
                        >
                          <Cable
                            size={16}
                            strokeWidth={2.4}
                            aria-hidden="true"
                          />
                          切换连接方式
                        </button>
                      </div>
                    </div>
                  ) : null}
                </div>
              </div>
            </div>
          </div>

          {/* 6. Actions */}
          <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
            <div className="ll-panel-body p-6">
              <h3 className="ll-panel-title text-sm uppercase tracking-wider text-base-content/50 mb-4 h-auto min-h-0">
                Actions
              </h3>
              <div className="flex flex-col gap-3">
                {softResetMutation.isSuccess ? (
                  <div className="ll-alert ll-alert-success shadow-sm text-xs sm:text-sm">
                    <span>
                      Soft reset requested (reason:{" "}
                      {softResetMutation.data?.reason ?? "manual"}).
                    </span>
                  </div>
                ) : null}
                {softResetError ? (
                  <div className="ll-alert ll-alert-error shadow-sm text-xs sm:text-sm">
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
                  className="ll-button ll-button-outline ll-button-sm text-error hover:bg-error hover:text-white"
                  onClick={handleSoftReset}
                >
                  Soft Reset
                </button>
                <button
                  type="button"
                  className="ll-button ll-button-outline ll-button-sm"
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

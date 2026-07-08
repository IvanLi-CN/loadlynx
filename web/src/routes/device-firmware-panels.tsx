import { useQueryClient } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useMemo, useState } from "react";
import {
  useDevdArtifactSelect,
  useDevdFlash,
  useDevdSession,
} from "../devd/hooks.ts";
import type { DevdTargetKind } from "../devd/types.ts";
import { upsertRealDevice } from "../devices/hooks.ts";
import { syncDevicesQueryCache } from "../devices/query-cache.ts";
import { useDeviceStore } from "../devices/store-context.tsx";
import type { useDeviceContext } from "../layouts/device-layout.tsx";
import {
  hasWebSerialSupport,
  parseFirmwareCatalog,
  runWebSerialDigitalFlash,
  WEB_SERIAL_FLASH_CONFIRMATION_TEXT,
  type WebSerialFlashResult,
} from "../web-serial/loadlynx-web-serial.ts";

type FirmwareDevice = ReturnType<typeof useDeviceContext>["device"];

export function DevdFirmwarePanel({ device }: { device: FirmwareDevice }) {
  const devd = device.devd;
  const [target, setTarget] = useState<DevdTargetKind>("digital_esp32s3");
  const [artifactId, setArtifactId] = useState("");
  const [manifestPath, setManifestPath] = useState("");
  const [dryRun, setDryRun] = useState(true);
  const [confirmText, setConfirmText] = useState("");
  const [expectedIdentity, setExpectedIdentity] = useState("");
  const [nonProjectAck, setNonProjectAck] = useState(false);
  const selectArtifactMutation = useDevdArtifactSelect(devd?.baseUrl);
  const flashMutation = useDevdFlash(devd?.baseUrl);
  const canUseDevd = Boolean(devd?.deviceId && devd?.leaseId);
  const needsGate = !dryRun && target === "digital_esp32s3";

  const runFlash = () => {
    if (!devd) return;
    const selectedArtifactId = artifactId.trim() || undefined;
    void (async () => {
      try {
        await selectArtifactMutation.mutateAsync({
          deviceId: devd.deviceId,
          manifestPath: manifestPath.trim(),
          artifactId: selectedArtifactId,
        });
        flashMutation.mutate({
          deviceId: devd.deviceId,
          leaseId: devd.leaseId,
          target,
          artifactId: selectedArtifactId,
          dryRun,
          confirmationPhrase: confirmText || undefined,
          expectedIdentityDeviceId: expectedIdentity.trim() || undefined,
          acknowledgeNonProjectFirmware: nonProjectAck,
        });
      } catch {
        // Mutation state renders the error.
      }
    })();
  };

  return (
    <section className="ll-panel bg-base-100 border border-base-200 shadow-sm">
      <div className="ll-panel-body gap-4">
        <div className="grid gap-4 md:grid-cols-3">
          <label className="ll-form-control">
            <div className="ll-label-row pb-1">
              <span className="ll-label-text">Target board</span>
            </div>
            <select
              id="firmware-target"
              name="firmware_target"
              className="ll-select"
              value={target}
              onChange={(event) =>
                setTarget(event.target.value as DevdTargetKind)
              }
            >
              <option value="digital_esp32s3">Digital ESP32-S3</option>
              <option value="analog_stm32g431">Analog STM32G431</option>
            </select>
          </label>
          <FirmwareTextInput
            id="firmware-artifact-id"
            name="firmware_artifact_id"
            label="Artifact ID"
            value={artifactId}
            onChange={setArtifactId}
            placeholder="Select or type a staged artifact id"
          />
          <FirmwareTextInput
            id="firmware-manifest-path"
            name="firmware_manifest_path"
            label="Catalog manifest"
            value={manifestPath}
            onChange={setManifestPath}
            placeholder="/path/to/firmware-catalog.json"
          />
        </div>

        <label className="ll-label-row cursor-pointer justify-start gap-3">
          <input
            id="firmware-dry-run"
            name="firmware_dry_run"
            type="checkbox"
            className="ll-checkbox ll-checkbox-sm"
            checked={dryRun}
            onChange={(event) => setDryRun(event.target.checked)}
          />
          <span className="ll-label-text">
            Dry-run only: verify target evidence without touching hardware
          </span>
        </label>

        {needsGate ? (
          <div className="grid gap-4 md:grid-cols-3">
            <FirmwareTextInput
              label="Confirmation"
              value={confirmText}
              onChange={setConfirmText}
              placeholder={WEB_SERIAL_FLASH_CONFIRMATION_TEXT}
            />
            <FirmwareTextInput
              label="Expected identity"
              value={expectedIdentity}
              onChange={setExpectedIdentity}
              placeholder="Optional current device_id"
            />
            <RiskAck checked={nonProjectAck} onChange={setNonProjectAck} />
          </div>
        ) : null}

        <div className="flex flex-wrap gap-3">
          <button
            type="button"
            className="ll-button ll-button-primary"
            disabled={
              !canUseDevd ||
              !manifestPath.trim() ||
              selectArtifactMutation.isPending ||
              flashMutation.isPending
            }
            onClick={runFlash}
          >
            {selectArtifactMutation.isPending || flashMutation.isPending ? (
              <span className="ll-loading ll-loading-spinner ll-loading-xs"></span>
            ) : null}
            {dryRun ? "Verify flash dry-run" : "Flash firmware"}
          </button>
        </div>

        {selectArtifactMutation.error || flashMutation.error ? (
          <div role="alert" className="ll-alert ll-alert-error text-sm">
            <span>
              {selectArtifactMutation.error instanceof Error
                ? selectArtifactMutation.error.message
                : flashMutation.error instanceof Error
                  ? flashMutation.error.message
                  : "Firmware operation failed"}
            </span>
          </div>
        ) : null}

        {flashMutation.data ? (
          <div className="ll-codeblock text-xs">
            <pre data-prefix="$">
              <code>
                {JSON.stringify(flashMutation.data.target_evidence, null, 2)}
              </code>
            </pre>
          </div>
        ) : null}
      </div>
    </section>
  );
}

export function WebSerialFlashPanel({ device }: { device: FirmwareDevice }) {
  const queryClient = useQueryClient();
  const deviceStore = useDeviceStore();
  const [artifactId, setArtifactId] = useState("");
  const [catalogFile, setCatalogFile] = useState<File | null>(null);
  const [firmwareFile, setFirmwareFile] = useState<File | null>(null);
  const [confirmText, setConfirmText] = useState("");
  const [expectedIdentity, setExpectedIdentity] = useState("");
  const [nonProjectAck, setNonProjectAck] = useState(false);
  const [result, setResult] = useState<WebSerialFlashResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const webSerialSupported = useMemo(() => hasWebSerialSupport(), []);

  const runFlash = () => {
    if (!catalogFile || !firmwareFile) return;
    void (async () => {
      setPending(true);
      setError(null);
      setResult(null);
      try {
        const catalog = await parseFirmwareCatalog(catalogFile);
        const flashResult = await runWebSerialDigitalFlash({
          catalog,
          artifactId: artifactId.trim() || undefined,
          firmwareFile,
          confirmationPhrase: confirmText,
          expectedIdentityDeviceId: expectedIdentity.trim() || undefined,
          acknowledgeNonProjectFirmware: nonProjectAck,
        });
        setResult(flashResult);
        const profile =
          flashResult.postFlashIdentity ?? flashResult.preFlashIdentity;
        if (profile) {
          const current = deviceStore.getDevices();
          const lanBaseUrlHints = [
            profile.hostname ? `http://${profile.hostname}` : undefined,
            profile.network?.hostname
              ? `http://${profile.network.hostname}`
              : undefined,
            profile.network?.ip ? `http://${profile.network.ip}` : undefined,
          ].filter((entry): entry is string => Boolean(entry));
          const next = upsertRealDevice(current, {
            name: profile.displayName ?? device.name,
            baseUrl: device.baseUrl,
            identityDeviceId: profile.deviceId,
            connectionMarks: device.connectionMarks,
            lan: device.lan,
            devd: device.devd,
            webSerial: {
              identityDeviceId: profile.deviceId,
              displayName: profile.displayName,
              profileCapturedAt: profile.capturedAt,
            },
            lanBaseUrlHints,
          });
          deviceStore.setDevices(next);
          syncDevicesQueryCache(queryClient, deviceStore.getDevices());
        }
      } catch (flashError) {
        setError(
          flashError instanceof Error
            ? flashError.message
            : "Web Serial flash failed",
        );
      } finally {
        setPending(false);
      }
    })();
  };

  return (
    <section className="ll-panel bg-base-100 border border-base-200 shadow-sm">
      <div className="ll-panel-body gap-4">
        <div>
          <h3 className="ll-panel-title text-base">Web Serial flash</h3>
          <p className="text-sm text-base-content/70">
            Uses browser-granted serial access and local files. It does not save
            OS port paths.
          </p>
        </div>
        {!webSerialSupported ? (
          <div role="alert" className="ll-alert ll-alert-warning text-sm">
            <span>
              This browser does not support Web Serial. Use Chrome/Edge, or
              install the released CLI/devd host tools.
            </span>
          </div>
        ) : null}

        <div className="grid gap-4 md:grid-cols-2">
          <FirmwareFileInput
            label="Firmware catalog JSON"
            accept="application/json,.json"
            onChange={setCatalogFile}
          />
          <FirmwareFileInput
            label="Firmware binary"
            onChange={setFirmwareFile}
          />
          <FirmwareTextInput
            label="Artifact ID"
            value={artifactId}
            onChange={setArtifactId}
            placeholder="Optional catalog artifact id"
          />
          <FirmwareTextInput
            label="Expected identity"
            value={expectedIdentity}
            onChange={setExpectedIdentity}
            placeholder="Optional current device_id"
          />
          <FirmwareTextInput
            label="Confirmation"
            value={confirmText}
            onChange={setConfirmText}
            placeholder={WEB_SERIAL_FLASH_CONFIRMATION_TEXT}
          />
          <RiskAck checked={nonProjectAck} onChange={setNonProjectAck} />
        </div>

        <div>
          <button
            type="button"
            className="ll-button ll-button-primary"
            disabled={
              !webSerialSupported || !catalogFile || !firmwareFile || pending
            }
            onClick={runFlash}
          >
            {pending ? (
              <span className="ll-loading ll-loading-spinner ll-loading-xs"></span>
            ) : null}
            Flash with Web Serial
          </button>
        </div>

        {error ? (
          <div role="alert" className="ll-alert ll-alert-error text-sm">
            <span>{error}</span>
          </div>
        ) : null}

        {result ? (
          <div className="ll-codeblock text-xs">
            <pre data-prefix="$">
              <code>
                {JSON.stringify(
                  {
                    artifact_id: result.artifact.artifact_id,
                    file: result.file.path,
                    sha256: result.sha256,
                    post_flash_identity: result.postFlashIdentity ?? null,
                  },
                  null,
                  2,
                )}
              </code>
            </pre>
          </div>
        ) : null}
      </div>
    </section>
  );
}

export function UsbSessionPanel({ device }: { device: FirmwareDevice }) {
  const devd = device.devd;
  const sessionQuery = useDevdSession(
    devd?.deviceId ?? null,
    devd?.leaseId ?? null,
    devd?.baseUrl,
  );
  const canUseDevd = Boolean(devd?.deviceId && devd?.leaseId);

  return (
    <section className="ll-panel bg-base-100 border border-base-200 shadow-sm">
      <div className="ll-panel-body gap-4">
        <div className="flex items-center justify-between">
          <h3 className="ll-panel-title text-base">USB session</h3>
          <button
            type="button"
            className="ll-button ll-button-sm ll-button-outline"
            disabled={!canUseDevd || sessionQuery.isFetching}
            onClick={() => void sessionQuery.refetch()}
          >
            Refresh
          </button>
        </div>
        {sessionQuery.data ? (
          <div className="grid gap-4 lg:grid-cols-2">
            <SessionTable
              title="Logs"
              rows={sessionQuery.data.logs}
              render={(entry) => (
                <>
                  <td className="font-mono">{entry.level}</td>
                  <td>{entry.target}</td>
                  <td>{entry.message}</td>
                </>
              )}
            />
            <SessionTable
              title="Trace"
              rows={sessionQuery.data.trace}
              render={(entry) => (
                <>
                  <td className="font-mono">{entry.direction}</td>
                  <td>{entry.summary}</td>
                </>
              )}
            />
          </div>
        ) : (
          <div className="text-sm text-base-content/60">
            No session snapshot loaded.
          </div>
        )}
      </div>
    </section>
  );
}

function FirmwareTextInput({
  id,
  name,
  label,
  value,
  onChange,
  placeholder,
}: {
  id?: string;
  name?: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <label className="ll-form-control">
      <div className="ll-label-row pb-1">
        <span className="ll-label-text">{label}</span>
      </div>
      <input
        id={id}
        name={name}
        className="ll-input"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
      />
    </label>
  );
}

function FirmwareFileInput({
  label,
  accept,
  onChange,
}: {
  label: string;
  accept?: string;
  onChange: (file: File | null) => void;
}) {
  return (
    <label className="ll-form-control">
      <div className="ll-label-row pb-1">
        <span className="ll-label-text">{label}</span>
      </div>
      <input
        type="file"
        accept={accept}
        className="ll-file-input"
        onChange={(event) => onChange(event.target.files?.[0] ?? null)}
      />
    </label>
  );
}

function RiskAck({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="ll-label-row cursor-pointer justify-start gap-3 pt-7">
      <input
        type="checkbox"
        className="ll-checkbox ll-checkbox-sm"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span className="ll-label-text">
        Acknowledge non-project firmware risk
      </span>
    </label>
  );
}

function SessionTable<T extends { id: string }>({
  title,
  rows,
  render,
}: {
  title: string;
  rows: T[];
  render: (row: T) => ReactNode;
}) {
  return (
    <div>
      <div className="text-xs uppercase tracking-wide opacity-60">{title}</div>
      <div className="mt-2 max-h-64 overflow-auto rounded border border-base-200">
        <table className="ll-table ll-table-xs">
          <tbody>
            {rows.map((entry) => (
              <tr key={entry.id}>{render(entry)}</tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

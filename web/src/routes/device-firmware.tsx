import { useMemo, useState } from "react";
import { PageContainer } from "../components/layout/page-container.tsx";
import {
  useDevdArtifactSelect,
  useDevdFlash,
  useDevdSession,
} from "../devd/hooks.ts";
import type { DevdTargetKind } from "../devd/types.ts";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import {
  hasWebSerialSupport,
  parseFirmwareCatalog,
  runWebSerialDigitalFlash,
  WEB_SERIAL_FLASH_CONFIRMATION_TEXT,
  type WebSerialFlashResult,
} from "../web-serial/loadlynx-web-serial.ts";

export function DeviceFirmwareRoute() {
  const { device } = useDeviceContext();
  const devd = device.devd;
  const [target, setTarget] = useState<DevdTargetKind>("digital_esp32s3");
  const [artifactId, setArtifactId] = useState("");
  const [manifestPath, setManifestPath] = useState("");
  const [dryRun, setDryRun] = useState(true);
  const [devdConfirmText, setDevdConfirmText] = useState("");
  const [devdExpectedIdentity, setDevdExpectedIdentity] = useState("");
  const [devdNonProjectAck, setDevdNonProjectAck] = useState(false);
  const [webArtifactId, setWebArtifactId] = useState("");
  const [webCatalogFile, setWebCatalogFile] = useState<File | null>(null);
  const [webFirmwareFile, setWebFirmwareFile] = useState<File | null>(null);
  const [webConfirmText, setWebConfirmText] = useState("");
  const [webExpectedIdentity, setWebExpectedIdentity] = useState("");
  const [webNonProjectAck, setWebNonProjectAck] = useState(false);
  const [webFlashResult, setWebFlashResult] =
    useState<WebSerialFlashResult | null>(null);
  const [webFlashError, setWebFlashError] = useState<string | null>(null);
  const [webFlashPending, setWebFlashPending] = useState(false);
  const selectArtifactMutation = useDevdArtifactSelect(devd?.baseUrl);
  const flashMutation = useDevdFlash(devd?.baseUrl);
  const sessionQuery = useDevdSession(
    devd?.deviceId ?? null,
    devd?.leaseId ?? null,
    devd?.baseUrl,
  );

  const canUseDevd = Boolean(devd?.deviceId && devd?.leaseId);
  const webSerialSupported = useMemo(() => hasWebSerialSupport(), []);
  const devdNeedsGate = !dryRun && target === "digital_esp32s3";

  return (
    <PageContainer className="space-y-6">
      <header className="flex flex-col gap-2">
        <h2 className="text-2xl font-bold">Firmware</h2>
        <p className="text-sm text-base-content/70">
          Flash through the local devd bridge or a Web Serial browser session.
          Real digital flashes require artifact hash evidence, explicit
          confirmation, and post-flash identity capture.
        </p>
      </header>

      {!canUseDevd ? (
        <div role="alert" className="ll-alert ll-alert-warning">
          <span>
            This device is not bound to an active devd USB lease. Connect it
            from the Devices page before using devd firmware operations.
          </span>
        </div>
      ) : null}

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

            <label className="ll-form-control">
              <div className="ll-label-row pb-1">
                <span className="ll-label-text">Artifact ID</span>
              </div>
              <input
                id="firmware-artifact-id"
                name="firmware_artifact_id"
                className="ll-input"
                value={artifactId}
                onChange={(event) => setArtifactId(event.target.value)}
                placeholder="Select or type a staged artifact id"
              />
            </label>

            <label className="ll-form-control">
              <div className="ll-label-row pb-1">
                <span className="ll-label-text">Catalog manifest</span>
              </div>
              <input
                id="firmware-manifest-path"
                name="firmware_manifest_path"
                className="ll-input"
                value={manifestPath}
                onChange={(event) => setManifestPath(event.target.value)}
                placeholder="/path/to/firmware-catalog.json"
              />
            </label>
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

          {devdNeedsGate ? (
            <div className="grid gap-4 md:grid-cols-3">
              <label className="ll-form-control">
                <div className="ll-label-row pb-1">
                  <span className="ll-label-text">Confirmation</span>
                </div>
                <input
                  className="ll-input"
                  value={devdConfirmText}
                  onChange={(event) => setDevdConfirmText(event.target.value)}
                  placeholder={WEB_SERIAL_FLASH_CONFIRMATION_TEXT}
                />
              </label>
              <label className="ll-form-control">
                <div className="ll-label-row pb-1">
                  <span className="ll-label-text">Expected identity</span>
                </div>
                <input
                  className="ll-input"
                  value={devdExpectedIdentity}
                  onChange={(event) =>
                    setDevdExpectedIdentity(event.target.value)
                  }
                  placeholder="Optional current device_id"
                />
              </label>
              <label className="ll-label-row cursor-pointer justify-start gap-3 pt-7">
                <input
                  type="checkbox"
                  className="ll-checkbox ll-checkbox-sm"
                  checked={devdNonProjectAck}
                  onChange={(event) =>
                    setDevdNonProjectAck(event.target.checked)
                  }
                />
                <span className="ll-label-text">
                  Acknowledge non-project firmware risk
                </span>
              </label>
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
              onClick={() => {
                if (!devd) return;
                const selectedArtifactId = artifactId.trim() || undefined;
                const selectedManifestPath = manifestPath.trim();
                void (async () => {
                  try {
                    await selectArtifactMutation.mutateAsync({
                      deviceId: devd.deviceId,
                      manifestPath: selectedManifestPath,
                      artifactId: selectedArtifactId,
                    });
                    flashMutation.mutate({
                      deviceId: devd.deviceId,
                      leaseId: devd.leaseId,
                      target,
                      artifactId: selectedArtifactId,
                      dryRun,
                      confirmationPhrase: devdConfirmText || undefined,
                      expectedIdentityDeviceId:
                        devdExpectedIdentity.trim() || undefined,
                      acknowledgeNonProjectFirmware: devdNonProjectAck,
                    });
                  } catch {
                    // The mutation state renders the error.
                  }
                })();
              }}
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

      <section className="ll-panel bg-base-100 border border-base-200 shadow-sm">
        <div className="ll-panel-body gap-4">
          <div>
            <h3 className="ll-panel-title text-base">Web Serial flash</h3>
            <p className="text-sm text-base-content/70">
              Uses browser-granted serial access and local files. It does not
              save OS port paths.
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
            <label className="ll-form-control">
              <div className="ll-label-row pb-1">
                <span className="ll-label-text">Firmware catalog JSON</span>
              </div>
              <input
                type="file"
                accept="application/json,.json"
                className="ll-file-input"
                onChange={(event) =>
                  setWebCatalogFile(event.target.files?.[0] ?? null)
                }
              />
            </label>
            <label className="ll-form-control">
              <div className="ll-label-row pb-1">
                <span className="ll-label-text">Firmware binary</span>
              </div>
              <input
                type="file"
                className="ll-file-input"
                onChange={(event) =>
                  setWebFirmwareFile(event.target.files?.[0] ?? null)
                }
              />
            </label>
            <label className="ll-form-control">
              <div className="ll-label-row pb-1">
                <span className="ll-label-text">Artifact ID</span>
              </div>
              <input
                className="ll-input"
                value={webArtifactId}
                onChange={(event) => setWebArtifactId(event.target.value)}
                placeholder="Optional catalog artifact id"
              />
            </label>
            <label className="ll-form-control">
              <div className="ll-label-row pb-1">
                <span className="ll-label-text">Expected identity</span>
              </div>
              <input
                className="ll-input"
                value={webExpectedIdentity}
                onChange={(event) => setWebExpectedIdentity(event.target.value)}
                placeholder="Optional current device_id"
              />
            </label>
            <label className="ll-form-control">
              <div className="ll-label-row pb-1">
                <span className="ll-label-text">Confirmation</span>
              </div>
              <input
                className="ll-input"
                value={webConfirmText}
                onChange={(event) => setWebConfirmText(event.target.value)}
                placeholder={WEB_SERIAL_FLASH_CONFIRMATION_TEXT}
              />
            </label>
            <label className="ll-label-row cursor-pointer justify-start gap-3 pt-7">
              <input
                type="checkbox"
                className="ll-checkbox ll-checkbox-sm"
                checked={webNonProjectAck}
                onChange={(event) => setWebNonProjectAck(event.target.checked)}
              />
              <span className="ll-label-text">
                Acknowledge non-project firmware risk
              </span>
            </label>
          </div>

          <div>
            <button
              type="button"
              className="ll-button ll-button-primary"
              disabled={
                !webSerialSupported ||
                !webCatalogFile ||
                !webFirmwareFile ||
                webFlashPending
              }
              onClick={() => {
                if (!webCatalogFile || !webFirmwareFile) return;
                void (async () => {
                  setWebFlashPending(true);
                  setWebFlashError(null);
                  setWebFlashResult(null);
                  try {
                    const catalog = await parseFirmwareCatalog(webCatalogFile);
                    const result = await runWebSerialDigitalFlash({
                      catalog,
                      artifactId: webArtifactId.trim() || undefined,
                      firmwareFile: webFirmwareFile,
                      confirmationPhrase: webConfirmText,
                      expectedIdentityDeviceId:
                        webExpectedIdentity.trim() || undefined,
                      acknowledgeNonProjectFirmware: webNonProjectAck,
                    });
                    setWebFlashResult(result);
                  } catch (error) {
                    setWebFlashError(
                      error instanceof Error
                        ? error.message
                        : "Web Serial flash failed",
                    );
                  } finally {
                    setWebFlashPending(false);
                  }
                })();
              }}
            >
              {webFlashPending ? (
                <span className="ll-loading ll-loading-spinner ll-loading-xs"></span>
              ) : null}
              Flash with Web Serial
            </button>
          </div>

          {webFlashError ? (
            <div role="alert" className="ll-alert ll-alert-error text-sm">
              <span>{webFlashError}</span>
            </div>
          ) : null}

          {webFlashResult ? (
            <div className="ll-codeblock text-xs">
              <pre data-prefix="$">
                <code>
                  {JSON.stringify(
                    {
                      artifact_id: webFlashResult.artifact.artifact_id,
                      file: webFlashResult.file.path,
                      sha256: webFlashResult.sha256,
                      post_flash_identity:
                        webFlashResult.postFlashIdentity ?? null,
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
              <div>
                <div className="text-xs uppercase tracking-wide opacity-60">
                  Logs
                </div>
                <div className="mt-2 max-h-64 overflow-auto rounded border border-base-200">
                  <table className="ll-table ll-table-xs">
                    <tbody>
                      {sessionQuery.data.logs.map((entry) => (
                        <tr key={entry.id}>
                          <td className="font-mono">{entry.level}</td>
                          <td>{entry.target}</td>
                          <td>{entry.message}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
              <div>
                <div className="text-xs uppercase tracking-wide opacity-60">
                  Trace
                </div>
                <div className="mt-2 max-h-64 overflow-auto rounded border border-base-200">
                  <table className="ll-table ll-table-xs">
                    <tbody>
                      {sessionQuery.data.trace.map((entry) => (
                        <tr key={entry.id}>
                          <td className="font-mono">{entry.direction}</td>
                          <td>{entry.summary}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          ) : (
            <div className="text-sm text-base-content/60">
              No session snapshot loaded.
            </div>
          )}
        </div>
      </section>
    </PageContainer>
  );
}

export default DeviceFirmwareRoute;

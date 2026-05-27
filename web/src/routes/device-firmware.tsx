import { useState } from "react";
import { PageContainer } from "../components/layout/page-container.tsx";
import {
  useDevdArtifactSelect,
  useDevdFlash,
  useDevdSession,
} from "../devd/hooks.ts";
import type { DevdTargetKind } from "../devd/types.ts";
import { useDeviceContext } from "../layouts/device-layout.tsx";

export function DeviceFirmwareRoute() {
  const { device } = useDeviceContext();
  const devd = device.devd;
  const [target, setTarget] = useState<DevdTargetKind>("digital_esp32s3");
  const [artifactId, setArtifactId] = useState("");
  const [manifestPath, setManifestPath] = useState("");
  const [dryRun, setDryRun] = useState(true);
  const selectArtifactMutation = useDevdArtifactSelect(devd?.baseUrl);
  const flashMutation = useDevdFlash(devd?.baseUrl);
  const sessionQuery = useDevdSession(
    devd?.deviceId ?? null,
    devd?.leaseId ?? null,
    devd?.baseUrl,
  );

  const canUseDevd = Boolean(devd?.deviceId && devd?.leaseId);

  return (
    <PageContainer className="space-y-6">
      <header className="flex flex-col gap-2">
        <h2 className="text-2xl font-bold">Firmware</h2>
        <p className="text-sm text-base-content/70">
          Flash digital or analog targets through the local devd bridge. The
          first action should stay in dry-run mode to verify target evidence and
          artifact hashes.
        </p>
      </header>

      {!canUseDevd ? (
        <div role="alert" className="alert alert-warning">
          <span>
            This device is not bound to an active devd USB lease. Connect it
            from the Devices page before using devd firmware operations.
          </span>
        </div>
      ) : null}

      <section className="card bg-base-100 border border-base-200 shadow-sm">
        <div className="card-body gap-4">
          <div className="grid gap-4 md:grid-cols-3">
            <label className="form-control">
              <div className="label pb-1">
                <span className="label-text">Target board</span>
              </div>
              <select
                id="firmware-target"
                name="firmware_target"
                className="select select-bordered"
                value={target}
                onChange={(event) =>
                  setTarget(event.target.value as DevdTargetKind)
                }
              >
                <option value="digital_esp32s3">Digital ESP32-S3</option>
                <option value="analog_stm32g431">Analog STM32G431</option>
              </select>
            </label>

            <label className="form-control">
              <div className="label pb-1">
                <span className="label-text">Artifact ID</span>
              </div>
              <input
                id="firmware-artifact-id"
                name="firmware_artifact_id"
                className="input input-bordered"
                value={artifactId}
                onChange={(event) => setArtifactId(event.target.value)}
                placeholder="Select or type a staged artifact id"
              />
            </label>

            <label className="form-control">
              <div className="label pb-1">
                <span className="label-text">Catalog manifest</span>
              </div>
              <input
                id="firmware-manifest-path"
                name="firmware_manifest_path"
                className="input input-bordered"
                value={manifestPath}
                onChange={(event) => setManifestPath(event.target.value)}
                placeholder="/path/to/firmware-catalog.json"
              />
            </label>
          </div>

          <label className="label cursor-pointer justify-start gap-3">
            <input
              id="firmware-dry-run"
              name="firmware_dry_run"
              type="checkbox"
              className="checkbox checkbox-sm"
              checked={dryRun}
              onChange={(event) => setDryRun(event.target.checked)}
            />
            <span className="label-text">
              Dry-run only: verify target evidence without touching hardware
            </span>
          </label>

          <div className="flex flex-wrap gap-3">
            <button
              type="button"
              className="btn btn-primary"
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
                    });
                  } catch {
                    // The mutation state renders the error.
                  }
                })();
              }}
            >
              {selectArtifactMutation.isPending || flashMutation.isPending ? (
                <span className="loading loading-spinner loading-xs"></span>
              ) : null}
              {dryRun ? "Verify flash dry-run" : "Flash firmware"}
            </button>
          </div>

          {selectArtifactMutation.error || flashMutation.error ? (
            <div role="alert" className="alert alert-error text-sm">
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
            <div className="mockup-code text-xs">
              <pre data-prefix="$">
                <code>
                  {JSON.stringify(flashMutation.data.target_evidence, null, 2)}
                </code>
              </pre>
            </div>
          ) : null}
        </div>
      </section>

      <section className="card bg-base-100 border border-base-200 shadow-sm">
        <div className="card-body gap-4">
          <div className="flex items-center justify-between">
            <h3 className="card-title text-base">USB session</h3>
            <button
              type="button"
              className="btn btn-sm btn-outline"
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
                  <table className="table table-xs">
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
                  <table className="table table-xs">
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

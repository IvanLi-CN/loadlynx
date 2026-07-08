import { useTranslation } from "react-i18next";
import type { CalibrationProfile } from "../../api/types.ts";
import type { ValidationIssue } from "../../calibration/validation.ts";
import {
  describeCalibrationDeviceBadge,
  describeCalibrationDraftBadge,
  describeCalibrationDraftStatus,
  describeCalibrationPreviewBadge,
  getCalibrationImportIssuePreview,
} from "./calibration-overview.ts";
import { formatDeviceCalKind, formatLocalTimestamp } from "./shared.ts";

export function CalibrationOverviewPanel(input: {
  deviceProfile?: CalibrationProfile;
  deviceCalKind: number | null;
  draftEmpty: boolean;
  draftIssueCount: number;
  expectedCalKind: number;
  importError: string | null;
  importIssues: ValidationIssue[] | null;
  isOffline: boolean;
  onSyncMode: () => void;
  previewAppliedAt: number | null;
  previewMatchesDraft: boolean | null;
  previewProfile: CalibrationProfile | null;
  profileUpdatedAt: number;
  statusMatchesActiveTab: boolean;
}) {
  const { t } = useTranslation();
  const {
    deviceProfile,
    deviceCalKind,
    draftEmpty,
    draftIssueCount,
    expectedCalKind,
    importError,
    importIssues,
    isOffline,
    onSyncMode,
    previewAppliedAt,
    previewMatchesDraft,
    previewProfile,
    profileUpdatedAt,
    statusMatchesActiveTab,
  } = input;
  const deviceUsingDefaults =
    deviceProfile?.active.source === "factory-default";
  const draftStatus = describeCalibrationDraftStatus({
    activeSource: deviceProfile?.active.source,
    draftEmpty,
    deviceUsingDefaults,
    hasDeviceProfile: Boolean(deviceProfile),
  });
  const draftBadge = describeCalibrationDraftBadge({
    draftEmpty,
    draftIssueCount,
  });
  const deviceBadge = describeCalibrationDeviceBadge({
    deviceUsingDefaults,
    hasDeviceProfile: Boolean(deviceProfile),
  });
  const previewBadge = describeCalibrationPreviewBadge({
    hasPreviewProfile: previewProfile !== null,
    previewMatchesDraft,
  });
  const importIssuePreview = getCalibrationImportIssuePreview(importIssues);

  return (
    <div className="ll-panel bg-base-100 shadow-xl border border-base-200">
      <div className="ll-panel-body gap-3">
        <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div className="text-sm space-y-1">
            <div>
              <span className="font-bold">Device active:</span>{" "}
              {deviceProfile ? (
                <>
                  source=
                  <span className="font-mono">
                    {deviceProfile.active.source}
                  </span>
                  , fmt=
                  <span className="font-mono">
                    {deviceProfile.active.fmt_version}
                  </span>
                  , hw=
                  <span className="font-mono">
                    {deviceProfile.active.hw_rev}
                  </span>
                </>
              ) : (
                <span className="text-base-content/60">--</span>
              )}
            </div>
            <div>
              <span className="font-bold">Last read:</span>{" "}
              {profileUpdatedAt ? (
                <span className="font-mono">
                  {formatLocalTimestamp(profileUpdatedAt)}
                </span>
              ) : (
                <span className="text-base-content/60">--</span>
              )}
            </div>
            <div>
              <span className="font-bold">Status:</span>{" "}
              {draftStatus ? (
                draftStatus
              ) : (
                <span className="text-base-content/60">--</span>
              )}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <div
              className={`ll-badge ${statusMatchesActiveTab ? "ll-badge-success" : "ll-badge-warning"}`}
              title={`device=${formatDeviceCalKind(deviceCalKind)} expected=${formatDeviceCalKind(expectedCalKind)}`}
            >
              cal_mode: {formatDeviceCalKind(deviceCalKind)}
            </div>
            {deviceCalKind !== expectedCalKind && !isOffline ? (
              <button
                type="button"
                className="ll-button ll-button-xs ll-button-ghost"
                onClick={onSyncMode}
              >
                Sync
              </button>
            ) : null}

            {draftEmpty ? (
              <div className="ll-badge ll-badge-neutral">Draft: none</div>
            ) : (
              <div className="ll-badge ll-badge-warning">Draft: needs sync</div>
            )}

            <div className={`ll-badge ll-badge-${deviceBadge.tone}`}>
              {deviceBadge.label}
            </div>

            {draftBadge ? (
              <div className={`ll-badge ll-badge-${draftBadge.tone}`}>
                {draftBadge.label}
              </div>
            ) : null}

            <div className={`ll-badge ll-badge-${previewBadge.tone}`}>
              {previewBadge.label}
            </div>

            {previewAppliedAt ? (
              <div className="ll-badge ll-badge-ghost">
                Preview applied {formatLocalTimestamp(previewAppliedAt)}
              </div>
            ) : null}
          </div>
        </div>

        {importError ? (
          <div role="alert" className="ll-alert ll-alert-error text-sm py-2">
            <div className="flex flex-col gap-2">
              <div className="font-bold">{importError}</div>
              {importIssuePreview.length > 0 ? (
                <ul className="list-disc pl-5">
                  {importIssuePreview.map((issue) => (
                    <li key={`${issue.path}:${issue.message}`}>
                      <span className="font-mono">{issue.path}</span>:{" "}
                      {issue.message}
                    </li>
                  ))}
                </ul>
              ) : null}
            </div>
          </div>
        ) : null}

        {!isOffline && !statusMatchesActiveTab ? (
          <output className="ll-alert ll-alert-info text-sm py-2">
            <span>
              <span className="font-mono">
                {t("calibration.syncingMode", {
                  kind: formatDeviceCalKind(expectedCalKind),
                })}
              </span>
            </span>
          </output>
        ) : null}
      </div>
    </div>
  );
}

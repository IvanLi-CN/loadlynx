import type { QueryObserverResult } from "@tanstack/react-query";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import type { HttpApiError } from "../api/client.ts";
import {
  getCalibrationProfile,
  postCalibrationApply,
  postCalibrationCommit,
  postCalibrationMode,
  postCalibrationReset,
  subscribeStatusStream,
  updateCc,
} from "../api/client.ts";
import type {
  CalibrationPointVoltage,
  CalibrationProfile,
  FastStatusView,
} from "../api/types.ts";
import { piecewiseLinear } from "../calibration/piecewise.ts";
import { useDevicesQuery } from "../devices/hooks.ts";

type RefetchProfile = () => Promise<
  QueryObserverResult<CalibrationProfile, HttpApiError>
>;

interface VoltageCandidate {
  id: string;
  mv: number;
  rawLocal?: number;
  rawRemote?: number;
}

interface CurrentCandidate {
  id: string;
  raw: number;
  ma: number;
  dac_code: number;
}

function mergeVoltageCandidatesByMv(
  localPoints: CalibrationPointVoltage[],
  remotePoints: CalibrationPointVoltage[],
): Array<{ mv: number; rawLocal?: number; rawRemote?: number }> {
  const byMv = new Map<
    number,
    { mv: number; rawLocal?: number; rawRemote?: number }
  >();

  for (const point of localPoints) {
    const entry = byMv.get(point.mv) ?? { mv: point.mv };
    entry.rawLocal = point.raw;
    byMv.set(point.mv, entry);
  }

  for (const point of remotePoints) {
    const entry = byMv.get(point.mv) ?? { mv: point.mv };
    entry.rawRemote = point.raw;
    byMv.set(point.mv, entry);
  }

  return Array.from(byMv.values()).sort((a, b) => a.mv - b.mv);
}

export function DeviceCalibrationRoute() {
  const { deviceId } = useParams({
    from: "/$deviceId/calibration",
  }) as {
    deviceId: string;
  };

  const devicesQuery = useDevicesQuery();
  const device = useMemo(
    () => devicesQuery.data?.find((entry) => entry.id === deviceId),
    [devicesQuery.data, deviceId],
  );

  if (devicesQuery.isLoading) {
    return (
      <div className="max-w-5xl mx-auto p-8 text-center text-base-content/60">
        Loading device...
      </div>
    );
  }

  if (!device) {
    return (
      <div className="max-w-5xl mx-auto space-y-4">
        <h2 className="text-2xl font-bold">Calibration</h2>
        <div role="alert" className="alert alert-error text-sm py-2">
          <span>
            Device not found. Please add the device first in{" "}
            <code className="code">Devices</code>.
          </span>
        </div>
        <Link to="/devices" className="btn btn-sm btn-outline">
          Back to devices
        </Link>
      </div>
    );
  }

  return <DeviceCalibrationPage deviceId={deviceId} baseUrl={device.baseUrl} />;
}

function DeviceCalibrationPage({
  deviceId,
  baseUrl,
}: {
  deviceId: string;
  baseUrl: string;
}) {
  const [activeTab, setActiveTab] = useState<"voltage" | "current">("voltage");

  // Live status stream (includes optional RAW fields in calibration mode).
  const [status, setStatus] = useState<FastStatusView | null>(null);

  useEffect(() => {
    // Reset state while switching devices/URLs.
    setStatus(null);

    const unsubscribe = subscribeStatusStream(
      baseUrl,
      (view) => setStatus(view),
      () => setStatus(null),
    );

    return () => unsubscribe();
  }, [baseUrl]);

  const isOffline =
    status === null ||
    status.analog_state === "offline" ||
    status.analog_state === "faulted";

  const profileQuery = useQuery<CalibrationProfile, HttpApiError>({
    queryKey: ["device", deviceId, "calibration", "profile"],
    queryFn: () => getCalibrationProfile(baseUrl),
    enabled: Boolean(baseUrl),
  });

  // Always attempt to reset mode when leaving the page.
  useEffect(() => {
    return () => {
      postCalibrationMode(baseUrl, { kind: "off" }).catch(console.error);
    };
  }, [baseUrl]);

  // Switch mode when changing tabs. Current tab selection is refined by the
  // CurrentCalibration component (CH1/CH2) on mount.
  useEffect(() => {
    if (activeTab === "voltage") {
      postCalibrationMode(baseUrl, { kind: "voltage" }).catch(console.error);
    }
  }, [activeTab, baseUrl]);

  return (
    <div className="flex flex-col gap-6 max-w-5xl mx-auto">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Calibration</h2>
        <div className="badge badge-neutral gap-2">
          {isOffline ? "OFFLINE / FAULT" : "ONLINE"}
        </div>
      </div>

      <div role="tablist" className="tabs tabs-boxed">
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === "voltage" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("voltage")}
        >
          Voltage
        </button>
        <button
          type="button"
          role="tab"
          className={`tab ${activeTab === "current" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("current")}
        >
          Current
        </button>
      </div>

      {activeTab === "voltage" ? (
        <VoltageCalibration
          baseUrl={baseUrl}
          status={status}
          profile={profileQuery.data}
          onRefetchProfile={profileQuery.refetch}
          isOffline={isOffline}
        />
      ) : (
        <CurrentCalibration
          baseUrl={baseUrl}
          status={status}
          profile={profileQuery.data}
          onRefetchProfile={profileQuery.refetch}
          isOffline={isOffline}
        />
      )}
    </div>
  );
}

function VoltageCalibration({
  baseUrl,
  status,
  profile,
  onRefetchProfile,
  isOffline,
}: {
  baseUrl: string;
  status: FastStatusView | null;
  profile: CalibrationProfile | undefined;
  onRefetchProfile: RefetchProfile;
  isOffline: boolean;
}) {
  const nextCandidateId = useRef(0);

  const [candidates, setCandidates] = useState<VoltageCandidate[]>([]);

  useEffect(() => {
    nextCandidateId.current = 0;

    const localPoints = profile?.v_local_points ?? [];
    const remotePoints = profile?.v_remote_points ?? [];
    const merged = mergeVoltageCandidatesByMv(localPoints, remotePoints);
    setCandidates(
      merged.map((entry) => ({
        id: `v-${nextCandidateId.current++}`,
        mv: entry.mv,
        rawLocal: entry.rawLocal,
        rawRemote: entry.rawRemote,
      })),
    );
  }, [profile]);

  const [inputV, setInputV] = useState("12.00");

  const applyMutation = useMutation({
    mutationFn: async (payload: {
      kind: "v_local" | "v_remote";
      points: CalibrationPointVoltage[];
    }) => postCalibrationApply(baseUrl, payload),
  });

  const commitMutation = useMutation({
    mutationFn: async (payload: {
      kind: "v_local" | "v_remote";
      points: CalibrationPointVoltage[];
    }) => postCalibrationCommit(baseUrl, payload),
  });

  const resetMutation = useMutation({
    mutationFn: async () => {
      await postCalibrationReset(baseUrl, { kind: "v_local" });
      await new Promise((resolve) => setTimeout(resolve, 200));
      await postCalibrationReset(baseUrl, { kind: "v_remote" });
    },
    onSuccess: async () => {
      const result = await onRefetchProfile();
      if (result.data) {
        nextCandidateId.current = 0;
        const merged = mergeVoltageCandidatesByMv(
          result.data.v_local_points ?? [],
          result.data.v_remote_points ?? [],
        );
        setCandidates(
          merged.map((entry) => ({
            id: `v-${nextCandidateId.current++}`,
            mv: entry.mv,
            rawLocal: entry.rawLocal,
            rawRemote: entry.rawRemote,
          })),
        );
      }
    },
  });

  const handleCapture = () => {
    const rawLocal = status?.raw.raw_v_nr_100uv;
    const rawRemote = status?.raw.raw_v_rmt_100uv;

    if (rawLocal == null || rawRemote == null) {
      alert("Raw values not available. Ensure calibration mode is enabled.");
      return;
    }

    const measuredMv = Math.round(Number.parseFloat(inputV) * 1000);
    if (!Number.isFinite(measuredMv) || measuredMv <= 0) {
      alert("Invalid voltage input.");
      return;
    }

    if (candidates.length >= 5) {
      alert("Too many points (max 5).");
      return;
    }

    setCandidates((prev) => [
      ...prev,
      {
        id: `v-${nextCandidateId.current++}`,
        mv: measuredMv,
        rawLocal,
        rawRemote,
      },
    ]);
  };

  const handleDeleteCandidate = (id: string) => {
    if (candidates.length <= 1) {
      alert("At least 1 point is required.");
      return;
    }
    setCandidates((prev) => prev.filter((point) => point.id !== id));
  };

  const localPreviewPoints = candidates.flatMap((point) => {
    if (point.rawLocal == null) return [];
    return [{ x: point.rawLocal, y: point.mv }];
  });

  const remotePreviewPoints = candidates.flatMap((point) => {
    if (point.rawRemote == null) return [];
    return [{ x: point.rawRemote, y: point.mv }];
  });

  const previewLocalV =
    status?.raw.raw_v_nr_100uv != null && localPreviewPoints.length >= 2
      ? piecewiseLinear(localPreviewPoints, status.raw.raw_v_nr_100uv) / 1000
      : null;

  const previewRemoteV =
    status?.raw.raw_v_rmt_100uv != null && remotePreviewPoints.length >= 2
      ? piecewiseLinear(remotePreviewPoints, status.raw.raw_v_rmt_100uv) / 1000
      : null;

  const localApplyPoints: CalibrationPointVoltage[] | null = (() => {
    const out: CalibrationPointVoltage[] = [];
    for (const point of candidates) {
      if (point.rawLocal == null) return null;
      out.push({ raw: point.rawLocal, mv: point.mv });
    }
    return out;
  })();

  const remoteApplyPoints: CalibrationPointVoltage[] | null = (() => {
    const out: CalibrationPointVoltage[] = [];
    for (const point of candidates) {
      if (point.rawRemote == null) return null;
      out.push({ raw: point.rawRemote, mv: point.mv });
    }
    return out;
  })();

  const canApplyOrCommit =
    !isOffline &&
    candidates.length >= 1 &&
    candidates.length <= 5 &&
    localApplyPoints != null &&
    remoteApplyPoints != null;

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      <div className="card bg-base-100 shadow-xl border border-base-200 col-span-1 md:col-span-2">
        <div className="card-body">
          <h3 className="card-title">Capture Point</h3>
          <div className="flex items-end gap-4">
            <label className="form-control w-full max-w-xs">
              <div className="label">
                <span className="label-text">Measured Voltage (V)</span>
              </div>
              <input
                type="number"
                step="0.01"
                className="input input-bordered"
                value={inputV}
                onChange={(event) => setInputV(event.target.value)}
                disabled={isOffline}
              />
            </label>
            <button
              type="button"
              className="btn btn-primary"
              onClick={handleCapture}
              disabled={isOffline || candidates.length >= 5}
            >
              Capture Point
            </button>
          </div>

          <div className="divider"></div>

          <div className="flex justify-between items-center bg-base-200 rounded-box p-2">
            <button
              type="button"
              className="btn btn-sm btn-ghost text-error"
              onClick={() => resetMutation.mutate()}
              disabled={isOffline}
            >
              Reset All
            </button>
            <div className="flex gap-2">
              <button
                type="button"
                className="btn btn-sm btn-ghost"
                onClick={async () => {
                  if (
                    !canApplyOrCommit ||
                    !localApplyPoints ||
                    !remoteApplyPoints
                  ) {
                    alert(
                      "Cannot apply: ensure you have 1..5 paired points (local+remote raw).",
                    );
                    return;
                  }
                  await applyMutation.mutateAsync({
                    kind: "v_local",
                    points: localApplyPoints,
                  });
                  await new Promise((resolve) => setTimeout(resolve, 200));
                  await applyMutation.mutateAsync({
                    kind: "v_remote",
                    points: remoteApplyPoints,
                  });
                }}
                disabled={!canApplyOrCommit || applyMutation.isPending}
              >
                Apply Preview
              </button>
              <button
                type="button"
                className="btn btn-sm btn-secondary"
                onClick={async () => {
                  if (!canApplyOrCommit) {
                    alert(
                      "Cannot commit: ensure you have 1..5 paired points (local+remote raw).",
                    );
                    return;
                  }
                  if (!localApplyPoints || !remoteApplyPoints) {
                    alert(
                      "Cannot commit: ensure you have 1..5 paired points (local+remote raw).",
                    );
                    return;
                  }
                  await commitMutation.mutateAsync({
                    kind: "v_local",
                    points: localApplyPoints,
                  });
                  await new Promise((resolve) => setTimeout(resolve, 200));
                  await commitMutation.mutateAsync({
                    kind: "v_remote",
                    points: remoteApplyPoints,
                  });
                }}
                disabled={!canApplyOrCommit || commitMutation.isPending}
              >
                Commit
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="stats shadow">
        <div className="stat">
          <div className="stat-title">Local Voltage (Active)</div>
          <div className="stat-value text-lg">
            {((status?.raw.v_local_mv ?? 0) / 1000).toFixed(3)} V
          </div>
          <div className="stat-desc">
            Raw: {status?.raw.raw_v_nr_100uv ?? "--"}
          </div>
        </div>
        <div className="stat">
          <div className="stat-title">Local Preview</div>
          <div className="stat-value text-lg text-primary">
            {previewLocalV == null ? "--" : `${previewLocalV.toFixed(3)} V`}
          </div>
          <div className="stat-desc">Candidate points: {candidates.length}</div>
        </div>
      </div>

      <div className="stats shadow">
        <div className="stat">
          <div className="stat-title">Remote Voltage (Active)</div>
          <div className="stat-value text-lg">
            {((status?.raw.v_remote_mv ?? 0) / 1000).toFixed(3)} V
          </div>
          <div className="stat-desc">
            Raw: {status?.raw.raw_v_rmt_100uv ?? "--"}
          </div>
        </div>
        <div className="stat">
          <div className="stat-title">Remote Preview</div>
          <div className="stat-value text-lg text-primary">
            {previewRemoteV == null ? "--" : `${previewRemoteV.toFixed(3)} V`}
          </div>
          <div className="stat-desc">Candidate points: {candidates.length}</div>
        </div>
      </div>

      <div className="card bg-base-100 shadow-xl border border-base-200 col-span-1 md:col-span-2">
        <div className="card-body">
          <h4 className="font-bold">Candidates</h4>
          <div className="overflow-x-auto max-h-64">
            <table className="table table-xs table-pin-rows">
              <thead>
                <tr>
                  <th>Value (mV)</th>
                  <th>Raw Local</th>
                  <th>Raw Remote</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {candidates.map((point) => (
                  <tr key={point.id}>
                    <td>{point.mv}</td>
                    <td>{point.rawLocal ?? "--"}</td>
                    <td>{point.rawRemote ?? "--"}</td>
                    <td className="text-right">
                      <button
                        type="button"
                        className="btn btn-ghost btn-xs text-error"
                        onClick={() => handleDeleteCandidate(point.id)}
                        disabled={isOffline || candidates.length <= 1}
                        aria-label={`Delete candidate ${point.id}`}
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}

function CurrentCalibration({
  baseUrl,
  status,
  profile,
  onRefetchProfile,
  isOffline,
}: {
  baseUrl: string;
  status: FastStatusView | null;
  profile: CalibrationProfile | undefined;
  onRefetchProfile: RefetchProfile;
  isOffline: boolean;
}) {
  const nextCandidateId = useRef(0);

  const [channel, setChannel] = useState<"ch1" | "ch2">("ch1");

  useEffect(() => {
    const kind = channel === "ch1" ? "current_ch1" : "current_ch2";
    postCalibrationMode(baseUrl, { kind }).catch(console.error);
  }, [baseUrl, channel]);

  const [candidates, setCandidates] = useState<CurrentCandidate[]>([]);

  useEffect(() => {
    nextCandidateId.current = 0;

    if (!profile) {
      setCandidates([]);
      return;
    }
    const points =
      channel === "ch1"
        ? profile.current_ch1_points
        : profile.current_ch2_points;
    setCandidates(
      points.map((point) => ({
        id: `c-${nextCandidateId.current++}`,
        raw: point.raw,
        ma: point.ma,
        dac_code: point.dac_code,
      })),
    );
  }, [profile, channel]);

  const [meterReadingA, setMeterReadingA] = useState("1.000");
  const [targetIMa, setTargetIMa] = useState("1000");

  const applyMutation = useMutation({
    mutationFn: async () => {
      const kind = channel === "ch1" ? "current_ch1" : "current_ch2";
      return postCalibrationApply(baseUrl, {
        kind,
        points: candidates.map((point) => ({
          raw: point.raw,
          ma: point.ma,
          dac_code: point.dac_code,
        })),
      });
    },
  });

  const commitMutation = useMutation({
    mutationFn: async () => {
      const kind = channel === "ch1" ? "current_ch1" : "current_ch2";
      return postCalibrationCommit(baseUrl, {
        kind,
        points: candidates.map((point) => ({
          raw: point.raw,
          ma: point.ma,
          dac_code: point.dac_code,
        })),
      });
    },
  });

  const resetMutation = useMutation({
    mutationFn: async () => {
      const kind = channel === "ch1" ? "current_ch1" : "current_ch2";
      return postCalibrationReset(baseUrl, { kind });
    },
    onSuccess: async () => {
      const result = await onRefetchProfile();
      if (!result.data) {
        return;
      }
      nextCandidateId.current = 0;
      const points =
        channel === "ch1"
          ? result.data.current_ch1_points
          : result.data.current_ch2_points;
      setCandidates(
        points.map((point) => ({
          id: `c-${nextCandidateId.current++}`,
          raw: point.raw,
          ma: point.ma,
          dac_code: point.dac_code,
        })),
      );
    },
  });

  const handleSetOutput = () => {
    const parsed = Number.parseInt(targetIMa, 10);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      alert("Invalid target current.");
      return;
    }
    updateCc(baseUrl, { enable: true, target_i_ma: parsed }).catch(
      console.error,
    );
  };

  const handleCapture = () => {
    const rawCur = status?.raw.raw_cur_100uv;
    const rawDac = status?.raw.raw_dac_code;

    if (rawCur == null || rawDac == null) {
      alert("Raw values not available. Ensure calibration mode is enabled.");
      return;
    }

    const measuredMa = Math.round(Number.parseFloat(meterReadingA) * 1000);
    if (!Number.isFinite(measuredMa) || measuredMa <= 0) {
      alert("Invalid current input.");
      return;
    }

    if (candidates.length >= 5) {
      alert("Too many points (max 5).");
      return;
    }

    setCandidates((prev) => [
      ...prev,
      {
        id: `c-${nextCandidateId.current++}`,
        ma: measuredMa,
        raw: rawCur,
        dac_code: rawDac,
      },
    ]);
  };

  const handleDeleteCandidate = (id: string) => {
    if (candidates.length <= 1) {
      alert("At least 1 point is required.");
      return;
    }
    setCandidates((prev) => prev.filter((point) => point.id !== id));
  };

  const activeMa =
    channel === "ch1" ? status?.raw.i_local_ma : status?.raw.i_remote_ma;
  const previewMa =
    status?.raw.raw_cur_100uv != null && candidates.length >= 2
      ? piecewiseLinear(
          candidates.map((point) => ({ x: point.raw, y: point.ma })),
          status.raw.raw_cur_100uv,
        )
      : null;

  const canApplyOrCommit =
    !isOffline && candidates.length >= 1 && candidates.length <= 5;

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      <div className="col-span-1 md:col-span-2 flex justify-center">
        <div className="join">
          <input
            className="join-item btn"
            type="radio"
            name="calibration-current-channel"
            aria-label="Channel 1 (Low Range)"
            checked={channel === "ch1"}
            onChange={() => setChannel("ch1")}
          />
          <input
            className="join-item btn"
            type="radio"
            name="calibration-current-channel"
            aria-label="Channel 2 (High Range)"
            checked={channel === "ch2"}
            onChange={() => setChannel("ch2")}
          />
        </div>
      </div>

      <div className="card bg-base-100 shadow-xl border border-base-200">
        <div className="card-body">
          <h3 className="card-title">Set Target</h3>
          <div className="flex gap-2 flex-wrap">
            <button
              type="button"
              className="btn btn-xs"
              onClick={() => setTargetIMa("500")}
            >
              0.5A
            </button>
            <button
              type="button"
              className="btn btn-xs"
              onClick={() => setTargetIMa("1000")}
            >
              1A
            </button>
            <button
              type="button"
              className="btn btn-xs"
              onClick={() => setTargetIMa("3000")}
            >
              3A
            </button>
            <input
              type="number"
              className="input input-sm input-bordered w-24"
              value={targetIMa}
              onChange={(event) => setTargetIMa(event.target.value)}
              disabled={isOffline}
            />
          </div>
          <button
            type="button"
            className="btn btn-primary mt-4"
            disabled={isOffline}
            onClick={handleSetOutput}
          >
            Set Output
          </button>
        </div>
      </div>

      <div className="card bg-base-100 shadow-xl border border-base-200">
        <div className="card-body">
          <h3 className="card-title">Capture Point</h3>
          <label className="form-control w-full">
            <div className="label">
              <span className="label-text">
                Meter Reading ({channel === "ch1" ? "Local" : "Remote"}) (A)
              </span>
            </div>
            <div className="join">
              <input
                type="number"
                step="0.001"
                className="input input-bordered join-item w-full"
                value={meterReadingA}
                onChange={(event) => setMeterReadingA(event.target.value)}
                disabled={isOffline}
              />
              <button
                type="button"
                className="btn btn-secondary join-item"
                onClick={handleCapture}
                disabled={isOffline || candidates.length >= 5}
              >
                Capture
              </button>
            </div>
          </label>
        </div>
      </div>

      <div className="stats shadow">
        <div className="stat">
          <div className="stat-title">Active Current</div>
          <div className="stat-value text-lg">
            {(((activeMa ?? 0) / 1000) as number).toFixed(4)} A
          </div>
          <div className="stat-desc">
            Raw: {status?.raw.raw_cur_100uv ?? "--"}
          </div>
        </div>
        <div className="stat">
          <div className="stat-title">DAC Code</div>
          <div className="stat-value text-lg font-mono">
            {status?.raw.raw_dac_code ?? "--"}
          </div>
        </div>
      </div>

      <div className="stats shadow">
        <div className="stat">
          <div className="stat-title">Preview Current</div>
          <div className="stat-value text-lg text-primary">
            {previewMa == null ? "--" : `${(previewMa / 1000).toFixed(4)} A`}
          </div>
          <div className="stat-desc">{candidates.length} points</div>
        </div>
      </div>

      <div className="card bg-base-100 shadow-xl border border-base-200 col-span-1 md:col-span-2">
        <div className="card-body">
          <div className="flex justify-between items-center mb-4">
            <h4 className="font-bold">Candidates ({channel})</h4>
            <div className="flex gap-2">
              <button
                type="button"
                className="btn btn-sm btn-ghost text-error"
                onClick={() => resetMutation.mutate()}
                disabled={isOffline || resetMutation.isPending}
              >
                Reset
              </button>
              <button
                type="button"
                className="btn btn-sm btn-ghost"
                onClick={() => applyMutation.mutate()}
                disabled={!canApplyOrCommit || applyMutation.isPending}
              >
                Apply
              </button>
              <button
                type="button"
                className="btn btn-sm btn-secondary"
                onClick={() => commitMutation.mutate()}
                disabled={!canApplyOrCommit || commitMutation.isPending}
              >
                Commit
              </button>
            </div>
          </div>

          <div className="overflow-x-auto max-h-48">
            <table className="table table-xs table-pin-rows">
              <thead>
                <tr>
                  <th>Raw</th>
                  <th>DAC</th>
                  <th>Value (mA)</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {candidates.map((point) => (
                  <tr key={point.id}>
                    <td>{point.raw}</td>
                    <td>{point.dac_code ?? "--"}</td>
                    <td>{point.ma}</td>
                    <td className="text-right">
                      <button
                        type="button"
                        className="btn btn-ghost btn-xs text-error"
                        onClick={() => handleDeleteCandidate(point.id)}
                        disabled={isOffline || candidates.length <= 1}
                        aria-label={`Delete candidate ${point.id}`}
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}

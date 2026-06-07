import { httpJsonQueued, isMockBaseUrl } from "./client-core.ts";
import {
  mockGetCalibrationProfile,
  mockPostCalibrationApply,
  mockPostCalibrationCommit,
  mockPostCalibrationMode,
  mockPostCalibrationReset,
} from "./client-mock.ts";
import type {
  CalibrationApplyRequest,
  CalibrationCommitRequest,
  CalibrationModeRequest,
  CalibrationPointCurrentWireCompact,
  CalibrationPointVoltageWireCompact,
  CalibrationProfile,
  CalibrationProfileWire,
  CalibrationResetRequest,
  CalibrationWriteRequestWire,
} from "./types.ts";

export function mapCalibrationProfileWireToUi(
  profile: CalibrationProfileWire,
): CalibrationProfile {
  return {
    active: profile.active,
    v_local_points: profile.v_local_points.map((point) => ({
      raw: point.raw_100uv,
      mv: point.meas_mv,
    })),
    v_remote_points: profile.v_remote_points.map((point) => ({
      raw: point.raw_100uv,
      mv: point.meas_mv,
    })),
    current_ch1_points: profile.current_ch1_points.map((point) => ({
      raw: point.raw_100uv,
      ua: point.meas_ma * 1000,
      dac_code: point.raw_dac_code,
    })),
    current_ch2_points: profile.current_ch2_points.map((point) => ({
      raw: point.raw_100uv,
      ua: point.meas_ma * 1000,
      dac_code: point.raw_dac_code,
    })),
  };
}

export function mapCalibrationProfileUiToWire(
  profile: CalibrationProfile,
): CalibrationProfileWire {
  return {
    active: profile.active,
    v_local_points: profile.v_local_points.map((point) => ({
      raw_100uv: point.raw,
      meas_mv: point.mv,
    })),
    v_remote_points: profile.v_remote_points.map((point) => ({
      raw_100uv: point.raw,
      meas_mv: point.mv,
    })),
    current_ch1_points: profile.current_ch1_points.map((point) => ({
      raw_100uv: point.raw,
      raw_dac_code: point.dac_code,
      meas_ma: Math.floor((point.ua + 500) / 1000),
    })),
    current_ch2_points: profile.current_ch2_points.map((point) => ({
      raw_100uv: point.raw,
      raw_dac_code: point.dac_code,
      meas_ma: Math.floor((point.ua + 500) / 1000),
    })),
  };
}

export function mapCalibrationWriteRequestToWire(
  payload: CalibrationApplyRequest,
): CalibrationWriteRequestWire {
  switch (payload.kind) {
    case "v_local":
    case "v_remote":
      return {
        kind: payload.kind,
        points: payload.points.map(
          (point): CalibrationPointVoltageWireCompact => [point.raw, point.mv],
        ),
      };
    case "current_ch1":
    case "current_ch2":
      return {
        kind: payload.kind,
        points: payload.points.map(
          (point): CalibrationPointCurrentWireCompact => [
            point.raw,
            point.dac_code,
            Math.floor((point.ua + 500) / 1000),
          ],
        ),
      };
  }
}

export async function getCalibrationProfile(
  baseUrl: string,
): Promise<CalibrationProfile> {
  if (isMockBaseUrl(baseUrl)) {
    return mockGetCalibrationProfile(baseUrl);
  }
  const payload = await httpJsonQueued<CalibrationProfileWire>(
    baseUrl,
    "/api/v1/calibration/profile",
  );
  return mapCalibrationProfileWireToUi(payload);
}

export async function postCalibrationApply(
  baseUrl: string,
  payload: CalibrationApplyRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationApply(baseUrl, payload);
  }
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/apply", {
    method: "POST",
    body: JSON.stringify(mapCalibrationWriteRequestToWire(payload)),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function postCalibrationCommit(
  baseUrl: string,
  payload: CalibrationCommitRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationCommit(baseUrl, payload);
  }
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/commit", {
    method: "POST",
    body: JSON.stringify(mapCalibrationWriteRequestToWire(payload)),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function postCalibrationReset(
  baseUrl: string,
  payload: CalibrationResetRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationReset(baseUrl, payload);
  }
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/reset", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

export async function postCalibrationMode(
  baseUrl: string,
  payload: CalibrationModeRequest,
): Promise<void> {
  if (isMockBaseUrl(baseUrl)) {
    return mockPostCalibrationMode(baseUrl, payload);
  }
  return httpJsonQueued<void>(baseUrl, "/api/v1/calibration/mode", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "text/plain",
    },
  });
}

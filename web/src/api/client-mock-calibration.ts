import { CALIBRATION_MAX_POINTS } from "../calibration/validation.ts";
import { HttpApiError } from "./client-core.ts";
import { getOrCreateMockDevice } from "./client-mock-state.ts";
import type {
  CalibrationApplyRequest,
  CalibrationCommitRequest,
  CalibrationModeRequest,
  CalibrationPointCurrentWireCompact,
  CalibrationPointVoltageWireCompact,
  CalibrationProfile,
  CalibrationProfileWire,
  CalibrationResetRequest,
} from "./types.ts";

function mapCalibrationProfileWireToUi(
  profile: CalibrationProfileWire,
): CalibrationProfile {
  return {
    active: profile.active,
    persistence: profile.persistence,
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

function mapCalibrationWriteRequestToWire(payload: CalibrationApplyRequest): {
  kind: CalibrationApplyRequest["kind"];
  points:
    | CalibrationPointVoltageWireCompact[]
    | CalibrationPointCurrentWireCompact[];
} {
  switch (payload.kind) {
    case "v_local":
    case "v_remote":
      return {
        kind: payload.kind,
        points: payload.points.map((point) => [point.raw, point.mv]),
      };
    case "current_ch1":
    case "current_ch2":
      return {
        kind: payload.kind,
        points: payload.points.map((point) => [
          point.raw,
          point.dac_code,
          Math.floor((point.ua + 500) / 1000),
        ]),
      };
  }
}

function mockCalValidationError(message: string): never {
  throw new HttpApiError({
    status: 400,
    code: "INVALID_REQUEST",
    message,
    retryable: false,
    details: null,
  });
}

function mockNormalizeWirePointsByRaw100uv<T extends { raw_100uv: number }>(
  kind: string,
  points: T[],
  measKey: string,
  getMeas: (point: T) => number,
): T[] {
  for (const point of points) {
    const raw = point.raw_100uv;
    if (!Number.isFinite(raw) || !Number.isInteger(raw)) {
      mockCalValidationError("raw_100uv must be an integer");
    }
    if (raw < -32768 || raw > 32767) {
      mockCalValidationError("raw_100uv out of range for i16");
    }

    const meas = getMeas(point);
    if (!Number.isFinite(meas) || !Number.isInteger(meas)) {
      mockCalValidationError(`${measKey} must be an integer`);
    }
  }

  const measDeduped: T[] = [];
  for (const point of points) {
    const meas = getMeas(point);
    const idx = measDeduped.findIndex((entry) => getMeas(entry) === meas);
    if (idx < 0) {
      measDeduped.push(point);
    } else {
      measDeduped[idx] = point;
    }
  }

  const sorted = measDeduped.slice();
  for (let index = 1; index < sorted.length; index++) {
    let pos = index;
    while (pos > 0 && sorted[pos - 1].raw_100uv > sorted[pos].raw_100uv) {
      const tmp = sorted[pos - 1];
      sorted[pos - 1] = sorted[pos];
      sorted[pos] = tmp;
      pos -= 1;
    }
  }

  const deduped: T[] = [];
  for (const point of sorted) {
    const last = deduped[deduped.length - 1];
    if (last && last.raw_100uv === point.raw_100uv) {
      deduped[deduped.length - 1] = point;
    } else {
      deduped.push(point);
    }
  }

  for (let index = 1; index < deduped.length; index++) {
    if (getMeas(deduped[index]) <= getMeas(deduped[index - 1])) {
      mockCalValidationError(`meas must be strictly increasing for ${kind}`);
    }
  }

  if (deduped.length === 0) {
    mockCalValidationError(
      `points must contain 1..${CALIBRATION_MAX_POINTS} items`,
    );
  }
  if (deduped.length > CALIBRATION_MAX_POINTS) {
    mockCalValidationError(`too many points (max ${CALIBRATION_MAX_POINTS})`);
  }

  return deduped;
}

function mockNormalizeVoltageWirePoints(
  kind: "v_local" | "v_remote",
  points: CalibrationPointVoltageWireCompact[],
): CalibrationProfileWire["v_local_points"] {
  const normalized = points.map((point) => ({
    raw_100uv: point[0],
    meas_mv: point[1],
  }));
  return mockNormalizeWirePointsByRaw100uv(
    kind,
    normalized,
    "meas_mv",
    (point) => point.meas_mv,
  );
}

function mockNormalizeCurrentWirePoints(
  kind: "current_ch1" | "current_ch2",
  points: CalibrationPointCurrentWireCompact[],
): CalibrationProfileWire["current_ch1_points"] {
  const normalized = points.map((point) => ({
    raw_100uv: point[0],
    raw_dac_code: point[1],
    meas_ma: point[2],
  }));

  for (const point of normalized) {
    const dac = point.raw_dac_code;
    if (!Number.isFinite(dac) || !Number.isInteger(dac)) {
      mockCalValidationError("raw_dac_code must be an integer");
    }
    if (dac < 0 || dac > 65535) {
      mockCalValidationError("raw_dac_code out of range for u16");
    }
  }

  return mockNormalizeWirePointsByRaw100uv(
    kind,
    normalized,
    "meas_ma",
    (point) => point.meas_ma,
  );
}

function mockProfileWireEqualsFactory(
  profile: CalibrationProfileWire,
  factory: CalibrationProfileWire,
): boolean {
  if (
    profile.active.fmt_version !== factory.active.fmt_version ||
    profile.active.hw_rev !== factory.active.hw_rev
  ) {
    return false;
  }

  const eqVoltage = (
    a: { raw_100uv: number; meas_mv: number }[],
    b: { raw_100uv: number; meas_mv: number }[],
  ) => {
    if (a.length !== b.length) {
      return false;
    }
    for (let index = 0; index < a.length; index++) {
      if (
        a[index].raw_100uv !== b[index].raw_100uv ||
        a[index].meas_mv !== b[index].meas_mv
      ) {
        return false;
      }
    }
    return true;
  };

  const eqCurrent = (
    a: { raw_100uv: number; raw_dac_code: number; meas_ma: number }[],
    b: { raw_100uv: number; raw_dac_code: number; meas_ma: number }[],
  ) => {
    if (a.length !== b.length) {
      return false;
    }
    for (let index = 0; index < a.length; index++) {
      if (
        a[index].raw_100uv !== b[index].raw_100uv ||
        a[index].raw_dac_code !== b[index].raw_dac_code ||
        a[index].meas_ma !== b[index].meas_ma
      ) {
        return false;
      }
    }
    return true;
  };

  return (
    eqCurrent(profile.current_ch1_points, factory.current_ch1_points) &&
    eqCurrent(profile.current_ch2_points, factory.current_ch2_points) &&
    eqVoltage(profile.v_local_points, factory.v_local_points) &&
    eqVoltage(profile.v_remote_points, factory.v_remote_points)
  );
}

export async function mockGetCalibrationProfile(
  baseUrl: string,
): Promise<CalibrationProfile> {
  const state = getOrCreateMockDevice(baseUrl);
  return mapCalibrationProfileWireToUi(structuredClone(state.calibration.ram));
}

export async function mockPostCalibrationApply(
  baseUrl: string,
  payload: CalibrationApplyRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  const wire = mapCalibrationWriteRequestToWire(payload);
  const ram = state.calibration.ram;
  ram.active.source = "user-calibrated";
  ram.persistence = { status: "ram-only" };

  switch (wire.kind) {
    case "v_local":
      ram.v_local_points = mockNormalizeVoltageWirePoints(
        wire.kind,
        wire.points as CalibrationPointVoltageWireCompact[],
      );
      break;
    case "v_remote":
      ram.v_remote_points = mockNormalizeVoltageWirePoints(
        wire.kind,
        wire.points as CalibrationPointVoltageWireCompact[],
      );
      break;
    case "current_ch1":
      ram.current_ch1_points = mockNormalizeCurrentWirePoints(
        wire.kind,
        wire.points as CalibrationPointCurrentWireCompact[],
      );
      break;
    case "current_ch2":
      ram.current_ch2_points = mockNormalizeCurrentWirePoints(
        wire.kind,
        wire.points as CalibrationPointCurrentWireCompact[],
      );
      break;
  }
}

export async function mockPostCalibrationCommit(
  baseUrl: string,
  payload: CalibrationCommitRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  await mockPostCalibrationApply(baseUrl, payload);
  state.calibration.ram.persistence = { status: "commit-verified" };
  state.calibration.eeprom = structuredClone(state.calibration.ram);
}

export async function mockPostCalibrationReset(
  baseUrl: string,
  payload: CalibrationResetRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  const { kind } = payload;

  if (kind === "all") {
    state.calibration.ram = structuredClone(state.calibration.factory);
    state.calibration.ram.persistence = { status: "factory-default" };
    state.calibration.eeprom = null;
    return;
  }

  const factory = state.calibration.factory;
  const ram = state.calibration.ram;

  switch (kind) {
    case "v_local":
      ram.v_local_points = structuredClone(factory.v_local_points);
      break;
    case "v_remote":
      ram.v_remote_points = structuredClone(factory.v_remote_points);
      break;
    case "current_ch1":
      ram.current_ch1_points = structuredClone(factory.current_ch1_points);
      break;
    case "current_ch2":
      ram.current_ch2_points = structuredClone(factory.current_ch2_points);
      break;
  }

  if (mockProfileWireEqualsFactory(ram, factory)) {
    state.calibration.ram = structuredClone(factory);
    state.calibration.ram.persistence = { status: "factory-default" };
    state.calibration.eeprom = null;
  } else {
    ram.active.source = "user-calibrated";
    ram.persistence = { status: "commit-verified" };
    state.calibration.eeprom = structuredClone(ram);
  }
}

export async function mockPostCalibrationMode(
  baseUrl: string,
  payload: CalibrationModeRequest,
): Promise<void> {
  const state = getOrCreateMockDevice(baseUrl);
  state.calibrationMode = payload.kind;
}

import { expect, test } from "vitest";
import { exportDiagnostics } from "./client.ts";
import {
  type DevdStatusPayload,
  getOrCreateMockDevice,
  mockGetStatus,
  mockUpdateCc,
  mockUpdateControl,
  normalizeDevdIdentity,
  normalizeDevdStatus,
} from "./client-mock.ts";

test("normalizeDevdIdentity preserves firmware and usb bridge metadata", () => {
  const identity = normalizeDevdIdentity("http://127.0.0.1:30180", {
    device_id: "llx-d68638",
    digital_fw_version: "digital test build",
    analog_fw_version: "analog test build",
    protocol_version: 1,
    uptime_ms: 123,
    network: {
      ip: "192.168.1.23",
      mac: "aa:bb:cc:dd:ee:ff",
      hostname: "loadlynx-d68638.local",
    },
    hostname: "loadlynx-d68638.local",
    short_id: "d68638",
    firmware: {
      target: "digital_esp32s3",
      package_version: "0.1.0",
      build_id: "digital test build",
      build_profile: "release",
      target_triple: "xtensa-esp32s3-none-elf",
      source_digest: "src 0x1234",
      features: ["net_http", "usb_cdc_jsonl"],
      protocol: "loadlynx.cdc.v1",
      defmt: {
        enabled: true,
        encoding: "defmt-espflash",
      },
    },
    usb_bridge: {
      transport: "usb_cdc_jsonl",
      protocol: "loadlynx.cdc.v1",
      lease_required: true,
      framing: "lf_json",
    },
    capabilities: {
      cc_supported: true,
      cv_supported: true,
      cp_supported: true,
      presets_supported: true,
      preset_count: 5,
      api_version: "2.0.0",
    },
  });

  expect(identity.hostname).toBe("loadlynx-d68638.local");
  expect(identity.short_id).toBe("d68638");
  expect(identity.firmware?.target).toBe("digital_esp32s3");
  expect(identity.firmware?.protocol).toBe("loadlynx.cdc.v1");
  expect(identity.usb_bridge?.transport).toBe("usb_cdc_jsonl");
  expect(identity.usb_bridge?.lease_required).toBe(true);
});

test("normalizeDevdIdentity supplies default usb bridge metadata for devd payloads", () => {
  const identity = normalizeDevdIdentity("http://127.0.0.1:30180", {
    device_id: "llx-devd-usb",
    uptime_ms: 1,
    network: {
      ip: "127.0.0.1",
      mac: "unknown",
      hostname: "loadlynx-devd-usb",
    },
    capabilities: {
      cc_supported: true,
      cv_supported: true,
      cp_supported: true,
      presets_supported: true,
      preset_count: 5,
      api_version: "devd-usb",
    },
  });

  expect(identity.usb_bridge).toEqual({
    transport: "usb_cdc_jsonl",
    protocol: "loadlynx.cdc.v1",
    lease_required: true,
    framing: "lf_json",
  });
});

test("normalizeDevdStatus preserves decoded flags and measurement_invalid analog state", () => {
  const view = normalizeDevdStatus({
    status: {
      mode: 2,
      state_flags: 0b11_1111,
      enable: true,
      target_value: 1234,
      i_local_ma: 1200,
      i_remote_ma: 34,
      v_local_mv: 5000,
      v_remote_mv: 4950,
      calc_p_mw: 6100,
      dac_headroom_mv: 25,
      loop_error: 7,
      sink_core_temp_mc: 41000,
      sink_exhaust_temp_mc: 39000,
      mcu_temp_mc: 36000,
      fault_flags: 0b1010,
    },
    uptime_ms: 42,
    link_up: false,
    hello_seen: false,
    analog_state: "measurement_invalid",
    fault_flags_decoded: ["OVERVOLTAGE", "SINK_OVER_TEMP"],
    state_flags_decoded: [
      "REMOTE_ACTIVE",
      "LINK_GOOD",
      "ENABLED",
      "UV_LATCHED",
      "POWER_LIMITED",
      "CURRENT_LIMITED",
    ],
  });

  expect(view.raw.mode).toBe(2);
  expect(view.link_up).toBe(false);
  expect(view.hello_seen).toBe(false);
  expect(view.analog_state).toBe("measurement_invalid");
  expect(view.fault_flags_decoded).toEqual(["OVERVOLTAGE", "SINK_OVER_TEMP"]);
  expect(view.state_flags_decoded).toEqual([
    "REMOTE_ACTIVE",
    "LINK_GOOD",
    "ENABLED",
    "UV_LATCHED",
    "POWER_LIMITED",
    "CURRENT_LIMITED",
  ]);
});

test("normalizeDevdStatus reuses the last good sample for omitted compat fields", () => {
  const previous = normalizeDevdStatus({
    status: {
      mode: 1,
      state_flags: 0b11,
      enable: true,
      target_value: 1500,
      i_local_ma: 1200,
      i_remote_ma: 55,
      v_local_mv: 5020,
      v_remote_mv: 4980,
      calc_p_mw: 6200,
      dac_headroom_mv: 22,
      loop_error: 4,
      fault_flags: 0,
    },
    uptime_ms: 1_000,
    sink_core_temp_mc: 41_000,
    sink_exhaust_temp_mc: 39_000,
    mcu_temp_mc: 36_000,
    link_up: true,
    hello_seen: true,
    analog_state: "ready",
    fault_flags_decoded: [],
    state_flags_decoded: ["REMOTE_ACTIVE", "LINK_GOOD"],
  });

  const view = normalizeDevdStatus(
    {
      status: {
        i_local_ma: 0,
        i_remote_ma: 0,
        v_local_mv: 49,
        v_remote_mv: 12,
        calc_p_mw: 0,
        enable: false,
        fault_flags: 0,
        state_flags: 2,
      },
      uptime_ms: 2_000,
      link_up: true,
      hello_seen: true,
      analog_state: "ready",
      fault_flags_decoded: [],
      state_flags_decoded: ["LINK_GOOD"],
    },
    previous,
  );

  expect(view.raw.uptime_ms).toBe(2_000);
  expect(view.raw.sink_core_temp_mc).toBe(41_000);
  expect(view.raw.sink_exhaust_temp_mc).toBe(39_000);
  expect(view.raw.mcu_temp_mc).toBe(36_000);
  expect(view.raw.mode).toBe(1);
  expect(view.raw.target_value).toBe(1_500);
  expect(view.raw.v_local_mv).toBe(49);
  expect(view.raw.v_remote_mv).toBe(12);
  expect(view.raw.enable).toBe(false);
  expect(view.state_flags_decoded).toEqual(["LINK_GOOD"]);
});

test("normalizeDevdStatus leaves missing thermal fields empty when no sample exists yet", () => {
  const view = normalizeDevdStatus({
    status: {
      i_local_ma: 0,
      i_remote_ma: 0,
      v_local_mv: 49,
      v_remote_mv: 12,
      calc_p_mw: 0,
      enable: false,
      fault_flags: 0,
      state_flags: 2,
    },
    uptime_ms: 2_000,
    link_up: true,
    hello_seen: true,
    analog_state: "ready",
    fault_flags_decoded: [],
    state_flags_decoded: ["LINK_GOOD"],
  });

  expect(view.raw.uptime_ms).toBe(2_000);
  expect(view.raw.sink_core_temp_mc).toBeUndefined();
  expect(view.raw.sink_exhaust_temp_mc).toBeUndefined();
  expect(view.raw.mcu_temp_mc).toBeUndefined();
});

test("normalizeDevdStatus derives CP mode when only control.mode is present", () => {
  const payload: DevdStatusPayload = {
    status: {},
    control: {
      mode: "cp",
      output_enabled: true,
      target_p_mw: 60000,
    },
  };
  const view = normalizeDevdStatus(payload);

  expect(view.raw.mode).toBe(3);
  expect(view.raw.enable).toBe(true);
  expect(view.raw.target_value).toBe(60000);
  expect(view.state_flags_decoded).toEqual([]);
});

test("normalizeDevdStatus does not default unknown mode payloads to CP", () => {
  const payload: DevdStatusPayload = {
    status: {},
    control: {
      output_enabled: true,
      target_p_mw: 60000,
    },
  };
  const view = normalizeDevdStatus(payload);

  expect(view.raw.mode).toBe(1);
  expect(view.raw.enable).toBe(true);
  expect(view.raw.target_value).toBe(0);
  expect(view.state_flags_decoded).toEqual([]);
});

test("mock device status starts from a valid protocol mode and decoded flags array", () => {
  const device = getOrCreateMockDevice("mock://contract-status");

  expect(device.status.raw.mode).toBe(1);
  expect(device.status.analog_state).toBe("ready");
  expect(device.status.state_flags_decoded).toEqual([]);
  expect(device.identity.firmware?.protocol).toBe("loadlynx.cdc.v1");
  expect(device.identity.usb_bridge?.transport).toBe("usb_cdc_jsonl");
  expect(device.pd?.epr_active).toBe(false);
  expect(device.pd?.epr_avs_pdos).toEqual([]);
});

test("demo mock devices boot into distinct live bench scenarios", async () => {
  const ccDevice = getOrCreateMockDevice("mock://demo-1");
  const cpDevice = getOrCreateMockDevice("mock://demo-2");

  expect(ccDevice.output_enabled).toBe(true);
  expect(ccDevice.active_preset_id).toBe(1);
  expect(cpDevice.output_enabled).toBe(true);
  expect(cpDevice.active_preset_id).toBe(3);

  ccDevice.simulation.lastWallClockMs = Date.now() - 1000;
  cpDevice.simulation.lastWallClockMs = Date.now() - 1000;

  const ccStatus = await mockGetStatus("mock://demo-1");
  const cpStatus = await mockGetStatus("mock://demo-2");

  expect(ccStatus.raw.enable).toBe(true);
  expect(ccStatus.raw.i_local_ma + ccStatus.raw.i_remote_ma).toBeGreaterThan(
    1_000,
  );
  expect(cpStatus.raw.enable).toBe(true);
  expect(cpStatus.raw.mode).toBe(3);
  expect(cpStatus.raw.calc_p_mw).toBeGreaterThan(25_000);
  expect(cpStatus.raw.v_remote_mv).toBeGreaterThan(
    ccStatus.raw.v_remote_mv + 4_000,
  );
});

test("mock status evolves into realistic non-zero readings when output is enabled", async () => {
  const baseUrl = "mock://demo-1";
  const device = getOrCreateMockDevice(baseUrl);
  device.simulation.lastWallClockMs = Date.now() - 1000;

  await mockUpdateControl(baseUrl, { output_enabled: true });
  const status = await mockGetStatus(baseUrl);

  expect(status.raw.enable).toBe(true);
  expect(status.raw.mode).toBe(1);
  expect(status.raw.v_remote_mv).toBeGreaterThan(10_500);
  expect(status.raw.v_remote_mv).toBeLessThan(12_200);
  expect(status.raw.i_local_ma + status.raw.i_remote_ma).toBeGreaterThan(1_000);
  expect(status.raw.calc_p_mw).toBeGreaterThan(10_000);
  expect(status.raw.sink_core_temp_mc).toBeGreaterThan(
    device.simulation.profile.ambientTempMc + 5_000,
  );
  expect(status.state_flags_decoded).toContain("ENABLED");
  expect(status.state_flags_decoded).toContain("REMOTE_ACTIVE");
  expect(status.state_flags_decoded).toContain("LINK_GOOD");
});

test("mock legacy CC updates stay visible across status polls", async () => {
  const baseUrl = "mock://legacy-cc-update";
  const device = getOrCreateMockDevice(baseUrl);
  device.active_preset_id = 1;
  device.output_enabled = false;

  const updated = await mockUpdateCc(baseUrl, {
    enable: true,
    target_i_ma: 2400,
  });
  expect(updated.enable).toBe(true);
  expect(updated.target_i_ma).toBe(2400);

  const status = await mockGetStatus(baseUrl);
  const afterPoll = getOrCreateMockDevice(baseUrl);
  expect(status.raw.enable).toBe(true);
  expect(afterPoll.output_enabled).toBe(true);
  expect(afterPoll.cc.enable).toBe(true);
  expect(afterPoll.cc.target_i_ma).toBe(2400);
  expect(afterPoll.presets[0]?.mode).toBe("cc");
  expect(afterPoll.presets[0]?.target_i_ma).toBe(2400);
});

test("different mock demo devices expose distinct operating scenarios", async () => {
  const ccBaseUrl = "mock://demo-1";
  const cpBaseUrl = "mock://demo-2";

  getOrCreateMockDevice(ccBaseUrl).simulation.lastWallClockMs =
    Date.now() - 1000;
  getOrCreateMockDevice(cpBaseUrl).simulation.lastWallClockMs =
    Date.now() - 1000;

  await mockUpdateControl(ccBaseUrl, { output_enabled: true });
  await mockUpdateControl(cpBaseUrl, { output_enabled: true });

  const ccStatus = await mockGetStatus(ccBaseUrl);
  const cpStatus = await mockGetStatus(cpBaseUrl);

  expect(ccStatus.raw.mode).toBe(1);
  expect(cpStatus.raw.mode).toBe(3);
  expect(cpStatus.raw.v_remote_mv).toBeGreaterThan(
    ccStatus.raw.v_remote_mv + 4_000,
  );
  expect(cpStatus.raw.calc_p_mw).toBeGreaterThan(ccStatus.raw.calc_p_mw);
});

test("mock diagnostics export matches firmware-facing diagnostics contract", async () => {
  const baseUrl = "mock://diagnostics-contract";
  const device = getOrCreateMockDevice(baseUrl);
  device.wifi.ssid = "BenchNet";
  device.wifi.source = "user";
  device.wifi.state = "configured";
  device.wifi.ip = null;
  device.wifi.last_error = null;
  device.status.link_up = true;
  device.status.raw.uptime_ms = 4242;
  device.status.raw.fault_flags = 3;

  const diagnostics = await exportDiagnostics(baseUrl);

  expect(diagnostics).toEqual({
    schema_version: 1,
    redaction: { psk: true },
    firmware_version: device.identity.digital_fw_version,
    wifi: {
      ssid: "BenchNet",
      source: "user",
      state: "configured",
      ip: null,
      last_error: null,
      psk: "<redacted>",
    },
    link_up: true,
    calibration_persistence: { status: "factory-default" },
    last_status: {
      uptime_ms: 4242,
      fault_flags: 3,
    },
  });
});

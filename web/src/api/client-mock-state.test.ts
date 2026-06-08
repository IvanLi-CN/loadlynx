import { expect, test } from "vitest";
import { exportDiagnostics } from "./client.ts";
import {
  type DevdStatusPayload,
  getOrCreateMockDevice,
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
      uptime_ms: 42,
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
    last_status: {
      uptime_ms: 4242,
      fault_flags: 3,
    },
  });
});

import { expect, test } from "vitest";
import { parseFirmwareIdentity } from "./loadlynx-web-serial.ts";

test("parseFirmwareIdentity accepts valid digital firmware metadata", () => {
  const firmware = parseFirmwareIdentity({
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
  });

  expect(firmware?.target).toBe("digital_esp32s3");
  expect(firmware?.protocol).toBe("loadlynx.cdc.v1");
  expect(firmware?.features).toEqual(["net_http", "usb_cdc_jsonl"]);
});

test("parseFirmwareIdentity rejects incomplete payloads", () => {
  expect(
    parseFirmwareIdentity({
      target: "digital_esp32s3",
      package_version: "0.1.0",
      protocol: "loadlynx.cdc.v1",
    }),
  ).toBeUndefined();
});

import { expect, test } from "vitest";

import { parseBackupImportText } from "./import-backup.ts";

test("parseBackupImportText accepts a valid backup and derives supported sections", () => {
  const result = parseBackupImportText(
    JSON.stringify({
      kind: "loadlynx.backup",
      schema_version: 1,
      created_at: "2026-06-08T00:00:00.000Z",
      sections: {
        presets: {
          presets: [],
        },
        settings: {
          wifi: {
            ssid: "lab",
            psk: "secret",
            source: "user",
          },
        },
      },
    }),
  );

  expect(result).toEqual({
    ok: true,
    backup: {
      kind: "loadlynx.backup",
      schema_version: 1,
      created_at: "2026-06-08T00:00:00.000Z",
      sections: {
        presets: {
          presets: [],
        },
        settings: {
          wifi: {
            ssid: "lab",
            psk: "secret",
            source: "user",
          },
        },
      },
    },
    supportedSections: ["presets", "settings.wifi"],
  });
});

test("parseBackupImportText rejects invalid JSON", () => {
  expect(parseBackupImportText("{bad-json")).toEqual({
    ok: false,
    error: "Invalid backup file.",
  });
});

test("parseBackupImportText preserves envelope validation errors", () => {
  expect(
    parseBackupImportText(
      JSON.stringify({
        kind: "not-loadlynx",
        schema_version: 1,
        sections: {},
      }),
    ),
  ).toEqual({
    ok: false,
    error: "Backup kind must be loadlynx.backup.",
  });
});

import {
  getSupportedBackupSections,
  validateBackupEnvelope,
} from "../../api/client.ts";
import type { BackupSectionKey, LoadLynxBackup } from "../../api/types.ts";

export type ParsedBackupImport =
  | {
      ok: true;
      backup: LoadLynxBackup;
      supportedSections: BackupSectionKey[];
    }
  | {
      ok: false;
      error: string;
    };

export function parseBackupImportText(text: string): ParsedBackupImport {
  let parsed: unknown;

  try {
    parsed = JSON.parse(text) as unknown;
  } catch {
    return {
      ok: false,
      error: "Invalid backup file.",
    };
  }

  try {
    const backup = validateBackupEnvelope(parsed);
    return {
      ok: true,
      backup,
      supportedSections: getSupportedBackupSections(backup),
    };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : "Invalid backup file.",
    };
  }
}

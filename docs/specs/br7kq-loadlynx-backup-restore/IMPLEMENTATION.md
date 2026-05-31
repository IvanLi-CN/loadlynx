# LoadLynx Backup & Restore Implementation（#br7kq）

## Status

- Implementation status: completed

## Scope

- CLI backup export/import orchestration.
- devd compatibility endpoint and USB operation for WiFi credential reads.
- Digital firmware USB WiFi credential read support.
- Web Settings Backup & Restore card and API helpers.
- Focused Rust/Web/Storybook coverage and visual evidence.

## Notes

- Restore safety is fail-closed: output disable must be confirmed before any non-dry-run write.
- Backup files may contain plaintext WiFi PSK and should be treated as sensitive user artifacts.

## Implementation Summary

- `loadlynx backup export/import` supports selectable sections, `--file -`, dry-run import, unknown-section warnings and fail-closed output-disable safety before restore writes.
- CLI backup import keeps the existing LAN WiFi write guard: restoring `settings.wifi` over LAN requires `--allow-insecure-lan-wifi`.
- WiFi restore preserves backup source: factory-source backups clear the user override; user-source backups write the backed-up credentials with `wait=false`.
- Digital firmware exposes WiFi credential reads over USB JSONL and LAN HTTP for explicit backup export; ordinary status and diagnostics continue to redact PSK.
- devd proxies `/api/v1/wifi/credentials` to USB and keeps ordinary diagnostics/traces on the existing redaction path.
- Web Settings includes a Backup & Restore card with export selection, import preview, restore selection, WiFi backup warnings, safety-blocked errors and partial restore results.
- Restore writes non-network sections before `settings.wifi`, so a `wait=false` WiFi reconfiguration cannot interrupt later selected restore sections over LAN.
- Storybook Settings stories cover import preview, completed restore and safety-blocked restore.

## Verification

- `cargo check --manifest-path tools/loadlynx-devd/Cargo.toml`
- `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml --bin loadlynx`
- `bun run check` in `web/`
- `bun run build` in `web/`
- `LOADLYNX_WEB_DEV_PORT=27301 bun run test:e2e` in `web/`
- `bun run build-storybook --quiet` in `web/`
- `just d-build`
- Real ESP32-S3 HIL on `/dev/cu.usbmodem212101`, firmware `src 0x2b8470fcd4e53493`:
  - devd direct flash and reset paths used the approved cached digital port.
  - LAN `/api/v1/identity` matched the flashed firmware digest.
  - `loadlynx backup export --url http://192.168.31.216 --file tmp/hil-backup-restore/lan-full.json --json` exported all sections with file mode `600` and plaintext WiFi PSK present.
  - `loadlynx backup import --url http://192.168.31.216 --file tmp/hil-backup-restore/full.json --allow-insecure-lan-wifi --json` restored presets, calibration, `settings.pd` and `settings.wifi` with `safety.output_disabled=true`.
  - Post-restore control readback confirmed `output_enabled=false`; PD saved readback matched fixed PDO 3 at 12000 mV and 2000 mA; WiFi credential readback reported factory source with PSK present.

## Visual Evidence

- Storybook canvas evidence is stored in `SPEC.md` under `## Visual Evidence`.

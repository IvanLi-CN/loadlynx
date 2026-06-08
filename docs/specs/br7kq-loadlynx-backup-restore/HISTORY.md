# LoadLynx Backup & Restore History（#br7kq）

## 2026-05-31

- Created the Backup & Restore spec for CLI/Web backup JSON, restore safety semantics, WiFi credential export, and visual evidence requirements.
- Implemented the CLI, Web, devd and digital firmware backup/restore surfaces, including fail-closed restore safety and Storybook visual evidence.
- Refined restore ordering so WiFi writes run after preset, calibration and PD sections, avoiding LAN reconnects before later selected restore writes complete.
- Preserved WiFi `source` during restore by clearing user WiFi overrides for factory-source backups.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

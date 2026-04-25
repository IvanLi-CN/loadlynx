# Implementation Notes

## Current behavior

- Analog `PD_STATUS.fixed_pdos` remains the only source of truth for real fixed PDO capabilities.
- Digital owner-facing surfaces now treat `fixed_pdos` as a live capability snapshot:
  - `/api/v1/pd`
  - on-device PD settings
  - Web PD/settings summaries
- Detached state, missing `PD_STATUS`, or SPR-only capabilities with `epr_capable=true` no longer inject a synthetic 28V fixed row into owner-facing capability lists.

## Request-path boundary

- The sink request path still keeps the existing EPR fixed 28V helper logic so a persisted 28V target can be interpreted internally during EPR entry.
- Owner-facing UI and HTTP validation no longer use that helper to pretend 28V is present in `fixed_pdos`.
- `PUT /api/v1/pd` accepts only real current capabilities for fixed/PPS selection; missing fixed PDOs now stay on the existing `selected PDO not present in capabilities` error path.

## UI behavior

- Fixed capability lists show only real fixed PDO rows.
- If the saved fixed selection is absent from the current live list, the UI no longer shows a missing 28V summary row; it falls back to “select PDO” / no visible fixed selection and keeps `Apply` disabled until the owner chooses a real row.
- Web summaries only mention `PDO #N` for fixed mode when that PDO is currently visible in the live capability list.

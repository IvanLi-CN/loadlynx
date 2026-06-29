# History

## Origin

- Migrated from legacy planning docs into the canonical specs taxonomy.

## Key Decisions

- Preserve the legacy spec ID `0020` and slug `web-instrument-control-ui` for traceability.
- Keep the original planning scope traceable while assigning long-lived requirements to `SPEC.md` and implementation/history records to companion documents.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

### Change log

- 2026-01-19: Implemented instrument-style “left monitor / right control” layout for `device-cc` (UI-only; API semantics unchanged).
- 2026-01-19: Polished high-fidelity instrument panel styling and module layout; updated Storybook play + E2E selectors.
- 2026-01-20: Tuned instrument palette + pill bar layout and adjusted numeric decimal separators for closer match with the mock design.
- 2026-01-20: Increased instrument label weight and narrowed decimal dot spacing.
- 2026-01-20: Web e2e (Playwright) green — 13 passed, 1 skipped.
- 2026-06-20: Promoted `device-cc` to the owner-facing `仪表盘`, embedded the full USB-PD panel into the dashboard, and aligned shell semantics with `m3n8p`.

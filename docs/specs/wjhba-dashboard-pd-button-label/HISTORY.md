# History

## Origin

- Migrated from legacy planning docs into the canonical specs taxonomy.

## Key Decisions

- Preserve the legacy spec ID `0019` and slug `dashboard-pd-button-label` for traceability.
- Keep the original planning scope traceable while assigning long-lived requirements to `SPEC.md` and implementation/history records to companion documents.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

### Change log

- 2026-01-19: Dashboard PD button uses active PD contract for voltage display, and pd_state derives from protocol/contract presence (no v_local-based inference; fixes false Error/red when the contract is established).

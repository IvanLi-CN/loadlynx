# History

- Legacy compatibility identity: `#wjhba`.

## Origin

- Migrated from legacy planning docs into the canonical specs taxonomy.

## Key Decisions

- Preserve the legacy spec ID `0019` and slug `dashboard-pd-button-label` for traceability.
- Keep the original planning scope traceable while assigning long-lived requirements to `SPEC.md` and implementation/history records to companion documents.
- Superseded by `docs/specs/dashboard-extended-voltage-toggle/SPEC.md`, which owns the current dashboard PD control and settings-entry contract.

## Documentation Model

`SPEC.md` is retained as the superseded early button-label contract. Current requirements live in `docs/specs/dashboard-extended-voltage-toggle/SPEC.md`.

### Change log

- 2026-01-19: Dashboard PD button uses active PD contract for voltage display, and pd_state derives from protocol/contract presence (no v_local-based inference; fixes false Error/red when the contract is established).

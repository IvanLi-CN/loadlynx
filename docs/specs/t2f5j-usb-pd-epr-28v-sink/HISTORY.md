# History

- 2026-03-19: Created the spec to land USB-PD EPR fixed 28V sink support across analog, protocol, digital persistence, and UI/API surfaces.
- 2026-03-21: Accepted software-path completion with HIL still blocked by non-EPR cable capability during EPR entry.
- 2026-04-25: Refined owner-facing semantics so `fixed_pdos` is live-only; synthetic 28V rows were removed from `/api/v1/pd`, on-device PD settings, and Web summaries while keeping the internal EPR request helper for persisted targets.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

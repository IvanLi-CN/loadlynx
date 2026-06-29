# History

## Origin

- Created to supersede the narrower responsive drawer/sidebar shell assumptions from `t4zh9` and to freeze the owner-facing `总览 / 仪表盘 / 系统` information architecture.

## Key Decisions

- Replace left-side navigation with a sticky top shell across all viewport sizes.
- Keep route compatibility (`/devices`, `/$deviceId/cc`, historical system subpaths) and redirect `/$deviceId/pd` into the dashboard PD panel.
- Treat desktop device switching as a right-side sheet and mobile device switching as an Overview-return flow instead of a navigation drawer.

## Documentation Model

`SPEC.md` owns the stable shell and information-architecture contract. Implementation progress and verification records live in `IMPLEMENTATION.md`; change rationale is captured here.

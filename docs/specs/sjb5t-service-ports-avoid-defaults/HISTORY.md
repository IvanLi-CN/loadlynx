# History

## Origin

- Migrated from legacy planning docs into the canonical specs taxonomy.

## Key Decisions

- Preserve the legacy spec ID `0025` and slug `service-ports-avoid-defaults` for traceability.
- Keep the original planning scope traceable while assigning long-lived requirements to `SPEC.md` and implementation/history records to companion documents.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

### 变更记录 / Change log

- 2026-01-21: 修复 `web/scripts/ports.ts` CLI key 校验，避免 `toString` 等原型属性被误判为有效 key；并按 Biome 要求格式化以通过 CI `bun run check`（PR #62 review fix）。

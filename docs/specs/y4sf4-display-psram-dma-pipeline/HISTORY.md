# History

## Origin

- Companion history initialized during docs/specs catalog migration.

## Key Decisions

- Preserve the existing spec ID `y4sf4` and canonical spec directory.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

### 变更记录（Change log）

- 2026-03-18: 创建 spec，冻结 PSRAM 专用 arena、三缓冲 render/present 管线、dirty-span flush、33ms cadence 与 baseline/candidate HIL 验收口径。
- 2026-03-18: 实现 PSRAM 三缓冲 + render/present 双任务，默认 chunk 按规格回退到 `4096B / 8 rows`，并补齐 baseline/candidate HIL 证据。
- 2026-03-18: 创建 reviewable PR #71，spec 状态切换为已完成。
- 2026-03-19: 继续在同一 PR 上收敛显示正确性与流畅度问题，细化 dirty rect、移除 clone-back、增加 frame-in-flight coalescing，并恢复 `8192B / 16 rows` 默认 chunk。

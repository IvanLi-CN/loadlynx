# Implementation

## Status

- Current status: 已完成
- Last updated: 2026-03-19

## Implementation Summary

This companion document records implementation status for the canonical spec. Existing implementation evidence remains in the spec body, referenced PRs, visual evidence, and related project documents.

## Remaining Gaps

- Refresh this implementation summary when the spec is next materially updated.

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已完成
- Prior catalog timestamp: 2026-03-19
- Prior catalog implementation note: PR #71；PSRAM 专用 framebuffer arena、多缓冲 present、真实 present-FPS、细粒度 dirty rect、pending 背压

### 状态

- Status: 已完成
- Created: 2026-03-18
- Last: 2026-03-19

### 实现前置条件（Definition of Ready / Preconditions）

- 已冻结“PSRAM 只承载原始 framebuffer 字节”的边界。
- 已冻结“三缓冲 + latest-wins pending slot + dirty-span present”的实现方向。
- 已冻结默认 cadence 为 33ms、首选 staging chunk 为 16 rows、回退 chunk 为 8 rows。
- 已冻结 HIL 对比口径：同板、同 selector、同 analog 30Hz。

### 文档更新（Docs to Update）

- `docs/specs/README.md`: 记录状态、Last 与 PR 号
- `docs/specs/y4sf4-display-psram-dma-pipeline/SPEC.md`: 跟踪 milestone、HIL 证据和 review 修复

### 计划资产（Spec assets）

- Directory: `docs/specs/y4sf4-display-psram-dma-pipeline/assets/`
- PR visual evidence source: 本次如需截图，只能放在该目录并从 `## Visual Evidence` 引用

### 资产晋升（Asset promotion）

None

### 实现里程碑（Milestones / Delivery checklist）

- [x] M1: 建立新 spec，并完成 baseline build/HIL 证据采集。
- [x] M2: 启用 PSRAM arena 与三 framebuffer 池，删除默认构建的单缓冲退化。
- [x] M3: 拆分 render/present 任务，接入 dirty-span flush 与 display counters。
- [x] M4: 完成 baseline/candidate 对比、spec sync、review 修复与 reviewable PR。

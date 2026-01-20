# Contracts: File formats（#0024）

本文件定义本计划涉及的文件格式契约，确保版本展示与 GitHub 跳转具备稳定数据来源。

## `web/public/version.json`

- Scope: internal
- Owner: Web
- Consumers: Web runtime UI（版本展示位）

### Current fields（已存在）

- `version: string`
- `builtAt: string`（ISO 8601）

### Proposed fields（新增；向后兼容）

- `git?: {`
  - `branch?: string`（例如 `main`、`feat/...`；若无法获取则可省略）
  - `sha?: string`（full SHA；若无法获取则可省略）
  - `shortSha?: string`（短 hash；若无法获取则可省略）
  - `tag?: string`（若当前构建 commit 有“精确 tag”，可填；例如 `v1.2.3`）
  `}`
- `repo?: {`
  - `url: string`（例如 `https://github.com/IvanLi-CN/loadlynx`）
  `}`

### Compatibility rules

- 允许旧版 `version.json` 仅包含 `version` 与 `builtAt`。
- UI 必须容错缺字段场景：缺 `repo.url` 或缺 `git.branch` 时不得报错，按“隐藏链接或降级到默认仓库/默认分支”处理。
- `git.tag` 仅在可稳定获取且需要跳转时使用；不得作为唯一溯源信息（避免 tag 清理导致链接失效）。

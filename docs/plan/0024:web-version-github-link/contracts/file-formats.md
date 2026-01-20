# Contracts: File formats（#0024）

本文件定义本计划涉及的文件格式契约，确保版本展示与 GitHub 跳转具备稳定数据来源。

## `web/public/version.json`

- Scope: internal
- Owner: Web
- Consumers: External checks / debugging（例如 Pages 线上核对），**非 UI 主来源**

### Current fields（已存在）

- `version: string`
- `builtAt: string`（ISO 8601）

### Proposed fields（新增；向后兼容）

- `git?: {`
  - `sha?: string`（full SHA；若无法获取则可省略）
  - `shortSha?: string`（短 hash；若无法获取则可省略）
  - `tag?: string`（若当前构建 commit 有“精确 tag”，可填；例如 `v1.2.3`）
  `}`
- `repo?: {`
  - `url: string`（例如 `https://github.com/IvanLi-CN/loadlynx`）
  `}`

### Compatibility rules

- 允许旧版 `version.json` 仅包含 `version` 与 `builtAt`。
- Consumers 必须容错缺字段场景：缺 `repo.url` / 缺 `git.sha` / 缺 `git.tag` 时不得报错，按“隐藏或降级展示”处理。
- `git.tag` 仅在可稳定获取且需要跳转时使用；不得作为唯一溯源信息（避免 tag 清理导致链接失效）。

### Notes

- `version.json` 的写入发生在构建期（由 `web/scripts/write-version.mjs` 写入到 `dist/` 与 `public/`），字段值应尽量来源于同一套构建期注入信息（例如 `VITE_APP_*` / `APP_EFFECTIVE_VERSION`），避免“UI 展示版本”与“version.json 版本”不一致。

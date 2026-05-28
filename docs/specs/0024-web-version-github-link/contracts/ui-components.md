# Contracts: UI components（#0024）

## `AppVersionLink`

- Kind: UI Component
- Scope: internal
- Owner: Web
- Consumers: `ConsoleLayout`

### Props（建议）

- `version: string | null`
- `repo: string | null`（`Owner/Repo`，例如 `IvanLi-CN/loadlynx`）
- `sha: string | null`（full SHA 优先；短 SHA 允许但不推荐）
- `tag: string | null`（稳定发布 tag；规则见 Plan 0024）

### Behavior

- `version` 为 null 时：不渲染或渲染降级文本（由实现选择其一，需符合验收标准）。
- 点击时：
  - 若 `tag` 存在且匹配“稳定发布 tag”：优先打开 `https://github.com/${repo}/releases/tag/${tag}`（或 `.../tree/${tag}`，以实现阶段最终选择为准）。
  - 否则若 `sha` 存在：打开 `https://github.com/${repo}/commit/${sha}`。
  - 否则：不打开链接（降级为不可点击文本或隐藏，需符合验收标准）。
- 默认 `target="_blank"`（新标签页）并带 `rel="noreferrer"`（或等价安全属性）。

### Data source（约定）

- `ConsoleLayout` 从 `import.meta.env` 读取构建期注入的 `VITE_APP_VERSION` / `VITE_APP_GIT_SHA` / `VITE_APP_GIT_TAG` / `VITE_GITHUB_REPO`，并将它们映射为上述 props。

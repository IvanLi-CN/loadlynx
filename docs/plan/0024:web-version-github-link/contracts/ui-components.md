# Contracts: UI components（#0024）

## `AppVersionLink`

- Kind: UI Component
- Scope: internal
- Owner: Web
- Consumers: `ConsoleLayout`

### Props（建议）

- `version: string | null`
- `repoUrl: string | null`
- `branch: string | null`
- `sha: string | null`
- `tag: string | null`
- `mode?: "branch" | "commit"`（默认 `"branch"`）

### Behavior

- `version` 为 null 时：不渲染或渲染降级文本（由实现选择其一，需符合验收标准）。
- 点击时：
  - 若 `tag` 存在且匹配“稳定发布 tag”（规则：`tag` 以 `v` 开头，例如 `vX.Y.Z` / `vX.Y.Z-rc.<timestamp>`）：优先打开 `${repoUrl}/releases/tag/${tag}`（或 `${repoUrl}/tree/${tag}`）
  - 否则 `mode="commit"`：打开 `${repoUrl}/commit/${sha}`（缺字段则回退到 branch）
  - 否则 `mode="branch"`：打开 `${repoUrl}/tree/${branch}`（缺字段则使用默认值）
- 默认 `target="_blank"`（新标签页）并带 `rel="noreferrer"`（或等价安全属性）。

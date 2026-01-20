# Contracts: Config（#0024）

本计划需要一个稳定的 GitHub 仓库基址用于跳转。

## `REPO_URL`

- Scope: internal
- Owner: Web
- Consumers: Web runtime UI

### Default value

- `https://github.com/IvanLi-CN/loadlynx`

### Override (optional)

- 允许通过构建时环境变量或 `version.json.repo.url` 覆盖（以实现阶段最终方案为准）。
- 若覆盖机制会增加复杂度，可只实现默认值（仍满足本计划核心需求）。


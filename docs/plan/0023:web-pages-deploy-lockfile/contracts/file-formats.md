# Contracts: File formats（#0023）

本文件定义本计划涉及的“文件形状/约定”，用于让本地开发与 CI 在依赖安装上达成一致。

## `web/bun.lock`

- Scope: internal
- Owner: Web
- Consumers: Developers、GitHub Actions（`web-check` / `web-pages`）
- Rule:
  - `web/bun.lock` 必须纳入版本控制。
  - 任何对 `web/package.json` 的依赖变更，都必须同步更新 `web/bun.lock`，并在 PR 中一并提交。
  - CI 必须使用“冻结 lockfile”的安装方式（例如 `bun ci` 或 `bun install --frozen-lockfile`），以确保可复现构建。

## `web/package-lock.json`

- Scope: internal
- Owner: Web
- Consumers: Developers、GitHub Actions
- 当前观察到 `web/` 同时存在 `bun.lock` 与 `package-lock.json`，并在 CI 的冻结安装场景下触发迁移/写回风险。
- Rule:
  - `web/package-lock.json` 将被移除，不再维护。
  - 本仓库 Web 依赖锁定以 `web/bun.lock` 为唯一真源。

# Contracts: CI config（#0023）

本文件定义本计划涉及的 CI 配置契约，目标是让 GitHub Pages 部署与 Web Check 具备一致的依赖安装行为与版本选择。

## GitHub Actions: Bun / Node 版本

- Scope: internal
- Owner: Repo
- Consumers: GitHub Actions workflows
- Rule:
  - `web-pages` 与 `web-check` 必须使用一致的 Bun 版本（避免 lockfile 解析/行为差异）。
  - 推荐统一使用 `oven-sh/setup-bun@v2`，并显式设置 `bun-version: latest`（两套 workflow 保持一致）。
  - Node 版本保持 `20`（与现有 workflow 一致）。

## GitHub Actions: 依赖安装命令

- Scope: internal
- Owner: Repo
- Consumers: GitHub Actions workflows
- Rule:
  - 依赖安装必须是“冻结 lockfile”模式：若 `bun.lock` 与 `package.json` 不一致则失败。
  - 安装命令统一为：`bun ci`（等价于 `bun install --frozen-lockfile`，语义更贴近 CI）。
  - workflow 必须在 `web/` 目录内执行安装与构建步骤。

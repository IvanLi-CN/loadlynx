# GitHub Pages：Web Deploy 失败修复（lockfile/CI）（#0023）

## 状态

- Status: 部分完成（2/4）
- Created: 2026-01-20
- Last: 2026-01-20

## 背景 / 问题陈述

- GitHub Pages 的 `Web Deploy (GitHub Pages)`（`.github/workflows/web-pages.yml`）在一次合并到 `main` 后运行失败，导致 Pages 上的 web app 停留在更早的成功部署版本，未能反映最新 `main`。
- 失败发生在 `web/` 的依赖安装阶段：CI 使用 `bun install --frozen-lockfile`，但 Bun 判定 lockfile 需要被改写（包含从 `package-lock.json` 触发的迁移/同步行为），因此直接退出。

## 目标 / 非目标

### Goals

- 合并/推送到 `main` 且包含 `web/**` 变更时，GitHub Pages 部署能稳定成功（build + deploy）。
- Pages 上的 web app 部署提交可追溯且与 `main` 一致（以 workflow `head_sha` 为准）。
- CI 中依赖安装保持可复现（deterministic），并在 lockfile 不一致时明确失败（而非隐式更新依赖）。

### Non-goals

- 不改动 Web App 功能与 UI（仅修复 CI 与依赖锁定策略/约定）。
- 不引入新的包管理器或新的 CI 工具链（继续使用 Node + Bun + 现有 Actions）。
- 不在本计划内调整 GitHub Pages 的域名、路由或部署目录结构（沿用现有流程）。

## 用户与场景

- 维护者合并 PR 到 `main` 后，希望 GitHub Pages 自动更新用于演示与回归验证。
- 贡献者在 PR 中改动 `web/**` 后，希望 `web-check` 与 `web-pages` 行为一致，减少“本地正常、线上不更新”的意外。

## 需求（Requirements）

### MUST

- 任意包含 `web/**` 变更的 `main` 推送，`Web Deploy (GitHub Pages)` workflow 必须成功（`build` 与 `deploy` job 均为 `success`）。
- CI 中依赖安装必须在 lockfile 不一致时失败并给出明确提示（不允许自动写回更新依赖）。
- 明确并落地 `web/` 的 lockfile 策略（单一真源），避免在 CI 的 `--frozen-lockfile` 下触发迁移/写回。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `web/bun.lock` | File format | internal | Modify | ./contracts/file-formats.md | Web | Developers / CI | 作为唯一 lockfile 真源 |
| `web/package-lock.json` | File format | internal | Delete | ./contracts/file-formats.md | Web | CI / Developers | 移除，避免迁移与双锁不一致 |
| `.github/workflows/web-pages.yml` | Config | internal | Modify | ./contracts/config.md | Repo | GitHub Actions | Bun 版本与安装命令对齐 |
| `.github/workflows/web-check.yml` | Config | internal | Modify（可选） | ./contracts/config.md | Repo | GitHub Actions | 与 pages 保持一致（建议） |

### 契约文档（按 Kind 拆分）

- [contracts/file-formats.md](./contracts/file-formats.md)
- [contracts/config.md](./contracts/config.md)

## 验收标准（Acceptance Criteria）

- Given 将包含 `web/**` 变更的 PR 合并到 `main`，When `Web Deploy (GitHub Pages)` workflow 触发，Then `build` 与 `deploy` job 均为 `success`，且 workflow 的 `head_sha` 等于该次合并后的 `main` HEAD。
- Given `bun.lock` 与 `package.json` 不一致，When CI 执行依赖安装，Then workflow 必须失败并提示需要更新 lockfile（不允许自动写回）。

## 约束与风险

- 当前 `web-pages` 与 `web-check` 使用不同 Bun 版本（pages: `1.1.34`；check: `1.3.5`），可能带来 lockfile 兼容性与行为差异。
- `web/` 同时存在 `bun.lock` 与 `package-lock.json`，在 `--frozen-lockfile` 场景下可能触发迁移/写回，导致 CI 失败。
- 调整 lockfile 与 CI 策略需要一次性对齐本地开发流程（避免后续重复踩坑）。

## 非功能性验收 / 质量门槛（Quality Gates）

- 不引入新的依赖管理工具；仅调整现有 workflow 与 lockfile 文件。
- 合并到 `main` 后，至少观察到一次 `web-check` 与一次 `web-pages` 的成功运行（以 GitHub Actions 结果为准）。

## 文档更新

- `web/README.md`：补充 lockfile 策略与“修改依赖后应更新哪些文件”的说明（避免未来再次出现 lockfile 不一致）。

## 里程碑（Milestones）

- [x] 对齐 Bun 版本与依赖安装命令（`web-pages` / `web-check` 一致；推荐使用 `bun ci` 或继续使用 `bun install --frozen-lockfile`）
- [x] 明确并落地 lockfile 策略（仅保留并维护一种 lockfile；避免自动迁移/双锁不一致）
- [ ] GitHub Actions 验证：`web-check` + `web-pages` 在 `main` 上均成功
- [ ] GitHub Pages 验证：部署 commit 与 `main` HEAD 对齐（通过 `head_sha`）

## 已确认口径（Decisions）

- `web/package-lock.json` 删除，只保留并维护 `web/bun.lock`。
- 对齐并升级 Bun 版本：`web-pages` 与 `web-check` 使用 `oven-sh/setup-bun@v2` 且 `bun-version: latest`。
- CI 依赖安装统一使用 `bun ci`（冻结 lockfile；不允许隐式更新）。
- “线上版本对齐”验收以 GitHub Actions 的 `head_sha` 为准。

## 假设（Assumptions）

- None

# GitHub Pages：Web Deploy 失败修复（lockfile/CI）

## 背景 / 问题陈述

- GitHub Pages 曾在 `main` 上独立构建 Web，导致其 package development version 与同一提交的正式 release version 分离。
- 旧流程的 lockfile 问题已解决；当前风险是独立 source build 可以覆盖已验证的 release Web artifact。

## 目标 / 非目标

### Goals

- 每个 release channel 的已验证 Web tarball 都能部署到 GitHub Pages，且页面版本与 release asset 一致。
- Pages 部署是 GitHub Release 创建的前置门槛；手动恢复只能选择已发布的 release tag。
- CI 中依赖安装保持可复现（deterministic），并在 lockfile 不一致时明确失败（而非隐式更新依赖）。

### Non-goals

- 不改动 Web App 功能与 UI（仅修复 CI 与依赖锁定策略/约定）。
- 不引入新的包管理器或新的 CI 工具链（继续使用 Node + Bun + 现有 Actions）。
- 不在本计划内调整 GitHub Pages 的域名、路由或部署目录结构。

## 用户与场景

- 维护者发布任一 release 后，希望 GitHub Pages 自动更新为该 release 的可追溯 Web artifact。
- 维护者在 Pages 故障后，希望能以明确的已发布 tag 恢复部署，而不会重新构建或改变版本。

## 需求（Requirements）

### MUST

- `Web Deploy (GitHub Pages)` 不得由 `main` push 触发，必须要求 `release_tag` 并验证下载的 release Web asset。
- `Release (LoadLynx)` 的 Web build 必须在打包前完成依赖安装、bundle budget 和 production preview smoke；Pages upload/deploy 失败必须阻止创建 GitHub Release。
- CI 中依赖安装必须在 lockfile 不一致时失败并给出明确提示（不允许自动写回更新依赖）。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `loadlynx-web-<tag>.tar.gz` | Release artifact | owner-facing | New contract | ./contracts/config.md | Release | GitHub Pages / operators | Pages 与 GitHub Release 的唯一 Web 来源 |
| `.github/workflows/release.yml` | Config | internal | Modify | ./contracts/config.md | Repo | GitHub Actions | build、preview、artifact 校验与 Pages 硬门槛 |
| `.github/workflows/web-pages.yml` | Config | internal | Modify | ./contracts/config.md | Repo | GitHub Actions | 只接受已发布 `release_tag` 的恢复部署 |

### 契约文档（按 Kind 拆分）

- [contracts/file-formats.md](./contracts/file-formats.md)
- [contracts/config.md](./contracts/config.md)

## 验收标准（Acceptance Criteria）

- Given `Release (LoadLynx)` 构建任一 release，When Pages upload/deploy 成功，Then GitHub Release 才会创建，且 Pages 的 shell version 与 `/version.json` 等于该 release version。
- Given 维护者手动运行 `Web Deploy (GitHub Pages)`，When 输入已发布 `release_tag`，Then workflow 只部署匹配的 release Web tarball；tag、asset 名与内嵌版本任一不一致都必须失败。
- Given `bun.lock` 与 `package.json` 不一致，When CI 执行依赖安装，Then workflow 必须失败并提示需要更新 lockfile（不允许自动写回）。

## 约束与风险

- Pages 在 GitHub Release 创建前短暂暴露 release candidate；若后续 release 创建失败，现有 release-failure 通知会提示维护者处置。
- 手动恢复只接受包含 `index.html`、`404.html` 和 `version.json` 的匹配 release asset，防止深链接或版本溯源静默退化。

## 非功能性验收 / 质量门槛（Quality Gates）

- 不引入新的依赖管理工具；仅调整现有 workflow 与 lockfile 文件。
- 合并到 `main` 后，至少观察到一次完整 Release run 的 Web、Pages 与 release jobs 均成功；手动恢复 workflow 必须能以已发布 tag 成功部署。

## 已确认口径（Decisions）

- release Web job 使用 `bun ci`、bundle budget 与 production preview smoke 后才可打包。
- `Web Deploy (GitHub Pages)` 没有 `push` trigger，要求显式 `release_tag` 并部署同名已发布 asset。
- “线上版本对齐”验收以 release version、tarball 内嵌版本与 Pages 响应三者一致为准。

## 假设（Assumptions）

- None

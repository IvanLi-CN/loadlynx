# Contracts: CI config（#rjkcw）

本文件定义 release Web artifact 到 GitHub Pages 的部署契约。

## Release Web build

- Scope: internal
- Owner: Repo
- Consumers: `Release (LoadLynx)` Web job
- Rule:
  - Web build 使用仓库固定的 Node/Bun 版本与 `bun ci`。
  - bundle budget、Playwright production preview smoke 与 SPA `404.html` 必须在 release Web tarball 打包前通过。
  - tarball 内 `index.html[data-shell-version]`、`version.json.version` 与 release tag 规范化后的 version 必须一致。

## GitHub Pages deployment

- Scope: internal
- Owner: Repo
- Consumers: `Release (LoadLynx)`、`Web Deploy (GitHub Pages)`
- Rule:
  - Release job 先上传并部署已验证的 Web tarball；Pages upload/deploy 失败时不得创建 GitHub Release。
  - `Web Deploy (GitHub Pages)` 仅由手动 `workflow_dispatch` 触发，必须输入已发布的 `release_tag`，下载同名 asset 并复用相同校验。
  - Pages workflow 不得从 `main` source build 生成新的 Web version。

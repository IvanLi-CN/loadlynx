# Contracts: Config（#0024）

本计划新增一组 **构建期注入（build-time injected）** 的前端环境变量，用于把版本号与 GitHub 溯源信息编译进 Web 产物（Vite client bundle），避免 UI 运行时依赖网络请求。

## Env vars（Vite）

> 说明：Vite 只会将以 `VITE_` 开头的环境变量暴露给前端（`import.meta.env`）。因此本计划的前端配置统一使用 `VITE_` 前缀。

### `VITE_APP_VERSION`

- Type: `string`
- Required (CI / production): yes
- Example: `0.1.0+3423686`
- Source of truth: `APP_EFFECTIVE_VERSION`（由 `.github/scripts/compute-version.sh` 计算）
- Injection rule: CI 在执行 `bun run build` 前将 `VITE_APP_VERSION` 设为与 `APP_EFFECTIVE_VERSION` 一致。

### `VITE_APP_GIT_SHA`

- Type: `string`
- Required (CI / production): yes（recommended）
- Example: `3423686c1c1f...`（40-char full SHA）
- Injection rule: CI 在执行 `bun run build` 前注入（优先使用 GitHub Actions 的 `GITHUB_SHA`，或 `git rev-parse HEAD`）。

### `VITE_APP_GIT_TAG`（optional）

- Type: `string`
- Default: unset / empty
- Meaning: “稳定发布 tag”，且必须 **精确指向** 当前 `VITE_APP_GIT_SHA`。
- Match rule: tag 名以 `v` 开头（例如 `v1.2.3`、`v1.2.3-rc.20260120123456`）。`dev-*` 等不作为稳定 tag。
- Injection rule: CI 构建前通过 `git describe --tags --exact-match --match "v*"` 或 `git tag --points-at HEAD --list "v*"` 计算；不存在则不注入。

### `VITE_GITHUB_REPO`

- Type: `string`
- Required (CI / production): yes（recommended）
- Example: `IvanLi-CN/loadlynx`
- Usage: UI 以此拼装 `https://github.com/${VITE_GITHUB_REPO}` 作为跳转基址。
- Default (when missing): `IvanLi-CN/loadlynx`

## Fallback / compatibility

- UI 的主来源为 `import.meta.env.VITE_APP_*`；当这些值缺失（例如本地开发未注入）时，版本展示位必须降级（隐藏或显示降级文本），但不得抛异常或影响主流程。

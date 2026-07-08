# Web Bundle Budget Gates

## 背景

`web/` 已经通过手工拆包把应用主产物压回合理范围，但 Storybook 静态构建仍会因为框架注入的 `vite-inject-mocker-entry.js` 触发 Vite 的通用大包告警。该告警把“项目可控 preview/app chunk”与“Storybook 自带测试运行时”混在一起，容易让 CI 与日常回归对真正的问题失焦。

## Goals

- 为 Web app 与 Storybook preview 建立可重复执行的 bundle budget 检查。
- 将 Storybook 框架注入的 mocker runtime 与项目自身 preview chunks 分开验收。
- 把 bundle budget 接入正式 CI 路径，而不是仅依赖人工阅读构建日志。
- 保持现有拆包策略与 Storybook 交互测试能力，不为消除 warning 回退功能。

## Non-goals

- 不要求在本计划内消除 Storybook 的框架运行时注入文件。
- 不引入新的打包器或替换 Storybook/Vite。
- 不把 CSS、字体或 manager runtime 作为本计划的首批预算目标。

## Requirements

### MUST

- `bun run check:bundle:app` 必须检查 `web/dist/assets/*.js`，任何单个 JS chunk 超过 `252 kB` 时失败。
- `bun run check:bundle:storybook` 必须检查 `web/storybook-static/assets/*.js`，任何单个 preview JS chunk 超过 `252 kB` 时失败。
- `bun run check:bundle:storybook` 还必须单独检查 `web/storybook-static/vite-inject-mocker-entry.js`，并使用独立阈值而不是把它混入 preview chunk 预算。
- `web/.storybook/main.ts` 必须把 Storybook 生产构建的 `chunkSizeWarningLimit` 调整为与仓库 bundle budget 策略一致，避免 Vite 的泛化 warning 继续误导。
- `web-check.yml` 与 `web-pages.yml` 必须执行 app bundle budget 检查。
- Storybook CI 路径必须在 `build-storybook` 之后自动执行 Storybook bundle budget 检查。

## Acceptance Criteria

- Given 运行 `bun run build`
  When 执行 `bun run check:bundle:app`
  Then 输出每个 app JS chunk 的大小与阈值，并在全部满足预算时返回成功。

- Given 运行 `bun run build-storybook --quiet`
  When 执行 `bun run check:bundle:storybook`
  Then Storybook preview chunks 与 `vite-inject-mocker-entry.js` 分开报告，且只有超出各自阈值时才失败。

- Given `web-check.yml` 或 `web-pages.yml` 在 CI 中执行
  When bundle 超出仓库定义预算
  Then CI 必须因 bundle budget step 失败，而不是依赖人工解读构建日志。

## Verification

- `cd web && bun run build`
- `cd web && bun run check:bundle:app`
- `cd web && bun run build-storybook --quiet`
- `cd web && bun run check:bundle:storybook`
- `cd web && bun run test:storybook:ci`

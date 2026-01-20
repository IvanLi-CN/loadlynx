# Web：版本号展示 + GitHub 分支跳转（#0024）

## 状态

- Status: 待实现
- Created: 2026-01-20
- Last: 2026-01-20

## 背景 / 问题陈述

- 当前 Web Console 的版本号在 CI **构建期**就已确定（`.github/scripts/compute-version.sh` 计算 `APP_EFFECTIVE_VERSION`），并通过环境变量注入到构建流程；同时构建会生成 `public/version.json`（由 `web/scripts/write-version.mjs` 写入）。但应用主界面未在“真实 UI（ConsoleLayout）”中稳定展示版本信息，也无法一键跳转到 GitHub 对应版本进行溯源。
- 需要把“版本号 + 溯源信息（tag/commit）”作为 **构建期注入的常量** 编译进前端产物（Vite client bundle），避免 UI 在运行时依赖网络 fetch 才能拿到版本号。
- 希望在界面合适位置展示版本号（便于截图/回归/反馈时定位），并提供到 GitHub 的跳转入口（参考 `tavily-hikari` 在 UI 中提供仓库链接与版本溯源的做法）。
- 备注：本仓库目前存在大量 `dev-YYYYMMDD-HHMMSS-<sha>` 风格的开发构建 tag，且会自动清理旧 tag；若将“版本按钮”强绑定到这类 tag，会存在链接随时间失效的风险。

## 目标 / 非目标

### Goals

- Web Console 在非 Storybook 运行时，界面上始终可见一个简洁的版本号展示位（例如 `0.1.0+7c981b7`）。
- 版本号展示位可点击跳转到 GitHub 的“对应版本”（优先 `v*` tag，其次 commit），便于快速定位代码来源。
- 版本信息不应依赖运行时网络请求；在构建期注入缺失等异常场景下，按 best-effort 降级但不影响主流程（不阻塞渲染）。

### Non-goals

- 不在本计划内新增“更新提示/自动刷新”等功能（若需要可另立计划）。
- 不改变现有路由结构与页面布局信息架构（只在现有布局中增加一个轻量入口）。
- 不引入新依赖（维持当前 React + TanStack Router + DaisyUI/Tailwind 栈）。

## 用户与场景

- 开发/维护者：查看线上 Pages 或本地预览时，能直接看到版本并一键打开 GitHub 分支，快速确认“是不是这次合并的代码”。
- 测试/协作方：截图/录屏反馈时，带上版本号，减少来回确认成本。

## 需求（Requirements）

### MUST

- 在 `ConsoleLayout` 的抽屉（drawer）底部提供版本展示位，默认显示构建期注入的 `VITE_APP_VERSION`（通过 `import.meta.env` 读取，版本信息编译进前端产物）。
- 点击版本展示位打开 GitHub 溯源页面（优先精确定位到当前构建对应的 Git 引用；详见“开放问题”中对 tag/commit 的决策）。
- 版本展示在以下场景均不应报错（best-effort）：
  - `VITE_APP_VERSION` / `VITE_APP_GIT_SHA` 缺失（例如本地未注入）
  - 解析 tag/commit 信息失败
- Storybook 运行时默认不显示版本展示位（避免影响组件截图/对比）。

## 已确认口径（Decisions）

- 版本展示位位置：抽屉（drawer）底部。
- 跳转策略：Option B（“`v*` tag 优先，否则 commit”）。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `web/public/version.json` | File format | internal | Modify | ./contracts/file-formats.md | Web | External checks | 增加 GitHub 溯源所需元数据（向后兼容；非 UI 主来源） |
| `AppVersionLink` | UI Component | internal | New | ./contracts/ui-components.md | Web | `web/src/layouts/console-layout.tsx` | 版本展示位（可点击） |
| `VITE_APP_VERSION` / `VITE_APP_GIT_*` / `VITE_GITHUB_REPO` | Config | internal | New | ./contracts/config.md | Web | Web runtime | 构建期注入（编译进 bundle），用于版本展示与 GitHub 跳转 |

### 契约文档（按 Kind 拆分）

- [contracts/file-formats.md](./contracts/file-formats.md)
- [contracts/ui-components.md](./contracts/ui-components.md)
- [contracts/config.md](./contracts/config.md)

## 验收标准（Acceptance Criteria）

- Given 打开 Web Console 任意页面，When 页面渲染完成，Then 在抽屉底部可见版本号展示位，且内容与构建期注入的 `import.meta.env.VITE_APP_VERSION` 一致。
- Given 点击版本号展示位，When 浏览器打开链接，Then 跳转到 GitHub 溯源页面（具体跳转目标按本计划的“开放问题”决策执行）。
- Given 构建期注入缺失（例如本地未设置 env vars），When 打开页面，Then 不影响主界面正常使用，且版本展示位以“隐藏或降级文本”的方式处理（不抛异常、不白屏）。
- Given Storybook 运行时，When 打开任意 story，Then 不显示版本展示位。

## 非功能性验收 / 质量门槛（Quality Gates）

- 不新增依赖；仅使用现有工具链与组件库。
- 版本信息加载与渲染为 best-effort：失败不阻塞 UI，不产生控制台噪音（除非明确需要 debug 日志）。

## 文档更新

- `web/README.md`：补充“版本信息如何生成/展示、如何跳转到 GitHub 溯源”的说明。

## 里程碑（Milestones）

- [ ] 构建期注入：将版本/溯源信息编译进 Vite bundle（`VITE_APP_VERSION` / `VITE_APP_GIT_*` / `VITE_GITHUB_REPO`）
- [ ] `version.json` 契约扩展（加入 repo / sha / tag 等溯源信息，向后兼容；用于外部核对，不作为 UI 主来源）
- [ ] 在 `ConsoleLayout` 增加 `AppVersionLink` 展示位（并在 Storybook runtime 隐藏；数据主来源为构建期注入）
- [ ] 本地预览与 Pages 验证：版本展示正常、GitHub 跳转正确

## 开放问题（需要主人回答）

- None

## 假设（Assumptions）

- 默认放在 `ConsoleLayout` 抽屉底部（主人已选 B），使用小号文字/徽标样式，不抢占主操作区域。
- `v*` tag 的判定规则：tag 名以 `v` 开头（例如 `v1.2.3` 或 `v1.2.3-rc.20260120123456`）；其他 tag（如 `dev-...`）不视为“稳定发布 tag”。

# 服务端口规范化（#0025）

## 状态

- Status: 已完成
- Created: 2026-01-20
- Last: 2026-01-21

## 背景 / 问题陈述

当前仓库内存在多类“会监听端口”的本地开发服务与测试服务，其中部分使用默认端口、部分在端口被占用时会自动换端口。结果是：

- 本地多项目并行开发时容易端口冲突；
- 端口自动漂移会导致 Playwright / Storybook / 文档链接指向不确定，产生“偶发失败、难复现”的体验；
- CI/本地脚本中使用动态选端口（例如 `get-port`）会掩盖冲突，降低可观测性。

本计划要把这些端口**固定化、抬高到高位端口、并禁止自动换端口**；同时允许通过环境变量显式覆盖。

## 目标 / 非目标

### Goals

- 端口规范化遵循**最小变更原则**：只修正“默认端口/动态端口/会自动漂移的端口”。对已经是高位且稳定的端口，不做无意义的迁移。
- 为仓库内“开发服务/测试服务”建立统一的端口分配表（Port registry）。
- 默认端口全部使用高位端口，避免常见默认值（如 5173/4173/6006/3000/8080 等）。
- 所有服务端口允许通过环境变量显式覆盖，并在文档中形成契约。
- 任一服务在端口被占用时必须**直接失败退出**（非零退出码 + 明确错误信息），不得自动选择其他端口。
- 消除配置漂移：Playwright、脚本、文档等引用同一份端口来源。

### Non-goals

- 不承诺“默认端口在所有机器上都绝对不冲突”（只提供显式覆盖与清晰失败）。
- 不引入新的端口探测/自动恢复机制（例如“扫描可用端口”“自动 +1”），避免隐式行为。
- 不改变 mDNS UDP 5353（协议固定端口）。
- 不迁移设备端（firmware）监听端口（HTTP 80、mDNS 5353 等）。

## 范围（Scope）

### In scope

- `web/` 本机开发/测试相关端口：
  - Vite dev / preview
  - Storybook dev
  - Storybook 静态站点测试服务器（`http-server`）

### Out of scope

- 设备端（firmware）监听端口（HTTP 80、mDNS 5353 等）。

## 用户与场景

**用户**
- 固件/前端开发者：本机运行 `web/` 开发服务、Storybook、E2E/组件测试。
- CI：运行 Playwright / Storybook 相关测试。
- HIL/设备调试者：通过浏览器访问设备 HTTP API（如适用）。

**典型场景**
- 同时打开多个仓库/多个 Vite 项目开发，要求端口互不干扰且失败可见。
- 在 CI 中启动一次性服务（Storybook static server / Vite webServer），要求端口确定、失败即报。

## 需求（Requirements）

### MUST

- 端口分配覆盖以下“服务类别”，并给出唯一默认端口：
  - `web/` Vite dev server（本地开发）
  - `web/` Vite preview server（本地预览构建产物）
  - `web/` Storybook dev server（本地组件工作台）
  - `web/` Storybook 静态站点服务（测试用 http-server）
- 所有上述端口均可通过环境变量覆盖（见契约文档）。
- 任何服务遇到端口占用：
  - 不得自动换端口；
  - 必须以非零退出码退出，并在日志中提示“端口被占用”与对应端口号。
- 移除/禁止使用“自动选端口”实现（例如脚本中的 `get-port`）。

## 接口契约（Interfaces & Contracts）

### 接口清单（Inventory）

| 接口（Name） | 类型（Kind） | 范围（Scope） | 变更（Change） | 契约文档（Contract Doc） | 负责人（Owner） | 使用方（Consumers） | 备注（Notes） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 端口环境变量（`LOADLYNX_*_PORT`） | Config | internal | New | `./contracts/config.md` | web | 开发者、CI、脚本 | 统一端口来源，显式覆盖 |
| Web/Storybook 脚本命令（`bun run ...`） | CLI | internal | Modify | `./contracts/cli.md` | web | 开发者、CI | 禁止自动换端口；固定端口 |

### 契约文档

- [contracts/config.md](./contracts/config.md)
- [contracts/cli.md](./contracts/cli.md)

## 约束与风险

- **协议约束**：mDNS 使用 UDP 5353（固定），不在端口规范化范围内。
- **行为风险**：Playwright `webServer.reuseExistingServer` 可能在端口被其他进程占用时“误复用”；需要在实现阶段明确策略（契约里要求失败可见）。

## 端口分配表（Port registry）

> 说明：默认端口为高位端口；如有冲突，必须通过环境变量显式覆盖。

| 服务 | 当前端口（仓库现状） | 建议默认端口（本计划） | 覆盖方式 |
| --- | --- | --- | --- |
| Web：Vite dev server | 25219 | 25219（保留，非默认且稳定） | `LOADLYNX_WEB_DEV_PORT` |
| Web：Vite preview server | 4173（Vite 默认） | 22848 | `LOADLYNX_WEB_PREVIEW_PORT` |
| Web：Storybook dev server | 6006（Storybook 常见默认） | 32931 | `LOADLYNX_STORYBOOK_PORT` |
| Web：Storybook 静态站点（测试用） | 动态（`get-port`） | 34033 | `LOADLYNX_STORYBOOK_TEST_PORT` |
| 设备：mDNS | 5353（固定） | 5353（固定） | N/A |

## 验收标准（Acceptance Criteria）

### Core path

- Given 端口未被占用  
  When 运行 `web/` 的 dev 服务  
  Then 实际监听端口等于 `LOADLYNX_WEB_DEV_PORT`（或默认端口），且不发生端口漂移。

- Given 端口被占用  
  When 运行任一服务（Vite dev/preview、Storybook dev、Storybook 测试静态站点）  
  Then 进程以非零退出码失败，并输出包含端口号的“端口占用”错误信息；不自动切换端口。

- Given 设置了端口环境变量覆盖  
  When 运行对应服务  
  Then 服务监听端口等于覆盖值，且 Playwright/脚本引用的 URL 与之同步。

### Edge cases

- Given `LOADLYNX_*_PORT` 为非数字或超出范围  
  When 启动服务  
  Then 进程在启动阶段失败，并提示变量名与非法值（不回退到默认端口、不自动找端口）。

## 实现前置条件（Definition of Ready / Preconditions）

（本计划已冻结：前置条件已满足。）

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- `web/` Playwright E2E：`bun run test:e2e`
- Storybook 测试：`bun run test:storybook:ci`（固定端口，不使用动态端口）

### Quality checks

- `web/`：`bun run lint`

## 文档更新（Docs to Update）

- `web/README.md`：补充端口环境变量与“端口占用即失败”的约定。
- （如需要）仓库根 `README.md`/`WORKFLOW.md`：仅在已有入口处追加链接，避免重复说明。

## 实现里程碑（Milestones）

- [x] M1: Vite dev/preview 端口契约化 + strict port（占用即失败）
- [x] M2: Storybook dev 端口契约化 + exact port（占用即失败）
- [x] M3: Storybook CI 静态站点改为固定端口（移除 `get-port`）
- [x] M4: Playwright baseURL/webServer.url 与端口来源一致；补齐文档与 CI 断言

## 方案概述（Approach, high-level）

- Vite：
  - `server.port` / `preview.port` 读 env（默认值来自本计划 Port registry）。
  - 启用 `server.strictPort=true` 与 `preview.strictPort=true`，禁止端口自动漂移。
- Storybook：
  - `storybook dev -p <port> --exact-port`，端口占用直接失败。
- Storybook CI 静态站点：
  - 用固定端口运行 `http-server`，删除 `get-port` 路径，避免端口漂移。

## Repo reconnaissance（最小必要事实核查）

实现阶段需要改动/对齐的入口点：

- `web/vite.config.ts`
- `web/playwright.config.ts`
- `web/package.json`
- `web/README.md`
- `web/scripts/ports.ts`

## 开放问题（需要主人回答）

None.

## 假设（需主人确认）

None.

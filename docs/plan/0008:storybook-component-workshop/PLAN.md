# Web Storybook 组件工作台：需求分析与概要设计（#0008）

## 状态

- Status: 已完成
- Created: 2025-12-22
- Last: 2025-12-23
- Source: migrated from `storybook-component-workshop.md` (removed)

## 1. 背景与目标

当前 `web/` 的 UI 组件与页面大多分散在 `src/routes/*.tsx` 中，缺少统一的“组件展示/验证入口”。这导致：

- 组件难以系统性浏览与复用沉淀；
- 关键交互（表单校验、弹窗确认、状态切换等）缺少组件级回归；
- UI 变更验证高度依赖手工点页面或 E2E，反馈慢、定位成本高。

目标是在 `web/` 引入 Storybook 作为“组件工作台”，并在 CI 中运行 Storybook 的交互测试，提升 UI 变更的可见性与回归能力。

## 2. 范围与非目标

### 范围（In scope）

- **覆盖项目内定义的所有组件**：
  - 顶层导出组件（页面/可复用组件）必须有独立 story；
  - 仅在父组件内部使用、无复用价值的简单子组件允许不单独拆分，但必须在父组件 story 的某个变体中可见并可交互。
- **交互测试纳入 CI**：基于 Storybook `play` 的 interaction tests（不做视觉回归/截图对比）。
- **暗色主题 + 全视口**：Storybook 默认暗色，提供移动/平板/桌面等视口切换能力并用于测试执行。
- **零副作用（Storybook 环境）**：
  - 不允许真实网络请求、不允许读写 `localStorage`；
  - 不触发设备扫描、SSE/轮询等外部行为；
  - 故事渲染可复现（避免依赖随机数/不受控时间）。

### 非目标（Out of scope）

- 不在本工作项内实现视觉回归（截图基线与对比）。
- 不用 Storybook 替代既有 Playwright E2E；E2E 仍用于端到端连通性验证。
- 不追求一次性重写 UI 架构；允许渐进式抽取与纯化（见“迁移计划”）。

## 3. 关键用例（核心场景）

- 开发时：在 Storybook 中快速查看组件状态矩阵（正常/错误/禁用/加载中/空态等），验证样式与交互。
- 评审/回归：CI 自动运行交互测试，避免基础交互被无意破坏（例如确认弹窗、表单校验、按钮禁用逻辑）。
- 维护：当页面组件从 `routes` 抽取后，Storybook 成为页面与组件的主要“演示与验证面”。

## 4. 现状扫描（与零副作用冲突点）

以下现有实现需要在后续实现阶段“隔离/注入”，否则 Storybook 渲染会产生副作用：

- `web/src/devices/device-store.ts`：直接读取/写入 `window.localStorage`。
- `web/src/routes/device-calibration.tsx`：读取/写入 `window.localStorage`（draft 与 current options）。
- `web/src/routes/device-settings.tsx`：使用 `window.confirm`（无法组件化测试）。
- `web/src/devices/scan-hooks.ts`：直接 `fetch(http://<ip>/api/v1/identity)` 扫描子网。
- `web/src/api/client.ts`：真实 `fetch`；mock backend 还存在定时更新（例如 mock status uptime tick）。
- `web/src/app.tsx`：`fetch("/version.json")`（虽然当前不在主入口使用，但属于项目内组件）。

## 5. 总体设计（模块边界与可测试性）

核心策略：把“副作用”收敛到可替换的接口层；Storybook 使用 in-memory 实现与静态 fixtures；生产环境使用浏览器/网络真实实现。

### 5.1 目录与模块划分（建议）

在 `web/src/` 内引入以下约定（具体落地以实现阶段为准）：

- `routes/`：路由容器（取参、调 hooks、拼装数据、调用副作用），原则上不作为 Storybook 的主要展示对象。
- `pages/`：页面纯渲染组件（props 驱动），Storybook 的页面级 stories 主要放这里。
- `components/`：可复用 UI 组件与复合组件（props 驱动）。
- `services/`：副作用接口定义与生产实现入口（API、store、clock、confirm 等）。
- `storybook/`：fixtures、decorators、helpers（仅 Storybook/测试使用）。

### 5.2 副作用接口（形状示例）

为满足“Storybook 不读写 localStorage、不打网络”，引入可注入接口（仅示意接口形状，避免实现细节）：

```ts
// 设备列表存储（生产：localStorage；storybook：memory）
export interface DeviceStore {
  list(): Promise<StoredDevice[]>;
  set(devices: StoredDevice[]): Promise<void>;
}

// 标定草稿与选项存储（生产：localStorage；storybook：memory）
export interface CalibrationStore {
  readDraft(deviceId: string, baseUrl: string): Promise<ParsedDraft | null>;
  writeDraft(deviceId: string, baseUrl: string, draft: StoredDraft | null): Promise<void>;
  readCurrentOptions(deviceId: string, baseUrl: string): Promise<CurrentOptions | null>;
  writeCurrentOptions(deviceId: string, baseUrl: string, options: CurrentOptions): Promise<void>;
}

// 设备 API（生产：fetch；storybook：fixtures 或 in-memory fake）
export interface DeviceApi {
  getIdentity(baseUrl: string): Promise<Identity>;
  getStatus(baseUrl: string): Promise<FastStatusView>;
  // ... 其它页面用到的 endpoints
}
```

注入方式建议使用 React Context（例如 `ServicesProvider`），由 `main.tsx` 在生产环境提供真实实现；Storybook `preview` 提供替代实现。

### 5.3 组件纯化规则

- **Storybook 目标组件应尽量 props 驱动**：不在组件内部直接 `fetch`、不直接访问 `localStorage`、不直接调用设备扫描。
- 与路由耦合的逻辑（`useParams`、`Link` 等）尽量留在 `routes` 容器层；页面组件通过 props 接收 `deviceId`、`onNavigate` 或链接数据。
- 与 Query 耦合的逻辑（`useQuery/useMutation`）优先留在容器层；纯组件接受 `data / isLoading / error / onAction`。

## 6. Story 组织与覆盖策略

### 6.1 Story 目录与命名

- `*.stories.tsx` 与被测组件同目录（或统一放到 `storybook/` 下，但优先贴近源码，便于维护）。
- Story title 按模块分层，例如：
  - `Components/Common/ConfirmDialog`
  - `Pages/Devices`
  - `Pages/DeviceStatus`
  - `Pages/Calibration`

### 6.2 覆盖要求

- 每个对外导出组件至少 1 个 story（默认态）。
- 每个关键交互路径至少 1 个带 `play` 的 story（见下一节）。
- 文件内非导出子组件允许不单独建 story，但必须在父 story 的某个变体中可触达（例如展开面板后出现）。

## 7. 交互测试（CI）

采用 `@storybook/test-runner`（基于 Playwright）执行：

- Smoke：所有故事可渲染（可按需排除明显不稳定/未完成的 stories）。
- Interaction：对选定 stories 执行 `play`，验证关键交互。

首批建议覆盖（MVP 下限）：

- `ConfirmDialog` / `AlertDialog`：打开/关闭、destructive 按钮禁用态。
- Devices：Add 表单校验（空值/非法 URL）、提交按钮 loading（由 story 内状态模拟）。
- DeviceStatus：错误提示渲染（由 props/fixtures 注入，不触发真实请求）。
- Calibration：Reset Draft/Reset Device Current 的确认路径（由 props 注入 handler，不写入存储）。

## 8. 暗色主题与视口

- Storybook 默认暗色（复用 `web/src/index.css` 与 DaisyUI 语义色）。
- Viewport：提供 mobile/tablet/desktop 等预设，Storybook 工具栏可切换。
- CI 交互测试：在暗色下执行；必要时对关键 stories 以不同 viewport 重复执行（按性能与稳定性再做裁剪策略）。

## 9. 兼容性与迁移计划（渐进式）

建议按以下顺序推进（实现阶段执行）：

1. 引入 Storybook 基础配置（React+Vite），确保样式与暗色一致。
2. 建立 `services` 接口与 Storybook fixtures（先让核心组件可零副作用渲染）。
3. 从高收益组件开始抽取：`ConfirmDialog/AlertDialog`、Devices 表单区块、Status/Settings 的 error banner 等。
4. 把 `device-store` 与 calibration 草稿存储替换为可注入 store（Storybook 使用 memory）。
5. 接入 test-runner 并在 `.github/workflows/web-check.yml` 添加 CI 步骤。
6. 扩展覆盖到更重页面（Calibration/CC），逐步降低 `routes/*.tsx` 体积并提升纯组件比例。

## 10. 风险点与待确认问题

### 风险点

- Storybook 自身可能使用浏览器存储保存 UI 状态；本工作项的“禁用 localStorage”约束应以**应用代码**为边界，避免与 Storybook 运行机制冲突。
- `routes/device-calibration.tsx` 与 `routes/device-cc.tsx` 体积大且耦合深：抽取需要明确 props 边界，避免引入隐性副作用。
- mock backend 当前带有时间推进逻辑（例如 uptime tick）：用于 Storybook 时需保证确定性（更倾向 fixtures 驱动，而非定时更新）。

### 待确认问题（后续实现阶段可再细化）

- CI 运行策略：是否对所有 stories 全视口执行，或只对“标记的关键 stories”做多视口（用于控制时长）。
- 是否需要在 CI 添加静态检查：禁止 `web/src/**` 中直接使用 `localStorage` / `fetch`（要求使用 `services` 注入层）。

## 11. Storybook 覆盖清单（2025-12-22）

目标：确保 `web/src/**` 下**对外导出的 React 组件 / route 组件**都能在 Storybook 中被“直接或间接”渲染与验证（尽量避免重复 stories），且 Storybook 环境不触发真实网络/子网扫描等副作用。

> 说明：表格中的 “Covered by” 使用的是 Storybook `title`；路径为对应 `*.stories.tsx` 文件位置。

| Exported component | Source | Covered by (Storybook title) | Story file | Coverage note |
| --- | --- | --- | --- | --- |
| `App` (legacy scaffold) | `web/src/app.tsx` | `Legacy/App (scaffold)` | `web/src/stories/legacy/app.stories.tsx` | 显式 story；Storybook runtime 会跳过 `/version.json` fetch。 |
| `AppLayout` (root layout) | `web/src/routes/app-layout.tsx` | `Routes/*` (any) | `web/src/stories/routes/*.stories.tsx` | 由 `createAppRouter()` 作为 root component 使用；任一路由 story 都会渲染它。 |
| `DevicesRoute` | `web/src/routes/devices.tsx` | `Routes/Devices` | `web/src/stories/routes/devices-route.stories.tsx` | 通过路由渲染（同样也覆盖了 index route `/`）。 |
| `DeviceCcRoute` | `web/src/routes/device-cc.tsx` | `Routes/CC` | `web/src/stories/routes/cc-route.stories.tsx` | 通过路由渲染（`/mock-001/cc`）。 |
| `DeviceStatusRoute` | `web/src/routes/device-status.tsx` | `Routes/Status` | `web/src/stories/routes/status-route.stories.tsx` | 通过路由渲染（`/mock-001/status`）。 |
| `DeviceSettingsRoute` | `web/src/routes/device-settings.tsx` | `Routes/Settings` | `web/src/stories/routes/settings-route.stories.tsx` | 通过路由渲染（`/mock-001/settings`）。 |
| `DeviceCalibrationRoute` | `web/src/routes/device-calibration.tsx` | `Routes/Calibration` | `web/src/stories/routes/calibration-route.stories.tsx` | 通过路由渲染（`/mock-001/calibration`）。 |
| `DevicesPanel` | `web/src/devices/devices-panel.tsx` | `Devices/Panel (No side effects)` | `web/src/devices/devices-panel.stories.tsx` | 显式 story（`MemoryDeviceStore` + `QueryClient`），零副作用。 |
| `DeviceStoreProvider` | `web/src/devices/store-context.tsx` | `Devices/Panel (No side effects)` / `Routes/*` | `web/src/devices/devices-panel.stories.tsx` / `web/src/stories/routes/*.stories.tsx` | Provider-only 组件：作为 stories harness 的 wrapper 被覆盖；无需单独 story。 |
| `ConfirmDialog` | `web/src/components/common/confirm-dialog.tsx` | `Common/ConfirmDialog` | `web/src/components/common/confirm-dialog.stories.tsx` | 显式 story（包含交互测试）。 |
| `AlertDialog` | `web/src/components/common/alert-dialog.tsx` | `Common/AlertDialog` | `web/src/components/common/alert-dialog.stories.tsx` | 显式 story（包含交互测试）。 |
| `RouteStoryHarness` (storybook-only helper) | `web/src/stories/router/route-story-harness.tsx` | `Routes/*` (indirect) | `web/src/stories/routes/*.stories.tsx` | 仅供 stories 使用的辅助组件；不作为产品组件覆盖目标，但在 routes stories 中被渲染。 |

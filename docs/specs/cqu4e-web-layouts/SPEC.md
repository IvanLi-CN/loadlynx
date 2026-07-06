# Web UI Layout 规范化（Layouts 抽象）

## 背景与问题

> 当前有效 shell 契约已由 [`m3n8p-web-top-nav-device-workspace`](../m3n8p-web-top-nav-device-workspace/SPEC.md) 接管。本文继续作为布局分层与 pathless layout 抽象的长期约束，不再声明 sidebar / drawer 为当前产品真相。

当前 `web/` 的布局主要由根路由组件（`AppLayout`）提供，但它同时承担了：

- **App Shell**：顶栏/侧栏/内容区的整体骨架；
- **跨页面业务副作用**：例如基于路由切换强制关闭 calibration mode；
- **页面容器规范缺失**：各页面分别用 `max-w-* / mx-auto / p-*` 自行控制宽度与间距，导致对齐不一致、复用困难。

这使得 UI 的“布局层”边界不清晰：布局变更会牵连业务逻辑，页面复用也会被迫复制容器/标题/导航结构。

## 目标

- 把 UI 中可复用的布局结构抽象成少量清晰的 layout（路由级布局为主）。
- 统一页面容器与间距规则，让各页面只关注“内容”，不再各写一套容器样式。
- 将 calibration mode 等跨页面副作用从纯布局中剥离，放到更贴近业务的布局/路由层级管理。
- **尽量保持 URL 不变**（通过 TanStack Router 的 pathless layout route 实现）。

## 非目标

- 不调整视觉主题（Tailwind/DaisyUI 配色、组件风格）与信息架构（导航项命名/层级）本身。
- 不引入新的路由或状态管理框架；保持 TanStack Router / React Query 现状。
- 不在本设计阶段提交实现代码（仅定义边界与迁移路径）。

## 方案概述：建议抽象的 Layouts

建议将布局拆为 3（必选）+ 1（本次选用）：

1. **RootLayout（全局壳）**
   - 只负责全局背景、Devtools、`<Outlet />`，不承担业务副作用。
   - 作为 TanStack Router 的 root route component。
2. **ConsoleLayout（App Shell）**
   - sticky 顶部导航壳层 + 主内容区（`<Outlet />`）。
   - 作为 **pathless layout route**（不改变 URL）。
3. **DeviceLayout（设备域布局）**
   - 作为 `$deviceId` 的父 layout，统一处理：
     - `deviceId` → 本地设备表查找；
     - 不存在/未加载的统一占位；
     - 设备子导航（CC/Status/Settings/Calibration）；
     - 向子页面提供 `device/baseUrl/identity` 等上下文（避免每页重复查询/判空）。
   - 约束：包括 calibration 在内的所有 `/$deviceId/*` 子页面都应优先通过 `DeviceLayout` 上下文获取 `device/baseUrl`，避免重复的 “再查一次 devices 列表/再判空”。
4. **Calibration（设备子页面，工具模式变体）**
   - calibration 仍属于 `/$deviceId/*` 的设备子页面，**必须复用 `DeviceLayout`**（设备查找/错误处理/上下文统一由 DeviceLayout 提供）。
   - 作为“工具模式（tool layout）”的代表页面：内容区 **全宽**，并向 App Shell 请求隐藏沉浸式辅助 UI；当前 shell 已无常驻侧栏/rail。

## 路由结构（保持 URL 不变）

使用 TanStack Router code-based routing，通过 pathless layout route 把 “Console Shell” 插入到现有 URL 上方：

- `rootRoute`：`RootLayout`
  - `consoleRoute`（pathless，`id: "console"`）：`ConsoleLayout`
    - `/devices`：Devices 页面
    - `/$deviceId`：`DeviceLayout`
      - `cc | status | settings`：各子页面内容
      - `calibration`：设备子页面（工具模式变体：隐藏常驻侧栏/rail + 全宽内容）

说明：TanStack Router 会按匹配优先级排序路由，pathless layout 不参与 URL 匹配，仅作为渲染结构的父级。

## 布局变体（工具模式 / 全宽）

为避免把 calibration 变成“另一个 App”，本方案仍复用 `ConsoleLayout` 的顶栏，并且 calibration **仍在 `DeviceLayout` 之下**；同时允许 `ConsoleLayout` 基于路由元信息切换布局变体：

- **默认变体（shell）**：sticky 顶栏 + 内容区（内容区内再用 `PageContainer` 做 `max-w-5xl` 收敛）
- **工具变体（tool）**：sticky 顶栏 + 内容区，内容区 **不限制宽度**（用于 Calibration 这类长表格/多列操作）。

另见：`docs/specs/m3n8p-web-top-nav-device-workspace/SPEC.md`（当前顶部导航 shell / 设备切换 / 系统域契约）。

实现建议（实现阶段落地）：

- 在 `calibration` route 上设置 `staticData`（例如 `{ layout: "tool" }`）
- `ConsoleLayout` 通过 router state / matches 读取当前激活的 `staticData`，据此决定是否切换 shell 变体
- `PageContainer` 提供 `variant="default" | "full"`，在工具页使用 `full`（避免 max width 限制）

## 模块边界与文件建议

建议新增目录（不强制，但有助于约束边界）：

- `web/src/layouts/`
  - `root-layout.tsx`
  - `console-layout.tsx`
  - `device-layout.tsx`
  - `calibration-layout.tsx`
- `web/src/components/layout/`
  - `page-container.tsx`：统一 `max-width / padding / gap` 的页面容器

设备域上下文建议提供：

- `DeviceContext`（React context）或 layout 内部自定义 hook
  - 暴露：`deviceId`, `device`, `baseUrl`, `identityQuery`（或 `identity`）等
  - 子页面通过 `useDeviceContext()` 读取，避免重复的 “find device / not found / baseUrl 判空” 逻辑

## calibration mode 管理（副作用下沉）

将当前放在 `AppLayout` 的“非 calibration 页面强制 off”逻辑下沉到更贴近业务的层级：

- 推荐：由 `CalibrationLayout`（或 `calibration` route component）负责：
  - **进入** calibration：根据 tab 选择设置 mode；
  - **离开** calibration：best-effort 设置 mode 为 `off`；
- 其它 device 子页面在 mount 时 best-effort `off`（兜底，避免刷新/异常导航导致设备残留在 calibration mode）。

这样 `ConsoleLayout` 可以保持纯布局，不再承担跨页面业务副作用。

## 页面容器规范（统一宽度/间距）

建议引入 `PageContainer` 作为页面内容的统一“外层容器”：

- `PageContainer` 是页面级宽度真相源，必须负责 `mx-auto` 居中，而不是把居中责任下放给各个页面。
- 默认内容宽度：`--ll-page-max-default = 80rem / 1280px`
  - 适用：`状态 / 设置 / 固件 / 关于` 这类阅读与表单混合页
- workspace 内容宽度：`--ll-page-max-workspace = 106.5rem / 1704px`
  - 适用：`总览` 这类多卡片、多列设备工作面
- full 容器：不设页面级上限，但密集仪表页面必须在内部 frame 自行定义并居中自己的最大宽度
  - 适用：`仪表盘`、`校准`
- 默认纵向间距：`gap-6`；更长页面可放宽到 `gap-8`
- 禁止同一页面同时混用“shell 1700px + inner 1280px + arbitrary mx-auto”而没有显式意图说明；这会在宽桌面上制造肉眼可见的不对称留白

### Responsive Breakpoints

- `320-767px`：mobile
  - 页面必须单列；主导航在 header 内横向滚动
- `768-1023px`：tablet / small laptop
  - 允许壳层换行，但左右 gutter 仍必须对称
- `1024-1439px`：desktop
  - 桌面设备 sheet 启用；多列布局在此区间开始稳定展开
- `1440-1728px`：wide desktop
  - 页面继续保持居中；新增宽度只转为等量左右边距
- `1729px+`：ultra-wide fallback
  - shell 与页面容器停止增长；额外空间留作对称外边距，不再继续拉伸阅读区

### 兼容屏幕基线

- `375x800`
- `768x1024`
- `900x800`
- `1200x800`
- `1440x900`
- `1728x1117`

Storybook viewport 集必须覆盖这些基线中的代表值；至少要能验证 mobile、tablet、desktop、wide desktop 与 ultra-wide fallback。

迁移时优先把各页面里分散的 `max-w-* mx-auto` 收敛到 `PageContainer`，减少视觉不一致。

## 兼容性与迁移策略

- **URL 不变**：通过 pathless layout route 插入 `ConsoleLayout`。
- **增量迁移**：
  1. 先引入 `PageContainer` 并替换各页面外层容器；
  2. 再引入 `DeviceLayout`，把 device 查找/错误处理/子导航上移；
  3. 最后重构 calibration mode 副作用的归属（从纯布局移出）。
- 迁移期间允许短期并存：未迁移页面仍可直接渲染在 `ConsoleLayout` 下。

## 测试计划（实现阶段）

- 静态检查：`bun run check` / `bun run lint`
- E2E：`bun run test:e2e`（覆盖基础导航与 device 子页面打开）
- 手工验证：
  - Devices 列表与添加设备；
  - 从 Devices 打开 CC/Status/Settings/Calibration；
  - 在 Calibration 与其它页面之间切换，确认 calibration mode 能可靠退出（设备端不残留）。

## 已确认决策

- Calibration：选择 **工具模式（B）**（全宽内容）
- 默认内容宽度基线：`max-w-5xl`

## 风险点与待确认

1. DeviceLayout 提供上下文的粒度：
   - 只提供 `device/baseUrl`，还是把 `identityQuery` 也上移统一管理？

2. 侧栏隐藏时的导航可用性：
   - 工具模式仍保留顶栏时，是否需要在顶栏提供“返回设备页/返回列表”的显式入口（避免用户迷路）？

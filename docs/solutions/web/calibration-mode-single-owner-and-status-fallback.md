# Calibration mode single owner 与 status fallback 模式

## 适用场景

- Web 页面既需要驱动设备进入某个运行模式，又需要消费该模式下才会出现的 RAW / 调试字段。
- 页面主要依赖 SSE 或其他长连接状态流，但设备连接资源较紧，容易出现短时断流。
- 页面还会从 localStorage / draft / URL 参数恢复局部 UI 状态，导致 mount 时的默认状态和真实目标状态发生竞争。

## 结论

- **设备模式写入必须单 owner。** 页签切换、按钮动作和初始化恢复都可以“提出目标 mode”，但真正调用 `postCalibrationMode(...)` 的地方必须唯一。
- **storage hydrate 必须早于自动 side effect。** 如果页面 mount 时先跑默认值驱动的 side effect，再从 storage 恢复真实页签，就会把设备短暂切到错误模式。
- **SSE 断流不等于页面离线。** 在嵌入式 HTTP 连接资源有限的设备上，短时断流或 worker 切换很常见；页面应该保留 last-good status，并自动回退到轻量 polling。
- **fallback polling 不能在正常 SSE 启动期就并行打开。** 否则页面一挂载就会同时占用 SSE + 轮询请求，反而把嵌入式 HTTP worker 挤爆；应等到 stream 真实报错或启动超时后再启用 fallback。
- **离开 calibration 页面时仍要保证 best-effort `off` 能落到设备。** 同设备子路由切换可以交给上层 `DeviceLayout` 统一 cleanup，但 calibration route 自己在整棵设备路由被卸载时仍要补一层 teardown，不能因为“只在 baseUrl 变化时 cleanup”就把设备遗留在 calibration mode。
- **RAW / 调试字段必须按 mode 做消费门控。** 当前页签的设备 mode 未对齐时，宁可显示“syncing”占位，也不要误展示旧模式的 RAW/DAC 数据。
- **动作链读取的 latest status 不能只依赖下一次 render。** 如果 `ensureMode()` 在暂停 SSE 时主动拉了一次 snapshot，就要同步更新共享 status ref；否则 `Capture` 这种紧跟在 mode sync 后的动作会读到旧的 `null`/过期 status。

## 推荐实现模式

1. 以页面当前目标 mode（通常来自 active tab）为单一真相源。
2. 用一个协调函数负责：
   - 发 `postCalibrationMode(...)`
   - 拉一次 snapshot fast path
   - 等待 SSE / snapshot 确认 `cal_kind`
3. 页面初始化时：
   - 先从 storage 恢复 active tab / draft
   - 再允许自动 mode sync effect 启动
4. 状态同步时：
   - SSE 成功收到消息 -> 标记 stream connected
   - SSE 出错 -> 不清空当前 status，只开启 fallback polling
   - fallback 成功 -> 更新 status，并在 SSE 恢复时停掉 fallback
5. 渲染时：
   - `deviceCalKind === expectedCalKind` 时才显示对应 RAW / DAC
   - 否则显示“正在同步模式”的轻量提示

## 本仓库落点

- 页面主逻辑：`/Users/ivan/Projects/Ivan/loadlynx/web/src/routes/device-calibration.tsx`
- Route Storybook：`/Users/ivan/Projects/Ivan/loadlynx/web/src/stories/routes/calibration-route.stories.tsx`
- Route harness：`/Users/ivan/Projects/Ivan/loadlynx/web/src/stories/router/route-story-harness.tsx`
- E2E 回归：`/Users/ivan/Projects/Ivan/loadlynx/web/tests/e2e/calibration.spec.ts`

## 为什么这套模式有效

- 它把“模式写入竞争”和“状态流瞬断”两个问题拆开处理：
  - 单 owner 消除 mode race；
  - last-good + fallback 消除状态瞬断带来的 UI 抖动。
- 它不需要修改设备 HTTP API shape，也不要求额外引入新协议字段。
- 对嵌入式设备尤其友好：即使 SSE 因资源紧张暂时断开，页面仍能用串行化 snapshot 维持可用状态。

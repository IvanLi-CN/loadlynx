# ESP32‑S3 HTTP 连接模型与前端 FIFO 探测设计

本文档记录数字板（ESP32‑S3）`net_http` 功能在 HTTP 连接处理与前端访问模式上的改动方案，目标是在常见浏览器并发场景下消除“设备在线但偶发 `ECONNREFUSED`”的问题，并为后续扩展提供清晰的设计基线。

## 1. 背景与问题

### 1.1 现状

- 数字板启用 `net_http` 后，通过 `firmware/digital/src/net.rs` 内的 HTTP 服务器处理以下端点：
  - `GET /api/v1/identity`
  - `GET /api/v1/status`（一次性快照 + SSE）
  - `GET/POST /api/v1/cc`
  - `POST /api/v1/soft-reset`
- 当前实现中，HTTP 服务器使用**单个** `TcpSocket` 在一个任务中循环：
  1. `stack.wait_config_up().await`
  2. `socket.accept(HTTP_PORT).await`
  3. `handle_http_connection(&mut socket, ...)`
  4. `socket.abort()`
- `/api/v1/status` 在 `Accept: text/event-stream` 情况下会升级为 SSE 流，并在对应连接存活期间持续写入事件。

### 1.2 问题表现

在以下场景中，前端偶发收到 `ECONNREFUSED`：

- 设备在线、Wi‑Fi 正常，SSE 和后续请求通常都能成功。
- 浏览器在短时间内对同一设备发起多条并发 HTTP 连接，例如：
  - 设备列表页对多个设备并发 `GET /api/v1/identity`；
  - CC 页面在首轮加载中并行请求 `/identity`、`/status`、`/cc`；
  - DevTools/预加载额外触发同源的预取请求。
- 在上述并发窗口中，对个别请求会直接返回连接拒绝（`ECONNREFUSED` / TCP RST），而非超时。

### 1.3 根因分析（嵌入式网络栈语义）

- `embassy-net` 的 TCP 模型中，并不存在独立的 `TcpListener`；而是通过**处于 `accept()` 调用中的 `TcpSocket` 集合**承担监听职责。
- 当**没有任何 `TcpSocket` 在执行 `accept()`** 时，新来的 SYN 将被直接拒绝（发送 TCP RST），对应到浏览器就是 `ECONNREFUSED`。
- 现有实现中：
  - 当唯一的 `TcpSocket` 被某个已建立连接（尤其 SSE `/api/v1/status`）占用并进入 `handle_http_connection` 时，系统中不再存在处于 `accept()` 状态的 socket。
  - 在此时间窗内，任何额外入站连接都无法被接受，只能被栈层直接拒绝。

结论：问题本质不是 `StackResources` 数量本身，而是“**只有一个 socket 监听且在处理期间没有其他 socket 处于 `accept()`**”，在存在长连接（SSE）和浏览器并发访问时必然会触发。

## 2. 目标与范围

### 2.1 设计目标

1. 数字板在启用 `net_http` 时，应在典型使用场景下避免出现“设备在线但偶发 `ECONNREFUSED`”。
2. 支持一条长期存在的状态流（SSE `/api/v1/status`）与一条控制通道（身份探测、状态快照、CC 控制、软复位等），在资源可控的前提下允许少量额外并发。
3. 前端在逻辑上将**单个设备的 HTTP 调用串行化（FIFO 控制队列）**，避免在一个浏览器实例内对同一设备产生过多并发。
4. 保持现有 HTTP API（路径与 JSON 模型）不变，对已有调用方透明。

### 2.2 范围与非目标

本次设计涵盖：

- 数字板固件：
  - HTTP 服务器从“单 socket 监听”改为“小型 worker 池”，确保始终有至少 1 个 `TcpSocket` 处于 `accept()`。
  - 为 SSE 保留长连接能力的同时，为控制请求提供独立的接受通道。
- Web 前端：
  - 对同一 baseUrl（同一设备）的 HTTP 调用引入简单 FIFO 队列，避免在单浏览器实例内对某个设备形成高并发。
  - 在设备首轮探测失败时，使用带抖动的重试与更友好的 UI 状态提示。

不在本次范围内的内容：

- 不新增外部网关/代理组件（例如独立 Node/Nginx 层），仅在设备与前端内部优化。
- 不修改现有 HTTP API 的路径、字段与语义。
- 不引入复杂的连接配额与全局速率限制（如“全局仅允许 1 条 SSE + N 条控制连接”的严格配额）；如后续需要会单独设计。

## 3. 设备端设计：HTTP worker 池

### 3.1 现有结构简述

当前 `firmware/digital/src/net.rs` 中的 HTTP server 采用“单任务 + 单 `TcpSocket` 循环”的结构，大致行为为：

- 初始化一对 RX/TX 缓冲区；
- 在循环中等待网络配置就绪；
- 创建一个 `TcpSocket`，设置超时；
- 调用 `socket.accept(HTTP_PORT)` 等待新连接；
- 将已建立连接交给 `handle_http_connection` 处理；
- 连接结束后调用 `socket.abort()` 并返回循环头。

特点：

- 单一任务，单一 `TcpSocket`。
- SSE 连接被占用期间，没有其他 socket 在 `accept()`。

### 3.2 目标结构：2 个 HTTP worker

设计方案（仅描述结构与职责，不在文档中展开完整代码）：

- 引入常量 `HTTP_WORKER_COUNT = 2`，表示 HTTP worker 的数量。
- 在网络初始化函数中：
  - 为每个 worker 分配一对独立的 RX/TX 缓冲区（保持与当前大小一致，例如 1024 字节）；
  - 为每个缓冲区对 spawn 一个 `http_worker` 任务。
- 每个 `http_worker` 任务执行以下循环：
  - 等待网络配置就绪；
  - 使用自身的缓冲区创建一个 `TcpSocket`；
  - 设置连接超时（保持与现有实现一致）；
  - 调用 `socket.accept(HTTP_PORT)` 监听并接受连接；
  - 将连接交给现有 `handle_http_connection` 处理；
  - 处理结束后调用 `socket.abort()` 并回到循环。

这样结构上变成“多个轻量 worker 共享同一个 `Stack` 和端口”，但每个 worker 拥有自己的 socket 和缓冲区。

预期行为：

- 至少 1 个 worker 可以被 SSE `/api/v1/status` 长连接占用；
- 另一个 worker 始终可用于接受短命令（`/identity`、`/status` snapshot、`/cc`、`/soft-reset` 等）；
- 在典型使用场景中（1 块板 + 1–2 个浏览器），不会再出现所有 socket 都不在 `accept()` 的情况，从而避免 `ECONNREFUSED`。

### 3.3 资源与超时考虑

- 缓冲区与任务：
  - 每个 worker 需要 2×1024 B 的 RX/TX 缓冲区，总计约 4 KiB；
  - 再加上任务栈，会带来有限但可接受的 SRAM 开销。
- `StackResources<6>`：
  - 当前 `StackResources<6>` 已在代码中配置，能够覆盖 2 个并发 TCP 连接（SSE + 控制）以及内部所需；
  - 若后续需要支持更多并发，可评估将 `StackResources` 提升为 8，并视实际内存占用调整。
- SSE 超时：
  - 现有实现统一使用 10 s 超时时间，并以 200 ms 间隔写 SSE 事件，正常情况下不会触发超时；
  - 本次改动不对超时策略做侵入式调整，仅保留现有逻辑。如后续定义“静默 SSE”模式（例如在 link_down 状态下降频），可单独优化。

## 4. 前端设计：按设备 FIFO 控制队列

### 4.1 现有访问模式

- `web/src/api/client.ts` 提供 `httpJson` 帮助函数，所有真实设备 HTTP 请求最终都通过它调用 `fetch`。
- `DevicesRoute` / `DeviceCcRoute` 中：
  - 设备列表页对每个设备并发 `GET /api/v1/identity`；
  - CC 页面在 `identity` 成功后启动：
    - `GET /api/v1/cc`；
    - `GET /api/v1/status` snapshot（带轮询）；
    - SSE `EventSource(/api/v1/status)`，并在成功收到 SSE 后停止轮询。
- 本地并没有针对单设备的“控制请求串行化”，即使 React Query 自身限制了重试和 refetch，浏览器仍可能对同一 baseUrl 并发多条连接。

### 4.2 目标：每个设备 1 条逻辑“控制队列”

设计原则：

- 对单个设备（同一 `baseUrl`）的 HTTP 请求，在逻辑上串行执行：
  - 包括 `/api/v1/identity`、`/api/v1/status` snapshot、`/api/v1/cc`、`/api/v1/soft-reset` 等；
  - SSE 使用浏览器内置的重连机制，单独存在。
- 对不同设备之间的请求仍然可以并行。

### 4.3 队列实现草案

在 `web/src/api/client.ts` 中：

1. 引入按 baseUrl 的 Promise 队列：
   - 使用一个 `Map<baseUrl, Promise>` 维护每个设备的“队尾 Promise”；
   - 每个新操作会链到对应设备的队尾，前一个操作结束（无论成功或失败）后再执行。

2. 基于上述队列封装 `httpJsonQueued`：
   - 函数签名与 `httpJson` 保持一致，增加 `baseUrl` 维度；
   - 内部通过队列包装实际的 `httpJson` 调用。

3. 调整真实设备相关 API：

- 对于 `isMockBaseUrl(baseUrl)` 为 `false` 的路径，将以下函数由 `httpJson` 改为 `httpJsonQueued`：
  - `getIdentity`
  - `getStatus`
  - `getCc`
  - `updateCc`
  - `postSoftReset`
- mock backend (`mock://`) 保持原有逻辑，不进入队列。

这样，对于每个设备：

- 所有控制类 HTTP 请求将串行进入队列执行；
- 任意时刻最多只有 1 条在途控制请求，减少对设备 HTTP 栈的瞬时压力。

### 4.4 重试与 UI 提示

为避免首轮探测时的短暂抖动导致“误报离线”，在现有 React Query 基础上增加：

- 对 `identity` / `status` 查询：
  - 配置有限次数重试（例如 `retry: 1` 或 `2`）；
  - 配置带抖动的退避（例如 `200–500 ms` 范围内随机）。
- 对 `status === 0 && code === "NETWORK_ERROR"` 或 `503 UNAVAILABLE/LINK_DOWN` 等明显可恢复错误：
  - 设备列表与 CC 页面显示“正在重试…”或“网络异常，稍后重试”类提示，而非立即标记为 Offline；
  - 保持现有 topError 展示链路错误的能力，便于调试。

SSE 相关逻辑保持不变：

- 一旦收到有效 SSE 事件，停止 snapshot 轮询；
- SSE 出错时恢复轮询，并在 UI 中做轻量级提示，而不是抛出未捕获异常。

## 5. 兼容性与风险

### 5.1 兼容性

- HTTP API：
  - 端点路径、方法与 JSON 模型不变；
  - 错误响应仍遵循统一 `ErrorResponse` 结构。
- 前端：
  - 上层行为对用户透明，只是在错误情况下更稳定、更友好；
  - mock backend 行为未变，开发体验不受影响。

### 5.2 风险与缓解

1. **SRAM 占用增加**
   - 额外引入 1 个 HTTP worker（总计 2 个），每个包含独立的 RX/TX 缓冲区和任务栈。
   - 缓解策略：
     - 保持 `HTTP_WORKER_COUNT = 2`，避免过度增加 worker 数；
     - 后续如需进一步压缩，可评估减小 HTTP 缓冲区或将 worker 数降为 1（在问题重新出现时再调回）。

2. **前端队列错误传播**
   - 若实现不当，前一个请求的异常可能导致队列 Promise 链“卡死”，使后续请求始终悬挂。
   - 缓解策略：
     - 队列内部始终在 `.catch` 分支中“吞掉”错误用于推进队列，但把实际错误重新抛出给调用方；
     - 对每个操作的错误在调用方独立处理，确保队列本身不会停滞。

3. **多浏览器并发同一设备**
   - 当前设计仅在单浏览器实例内部做 per‑device FIFO，不对不同设备、不同浏览器实例之间的并发做强约束。
   - 在“多个浏览器/标签页同时访问同一设备”的极端场景下，数字板仍可能看到 2 条以上控制连接并发。
   - 缓解策略：
     - HTTP worker 池本身能承受少量额外并发；
     - 如后续确有需求，可在固件中基于原子计数添加简单的并发限制（例如 “SSE 并发 >1 时返回 429 RATE_LIMITED”）。

## 6. 测试建议

1. **单浏览器基本功能**
   - 固件启用 `net_http` 并连接同一局域网。
   - 打开前端 `/devices` 页面：
     - 设备列表中所有设备应在数秒内从“Checking…”转为“Online”，无 `ECONNREFUSED` 报错；
     - 首轮探测若出现网络抖动，应看到短暂的“正在重试”提示而非立即 Offline。
   - 进入某设备的 CC 页面：
     - `identity`、`cc`、`status` 首轮加载成功；
     - “Link: up / Analog state: ready”等信息随 SSE 事件平滑更新。

2. **并发请求与 DevTools 场景**
   - 打开设备 CC 页面，同时打开浏览器 DevTools（Network 面板保持记录）；
   - 手动触发多次“Test connectivity”按钮以及快速多次提交 CC 修改：
     - 观察 Network 面板中的请求，确认没有 `ECONNREFUSED`；
     - 设备侧日志中 HTTP 处理任务正常（无大量 accept 错误或异常终止）。

3. **多标签页压力场景（可选）**
   - 在同一浏览器中打开两个 CC 页面标签，指向同一设备；
   - 重复第 2 步测试，确认在典型局域网环境下仍无明显错误。

本设计为数字板 HTTP 服务与前端访问模式在可靠性方面的 v1 调整基线；如后续在更高并发环境下暴露新问题，可在此基础上进一步引入连接配额、速率限制或边缘代理等增强措施。

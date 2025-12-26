# LoadLynx 网络控制接口与 Web 控制台需求

本文档描述基于 ESP32‑S3 Wi‑Fi 能力与新 Web 控制台的整体需求，用于支撑多设备发现、基础恒流（CC）模式远程控制，以及后续扩展。

## 1. 背景与目标

- 数字板：ESP32‑S3 作为主控，现有功能包含本地显示 UI、旋钮输入与与模拟板（STM32G431）之间的 UART 协议链路。
- 新目标：
  - 为 ESP32‑S3 增加可靠的 Wi‑Fi STA 联网能力，并提供基础 HTTP+JSON 控制/查询 API。
  - 在仓库根目录的 `web/` 下创建一个新的 Web App，用于多设备管理与 CC 模式远程控制。
  - 保持现有本地 UI 与 UART 协议行为不变，在其基础上叠加网络控制能力。

## 2. 范围与不在范围

在范围内：

- 新建 `web/` 前端子项目（与 `paste-preset` 工程规范保持一致）。
- ESP32‑S3 固件：
  - Wi‑Fi 配网（通过 `.env`/环境变量驱动的 STA 模式）。
  - 设备识别信息（ID、固件版本、网络信息）。
  - 基础 CC 模式控制 API（启停、目标电流、软限值）。
- Web 控制台：
  - 多设备列表 / 管理界面。
  - 单设备 CC 控制界面（滑块 + 状态展示）。
  - 使用前端 Mock 完成早期开发与交互验证，后续接入真实 ESP API。

暂不在范围内（可以作为后续扩展）：

- 自动化局域网扫描 / mDNS 发现（当前版本以“手动添加设备 + 连通性检测”为主）。
- 修改模拟板（STM32G431）固件协议字段（例如新增真正的“最小维持电压”参数）。
- 完整的认证/鉴权（当前假定在受控局域网内使用）。

## 3. 总体架构概览

- 硬件侧：
  - ESP32‑S3 通过 Wi‑Fi 以 STA 模式接入指定 AP，并运行一个 HTTP 服务器。
  - ESP32‑S3 通过现有 UART 协议（`loadlynx-protocol`）与 STM32G431 交互，维持 `FastStatus` 遥测与 SetPoint/LimitProfile/SoftReset/SetEnable 控制闭环。
- 软件侧：
  - 新 Web App（`web/`）通过 HTTP 调用 ESP32‑S3 暴露的 REST 风格 API。
  - Web App 使用 TanStack Router 管理路由，TanStack Query 管理数据请求与缓存，支持多设备。
  - 前端在开发阶段可通过 Mock 层模拟 ESP API，提前完成交互设计与状态管理。

## 4. Web 子项目需求（`web/`）

### 4.1 工程与技术栈要求

- 技术栈对齐 `paste-preset`：
  - 构建：Vite + React 19 + TypeScript。
  - 包管理与运行：Bun（`bun install`、`bun run`）。
  - Lint/格式化：Biome（`biome lint/check/format`）。
  - 端到端测试：Playwright。
  - 前端路由：TanStack Router（使用 path 路由配置）。
  - 数据请求：TanStack Query。
- 目录与配置：
  - 在 `web/` 下创建与 `paste-preset` 结构一致的工程骨架：
    - `package.json`：脚本与依赖定义；
    - `tsconfig*.json`：应用与 node 配置；
    - `vite.config.ts`：Vite 配置；
    - `playwright.config.ts`：E2E 配置；
    - `biome.json`：检查与格式化规则；
    - `lefthook.yml`：本地 Git 钩子配置；
    - `.github/workflows`：CI 与（可选）GitHub Pages 部署流程；
    - `scripts/write-version.mjs` 与 `.github/scripts/compute-version.sh`：版本号计算与 `version.json` 输出。
- 依赖版本策略：
  - 新增依赖（TanStack Router/Query 等）时，必须查阅官方文档，确保与当前 React/Vite/TypeScript 版本兼容。
  - 优先选择当前稳定版本（非 beta/rc），如需使用 next 版本应在文档中注明原因与风险。
  - CI 中固定 Bun 与 Node 版本（与 `paste-preset` 一致或更高），避免隐性兼容性问题。

### 4.2 路由设计

- 顶层路由形态：
  - 设备发现与管理页：`/devices`
  - 单设备功能页：`/:deviceId/:functionPath*`
    - 例如：
      - `/:deviceId/cc`：恒流控制主界面；
      - `/:deviceId/status`：详细状态页（可选后续扩展）；
      - `/:deviceId/settings`：设备级配置（保留扩展位）。
- 路由优先级与冲突避免：
  - 使用 TanStack Router 的 route config 显式声明静态 `/devices` 路由，并确保其优先于通配 `/:deviceId/*`。
  - 避免设备 ID 与函数路径重名导致的匹配歧义（例如不使用 `devices` 作为合法 `deviceId`），必要时对 `deviceId` 进行简单格式限制（如仅允许十六进制/短 UUID）。
- 设备 ID 体现：
  - 所有“具体设备功能页”必须在路径中包含 `deviceId`，以便：
    - 链接可被分享与书签化；
    - 日志/诊断时能够直接定位对应设备。

### 4.3 布局与交互概览

- 桌面布局（优先仿真 1440×900 及以上视口）：
  - 顶部页头（横向）：
    - 左侧：项目名称（如 “LoadLynx Control”），全局状态提示（例如 Agent 连接状态）。
    - 中部：当前设备选择器（下拉列表 / 搜索选择），仅在进入 `/:deviceId/...` 路由时激活。
    - 右侧：快捷操作（例如“添加设备”、“刷新状态”、“主题切换”等）。
  - 左侧竖向功能栏：
    - 全局入口：“设备发现与管理”（跳转 `/devices`）。
    - 当前设备相关功能：
      - “恒流控制（CC）”
      - “运行状态”
      - “设备设置”
  - 右侧主内容区域：
    - 按当前路由展示对应页面（设备管理或 CC 控制等）。

### 4.4 设备发现与管理（`/devices`）

- 功能定义：
  - 管理“已知设备”列表，而非实现完整的自动扫描。
  - 每个设备至少包含：
    - 显示名；
    - 设备 ID（与路由中 `deviceId` 一致）；
    - 基础访问 URL（例如 `http://host-or-ip:port`）；
    - 最近在线状态（通过调用 `/api/v1/identity` 或 `/api/v1/status` 得到）。
  - 支持的操作：
    - 添加设备（输入 ID 与 Base URL，前端验证可选地进行一次 Ping 调用）。
    - 编辑设备信息（名称、Base URL）。
    - 删除/隐藏设备。
- 存储策略：
  - 首版使用浏览器 `localStorage` 维护本地设备列表。
  - 后续可扩展为后端配置服务或同步配置文件。
- 路由行为：
  - `/devices` 不包含 `deviceId`，不依赖当前设备选择器。
  - 从 `/devices` 点击某设备进入具体功能时，跳转到 `/:deviceId/cc` 或其它设备路由。

### 4.5 CC 控制界面（`/:deviceId/cc`）

- 数据来源：
  - 来自 ESP API 的：
    - `FastStatus` 摘要（电压/电流/功率/温度/故障状态）。
    - 当前 CC 控制状态（启停、目标电流、软件限值配置）。
  - 页面通过 TanStack Query：
    - 轮询或刷新 `status`（例如 1–2 s 一次，可按前端设置调整）。
    - 使用 Mutation 提交控制命令。
- UI 结构（主视图）：
  - 上半部分：状态总览
    - 类似硬件显示界面，将主电压/主电流/功率以卡片或大数字展示。
    - 展示运行模式、故障标志、温度等概要信息。
  - 下半部分：控制区（滑块 + 开关）
    - 滑块 1：目标电流（恒流模式 setpoint）
      - 单位 mA/A，范围与当前硬件能力及软限值绑定（例如 0–5 A）。
    - 滑块 2：电压阈值（“最低维持”或“保护”）
      - 当前硬件协议仅提供 `LimitProfile.ovp_mv`（过压保护），没有“最低维持电压”字段：
        - 首版需求：UI 与 API 层预留该参数，暂不修改模拟板协议；
        - 实现方式：在数字侧应用策略（例如结合当前 `v_remote_mv` 与目标阈值，决定是否维持输出或建议用户降流），具体算法在后续设计时细化。
      - 支持模式切换开关：
        - “保护模式”：阈值触发后强制降流或断开；
        - “维持模式”：偏向提示/调节，而不立即断开（由数字侧算法实现）。
    - 滑块 3：功率上限（软保护）
      - 映射到 `LimitProfile.max_p_mw` 字段，实现软功率限制；
      - 通过与当前实测功率对比，决定是否自动降低 setpoint 或触发保护动作。
    - 公共控制开关：
      - “输出启停”：映射 `SetEnable.enable`，协调 `SetPoint` 发送与安全 gating。
      - “远程优先”开关（可选）：
        - 若开启，则以网络控制为主，旋钮更像“本地 override”；首版可以只实现“最新写入优先”策略，并在文档中描述。
- 交互行为：
  - 滑块移动时，采用节流与去抖动策略：
    - 本地 UI 即时更新预览值；
    - 向 ESP 发送命令时合并短时间内的多次变更（例如 100–200 ms 内只发送一次）。
  - 控制命令成功后，页面立即以返回值刷新状态，以免等待下一轮轮询。
  - 错误状态（如链路掉线、模拟板故障）在页面上高亮展示，并禁用相关滑块/开关。

### 4.6 Mock 接口与前端开发

- Mock 目标：
  - 在 ESP API 未稳定前，前端可以独立完成 UI/路由/状态管理及大部分交互。
- 建议方案：
  - 使用 MSW（Mock Service Worker）或类似工具，在开发模式下拦截 `/api/v1/*` 请求返回模拟数据。
  - 提供一组可配置的“虚拟设备”与 State：
    - 模拟正常运行、链路掉线、过温/过流故障等场景；
    - 模拟不同的限值配置和响应时间。
  - 在 E2E 测试中可结合 Playwright 的网络拦截能力，复用 Mock 逻辑。

## 5. ESP32‑S3 固件：联网能力与配置

### 5.1 Wi‑Fi 配置来源

- 配置源优先级：
  1. 仓库根目录 `.env` 文件（推荐命名）；
  2. 构建环境变量（`DIGITAL_WIFI_*`）。
- 建议配置键：
  - `DIGITAL_WIFI_SSID`：接入 AP 的 SSID。
  - `DIGITAL_WIFI_PSK`：密码（WPA2/WPA3 等）。
  - `DIGITAL_WIFI_HOSTNAME`：设备在局域网中的主机名（可选）。
  - `DIGITAL_WIFI_STATIC_IP`、`DIGITAL_WIFI_NETMASK`、`DIGITAL_WIFI_GATEWAY`、`DIGITAL_WIFI_DNS`：静态 IP 配置（可选，首版仍以 DHCP 为主，静态配置为备用方案）。
- `.env` 文件不纳入版本控制，防止泄露敏感信息。

### 5.2 build.rs 职责扩展

在现有版本号注入逻辑基础上，`firmware/digital/build.rs` 需要增加：

- 监视配置文件变更：
  - `cargo:rerun-if-changed=<repo_root>/.env`。
- 解析 `.env` 与环境变量：
  - 使用简单键值解析（`KEY=VALUE`），忽略注释与无效行；
  - 字段缺失时尝试从 `std::env::var` 读取。
- 将配置注入编译期环境：
  - 通过 `println!("cargo:rustc-env=LOADLYNX_WIFI_SSID=...")` 等形式导出；
  - 必须存在的字段（SSID/PSK）缺失时，构建失败并输出明确错误信息；
  - 避免把完整配置写入日志或版本文件，仅通过 `env!` 在代码中访问。

### 5.3 运行时联网流程（高层设计）

- 初始化顺序：
  1. 完成现有 SoC、外设、显示与 UART 链路初始化；
  2. 在不阻塞关键任务的前提下启动 Wi‑Fi 初始化任务；
  3. 若配置中启用 Wi‑Fi，则进入 STA 模式连接指定 AP。
- 连接管理：
  - 采用 Embassy/async 风格的 Wi‑Fi 驱动与网络栈（具体 crate 在实现时按最新文档选型）；
  - 支持重连与指数退避，失败时进行限频日志输出；
  - 维护一个简单的 `WifiState`：
    - `connected` 标志；
    - 当前 IP/MAC/网关/DNS；
    - 最近错误原因（超时、认证失败等）。
- 与其他任务的关系：
  - Wi‑Fi 连接失败或中断不应影响本地 UI 与 UART 链路；
  - HTTP API 仅在 `WifiState.connected == true` 且 IP 有效时对外提供服务，其他情况下返回适当错误。

## 6. ESP32‑S3 固件：设备识别与 CC 控制 API

### 6.1 设备识别

- 识别字段（通过 API 提供）：
  - `device_id`：建议基于 MAC 地址和一个短前缀生成（例如 `llx-XXXXXX`），确保在局域网内唯一且人类可读。
  - `digital_fw_version`：来自 `LOADLYNX_FW_VERSION`。
  - `analog_fw_version`：若已收到模拟板 `HELLO`，则从中解码/映射一个简要版本号；否则可为 `null` 或 `"unknown"`.
  - `protocol_version`：来自 `loadlynx-protocol::PROTOCOL_VERSION`。
  - 网络信息：`ip`, `mac`, `hostname`。
  - 运行时间：`uptime_ms`。
- API 端点示例：
  - `GET /api/v1/identity` → 返回上述字段。

### 6.2 基础状态查询

- API 端点建议：
  - `GET /api/v1/status`
    - 返回 FastStatus 的 JSON 映射，以及若干派生字段：
      - `mode`、`state_flags`、`fault_flags`；
      - `i_total_ma`、`v_main_mv`、`p_main_mw`；
      - 链路状态（如 `link_up`、`hello_seen`）；
      - 当前 `LimitProfile` 与 `SetEnable` 状态。
- JSON 映射约定：
  - 尽量保持与 `loadlynx-protocol` 结构一一对应，字段名采用小写加下划线或小驼峰（选一并在实现时固定）。
  - 对于位标志，提供：
    - 原始 `fault_flags`（整数）；
    - 解码后的数组（例如 `["OVERCURRENT", "SINK_OVER_TEMP"]`），便于前端直接展示。

### 6.3 CC 控制 API

- 统一控制模型（内部）：
  - 在数字板固件中引入一个集中管理的控制状态，例如：
    - `enable`（布尔，负载开关 / load switch，默认 `false`）；
    - `target_i_ma`（设置值 / setpoint，mA，UI 展示值；`enable=false` 时也允许为非 0）；
    - `effective_i_ma`（生效值 / effective，mA，实际下发 SetPoint.target_i_ma，`enable ? target : 0`）；
    - `limit_profile`（当前软限值）；
    - 控制来源标记（local/remote，用于未来行为差异化）。
  - 该语义为破坏性变更，需通过 `identity.capabilities.api_version="2.0.0"` 标识，便于客户端按版本适配。
  - 强制安全规则（A）：当 `target_i_ma == 0` 时必须强制 `enable=false`，避免从 0 调到非零时意外上负载。
  - 现有旋钮 UI 与遥测逻辑改为与该模型交互；SetPoint 的下行发送从该状态生成（`effective_i_ma`）。
    - 本次“负载开关”不使用 `SetEnable` 实现（硬件驱动/供电开关语义独立）。
- 建议 API 端点：
  - `GET /api/v1/cc`
    - 返回当前控制视图：
      - `enable`；
      - `target_i_ma`；
      - `effective_i_ma`；
      - 当前 `limit_profile`（max_i_ma, max_p_mw, ovp_mv, temp_trip_mc, thermal_derate_pct）；
      - 与当前 `FastStatus` 的关键测量值摘要（`i_total_ma`、`v_main_mv`、`p_main_mw`）。
  - `PUT /api/v1/cc`
    - 请求体示例：

      ```json
      {
        "enable": true,
        "target_i_ma": 1500,
        "max_p_mw": 60000,
        "ovp_mv": 40000,
        "voltage_mode": "protect",
        "power_mode": "protect"
      }
      ```

    - 行为：
      - 对输入字段进行范围检查（基于当前硬件能力与安全余量）；
      - 更新内部控制状态；
      - 应用 A 规则：`target_i_ma==0` 强制 `enable=false`；
      - 按需发送 SetPoint/LimitProfile 帧（SetPoint 使用 `effective_i_ma`）；
      - 返回更新后的完整 CC 状态。
- 错误处理：
  - `400 Bad Request`：参数缺失或超出安全范围。
  - `409 Conflict`：模拟板当前处于 Faulted/CalMissing 等不允许修改控制的状态。
  - `503 Service Unavailable`：UART 链路未就绪或 Wi‑Fi 未连接。
  - 错误响应统一包含错误码与适合在前端展示的简短消息。

### 6.4 辅助控制端点（预留）

- `POST /api/v1/soft-reset`
  - 触发现有 SoftReset 握手（带合适的 `SoftResetReason`），用于远程清故障与恢复。
- 将来可扩展：
  - 标定写入/读回；
  - 高级模式切换（CV/CP 等）。

## 7. Web 客户端接入 ESP API

### 7.1 TanStack Query 接入策略

- Query key 约定：
  - 单设备身份：`["device", deviceId, "identity"]`。
  - 单设备状态：`["device", deviceId, "status"]`。
  - CC 控制视图：`["device", deviceId, "cc"]`。
  - 设备列表（本地存储）：`["devices"]`。
- 轮询与刷新：
  - 对 `status` 可采用定时轮询（例如 1–2 s），也可在用户交互后手动 `refetch`；
  - 控制 API 调用成功后，主动更新/失效相关 Query，保证 UI 一致性。

### 7.2 路由与 API 的绑定

- 每个需要与某设备交互的页面，均通过 `deviceId` 参数与对应 Base URL 解析出 API 根路径：
  - 例如在设备管理中记录 `baseUrl`，在进入 `/:deviceId/...` 路由时从设备列表中查出对应地址；
  - 最终请求 URL 形如下：
    - `${baseUrl}/api/v1/identity`
    - `${baseUrl}/api/v1/status`
    - `${baseUrl}/api/v1/cc`
- 当无法在本地找到 `deviceId` 对应的配置时：
  - 显示“未配置设备”错误提示；
  - 引导用户跳转 `/devices` 进行添加或修复。

### 7.3 设备发现与控制联动

- 在 `/devices` 页面：
  - 对每台设备调用 `/identity`（必要时加上 `/status`）检测在线状态。
  - 根据结果更新设备卡片上的“在线/离线”标记与摘要信息（版本号、IP 地址等）。
- 在 `/:deviceId/cc` 页面：
  - 首次进入时先加载 `/identity`，校验设备类型和协议版本；
  - 随后启动 `/status` 和 `/cc` 的 Query，用于刷新 UI；
  - 若发现协议版本不兼容或 API 缺失，可以显示“版本不匹配”提示，并引导用户进行固件升级。

## 8. 实施与验证要点

- 前端：
  - 初始化阶段优先完成工程骨架与路由/布局，再接入 Mock API；
  - 确保 CI（lint/check/build/test:e2e）在本仓库通过；
  - 在桌面视口下验证布局与交互，后续再评估移动端适配。
- 固件：
  - 在开发板上验证 Wi‑Fi 连接的稳定性与重连行为；
  - 确认 HTTP API 响应时间不会明显干扰 UART 链路与 UI 刷新；
  - 在日志中增加有限的联网与 API 访问记录，便于现场诊断。
- 联调：
  - 先用 Mock 完成前端逻辑，再切换为真实设备并对照 FastStatus/UI 行为；
  - 对典型使用场景（正常运行、过流/过温、链路掉线）编写简单的手工测试用例，以备后续 PR 中引用。

# mDNS 与局域网发现设计草案

## 背景与目标

数字板（ESP32-S3）当前使用 `esp-hal` + `esp-wifi` + `embassy-net` 提供 HTTP API（端口 80），Web 前端通过 `baseUrl` 与每台设备通信，并已经为 CORS 与 Private Network Access 配置了必要的响应头，允许远端托管的 Web 前端访问局域网中的设备。

现状问题是：用户需要手动输入设备 IP 地址，既不友好又容易出错。硬件侧已经具备本地显示屏，可以在设备上显示名称与网络信息。本设计希望利用 mDNS/.local 主机名和有限范围的局域网扫描，改善设备发现体验，同时保持实现简单、行为可预测。

目标概括如下：

- 为每块数字板提供稳定可读的 `.local` 主机名，并通过 mDNS 在局域网内广播，使 `http://loadlynx-XXXX.local` 这一类 URL 可用；
- 在 Web 前端提供一个受控的“局域网扫描”入口，基于已有的 `/api/v1/identity` 接口，在小范围子网内发现同网段的设备，降低手动录入成本；
- 兼顾企业网络 / 复杂 Wi-Fi 拓扑中 mDNS 与主动扫描可能受限的现实，给出合理的降级路径与文案提示。

本设计仅覆盖需求分析和概要设计，不包含具体实现代码。

## 范围与非目标

### 本次范围

- 设备侧（ESP32 数字板）
  - 设计在现有 `esp-hal + esp-wifi + embassy-net` 网络栈上新增 mDNS 支持的方案；
  - 约定 mDNS 主机名 / 服务名命名规范，以及如何从硬件 ID 派生短 ID；
  - 设计 mDNS 任务模型：如何绑定 UDP 5353、加入多播组、周期性广播与响应查询；
  - 定义与现有 `net.rs`/网络任务的集成方式，确保 mDNS 故障不会影响主业务逻辑。

- Web 前端
  - 设计“通过 `.local` 主机名手工添加设备”的推荐路径与 UI 提示；
  - 设计“点击扫描局域网（有限子网）”的交互流程和技术约束：
    - 扫描范围、并发与超时时间；
    - 如何通过 `/api/v1/identity` 严格识别 LoadLynx 设备；
    - 在企业网络等环境下的风险与降级策略。

### 非目标

- 不实现实际 mDNS 协议代码或引入具体 crate，仅选定优先方案（自实现 vs 第三方库）；
- 不改动 MCU Agentd、硬件电路或 bootloader；
- 不解决跨网段、跨 VLAN 的集中发现问题，该类需求应通过单独的 discovery 服务或本地 helper 处理；
- 不为扫描行为增加新的后端服务端点，扫描逻辑限定在浏览器执行。

## 关键用例与流程

### 用例 1：通过 `.local` 手动添加设备

1. 数字板上电并连接到指定 Wi-Fi，`embassy-net` 完成 IP 获取（DHCP 或静态）。
2. 固件根据设备唯一 ID（例如 MAC 地址）生成短 ID：
   - 例如取 MAC 地址后 3 字节，编码为 6 位十六进制：`a1b2c3`；
   - 设备主机名为 `loadlynx-a1b2c3`，mDNS 域名为 `loadlynx-a1b2c3.local`。
3. mDNS 任务加入多播组 `224.0.0.251:5353`，开始对 `loadlynx-a1b2c3.local` 的 A 记录查询进行应答；可选地同时发布 `_loadlynx._tcp.local` 的 DNS-SD 服务记录（指向端口 80）。
4. 设备屏幕显示：
   - `Name: loadlynx-a1b2c3`
   - `Host: loadlynx-a1b2c3.local`
   - `IP: 192.168.1.23`
5. 用户在 Web 前端的“添加设备”对话框中输入 `http://loadlynx-a1b2c3.local` 作为 `baseUrl`；
6. Web 前端通过 `GET /api/v1/identity` 对该 URL 进行验证：
   - 返回的 JSON 中携带固定的 LoadLynx 识别字段（例如 `device_id`、`fw_identity` 前缀等）；
   - 验证通过后，将该设备加入设备列表。
7. 若 `.local` 在当前系统或网络中不可解析，前端提示用户改用屏幕展示的 IP 地址重试，并简要说明可能原因（系统或网络未启用 mDNS、多播被屏蔽等）。

### 用例 2：点击扫描当前子网

1. 用户打开 Web 前端的“添加真实设备”对话框，点击“扫描当前网络”按钮。
2. 前端推断浏览器当前所处的本地 IP 与子网掩码（优先简单场景）：
   - 例如浏览器本机地址为 `192.168.1.50`，则默认扫描 `192.168.1.0/24`；
   - 不支持在 UI 中随意输入任意网段，避免被用作端口扫描工具。
3. 前端弹出说明提示：
   - 将在当前子网内通过 HTTP 请求尝试发现 LoadLynx 设备；
   - 扫描需要数秒时间，在部分企业或受管网络中可能被限制或拦截；
   - 此操作仅在用户主动点击时执行，不在后台自动反复扫描。
4. 前端按以下策略对候选 IP 发起 `GET http://<IP>/api/v1/identity` 请求：
   - 同时进行的请求数限制在 16–32 个连接；
   - 单个 IP 超时时间约 300–500 ms；
   - 对网络错误或超时直接忽略，不重试；
   - 收到 200 响应后，解析 JSON，只有在识别字段匹配预期（例如包含 `device_id` 且有固定前缀，`fw_identity` 符合 LoadLynx 版本格式）时，才视为有效设备。
5. 扫描完成后，前端展示发现的候选设备列表，包括：IP 地址、（如有）`identity.hostname`、固件版本等；
6. 用户勾选要添加的设备，点击“添加到列表”，前端将以 IP 为 `baseUrl` 存储，必要时在显示名中附加 hostname 或短 ID；
7. 后续 Web 前端仍通过已有的 API 与设备交互，不依赖扫描结果持续更新。

## 数据与领域模型

### 设备标识与命名

固件和 Web 前端需要对“设备标识”使用一致的抽象，以支持 `.local` 名称和短 ID 显示：

- 设备侧已有字段：
  - `identity.device_id`：当前用于唯一标识设备的 ID（字段已存在）。

- 建议新增或增强字段（接口/显示层面，实际存储形式可按现有实现微调）：
  - `identity.hostname: Option<String>`：
    - 对支持 mDNS 的固件，填充为 `loadlynx-<short-id>.local`；
    - 对旧固件/不支持 mDNS 的场景可为空；
  - `identity.short_id: Option<String>`（可选）：
    - 与屏幕显示和 mDNS 主机名共享的短 ID（例如 MAC 后 3 字节）。

### Web 侧设备模型（概念）

Web 前端的设备配置（保存在浏览器本地存储或后端）应包含以下信息：

- `baseUrl: string`：与设备进行 HTTP 通信的基础 URL（可以是 IP 或 `.local`）；
- `displayName?: string`：设备的人类可读名称，可由用户自定义；
- `preferredHost?: string`：记录 `.local` 名称或 IP 地址，供 UI 展示 与后续迁移使用；
- `lastSeenAt?: Date`：上次成功访问设备的时间戳，用于扫描后更新状态。

具体字段名可以在实现阶段与现有 `device-store` 结构对齐，设计重点是保留“通信 URL / 显示名 / 主机名 / 最近活跃时间”这四类信息。

## 接口与模块边界

### 数字板固件（ESP32）侧

#### 新增 mDNS 模块

建议在 `firmware/digital/src` 下单独引入模块，例如 `mdns.rs`，职责集中在 mDNS 报文的收发与配置：

- 对外接口草案：

```rust
pub struct MdnsConfig {
    pub hostname: &'static str,      // "loadlynx-a1b2c3.local"
    pub service_name: &'static str,  // "_loadlynx._tcp.local"（可选）
    pub port: u16,                   // 80
}

pub async fn mdns_task(
    stack: &'static embassy_net::Stack<WifiDevice<'static>>,
    cfg: MdnsConfig,
) {
    // 内部负责 socket 初始化、多播加入、报文收发
}
```

- 内部行为：
  - 使用 `embassy_net::udp::Socket` 绑定 UDP 5353 端口；
  - 通过 `stack` 加入 `224.0.0.251` 多播组；
  - 周期性发送 A 记录广播（将 `hostname` 解析为当前 IPv4 地址）；
  - 如启用 DNS-SD，则额外发送 `_loadlynx._tcp.local` 的 PTR/SRV/TXT 记录；
  - 监听 mDNS 查询，当问题匹配 `hostname` 或服务名时构造并返回应答；
  - 所有错误通过日志记录，不向上传播为致命错误。

#### 与现有网络任务的集成

在 `firmware/digital/src/net.rs` 中，当前已有：

- Wi-Fi 连接与 `embassy_net::Stack` 的初始化；
- HTTP 服务器任务 `http_worker` 以及关联的 `spawn_wifi_and_http(...)` 辅助函数。

集成方式建议如下：

1. 在 `spawn_wifi_and_http(...)` 中，在 `embassy_net::new(...)` 得到 `stack` 后，构造 `MdnsConfig`：
   - `hostname`: 基于短 ID 拼接 `loadlynx-<short-id>.local`；
   - `service_name`: 固定为 `_loadlynx._tcp.local`（如启用 DNS-SD）；
   - `port`: 固定为 80。
2. 使用 `spawner.spawn(mdns_task(stack, cfg))?;` 启动 mDNS 任务；
3. 确保 mDNS 任务与现有 `net_task`、`wifi_task` 生命周期一致或稍后启动，但不对它们的错误处理造成影响。

### Web 前端侧

#### 扩展设备 hooks

在 `web/src/devices/hooks.ts` 中，除现有获取设备列表、添加设备的 hooks 外，增加：

- `useScanSubnetMutation()`（命名示例）：
  - 输入：可选参数 `scanOptions`，例如是否允许扩展子网范围；
  - 行为：根据当前上下文推断子网，对 IP 列表进行有限扫描，返回候选设备数组；
  - 输出：
    - `devices: DiscoveredDevice[]`，包含 IP、hostname（如从 `/identity` 获取）、固件版本等；
    - `isScanning: boolean`、`progress: number` 等状态；
    - 错误信息（例如扫描被浏览器或网络策略禁止）。

#### 更新设备管理界面

在 `web/src/routes/devices.tsx` 中：

- 在“添加真实设备”的对话框中：
  - 增加输入示例文案：
    - “推荐使用设备屏幕显示的 `.local` 名称，例如 `http://loadlynx-a1b2c3.local`；若无效则改用 IP 地址”；
  - 增加“扫描当前网络”按钮：
    - 点击后调用 `useScanSubnetMutation`，并展示进度与提示说明；
    - 扫描完成后，在对话框下方展示候选设备列表；
    - 用户勾选后批量添加，将 IP 作为 `baseUrl` 存储。

#### Web → 设备接口约束

- 发现与验证统一通过已有的 HTTP API：
  - `GET /api/v1/identity`
- 不新增专用 discovery API 或中间层服务；
- 利用响应中的既有字段（例如 `device_id`、固件版本字符串）作为 LoadLynx 设备的判别依据，必要时可以在实现阶段为 identity JSON 增加一个简单的 magic 字段或固定前缀，以降低误识别风险。

## 兼容性与迁移

### 旧固件与无 mDNS 场景

- 旧版本固件可能不提供 mDNS：
  - `identity.hostname` 缺省或为空；
  - Web 前端通过字段是否存在来决定是否展示 `.local` 相关文案；
- 屏幕显示策略：
  - 即使 mDNS 不可用（网络或系统不支持），屏幕仍然显示短 ID 与 IP，作为用户定位设备的兜底手段；
  - `.local` 无法访问时，前端提示用户回退到 IP。

### 浏览器与操作系统

- 不同平台对 `.local` / mDNS 的支持差异较大：
  - macOS/iOS/Linux/Android 通常内置 mDNS 支持；
  - Windows 需要 Bonjour 或系统自带 mDNS 服务，部分企业镜像可能禁用；
  - 一些路由器/AP 会屏蔽或隔离多播流量。
- 设计层面通过：
  - UI 文案明确 `.local` 的期望和失败时的替代方案；
  - 不将 `.local` 作为唯一访问方式，而是作为“更友好”的首选选项。

### 网络环境

- 局域网扫描仅针对简单拓扑（家庭/实验室）设计：
  - 默认只扫描浏览器当前所处地址的 /24 网段；
  - 不支持选择多个网段或跨 VLAN 扫描；
  - 只在用户显式操作时执行，避免后台持续流量。
- 在受管网络或复杂企业环境下，上述扫描可能被防火墙或 IDS 限制：
  - 前端应在提示中说明扫描可能失败，并明确这不影响手工添加 IP / `.local` 的主流程。

## 风险与注意事项

### mDNS 相关

- 多播可达性：
  - 一些 Wi-Fi AP 对 224.0.0.251 的转发存在问题，可能导致 `.local` 名称在部分环境下不可用；
  - 需要在文档中说明 `.local` 是“尽力而为”，而非硬依赖。

- 资源占用：
  - mDNS 任务需要一个 UDP socket 和若干缓冲区，占用少量 RAM；
  - 需要在实现阶段关注 embassy-net 的多播支持细节，避免与现有 UDP 使用模式冲突。

### 局域网扫描

- 安全感知：
  - 对 /24 网段进行并行 HTTP 探测在部分安全策略严格的网络中可能被视作端口扫描行为；
  - 必须将扫描设计为“用户点击一次，短时间内完成”的操作，并加入说明提示。

- 性能与体验：
  - 过大的并发数或过长的超时会导致页面卡顿或路由器压力较大；
  - 建议并发连接数控制在 16–32 之间，超时控制在 300–500 ms 之间，并支持显示进度条或简单百分比。

### 命名与一致性

- 短 ID 生成规则一旦确定，应保持稳定：
  - 例如明确“使用 MAC 后 3 字节，十六进制小写”；
  - 规则变更会导致屏幕显示的名称与历史文档或用户记录不一致，需要谨慎。

## 推荐选项与后续工作

- 短 ID：采用 MAC 地址后 3 字节，编码为 6 位十六进制小写；
  - 设备主机名：`loadlynx-<short-id>`；
  - mDNS 域名：`loadlynx-<short-id>.local`；
  - 屏幕上统一使用 `loadlynx-<short-id>` 进行展示。

- mDNS 功能：
  - 第一阶段至少实现主机名解析（A 记录应答），保证在支持 mDNS 的系统上 `.local` 可访问；
  - 如实现成本可控，则同时发布 `_loadlynx._tcp.local` 的 DNS-SD 服务，为未来基于 DNS-SD 的 discovery 打好基础。

- 局域网扫描：
  - 限定在浏览器当前 /24 网段，用户显式点击触发；
  - 使用 `/api/v1/identity` 作为唯一识别入口，严格检查返回结构以避免误判；
  - 在 UI 中增加清晰提示，特别提醒在企业网络中的使用限制。

后续在实现阶段，可以在当前分支 `feat/mdns-discovery` 上：

1. 在 `firmware/digital` 中增加 `mdns.rs` 并在 `net.rs` 集成 mDNS 任务；
2. 扩展 `/api/v1/identity` 的 JSON，增加 `hostname`（以及可选的 `short_id`）；
3. 在 Web 前端实现 `.local` 添加路径与局域网扫描 hooks/UI；
4. 在本地网络和至少一个受管网络环境中做实际验证，评估 mDNS 与扫描行为的可用性与风险。

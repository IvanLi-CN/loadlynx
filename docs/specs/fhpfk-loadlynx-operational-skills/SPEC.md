# LoadLynx operational skills packaging and workflow boundary（#fhpfk）

## 状态

- Status: 已更新；用户侧硬件操作仅允许 CLI。CLI 硬件记忆已实现为用户级配置，CLI WiFi 配置仍是实现门槛，不能在 skill 中伪装成已发布能力。

## 背景 / 问题陈述

LoadLynx 需要两份 agent skill。用户版面向没有源码工程的普通用户机器，必须围绕 GitHub Releases 发布的程序和固件资产组织；开发者版是用户版的超集，面向源码工程、工具链、发布维护与 HIL 调试，必须先继承用户版的 CLI-only 业务操作边界，再确认或准备 checkout。

之前的拆分把用户侧写成 Web/CLI 混合操作，容易让 agent 通过 Web UI 操作硬件。最终边界应当更严格：skill 驱动的用户侧硬件操作只允许 released `loadlynx` CLI；连接顺序是 USB/devd 优先，HTTP 其次；CLI 还必须负责记忆用户保存或连接过的硬件，方便后续查找。

## 目标 / 非目标

### Goals

- 保持两个可安装 skill：
  - `loadlynx-user-operations`：普通用户机器；只使用 GitHub Releases 发布的 host tools 与固件资产。
  - `loadlynx-developer-operations`：开发/维护机器；继承用户版 CLI-only 业务操作、USB 优先/HTTP fallback、CLI 硬件记忆与能力门禁，再验证本地工程，必要时 clone canonical repo，最后使用源码命令。
- 用户版覆盖 released `loadlynx` CLI 硬件操作、USB/devd 优先连接、HTTP fallback、CLI 硬件记忆、设备身份/状态/遥测、电子负载输出与模式/预设/设定值能力自检、USB-PD 能力自检、GitHub Release 固件下载、CLI 烧录，以及 CLI WiFi 配置能力自检。
- 开发者版覆盖用户版全部业务能力边界，并额外覆盖源码构建、`just` 命令、devd/CLI 本地运行、release 资产维护、设备业务能力在 CLI/devd/firmware 中的实现与验证、固件、WiFi、校准、reset/monitor、HIL。
- 所有用户侧 CLI 功能必须先验证当前安装的 release 确实实现；缺失命令必须停止并升级为开发者实现/发布工作。
- 保持 `vercel-labs/skills` 能从仓库发现并安装两个 skill。

### Non-goals

- 不把缺失的 CLI WiFi 配置写成已实现功能。
- 不在 skill 文档内设计完整桌面安装器、代码签名、公证或自动更新。
- 不在本 spec 直接实现固件协议、devd API 或 CLI WiFi 命令；这些属于开发者 skill 引导的后续源码工作。

## 范围

### In scope

- `skills/loadlynx-user-operations/`
- `skills/loadlynx-developer-operations/`
- `AGENTS.md` 的 skill 路由说明。
- `README.md` 中 released host tools、用户/开发路径边界说明。
- `tools/loadlynx-devd/src/bin/loadlynx.rs` 的用户级 CLI 硬件记忆。
- `.github/workflows/release.yml` 的发布资产要求。
- `docs/specs/README.md` 的规格索引。

### Out of scope

- 桌面 GUI installer。
- WiFi runtime 配置协议/CLI 的具体实现。
- Web UI、固件控制协议或 devd API 的大范围重构。

## 术语定义

- `host-tools`：GitHub Releases 发布的用户机器工具包，文件名为 `loadlynx-host-tools-<platform>.tar.gz`。
- `loadlynx-devd`：`host-tools` 内的本地 USB CDC bridge 守护程序。`serve` 提供 CLI IPC；`bridge-http` 提供 loopback-only 浏览器 bridge。
- `loadlynx`：`host-tools` 内的 released CLI。当前源码可见命令包括 `discover`、`devices`、`status`、`output set`、`usb-port set`、`hardware`、`flash`、`reset`、`monitor`；是否包含 WiFi 命令必须以用户安装版本的 `loadlynx --help` 为准。
- CLI 硬件记忆：`loadlynx hardware ...` 维护的用户级设备清单，覆盖可连接设备、最近连接设备、已记住设备、保存设备与遗忘设备。默认路径为 macOS `~/Library/Application Support/LoadLynx/devices.json`，Linux `${XDG_CONFIG_HOME:-~/.config}/loadlynx/devices.json`，Windows `%APPDATA%\LoadLynx\devices.json`；`LOADLYNX_HOME` 可覆盖目录。
- 固件 catalog：GitHub Release 中用于描述固件 artifact、target、hash、flash 文件的 JSON 文件；devd/CLI 烧录必须基于 catalog 或等价 artifact 选择结果校验 hash 和 target。
- source checkout：包含 `Justfile`、`tools/loadlynx-devd/Cargo.toml`、`firmware/analog`、`firmware/digital` 的 LoadLynx 仓库工作区。

## 需求列表

### MUST

- 每个 skill 必须包含 `SKILL.md` 和 `agents/openai.yaml`。
- `vercel-labs/skills` 必须能从仓库发现并安装两个 skill。
- 用户版不得要求源码 checkout、`just`、Rust、Bun、`mcu-agentd`、`espflash` 或 `probe-rs`。
- 用户版必须提供 host-tools 安装流程：选择 GitHub Release、运行 `install-loadlynx-host.sh` / `install-loadlynx-host.ps1` 或手动下载平台 archive、用 `SHA256SUMS` 校验、安装到用户目录、只输出 `PATH` 提示、验证 `loadlynx-devd --help` 和 `loadlynx --help`。
- 用户版通过 skill 操作硬件时只支持 `loadlynx` CLI，不得使用 Web UI、raw HTTP 写入、浏览器 local storage 或 Web 控制台作为 agent 硬件操作路径。Web Serial 是正式人类浏览器路径，但不是 skill-driven agent 操作路径。
- 用户版连接顺序必须是 USB/devd 优先，HTTP 其次。HTTP 只在 USB 不可用、用户明确选择 HTTP、或使用已保存 HTTP 设备时作为 fallback。
- 用户版必须要求 CLI 记忆用户保存或连接过的硬件，包括可列出可连接设备、列出最近连接设备、列出已记住设备、选择、更新、遗忘曾经连接过的 USB 与 HTTP 设备；同一硬件同时有 USB 与 HTTP 记录时优先 USB。当前实现使用 `loadlynx hardware available/recent/path/list/save/forget`、`loadlynx status --hardware <id>`，并在 `status --device` / `status --url` 成功后 best-effort 自动更新用户级记忆，自动记忆写入失败不得遮蔽成功的 status 查询。
- 用户版必须按业务功能组织硬件操作：身份与状态确认、遥测查看、电子负载输出控制、CC/CV/CP/预设能力自检、USB-PD 能力自检、固件更新、WiFi 配置能力自检。连接方式和硬件记忆只是进入业务操作的前置步骤。
- 用户版只能展示 installed CLI 真实支持的业务写操作；若 CLI 尚未提供 preset、CC/CV/CP 或 USB-PD 设置命令，必须明确阻断并升级为开发者实现/发布工作。
- 用户版必须覆盖从 GitHub Release 下载固件资产/catalog 的流程；如果 Release 缺少必要固件资产/catalog，必须停止并报告该 release 不支持用户侧固件烧录。
- 用户版必须覆盖 CLI 烧录流程，但只能在当前 `loadlynx flash --help` 显示所需参数、devd 可用、target/artifact/lease/hash 证据齐全时继续。
- 用户版必须要求真实 ESP32-S3 flash 具备 artifact/hash/target evidence、owner 明确确认、非项目固件风险确认（如适用）和 post-flash identity capture；不能只凭 flash 命令成功宣称设备可用。
- owner 确认可以是自然语言；skill 不得要求 owner 以固定短语或固定命令字符串回答，只要动作与目标清楚无歧义即可。
- 用户版必须覆盖 CLI WiFi 配置入口，但只能在当前 `loadlynx --help` 暴露已实现 WiFi 命令时给出步骤；如果命令不存在，必须明确报告未实现，不得伪造命令。
- 用户版不得使用源码命令、Web UI、raw HTTP 写入、手动编辑端口缓存或 developer selector 绕过 released CLI 能力缺口。
- 开发者版必须先验证 source checkout；如果没有 checkout 且任务需要源码工作，才允许执行：

```bash
git clone https://github.com/IvanLi-CN/loadlynx.git
cd loadlynx
```

- 开发者版必须通过 `just` 运行本地 CLI/devd/固件常用入口：`just devd-build`、`just devd-test`、`just devd-serve`、`just loadlynx <args>`、`just a-build`、`just d-build`。
- 开发者版必须保留项目开发端口/探针缓存、devd lease、`mcu-agentd`、WiFi secret、校准、flash/reset/monitor 与 HIL 的安全门禁；具体缓存文件名只在开发者 skill 中列出。
- Release 维护不得发布声称支持用户 CLI 烧录或 WiFi 配置、但缺少对应程序/固件资产/命令实现的 Release。
- Release 维护必须发布 installers、host-tools archives、firmware catalog/assets、Web bundle 和覆盖全部资产的 `SHA256SUMS`。

### SHOULD

- 用户版优先 USB/devd，然后 HTTP；mDNS 只作为 HTTP fallback 的便利项，并保留手工 URL/IP fallback。
- 开发者版应把缺失工具作为 setup/maintenance 问题，而不是把普通用户引导进源码构建。

## 功能与行为规格

### 用户技能

- 适用对象：普通用户、owner-facing 操作、没有源码 checkout 的机器。
- 默认入口：GitHub Releases 发布的 `host-tools` 与 firmware assets。
- CLI-only：skill 驱动的硬件操作只允许 released `loadlynx` CLI。
- USB 优先：CLI 通过 Unix socket / Windows named pipe 连接 `loadlynx-devd serve`，并可由 `loadlynx` sibling auto-start。
- HTTP 其次：当 USB 不可用或用户选择 HTTP 时，用 CLI 对 explicit URL/IP/mDNS host 操作。
- 硬件记忆：CLI 负责保存、列出可连接设备、列出最近连接设备、列出已记住设备、选择、更新、遗忘曾经连接过的 USB 与 HTTP 设备，后续操作优先使用保存的 USB 设备；该记忆属于用户配置目录，不属于源码 checkout 或项目开发端口缓存。
- 业务能力：通过 CLI 完成设备身份确认、状态/遥测读取、输出开关、电子负载 CC/CV/CP/预设能力自检与已发布命令调用、USB-PD Fixed/PPS 能力自检与已发布命令调用。
- 固件：下载 Release 固件 catalog/assets，先 dry-run，再在身份、target、artifact、hash、lease/session、owner 明确确认和 post-flash identity capture 清楚时 real flash。
- WiFi：只使用 released CLI 中真实存在的 WiFi 命令；保护 PSK，不把密钥写入聊天、日志、截图、trace 或 PR。
- 禁止项：clone、`just`、源码构建、`mcu-agentd`、probe selector、HIL、校准写入、raw HTTP 绕过、手改缓存。

### 开发者技能

- 适用对象：开发、维护、调试、发布验证与 HIL。
- 默认入口：验证 checkout；没有 checkout 且任务需要源码时 clone canonical repo。
- 允许项：构建、测试、devd、CLI、Web、release workflow、USB CDC 证据、固件烧录、WiFi 实现/配网、校准、reset/monitor、`mcu-agentd`、HIL。
- Web Serial：GitHub Pages 与 release Web bundle 的正式人类路径，使用 `esptool-js`、release catalog/assets、浏览器授权端口、identity/profile memory；不保存 OS 端口路径。
- 业务开发项：把用户需要的身份/状态/遥测、电子负载输出/预设/CC/CV/CP、USB-PD 设置、固件更新、WiFi 配置落实到 firmware/protocol/devd/CLI/release 的完整链路。
- 禁止项：在非项目上下文中猜路径或对普通用户机器执行开发命令。

## SKILL 大纲

### `loadlynx-user-operations`

- Frontmatter:
  - `name`: `loadlynx-user-operations`
  - 触发范围：普通用户机器上的 released host-tools、GitHub Release 固件、USB 优先 / HTTP fallback、CLI 硬件记忆、released CLI 烧录/WiFi。
- Start Here:
  - `npx skills add https://github.com/IvanLi-CN/loadlynx --skill loadlynx-user-operations`
  - 验证 `loadlynx --help` / `loadlynx-devd --help`。
  - 缺失命令时停止，不切换到源码命令。
- Install Released Host Tools:
  - 从 GitHub Releases 选择 stable 或 owner 接受的 prerelease。
  - 下载 `loadlynx-host-tools-<platform>.tar.gz`。
  - 安装到用户目录，配置用户级 `PATH`，验证两个二进制。
- Connect Hardware:
  - USB/devd 优先，HTTP fallback 其次。
  - USB 时使用 `loadlynx-devd serve` 的默认 IPC endpoint，必要时用 `--endpoint <path-or-pipe>` 覆盖。
  - 只使用 released CLI 的用户选择流程，不手改项目开发端口/探针缓存。
- CLI Hardware Memory:
  - CLI 是保存硬件的唯一用户侧来源，不使用 Web local storage 或项目缓存。
  - CLI 支持 `loadlynx hardware available/recent/path/list/save/forget`、`loadlynx status --hardware <id>`，并在成功 `status --device` / `status --url` 后 best-effort 自动更新曾经连接过的 USB 与 HTTP 设备。
  - `hardware available` 列出 devd 当前可见 USB 设备和已保存 HTTP fallback；`--scan` 会先刷新 devd 可见性。devd 不可达时输出 USB 错误并保留 HTTP fallback。
  - `hardware recent` 按最近成功连接或保存时间倒序列出已记住设备。
  - 硬件记忆位于用户配置目录，`LOADLYNX_HOME` 可用于测试或高级覆盖。
  - 已保存同一硬件同时存在 USB 与 HTTP 时优先 USB。
- Download Released Firmware:
  - 下载 release 固件 catalog 与其引用文件。
  - 缺少 catalog/asset 或 CLI/devd 无法选择 catalog 时停止。
- User Workflows:
  - CLI `devices` / `status`，用于身份、固件版本、网络身份、uptime、链路、模拟板状态、故障、温度、电压、电流、功率、USB-PD 状态。
  - CLI 输出控制与结果验证。
  - CLI 电子负载业务：CC/CV/CP 设定值、limits、preset 编辑/应用，只在 installed CLI 暴露对应命令时执行。
  - CLI USB-PD 业务：Source capabilities、当前 contract、Fixed/PPS apply、Safe5V/extended-voltage gate，只在 installed CLI 暴露对应命令时执行。
  - CLI firmware dry-run / real flash，必须有 device、target、artifact、hash、lease/session 证据。
  - CLI WiFi 配置，必须先确认 installed CLI 真实存在 WiFi 命令。
- Escalate Out:
  - 缺失 CLI 功能、源码构建、release workflow、`just`、`mcu-agentd`、HIL、校准、协议/API 实现进入开发者技能。

### `loadlynx-developer-operations`

- Frontmatter:
  - `name`: `loadlynx-developer-operations`
  - 触发范围：source checkout、clone、Just、本地 devd/CLI/Web、固件、release、WiFi、校准、HIL。
- Start Here:
  - `npx skills add https://github.com/IvanLi-CN/loadlynx --skill loadlynx-developer-operations`
  - 运行 repo 命令前证明 checkout。
  - 没有 checkout 且任务需要源码时 clone `https://github.com/IvanLi-CN/loadlynx.git`。
- Tooling Checks:
  - 检查 `just`、Rust embedded targets、ESP toolchain、Web toolchain、devd Rust toolchain。
  - 用 `just devd-build` / `just devd-test` / `just devd-serve` / `just loadlynx <args>`。
  - Release 维护验证 host-tools、固件 catalog/assets 与 release notes 一致。
- Business Capability Development:
  - 用户业务能力必须在 released CLI 中可发现、可执行、可验证；Web-only 或 raw-HTTP-only 不算完成。
  - 新增业务能力时同步 firmware/protocol、devd API、CLI 命令/help、测试、secret redaction、release packaging 与 skill/spec 文档。
  - 范围包括身份/状态/遥测、输出/预设/CC/CV/CP、USB-PD Fixed/PPS、固件更新和 WiFi。
- Device Selection:
  - 禁止猜测或静默切换硬件目标。
  - CLI/devd ESP32-S3 USB CDC 使用 owner 批准的项目开发端口缓存。
  - selector/cache 写入必须有 owner 对具体目标与动作的明确授权；不得要求 owner 用固定短语或固定命令字符串认证授权。
- devd And USB CDC:
  - 使用 `loadlynx-devd`，不走 `mcu-agentd` selector。
  - USB 写入必须持有 devd lease，并用 JSONL frames 或 request/response 证明真机覆盖。
  - trace/log 离开 devd 前必须脱敏 WiFi PSK。
- Firmware, Release, And HIL:
  - `just a-build` / `just d-build`。
  - CLI/devd digital flash 使用 devd direct `espflash`，不回退 `just agentd flash digital`。
  - analog 与非 devd 流程通过 `mcu-agentd`。
  - release workflow 需要在发布前构建并上传被用户技能依赖的程序与固件资产。
- WiFi And Calibration:
  - WiFi 可涉及源码配置、协议、devd API、CLI、release packaging。
  - Runtime user WiFi 不得在 released CLI 缺失时写成可用。
  - 校准写入为维护操作，保持单写者和 before/after 证据。
- Validation:
  - 按变更面选择 `just devd-test`、相关 `cargo test`、`just a-build`、`just d-build`、非硬件 Web checks、workflow lint。

## 验收标准

- Given 仓库包含两个 skill
  When 运行 `npx skills add . --list`
  Then 输出只列出 `loadlynx-user-operations` 与 `loadlynx-developer-operations`。
- Given 一个空临时目录
  When 使用 `npx skills add https://github.com/IvanLi-CN/loadlynx --skill loadlynx-user-operations --skill loadlynx-developer-operations`
  Then 两个 skill 都安装到 `.agents/skills/`，并包含 `SKILL.md` 与 `agents/openai.yaml`。
- Given 用户请求安装程序
  When agent 使用用户技能
  Then agent 只引导安装 GitHub Releases 的 `host-tools`，不要求源码 checkout。
- Given 用户请求操作硬件
  When agent 使用用户技能
  Then agent 只使用 `loadlynx` CLI；USB/devd 优先，HTTP 作为 fallback，不使用 Web UI。
- Given 用户请求查找曾经连接过的硬件
  When installed `loadlynx --help` exposes `hardware` and `status --hardware`
  Then agent lists saved hardware through `loadlynx hardware list` and uses `--hardware <id>` before manual scanning.
- Given 用户请求 GitHub 固件下载与 CLI 烧录
  When release 提供 firmware catalog/assets 且 CLI/devd 支持选择 artifact
  Then agent 引导下载 catalog/assets、启动 devd、先 dry-run、再在证据齐全时 real flash。
- Given 用户请求 CLI WiFi 配置
  When installed `loadlynx --help` 没有 WiFi 命令
  Then agent 明确报告当前 release 不支持 CLI WiFi 配置，并升级到开发者实现/发布任务。
- Given agent 不在 LoadLynx checkout 内且任务需要源码
  When agent 使用开发者技能
  Then agent 可以 clone `https://github.com/IvanLi-CN/loadlynx.git`，进入 checkout 后再运行项目命令。

## 非功能性验收 / 质量门槛

- `quick_validate.py` 对两个 skill 均通过。
- `npx skills add . --list` 可发现两个 skill。
- `git diff --check` 通过。
- 仓库不保留旧 USB/devd skill 路径引用。
- skill 不得把未实现的 CLI WiFi 配置或缺失的 Release firmware catalog 写成已可用功能。

## 文档更新

- 更新 `AGENTS.md` 中的 skill 路由。
- 更新 `README.md` 的 released host-tools、用户/开发路径与 CLI 能力边界。
- 新增本规格，并在 `docs/specs/README.md` 登记。

## 实现里程碑

- [x] M1: 保持两个 skill，改为用户版 / 开发者版。
- [x] M2: 用户版写入 released host-tools 安装、USB 优先 / HTTP fallback、GitHub 固件下载、CLI 烧录、CLI WiFi 能力自检与 CLI 硬件记忆流程。
- [x] M3: 开发者版写入 checkout 检测、必要时 clone、`just` 本地 devd/CLI/固件工作流。
- [x] M4: 补齐 `agents/openai.yaml` 与 `vercel-labs/skills` 安装验证。
- [ ] M5: 若要真正开放用户 CLI WiFi 配置，先实现并发布 `loadlynx wifi ...`、devd/firmware协议与持久化。
- [x] M6: 实现 CLI 用户级硬件记忆：保存、列出可连接设备、列出最近连接设备、列出已记住设备、选择、更新、遗忘 USB 与 HTTP 设备，并保存到用户配置目录。

## 风险与开放问题

- 当前 release host-tools 是压缩包中的 CLI/daemon binaries，不是签名桌面安装器。
- 当前源码可见的 `loadlynx` CLI 没有 WiFi 配置命令；用户版只能做能力自检和阻断，不能给出假命令。
- 用户侧 GitHub 固件烧录依赖 Release 实际发布 firmware catalog/assets，并依赖 CLI/devd 能选择和校验这些 artifact。

## References

- `README.md`
- `WORKFLOW.md`
- `docs/specs/e3rv6-loadlynx-devd-control-plane/SPEC.md`
- `docs/solutions/devices/local-device-control-plane.md`

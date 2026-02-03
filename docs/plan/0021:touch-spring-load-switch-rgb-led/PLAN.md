# 触摸弹簧（GPIO14）负载开关 + RGB 指示 + 语音播放（MAX98357A / I²S）（#0021）

## 状态

- Status: 已完成
- Created: 2026-01-19
- Last: 2026-02-03

## 背景 / 问题陈述

- 需要一个“无需旋钮按压”的本地交互入口，用于快速切换数字板（ESP32‑S3）CC 负载开关（load switch）。
- 希望利用 3 个未占用 GPIO 输出 PWM，驱动一颗离散 RGB LED，用于提供“无需看屏幕”的状态提示。
- 新版数字板增加语音播放能力：通过 I²S 数字音频功放 **MAX98357AETE+T** 驱动扬声器，用于语音提示/播报。

## 目标 / 非目标

### Goals

- 在 ESP32‑S3 的 `GPIO14` 上接入触摸弹簧，作为**电容触摸按键**输入，用于切换负载开关（`load_enabled`）。
- 使用 3 个未占用 GPIO 通过 `LEDC` 输出 PWM，驱动离散 RGB LED，并用颜色/闪烁表达关键状态。
- 增加语音播放：使用 ESP32‑S3 的 I²S 外设输出音频到 **MAX98357A**，实现“短语音片段”播放（提示/播报）。
- 与既有“负载开关语义”保持一致：复用 #0001 的控制模型（`set_*` 与 `load_enabled` 分离；`effective_*` 派生）。
- 不引入新的对外协议与对外 API（除非后续明确需要）。

### Non-goals

- 不替代/重做屏幕触控（FT6336U）与旋钮交互；本计划只新增“触摸弹簧”这一入口（不改变既有旋钮输入；触摸弹簧作为额外入口）。
- 不增加 deep sleep / wakeup（esp-hal `touch` 目前不覆盖 deep sleep 唤醒）。
- 不修改模拟板（STM32G431）控制闭环与功率链路；负载开关仍按 #0001 通过“下发生效值=0”实现。
- 不做 TTS/语音识别/流媒体播放；仅覆盖本机固件触发的“短语音片段”播放（可预置资源）。

## 范围（Scope）

### In scope

- 数字板固件（ESP32‑S3）：
  - 新增 `GPIO14` 触摸按键输入（使用 ESP32‑S3 Touch Sensor 外设 `TOUCH`）。
  - 触摸检测的去抖/阈值/校准策略（避免误触发）。
  - 触摸触发对 `load_enabled` 的切换（复用既有负载开关通路与仲裁规则）。
  - RGB LED PWM 驱动（3 路 `LEDC` channel + 1 个 timer），以及状态到颜色/闪烁的映射。
  - 关键日志与计数器（便于 HIL 调参、定位误触发）。
  - I²S 语音播放（ESP32‑S3 → MAX98357A）：
    - I²S 外设初始化（TX master）。
    - 基础 PCM 播放队列/调度（仅语音片段，不做混音器）。
    - 最小可验证触发入口（实现阶段可用一个“固定调试入口/隐藏按钮/串口命令”作为最小闭环）。
- 文档：
  - `docs/interfaces/touch-switch-and-rgb-led.md`：触摸开关 + RGB 直驱的行为与电气/布局约束汇总。
  - 更新 `docs/interfaces/pinmaps/esp32-s3.md`：标注 `GPIO14`、RGB 三路 PWM 与 I²S（三线）占用，并注明“连续封装引脚：I²S=Pin 40/41/42、RGB=Pin 43/44/45”约束。

### Out of scope

- Web/HTTP 控制语义变更（默认不改；如需远程控制 LED 或触摸开关策略，再另开计划）。
- 新增/修改 UART 协议字段（默认不改）。
- 硬件 BOM 调整（如需外置触摸 IC/LED 驱动 IC，再另开计划）。

## 需求（Requirements）

### MUST

#### 触摸按键（GPIO14）

- 使用 ESP32‑S3 Touch Sensor 外设实现电容触摸读取：`GPIO14` 对应 `TouchPad14`（见 esp-hal 的 `touch`/`TouchPad` 与芯片 GPIO 映射）。
- 触摸输入采用直连触摸方案（不引入外置触摸 IC / 数字电平输入）。
- 机械/介质约束（已确认）：
  - 面板为**亚克力 0.8–1.0 mm**，且**不允许开孔**；
  - 触摸弹簧顶端**顶住面板背面**作为电极（无明显空气间隙）。
- ESD/浪涌防护（建议但不强制）：
  - 触摸电极虽然隔着亚克力，但 ESD 仍可能通过电容耦合注入到电极/走线，导致误触发或芯片复位；建议在触摸线**外侧（靠近电极/面板边缘）**预留超低电容 ESD 器件焊盘，并在 MCU 侧加入 `100–470Ω` 串联电阻作为“限流 + 去振铃”。
  - **器件选择关键约束**：Touch 线的寄生电容越大越难调参；因此 ESD 器件需选“超低电容”型号（通常 `<=1pF` 量级）。例如 `CESD5V0D5` 这类 TVS 的结电容在数据手册中给到 `Cj≈95pF`（`Vr=0V, f=1MHz`），会显著拉低灵敏度与抗噪裕量，**不建议**用于 Touch 电极。
- 上电后需要“校准窗口”（例如延迟 + N 次采样），建立 baseline，并在运行中允许缓慢自适应（避免环境漂移导致阈值失效）。
- 触摸识别必须具备抗抖动与抗噪声能力：
  - 以“触摸按下边沿（touch-down）”作为触发点；
  - 带最小间隔（lockout/cooldown），避免一次触摸导致多次 toggle；
  - 单击切换 `ON/OFF`（不做长按重复触发）。
- 触摸按键触发的 `load_enabled` 切换行为必须与 #0001 一致：
  - 当“设置值=0”时不得触发开启（仍遵循 #0001 的强制安全规则）。
  - 与既有入口（旋钮按键、HTTP `/api/v1/cc` 写入）同模型：均通过更新 digital-side CC 模型（setpoint + `load_enabled`）实现；仲裁规则为“最后写入生效（last-writer-wins）”，且始终受 `set_*==0 => load_enabled=false` 约束。

#### RGB LED（3 路 PWM）

- 使用 ESP32‑S3 `LEDC` 输出 3 路 PWM，驱动离散 RGB LED：
  - PWM 频率与分辨率需固定（建议与背光/风扇/蜂鸣器不同 timer，避免互相牵连）。
  - 本计划目标 LED 为**共阳**、且**无三极管/PMOS/NMOS 驱动**：GPIO 直接灌电流（sink），因此 PWM 极性为 **active-low**（`GPIO=LOW` 时点亮）。
  - 共阳端（COM）接 `3V3`（已确认）。
  - 每个颜色通道必须串联限流电阻（R/G/B 分别独立），并以“可视亮度足够 + GPIO 安全电流范围”为上限保守设定。
- 直驱电流与阻值建议（冻结，保证可落地）：
  - **直驱前提**：共阳端只能接 `3V3`（VDDIO）。不得把共阳端接 `5V` 并直连 GPIO（否则会通过 MCU 钳位二极管向 `3V3` 反灌电流）。
  - **电流上限**：ESP32‑S3 数据手册 DC 特性给出 `I_OL=28mA`（`VDD=3.3V`，`V_OL=0.495V`，`PAD_DRIVER=3`）。为留足裕量并避免三色同时点亮的热/噪声风险，本计划把**每色峰值电流**目标冻结为：
    - 红：`I_peak <= 8mA`
    - 绿/蓝：`I_peak <= 4mA`（`3V3` 供电下余量更小，且 LED Vf 离散度更敏感）
  - **限流电阻起始值**（以常见 RGB 指示灯 `Vf@几mA` 估算；最终以实测亮度微调）：
    - 红：`R=180Ω`（约 7–8mA，`Vf≈2.0V`）
    - 绿：`R=100Ω`（约 1–3mA，`Vf≈3.0–3.2V`）
    - 蓝：`R=100Ω`（约 1–3mA，`Vf≈3.0–3.2V`）
  - **结论**：在“直驱 + COM=3V3”约束下，绿/蓝难以稳定做到“20mA/色”；若结构件/导光导致亮度仍不足，只能回到“更高效率 LED / 光学优化 / 改硬件驱动”的路线重新评估。
- RGB LED 至少表达以下状态（颜色/闪烁规则见“状态 → 颜色/闪烁（冻结）”）：
  - `load_enabled`（负载开关开/关）
  - 异常状态（黄色），用于提示“用户认为应该开启但被系统阻止/故障”等情况（见“状态 → 颜色/闪烁（冻结）”）
- 不能影响现有 LEDC 用途（背光、风扇 PWM、蜂鸣器）；需要明确占用的 timer/channel 编号与 GPIO。

##### 状态 → 颜色/闪烁（冻结）

- 优先级（高→低）：**异常（黄）** > **load_enabled=ON（绿）** > **load_enabled=OFF（红）**。
- 颜色定义：
  - `load_enabled=true`：绿色常亮
  - `load_enabled=false`：红色常亮
  - `abnormal=true`：黄色闪烁（覆盖绿/红）
- `abnormal` 判定（可实现/可测试，复用 digital 已有“阻止/告警”来源）：
  - `current_load_block_abbrev().is_some()`（例如：`FAULT_*`、UVLO、关键 link 告警等）；
  - 或“本次触摸/用户操作尝试开启但被阻止”的窗口内（TTL=3s）：`set_*==0` 或 `current_load_enable_block_abbrev(min_v_mv).is_some()`。
- 闪烁建议：黄 `2Hz`（实现时允许在 `1–4Hz` 内微调）。

#### 可观测性与安全默认

- 上电默认 `load_enabled=false`（保持现状）；RGB LED 默认处于“安全/不误导”的初始状态（例如 OFF 或 standby 色）。
- 需要提供最小日志与计数器：触摸原始值范围、baseline/阈值、触发次数、抖动/被抑制次数（用于快速调参）。

#### 语音播放（MAX98357A / I²S）

- 采用 MAX98357A 的 I²S 数字输入：仅需要 `BCLK`、`LRCLK`、`DIN` 三线；不需要外部 `MCLK`。
- I²S 三线必须分配到 **3 个连续封装引脚**（便于布线与走线长度控制）：
  - `GPIO35=I2S_BCLK`（封装 Pin 40）
  - `GPIO36=I2S_LRCLK`（封装 Pin 41）
  - `GPIO37=I2S_DIN`（封装 Pin 42）
- 音频数据格式基线（实现基线，可在不破坏接口的前提下调整）：
  - 标准 I²S（Philips），`16-bit` 或 `32-bit` 采样宽度；
  - `mono`（单声道），采样率优先在 `16kHz/22.05kHz/44.1kHz/48kHz` 中选择（以存储与听感折中为准）。
  - 当前固件实现基线：`8kHz`（WAV PCM16LE mono 资产；固件侧做 mono→stereo duplication 输出）。
- 连接 `SD_MODE` 作为 **AMP_EN/Shutdown**：
  - `GPIO34=AMP_SD_MODE`（封装 Pin 39）→ MAX98357A `SD_MODE`
  - 运行语义（I²S/LJ）：低=shutdown；高=Left（本计划输出 mono PCM，因此固定用“高=Left”即可）。
  - 若后续需要 (L+R)/2 混音输入，可按 datasheet 用电阻分压把 `SD_MODE` 拉到对应档位；本计划先不引入该复杂度。
- `GAIN_SLOT` 等其它配置脚允许用硬件 strap 固定；默认不额外占用 GPIO。

## 接口契约（Interfaces & Contracts）

None（本计划默认不新增/修改/删除 UART 协议、HTTP API、文件格式或 CLI）。

## 验收标准（Acceptance Criteria）

- Given 设备上电且 `set_* > 0` 且 `load_enabled=false`，
  When 触摸弹簧被一次“短触”触发，
  Then `load_enabled` 变为 `true`，并在下一次下发中让 `effective_*` 生效（按 #0001 的规则计算）。
- Given `set_* == 0`，
  When 触摸弹簧被触发，
  Then 不允许开启负载（`load_enabled` 必须保持 `false`）。
- Given 连续抖动/轻触/电噪声，
  When 触摸输入在短时间内多次波动，
  Then 不产生“多次 toggle”（满足最小 lockout/cooldown 约束）。
- Given 面板为亚克力 0.8–1.0 mm 且 RGB PWM 正常工作，
  When 用户在图标区域完成一次“短触”，
  Then 触摸识别仍稳定（不因 PWM 造成误触发/漏触发到不可用的程度；可通过调参与软件兜底满足）。
- Given `load_enabled`、link、fault 状态变化，
  When 状态更新，
  Then RGB LED 显示对应的颜色/闪烁（规则按本计划冻结的映射表执行）。
- Given MAX98357A 已接入且 I²S 初始化完成，
  When 触发一次“语音测试播放”（最小可验证入口），
  Then 扬声器可以稳定播放出可辨识的语音片段，且播放期间系统不掉线/不卡死（不会影响主 UI 刷新到不可用）。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing / HIL

- Build：
  - `just d-build`
- HIL（硬件在环）：
  - `just agentd-get-port digital`（确认当前使用的串口选择）
  - `just agentd flash digital`
  - `just agentd monitor digital --from-start`，观察：
    - 触摸任务启动日志（含 baseline/阈值配置）
    - 触摸触发计数递增且无明显误触发
    - `load_enabled` 切换与 UI/遥测一致
    - RGB LED 行为与定义一致
    - 语音播放测试可触发，I²S 初始化与播放日志正常（不出现持续 underrun/重启）

### Quality checks

- `just fmt`
-（可选）`cargo clippy --all-targets --all-features -D warnings`（若当前工具链可用且不引入额外噪声）

## 文档更新（Docs to Update）

- `docs/interfaces/pinmaps/esp32-s3.md`：将 `GPIO14` 标注为 `TOUCH_SPRING`（或等价网络名），并写明上电毛刺/触摸抗扰建议；新增/更新 RGB 三路 PWM 的 GPIO 占用与注意事项。
- `docs/interfaces/pinmaps/esp32-s3.md`：新增/更新 `I2S_BCLK/I2S_LRCLK/I2S_DIN` 与 `RGB_R/G/B` 的 GPIO 占用，并注明“连续封装引脚：I²S=Pin 40/41/42、RGB=Pin 43/44/45”约束。

## 实现里程碑（Milestones）

- [x] M1: 数字板：接入 `GPIO14` TouchPad，完成校准+去抖，并能稳定切换 `load_enabled`
- [x] M2: 数字板：接入 RGB 三路 LEDC PWM + 状态映射（颜色/闪烁）并通过 HIL 验证
- [x] M3: 数字板：接入 I²S（MAX98357A）语音播放最小闭环（可触发播放 + 基础日志 + 不影响主循环）
- [x] M4: 文档：更新 ESP32‑S3 pinmap 与 HIL 验证记录（日志片段/结论）

## 方案概述（Approach, high-level）

- 触摸按键：
  - 使用 `esp-hal` 的 `touch` 驱动（`Touch::continuous_mode` + `TouchPad::<GPIO14>`）。
  - 启动后进行 baseline 采样窗口，并在运行时用慢速 EMA/窗口最小值进行漂移补偿；触发采用“阈值 + 滞回 + 时间锁”。
  - 触发动作复用既有“负载开关 toggle”路径，确保与 #0001 的安全规则一致；并且不依赖旋钮按键路径（旋钮按键在本计划中不要求可用）。
- RGB LED：
  - 使用 `LEDC` 的独立 `Timer3`（低速）与 `Channel3/4/5`（建议；最终以实际空闲资源为准）。
  - 统一一个 PWM 频率/分辨率给 RGB 三色，避免颜色漂移与肉眼可见闪烁。
  - 状态映射采用小型状态机：优先级（异常 > load_enabled=ON > load_enabled=OFF）。
  - 共阳 LED 的 GPIO 输出逻辑需要反相：`duty=0` 表示“全亮”、`duty=100%` 表示“全灭”（或等价的 duty 反转函数）。
- 语音播放：
  - 使用 ESP32‑S3 I²S（TX master）输出标准 I²S 到 MAX98357A；不引入外部 MCLK。
  - 语音片段以“预置 PCM 资源 + 简单播放队列”的方式落地（后续若需要压缩格式/更复杂调度再扩展）。

## 风险与开放问题（Risks & Open Questions）

- 风险：
  - 电容触摸对布线/人体/环境非常敏感，阈值需要在硬件上实测调参；ESD 与手触噪声可能导致误触发。
  - `GPIO14` 属于上电早期可能出现短脉冲的组（见 pinmap 文档），需通过“启动延迟 + 校准窗口 + lockout”规避误触发。
  - 机械集成风险：若 RGB LED 与触摸电极（弹簧）在物理上过近，且 LED 采用 PWM 调光，PWM 边沿/走线寄生电容可能耦合进触摸通道，造成阈值漂移或误触发；需要通过布局隔离与软件去抖共同兜底。
  - EMI 风险：ESP32‑S3 数据手册对 Touch Sensor 的抗干扰能力有明确限制（CS 测试未覆盖/应用场景受限）；若整机电磁环境较差，可能需要更强的屏蔽/接地策略，或退回外置触摸方案。
  - I²S 边沿噪声风险：`I2S_BCLK` 为高速时钟，若与触摸/模拟采样走线靠近可能引入干扰；需要布局隔离与必要的串阻/走线控制。
  - 音频 pop/底噪风险：功放上电/使能与 I²S 时钟启停可能产生 pop；必要时通过 `SD_MODE` 管脚或“先送零数据后开声”策略兜底。
- 已冻结的决定：
  - `load_enabled=ON`：绿色；`load_enabled=OFF`：红色；异常：黄色（见“状态 → 颜色/闪烁（冻结）”）。
  - 面板与触摸：亚克力 0.8–1.0 mm、面板不允许开孔、弹簧顶住背面；RGB 共阳端接 3V3。
- 仍存在的不确定性（不阻塞固件实现，但可能影响“最终观感/调参时间”）：
  - 图标透光/扩散工艺会决定均匀性（是否“中心热点”）。本计划把“图标可见被照亮”作为最低验收；若后续需要“均匀背光”视觉指标，再单独开硬件/结构计划。

## 触摸弹簧 + 指示灯布局建议（Mechanical notes）

- 面板**不允许开孔**时：触摸必须通过面板介质实现（finger→panel→electrode）。
- 已确认面板为**亚克力 0.8–1.0 mm**，且弹簧顶端**顶到背面**：这对灵敏度是利好，但也要求弹簧固定更稳（避免微动导致读数漂移）。
- RGB LED 放在触摸区域“中心”本身很常见，**不是禁忌**；关键是避免 PWM 走线/LED 结电容/回流电流耦合到触摸电极导致误触发，并让图标尽量均匀发光。
- 推荐形态（低成本、可落地）：
  - LED 放在图标下方（PCB 背面或正面皆可）+ 面板背面扩散层/导光件，让图标区域均匀发光；
  - 触摸电极优先用“PCB 铜皮电极”（圆盘/环形）贴近面板背面；若坚持“触摸弹簧”，建议让弹簧顶端**顶住面板背面**作为电极抬高（缩短介质厚度），但不要暴露到面板外侧；
  - 电气隔离：LED 的 R/G/B 走线不要穿越电极下方，尽量从电极外圈绕行；触摸电极附近保持 ground keep-out；必要时加接地护环（guard ring）并让 `GPIO14` 走线最短、远离 `I2S_BCLK` 等高速时钟与 RGB PWM 走线。
- 软件兜底（若仍受 PWM 干扰）：允许在触摸采样窗口短暂“冻结/关闭”RGB PWM（人眼不可见的短窗），以降低触摸测量抖动。
- 共阳 + 无三极管：建议 COM 接 `3V3`；R/G/B 各自串联限流电阻，GPIO 只做灌电流（active-low）。绿/蓝在 3V3 供电下亮度可能偏低，需接受或改硬件方案（加三极管/改供电）。

## 假设（Assumptions）

- GPIO 分配（待原理图确认）：
  - MAX98357A `SD_MODE`：`GPIO34`（封装 Pin 39，`AMP_SD_MODE`，用于 AMP_EN/Shutdown）
  - I²S（MAX98357A）：`GPIO35/36/37`（封装 Pin 40/41/42，连续）
  - RGB PWM：`GPIO38/GPIO39/GPIO40`（封装 Pin 43/44/45，连续；顺序为 `R/G/B`；实际以 PCB 网络名与装配方向为准）
- 光学假设：面板图标区域具备基本透光/扩散条件；允许一定中心热点（无开孔前提下以低成本方案优先）。

## 变更记录（Change log）

- 2026-01-19: 创建计划 #0021
- 2026-01-19: 补齐硬件约束：亚克力 0.8–1.0mm、弹簧顶到背面、RGB 共阳端接 3V3
- 2026-01-19: 冻结范围与验收：状态映射（绿/红/黄）、异常判定与闪烁优先级
- 2026-01-21: 扩充语音播放（MAX98357A / I²S），并按“连续封装引脚”重排 GPIO（AMP_SD_MODE=GPIO34；I²S=GPIO35/36/37；RGB=GPIO38/39/40）
- 2026-02-01: HIL：触摸弹簧 + RGB 已跑通；新增 I²S/MAX98357A 输出任务与启动自检音（用于后续语音片段落地）
- 2026-02-01: HIL：降低“未触碰即触发”的误触风险（提高阈值 + 连续采样判定）
- 2026-02-02: 数字板：扬声器启动自检改为 WAV(PCM16LE mono, 8kHz) 资产播放（I²S=8kHz, mono→stereo duplication；固件侧 PCM digital gain +6dB）；boot playlist: 440/554/659/880 + test melody；PC 可直接预览 `firmware/digital/assets/audio/*.wav`
- 2026-02-03: 数字板：扬声器播放尾部等待改为按 playlist 时长（避免 UI 音效额外阻塞多秒）

## 参考（References）

- `docs/interfaces/pinmaps/esp32-s3.md`
- `docs/plan/0001:cc-load-switch-toggle/PLAN.md`
- esp-hal v1.0.0（本仓库 `digital` 依赖版本）：
  - `esp_hal::touch`（Touch Sensor）
  - `esp_hal::ledc`（PWM / LEDC）
  - `esp_hal::i2s`（I²S / audio）

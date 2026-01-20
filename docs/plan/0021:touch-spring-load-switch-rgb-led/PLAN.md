# 触摸弹簧（GPIO14）负载开关 + RGB PWM 指示灯（#0021）

## 状态

- Status: 部分完成（1/3）
- Created: 2026-01-19
- Last: 2026-01-20

## 背景 / 问题陈述

- 需要一个“无需旋钮按压”的本地交互入口，用于快速切换数字板（ESP32‑S3）CC 负载开关（load switch）。
- 希望利用 3 个未占用 GPIO 输出 PWM，驱动一颗离散 RGB LED，用于提供“无需看屏幕”的状态提示。

## 目标 / 非目标

### Goals

- 在 ESP32‑S3 的 `GPIO14` 上接入触摸弹簧，作为**电容触摸按键**输入，用于切换负载开关（`load_enabled`）。
- 使用 3 个未占用 GPIO 通过 `LEDC` 输出 PWM，驱动离散 RGB LED，并用颜色/闪烁表达关键状态。
- 与既有“负载开关语义”保持一致：复用 #0001 的控制模型（`set_*` 与 `load_enabled` 分离；`effective_*` 派生）。
- 不引入新的对外协议与对外 API（除非后续明确需要）。

### Non-goals

- 不替代/重做屏幕触控（FT6336U）与旋钮交互；本计划只新增“触摸弹簧”这一入口（不改变既有旋钮输入；触摸弹簧作为额外入口）。
- 不增加 deep sleep / wakeup（esp-hal `touch` 目前不覆盖 deep sleep 唤醒）。
- 不修改模拟板（STM32G431）控制闭环与功率链路；负载开关仍按 #0001 通过“下发生效值=0”实现。

## 范围（Scope）

### In scope

- 数字板固件（ESP32‑S3）：
  - 新增 `GPIO14` 触摸按键输入（使用 ESP32‑S3 Touch Sensor 外设 `TOUCH`）。
  - 触摸检测的去抖/阈值/校准策略（避免误触发）。
  - 触摸触发对 `load_enabled` 的切换（复用既有负载开关通路与仲裁规则）。
  - RGB LED PWM 驱动（3 路 `LEDC` channel + 1 个 timer），以及状态到颜色/闪烁的映射。
  - 关键日志与计数器（便于 HIL 调参、定位误触发）。
- 文档：
  - 更新 `docs/interfaces/pinmaps/esp32-s3.md`：标注 `GPIO14` 与 RGB 三路 GPIO 的占用与注意事项。

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
  - 记录（2026-01-20）：
    - 启动日志出现 `touch spring calibrated: baseline=...`，且 `stats` 中 `touch_spring_reads` 持续递增（证明触摸采样在跑）。
    - 当前硬件未完成（触摸弹簧/RGB 连接未就绪），因此暂无法验证：触摸短触 toggle、异常黄闪、红/绿常亮等行为；后续补 HIL。

### Quality checks

- `just fmt`
-（可选）`cargo clippy --all-targets --all-features -D warnings`（若当前工具链可用且不引入额外噪声）

## 文档更新（Docs to Update）

- `docs/interfaces/pinmaps/esp32-s3.md`：将 `GPIO14` 标注为 `TOUCH_SPRING`（或等价网络名），并写明上电毛刺/触摸抗扰建议；新增/更新 RGB 三路 PWM 的 GPIO 占用与注意事项。

## 实现里程碑（Milestones）

- [ ] M1: 数字板：接入 `GPIO14` TouchPad，完成校准+去抖，并能稳定切换 `load_enabled`
- [ ] M2: 数字板：接入 RGB 三路 LEDC PWM + 状态映射（颜色/闪烁）并通过 HIL 验证
- [x] M3: 文档：更新 ESP32‑S3 pinmap 与 HIL 验证记录（日志片段/结论）

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

## 风险与开放问题（Risks & Open Questions）

- 风险：
  - 电容触摸对布线/人体/环境非常敏感，阈值需要在硬件上实测调参；ESD 与手触噪声可能导致误触发。
  - `GPIO14` 属于上电早期可能出现短脉冲的组（见 pinmap 文档），需通过“启动延迟 + 校准窗口 + lockout”规避误触发。
  - 机械集成风险：若 RGB LED 与触摸电极（弹簧）在物理上过近，且 LED 采用 PWM 调光，PWM 边沿/走线寄生电容可能耦合进触摸通道，造成阈值漂移或误触发；需要通过布局隔离与软件去抖共同兜底。
  - EMI 风险：ESP32‑S3 数据手册对 Touch Sensor 的抗干扰能力有明确限制（CS 测试未覆盖/应用场景受限）；若整机电磁环境较差，可能需要更强的屏蔽/接地策略，或退回外置触摸方案。
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
  - 电气隔离：LED 的 R/G/B 走线不要穿越电极下方，尽量从电极外圈绕行；触摸电极附近保持 ground keep-out；必要时加接地护环（guard ring）并让 `GPIO14` 走线最短、远离 RGB PWM 三路走线与 LED 走线。
- 软件兜底（若仍受 PWM 干扰）：允许在触摸采样窗口短暂“冻结/关闭”RGB PWM（人眼不可见的短窗），以降低触摸测量抖动。
- 共阳 + 无三极管：建议 COM 接 `3V3`；R/G/B 各自串联限流电阻，GPIO 只做灌电流（active-low）。绿/蓝在 3V3 供电下亮度可能偏低，需接受或改硬件方案（加三极管/改供电）。

## 假设（Assumptions）

- GPIO 分配（待原理图确认）：`GPIO35/36/37` 作为 RGB PWM 输出。
- 颜色通道映射（可调）：`GPIO35=R`、`GPIO36=G`、`GPIO37=B`（实际以 PCB 网络名与装配方向为准）。
- 光学假设：面板图标区域具备基本透光/扩散条件；允许一定中心热点（无开孔前提下以低成本方案优先）。

## 变更记录（Change log）

- 2026-01-19: 创建计划 #0021
- 2026-01-19: 补齐硬件约束：亚克力 0.8–1.0mm、弹簧顶到背面、RGB 共阳端接 3V3
- 2026-01-19: 冻结范围与验收：状态映射（绿/红/黄）、异常判定与闪烁优先级
- 2026-01-20: 实现 digital：GPIO14 触摸弹簧采样+校准+cooldown → toggle `load_enabled`；GPIO35/36/37 LEDC PWM（active-low）用于红/绿/黄闪状态指示；补充 HIL 记录与 pinmap 占用（HIL: 待硬件就绪再验证）

## 参考（References）

- `docs/interfaces/pinmaps/esp32-s3.md`
- `docs/plan/0001:cc-load-switch-toggle/PLAN.md`
- esp-hal v1.0.0（本仓库 `digital` 依赖版本）：
  - `esp_hal::touch`（Touch Sensor）
  - `esp_hal::ledc`（PWM / LEDC）

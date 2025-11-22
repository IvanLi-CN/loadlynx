# MCU↔MCU 串口通信方案（硬件 + 软件）

范围
- 适用对象：STM32G431（`firmware/analog`）与 ESP32‑S3（`firmware/digital`）之间的板间链路。
- 本文仅聚焦“通信方案（硬件+软件）”，不展开其他备选总线/器件对比。

目标
- 简单可靠、易调试，满足实时控制与 50–100 Hz 遥测。
- 消息可演进（版本/向后兼容），链路异常时有安全兜底。

一、硬件链路（来自仓库现有文档 + 确认信息）
- 物理接口：UART（仓库现有说明明确 MCU ↔ ESP 默认 UART）。
  - STM32G431 端：USART3，`PC10=TX`、`PC11=RX`（见 `loadlynx.ioc:204-206`）。
  - ESP32‑S3 端：UART1，`GPIO17=TX`、`GPIO18=RX`（`docs/interfaces/pinmaps/esp32-s3.md:52-53`）。
- 逻辑电平：3.3 V，8N1（最终以原理图为准）。
- 数字隔离器（已确认）：TI ISO7721DR（8‑pin SOIC‑D，默认输出高）
  - 通道方向：1 发 / 1 收；建议映射：`S3_TXD → ISO_INA/OUTA → G431_RXD`，`G431_TXD → ISO_INB/OUTB → S3_RXD`。
  - 供电：隔离两侧各自 3.3 V（器件支持 2.25–5.5 V）。
  - 关键指标（摘自数据手册）：数据率至 100 Mbps，典型 tPD≈11 ns，CMTI 典型 ±100 kV/µs，D 封装 UL1577 3 kVrms。
- 走线与防护：
  - 板间连接器走线短、低阻；TX 串 22–47 Ω 抑制振铃；接口侧布置 TVS/ESD。
  - 隔离器紧邻连接器，保持隔离缝隙处爬电/电气间隙满足封装要求。
- 电源与时序：
  - 电源树：12 V（风扇）、5 V（逻辑/USB）、3.3 V（MCU/ESP）。
  - 上电顺序：先 3.3 V，再使能模拟前端；隔离两侧各自就近去耦。

二、软件协议（帧格式 + 消息集）
- 帧封装：遵循仓库 README 建议采用“CBOR/SLIP”。推荐实现：
  - SLIP 作为帧边界（0xC0 分隔、0xDB 转义）。
  - 载荷使用 CBOR 编码（小端数值约定）。
  - 在 CBOR 末尾附加 CRC‑16/CCITT‑FALSE（poly 0x1021, init 0xFFFF）作整帧校验。

- 帧头（建议，便于版本与可靠性）
  - `ver:u8` 协议主版本（初始 0x01）。
  - `flags:u8` 位0=ACK_REQ，位1=IS_ACK，位2=IS_NACK，位3=IS_RESP。
  - `seq:u8` 包序号（0..255 回绕）。
  - `msg:u8` 消息 ID（见下）。
  - `len:u16LE` 负载字节数。

- 可靠性
  - 控制/配置帧默认置位 ACK_REQ；对端用 ACK/NACK 回应（回显 `seq`/`msg`）。
  - 遥测帧默认无需 ACK（50–100 Hz）；需要时可临时打开 ACK 诊断。
  - 超时/出错重试：建议 3 次退避（如 5/10/20 ms）。
  - 心跳/看门狗：空闲心跳 10 Hz；>300 ms 无有效帧标记降级，主控侧可触发安全失能。

- 消息集合（初版建议）
  - 0x01 Hello（G431→S3，上电一次：协议/固件信息）
  - 0x02 Ping（双向：保活/测延时）
  - 0x03 Ack / 0x04 Nack（确认/拒绝）
  - 0x10 Status（G431→S3 周期遥测：时间戳、电压mV、电流mA、功率mW、温度mdegC、模式、使能、故障标志）
  - 0x11 Fault（G431→S3 事件：故障标志/是否锁存）
  - 0x20 SetEnable（S3→G431：使能/失能）
  - 0x21 SetMode（S3→G431：模式 CC/CV/CP/CR）
  - 0x22 SetPoint（S3→G431：设定值 + 单位，如 mA/mV/mW/mΩ）
  - 0x23 SetLimits（S3→G431：电流/功率/电压上下限）
  - 0x24 GetStatus（S3→G431：请求立即返回 Status）
  - 0x30 CalRead / 0x31 CalWrite（标定数据读写，按需启用）

- 角色
  - ESP32‑S3：主控（UI/日志/标定/桥接），下发控制命令，轮询/订阅遥测。
  - STM32G431：实时控制与保护（独立安全），周期上报遥测/故障，关键故障本地强制失能。

- 波特率与节拍（建议）
  - 波特率：460800–1M（结合布线与时钟抖动评估，最终以硬件定型为准）。
  - 遥测：空闲 10 Hz，工作 50–100 Hz；控制命令实时响应。

三、联调步骤（最小可用）
- 上电：G431 发送 Hello → S3 回 Ack；随后 G431 周期发 Status（10 Hz）。
- 设定：S3 下发 SetEnable/SetPoint（带 ACK_REQ），G431 回 Ack 并在后续 Status 中反映变更。
- 故障：注入温度/电压等异常 → G431 立即发 Fault 并本地失能；S3 呈现状态。

四、落地建议（代码结构）
- 共享库（`libs/`）：定义消息 ID、CBOR 结构体、SLIP 与 CRC 工具、无分配缓冲。
- G431：USART + DMA 接收环形缓冲，任务解析帧并投递到控制环。
- S3：UART 接收空闲超时分帧，任务管理重传/心跳与上层 UI/记录。

五、待确认项（跨硬件文档）
- 双端 UART 实例与引脚映射、连接器引脚定义。
- 数字隔离器：ISO7721DR 已确认；请在原理图标注通道方向与复位默认态（与 UART idle‑high 一致）。
- 波特率回退策略、错误计数阈值、降级/失能策略。

## MCU↔MCU 数据与频率规划（v0）

> 本节聚焦“需要传输的数据集合 + 估算的帧体积/频率”，供协议实现前进行容量评估；帧封装与安全策略仍以上文说明为准。

### STM32G431 → ESP32‑S3（遥测、事件、诊断）

| 数据块 | 字段概要 | 单帧字节 | 更新频率 | 估算带宽 | 备注 |
| --- | --- | --- | --- | --- | --- |
| `FAST_STATUS` (0x10) | `uptime_ms`、`seq`、`mode`、`state_flags`、`enable`、`target_value`、`i_local_ma`、`i_remote_ma`、`v_local_mv`、`v_remote_mv`、`calc_p_mw`、`dac_headroom_mv`、`loop_error`、`sink_core_temp_mc`、`sink_exhaust_temp_mc`、`mcu_temp_mc`、`fault_flags` | ≈46 B | 默认 60 Hz（UI/日志），上限 100 Hz | 2.8 kB/s ≈ 22.4 kbps（100 Hz 时 36.8 kbps） | 高速遥测：同时上报本地/远端电压与两路电流、散热片 + PCB 温度 |
| `SLOW_HOUSEKEEPING` (0x12) | `vin_mv`、`vref_mv`、`board_temp`、`cal_state`、`diag_counters`、预留 | ≈16 B | 5 Hz | 80 B/s ≈ 0.64 kbps | 提供供电、校准、累计计数等慢变化信息 |
| `FAULT_EVENT` (0x11) | `timestamp_ms`、`fault_bits`、`fault_code`、`latched`、`extra` | ≈12 B | 按事件触发（预计 <5 Hz 峰值） | ≤60 B/s ≈ 0.48 kbps | 故障瞬时上报，附带锁存状态与附加参数 |
| `CAL_CHUNK` (0x30) | `offset_index`、`payload[32]`、`crc` | ≈48 B | 0.5–1 Hz，仅在标定模式 | ≤48 B/s ≈ 0.38 kbps | 标定/量产阶段启用，平时关闭 |
| `ADC_CAPTURE` (0x40) | `sample_rate`、`count`、`samples[128×u16]`、`checksum` | ≈260 B | ≤5 Hz（诊断时短时开启） | ≤1.3 kB/s ≈ 10.4 kbps | 供调试/上位机抓波使用，默认不发 |

**典型带宽**（不含诊断）：`FAST_STATUS + SLOW_HK + 心跳/ACK 开销` ≈ 2.6 kB/s（≈ 20.8 kbps）。<br>
**最坏情况**（100 Hz 遥测 + ADC 抓取）≈ 5.7 kB/s（≈ 45.6 kbps），仍低于 460800 baud 的 10% 左右。

### ESP32‑S3 → STM32G431（控制、配置、维护）

| 数据块 | 字段概要 | 单帧字节 | 更新频率 | 估算带宽 | 备注 |
| --- | --- | --- | --- | --- | --- |
| `SETPOINT_STREAM` (0x21) | `seq`、`mode`、`enable`、`target_value`、`slew_limit`、`i_limit_ma`、`p_limit_mw`、`profile_id` | ≈18 B | 默认 50 Hz（旋钮/远程 UI），峰值 100 Hz | 0.9 kB/s ≈ 7.2 kbps（100 Hz 时 14.4 kbps） | 连续设定流（含限值快照），全部要求 ACK |
| `LIMIT_PROFILE` (0x23) | `max_i`、`max_p`、`ovp_mv`、`temp_trip`、`thermal_derate`、预留 | ≈20 B | 0.2–1 Hz（用户修改时） | ≤20 B/s ≈ 0.16 kbps | 每次下发表征最大允许功率/电流的参数，并附 ESP 根据风扇控制计算出的 `thermal_derate`，便于版本化 |
| `CONTROL_CMD` (0x20/0x24/0x25 等) | `SetEnable`、`ModeSwitch`、`GetStatus`、`FaultClear` 等短指令 | 8–12 B | 0–20 Hz（按键/脚本触发） | ≤160 B/s ≈ 1.3 kbps | 均带 ACK_REQ，失败可按 5/10/20 ms 退避重试 |
| `SOFT_RESET_REQ` (0x26) | `reason`（u8，0=manual、1=fw_update、2=ui_recover、3=link_recover）、`timestamp_ms` | 6 B | 上电后一次；或 UI/脚本按需触发（<0.2 Hz） | ≈1.2 B/s | 数字侧请求模拟侧“软复位”：G431 先本地失能并清空状态，再回复 `SOFT_RESET_ACK`；完成后重新发送 `HELLO` 进入握手 |
| `CAL_RW` (0x30/0x31) | `index`、`payload[32]`、`crc` | ≈48 B | 0.5 Hz（标定/量产） | ≤24 B/s ≈ 0.19 kbps | 与上行 `CAL_CHUNK` 配对，用于 EEPROM/FLASH 同步 |
| `PING/HEARTBEAT` (0x02) | `timestamp`、`nonce` | 6 B | 10 Hz | 60 B/s ≈ 0.48 kbps | 空闲期保持链路活跃，>300 ms 无回应即判为降级 |
| `RESERVED_FOTA` (0x50+) | （暂未定义——需后续 bootstub/升级协议落地） | 0 B | 0 Hz | 0 | 当前项目未实现固件块传输；仅保留 ID 以免未来扩展时与现有消息冲突 |

**典型带宽**：`SETPOINT_STREAM + HEARTBEAT + 零星命令` ≈ 1.0 kB/s（≈ 8.0 kbps）。<br>
**最坏情况**（含固件升级）≈ 11.2 kB/s（≈ 89.6 kbps）。

### 带宽结论与后续事项

- 典型双向总吞吐 ≈ 30 kbps，预留 >10× 余量供 SLIP/CBOR、重传与未来字段扩展。
- 遥测与设定频率允许在运行时调节：UI 刷新 <30 Hz 时可将 `FAST_STATUS` 降至 50 Hz，以降低平均带宽与功耗。
- 诊断/固件升级帧仅在专用模式启用；启用时需对 UI 提示“高带宽模式”，同时降低或暂停 UI 级 streaming。
- 下一步：在 `libs/` 中固化上述字段结构（Rust `struct` + CBOR map），并同步生成上位机/测试脚本使用的 schema。

### 心跳与失联保护

- **双向心跳机制**
  - G431 每 10 Hz 发送 `FAST_STATUS` 或 `PING`（当进入 Idle/降频模式时也至少保持 `PING` 频率），帧内附 `uptime_ms`。
  - S3 在空闲/后台线程中每 10 Hz 发送 `PING/HEARTBEAT`（0x02），并在控制帧头置 `ACK_REQ`；若当前已有 `SETPOINT_STREAM` 流，则心跳 piggyback 在控制流上即可。
- **STM32 侧掉线判定与动作**
  - 若 300 ms 内未收到任一来自 ESP 的帧（`Ping/Ack/Setpoint/...`），即视为上位域失联：
    1. 进入 `LINK_LOST` 状态，立刻拉低 `enable`、失能 DAC/驱动；
    2. 保持本地保护继续工作（过流/过温/欠压仍然有效）；
    3. 以 5 Hz 发送 `FAULT_EVENT`（`fault_code=LINK_LOST`）直到链路恢复。
- **ESP32 侧掉线判定与动作**
  - 若 300 ms 未收到 `FAST_STATUS` 或 `FAULT_EVENT`，或 `seq` 未更新，则：
    1. 在 UI 显示“模拟域离线”并禁止用户发送新的控制命令；
    2. 继续以 10 Hz 发送 `PING`，若 1 s 内仍无响应则提示用户检查线缆/电源；
    3. 可允许用户发“重新握手”命令（重新触发 `HELLO`/`CAL_REQ` 流程）。
- **恢复流程**：任意一侧在失联后再次收到合法帧（CRC 正确、`ver` 兼容）即退出降级。G431 在退出时仍要求重新确认 `enable`（即必须等待新的 `SetEnable=1`），避免误导出力；S3 在 UI 中同步清除告警。

### 软复位序列（数字侧触发、无需掉电）

- **目的**：在开发/调试阶段保持持续供电时，避免模拟侧残留旧状态（积分、PID 内部缓存、限值、故障锁存）导致行为不一致。
- **触发**：
  - 数字板上电后、在首次 `HELLO` 成功前发送一次 `SOFT_RESET_REQ(reason=fw_update, timestamp_ms=now)`；
  - UI/上位机可通过脚本/按钮按需再发（限速 <0.2 Hz）。
- **模拟侧动作（G431）**：收到请求即刻进入安全态：
  1. 立刻拉低 `enable`、清零 DAC/驱动、停止 PWM；
  2. 清空控制环内部状态（积分器、限幅、滑动平均、故障锁存、序号基线等）；
  3. 记录 `reset_reason`，回复 `SOFT_RESET_ACK`，随后重新发送 `HELLO` 开启握手；
  4. 在完成重新握手前，保持 `SetEnable=0` 且忽略控制类命令，直到收到新的 `SetEnable=1`。
- **数字侧动作（S3）**：
  - 发送 `SOFT_RESET_REQ` 时设置 `ACK_REQ`；若 150 ms 未收到 `SOFT_RESET_ACK`，重试 2 次，仍超时则提示 UI“软复位失败，可尝试电源循环/检查链路”。
  - 收到 `SOFT_RESET_ACK` 后等待新的 `HELLO`，确认版本匹配再继续下发 `CAL_RW`/`SETPOINT_STREAM`。
- **幂等性**：模拟侧将重复请求视为重新进入安全态的 idempotent 操作，连续触发不会破坏状态机；`reason` 字段仅用于日志/诊断。
- **故障降级**：若模拟侧在 300 ms 内未重新发出 `HELLO`，数字侧应在 UI 中标记“等待重握手”，并禁止控制命令。

### 近/远端电压与双电流采样

- **采样对**：STM32G431 的 ADC 同时采集本地 Kelvin 点与远端 Sense 线的一对电压（`v_local_mv`、`v_remote_mv`），以及两路电流（`i_local_ma`、`i_remote_ma`）。其中：
  - `i_local` 来源于主功率分流器，覆盖 0–15 A 区间，作为闭环控制主量；
  - `i_remote` 来源于辅助分流/霍尔传感，可覆盖低电流精度或第二通道（视硬件装配），主要提供 UI/记录与一致性校验。
- **远端电压判定**：远端 Sense 线可能未接入，被动浮空时读数不可用。G431 按以下规则决定是否采用远端值：
  1. 监测 `sense_remote_present`（硬件通过 FPC 检测或差分防呆）：仅当该信号有效时才尝试使用远端值；
  2. 验证远端差分在允许范围内（0.5–55 V）且绝对值未饱和（ADC 原码不接近 0 或满量程）；
  3. 连续 3 帧满足条件即置位 `state_flags[REMOTE_SENSE_ACTIVE]`，闭环输出、电压显示均以 `v_remote` 为准；
  4. 任一条件失败则在 2 帧内退出远端模式，回退至 `v_local` 并清除标志。
- **字段含义**：
  - `v_local_mv`/`i_local_ma` 始终反映板载测点，供日志/诊断使用；
  - `v_remote_mv`/`i_remote_ma` 仅在 `REMOTE_SENSE_ACTIVE=1` 时用于控制，否则 UI 仍可显示（但会叠加“未连接”提示）。
- **UI 映射**：ESP32‑S3 根据 `state_flags` 决定右侧“REMOTE/LOCAL” 卡片的强调态；若远端失效，则 REMOTE 显示 `--.--` 并提示用户检查 Sense 线。

### 散热片温度传感器布点

- **Tag1 — `sink_core_temp_mc`**：10 k NTC 贴附在两颗 MOSFET 之间的铝散热片中心，处于风洞内部但远离直接出风位置，用于捕捉 FET 热源附近的最高温度。闭环降额、温度保护以此为主依据。
- **Tag2 — `sink_exhaust_temp_mc`**：同规格 NTC 固定在散热片侧面、靠近出风口但避开风直接吹拂（位于“吹不到风”的壳体侧壁），反映整体热质量/外壳温度，辅助判断风扇异常或环境散热不佳。
- **遥测语义**：`FAST_STATUS` 同时上报两个通道，ESP 可以：
  - 对 `sink_core_temp_mc` 设快速阈值（如 90 °C 降额、100 °C 触发故障）。
  - 对 `sink_exhaust_temp_mc` 设较慢积分阈值，检测风扇停转或壳体散热恶化（若核心温度正常但侧温持续上升，提示风道堵塞）。
- **标定**：量产时需在 25 °C 环境下测量两个传感器的原始 ADC 与实际温度，写入 ESP EEPROM；运行中 G431 使用校准后的 m°C 数值。
- **MCU 内部温度**：`mcu_temp_mc` 取自 STM32G431 的片上温度传感器，频繁上报以提供 PCB 参考温度。它用于：
  - 佐证 MOSFET 引脚与铜箔的热耦合情况（若散热片温度低但 MCU 温度高，说明 PCB 积热或外壳隔热问题）。
  - 在风扇/散热片被更换时，校验 PCB 与散热体之间的导热垫或螺丝是否压紧。
  - 作为最后的热保护冗余：若 `mcu_temp_mc` 超过 110 °C，可直接触发 `FAULT_EVENT` 并记录 `fault_code=PCB_OVERHEAT`，即便散热片传感器仍在正常范围。

### 风扇控制职责（ESP 主导）

- **硬件归属**：PWM（`FAN_PWM`）与转速反馈（`FAN_TACH`）均接在 ESP32‑S3（`GPIO39/40`，参考 `docs/interfaces/pinmaps/esp32-s3.md:81-82`），STM32G431 仅通过温度/功率遥测提供输入。
- **控制路径**：
  1. G431 在 `FAST_STATUS` 内上报 `sink_core_temp_mc`、`sink_exhaust_temp_mc`、功率与负载信息。
  2. S3 根据这些数据执行风扇曲线（或闭环 PID），直接生成 PWM，占空比与 `thermal_derate`（0–100%，写入 `LIMIT_PROFILE`）并保存在自身 EEPROM。
  3. 当散热不足时，S3 降低 `thermal_derate` → 通过 `LIMIT_PROFILE` 或即时 `CONTROL_CMD` 告知 G431：最大允许功率/电流需要降额。
- **遥测协同**：
  - STM32 不再上报 `fan_pwm`/`fan_rpm` 字段；此类数据由 ESP32 本地记录，可通过 UI 或上位机直接读取 ESP 端日志。
  - `SLOW_HOUSEKEEPING` 中亦不包含风扇配置 ID，相关配置完全由 ESP 侧管理。
- **容错**：ESP 负责风扇供电/报警逻辑（例如检测 Tach 丢失并提示用户），G431 若检测到高温仍会触发本地 `FAULT_EVENT` 并失能功率级，形成双重保护。

### 标定数据主存与上电同步

- **持久化所在**：当前硬件未为 STM32G431 配置 EEPROM/Flash 分区；所有校准参数（增益/偏置/温补/风扇曲线等）由 ESP32‑S3 通过其本地 EEPROM/Flash 保存，视作“唯一可信源”。
- **上电握手**：
  1. G431 上电即发送 `HELLO` 并置 `CAL_REQ` 标志，控制环保持保守安全值（仅允许 Idle）。
  2. S3 完成自检后，从本地 EEPROM 读取标定块，逐帧下发 `CAL_RW`（0x30/0x31）。
  3. G431 收齐所有块并验证 CRC 后，回复确认并清除 `CAL_REQ`，此后才允许 `SetEnable`/`SetPoint`。
- **版本/回退**：ESP 端需要维护 `CAL_VERSION` 与 `HW_REV`，若板子更换传感器或参数失配，可通过 `CAL_CHUNK` 上行把运行中的实时值回传到 ESP/上位机，以便重新烧录；若数据损坏，G431 仍能用“默认限幅”运行但输出被限制。
- **量产流程**：在治具写入阶段，ESP 作为控制端将新的标定结果通过 `CAL_RW` 推送给 G431，同时写入自身 EEPROM；流程完成后立刻做一次断电重启，验证“冷启动 → ESP 下发 → G431 恢复”这条链路。
- **容错建议**：若 300 ms 内未能完成首次 `CAL_RW`，ESP 应提示 UI “标定未加载”，G431 保持失能状态，防止使用未匹配的补偿系数。

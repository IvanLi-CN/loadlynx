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

### SetPoint 可靠传输方案（v1）

- 目的：确保 S3 下发的每次 SetPoint 在 G431 侧都被确认或重传，避免控制目标缺失导致两端状态漂移。
- 消息与标志：使用 `MSG_SET_POINT`（0x22），请求帧带 `FLAG_ACK_REQ`，确认帧带 `FLAG_IS_ACK`（`FLAG_IS_NACK` 可选用于回报解析失败）。
- 确认格式：
  - 头部 `seq`/`msg` 回显；`len=0` 时 payload 为空（可选回显 `target_i_ma` 便于调试）。
  - CRC 仍覆盖头部与 payload。
- 数字侧（ESP32‑S3）发送侧状态机（当前固件已实现 v0 版本）：
  - 发送 SetPoint 后进入等待 ACK 状态，超时约 40 ms 未收 ACK 则按 40/80/160 ms 退避重发，最多 3 次。
  - 收到匹配 `seq`/`msg` 的 ACK 即清除等待；若收到 NACK（预留）则立即重发或提示错误。
  - 新目标出现时“最新值优先”：可取消旧帧、使用新 `seq` 立即发送，重置等待计时。
  - 若遥测中的 `target_value` 与本地期望值连续多帧不一致，会触发“telemetry-mismatch”原因的强制重发。
- 模拟侧（STM32G431）接收侧（当前固件已实现 v0 版本）：
  - 成功解析并应用 SetPoint 后立即回 ACK；若 `seq` 与上次相同，则视为重传，仅回 ACK 不重复应用（幂等）。
  - 解析错误时可回 NACK，附错误码或最少头部（目前固件主要通过日志报告解码失败，NACK 仍为规划中的扩展）。
- 软复位协同：软复位握手完成后双方可重置与 SetPoint 相关的 `seq` 记忆，避免旧重传被误判（当前实现中，由上电后固定的初始 `seq` 与短重试窗口自然限制了该问题）。

- 消息集合与实现状态（v0）
  - 0x01 `HELLO`：G431→S3，上电或软复位后单次发送；当前固件已实现 v0 版本，载荷包含 `protocol_version` + 简单 `fw_version` 标识。
  - 0x02 `PING`：双向心跳/测延时；当前固件尚未实现，ID 预留给未来独立心跳帧（当前版本仅依靠 `FAST_STATUS`/控制帧作为隐式心跳）。
  - 0x03/0x04 `ACK`/`NACK`：原计划作为独立确认帧；当前固件不使用独立消息 ID，而是复用头部 `flags`（`FLAG_IS_ACK`/`FLAG_IS_NACK`）配合原始 `msg` 实现确认（例如 SetPoint ACK），ID 预留。
  - 0x10 `FAST_STATUS`：G431→S3 周期遥测；当前固件已实现 v0，字段与 `loadlynx_protocol::FastStatus` 结构一致（见下文表格）。
  - 0x11 `FAULT_EVENT`：G431→S3 故障事件帧；当前版本尚未启用此帧，故障状态通过 `FAST_STATUS.fault_flags` 传输，ID 预留。
  - 0x12 `SLOW_HOUSEKEEPING`：慢速供电/诊断帧；尚未实现，仅用于容量规划。
  - 0x13 `PdStatus`：G431→S3，USB‑PD 状态与能力摘要（Attach、合同电压/电流、可用 Fixed/PPS 档位及其最大电流）；规划中。
  - 0x20 `SetEnable`：S3→G431，布尔使能；当前固件已实现 v0，用于配合 `CAL_READY` 与 `FAULT_FLAGS` 做出力 gating。
  - 0x21 `SetMode`：S3→G431，**原子 Active Control（v1 冻结）**：一次下发 `preset_id + output_enabled + mode + target + limits`（见下文 “SetMode（0x21）原子控制帧”）。
  - 0x22 `SetPoint`：S3→G431，恒流设定值（mA，带 ACK）；当前固件已实现 v0 版本，将 `target_i_ma` 视为**两通道合计目标电流**，由 G431 在本地按“<2 A 单通道、≥2 A 双通道近似均分”的策略在 CH1/CH2 间拆分电流，由 `setpoint_tx_task` 实现 ACK 等待与退避重传。
  - 0x23 `SetLimits`/`LIMIT_PROFILE`：S3→G431，功率/电流/温度限值；尚未实现，未来用于热降额与风扇协同。
  - 0x24 `GetStatus`：S3→G431，请求立即返回一帧 FastStatus；协议 crate 中已有类型与编码函数，但固件尚未在运行路径中使用。
  - 0x25 `CalMode`：S3→G431，校准 Raw 遥测模式选择；仅在用户校准界面启用，用于指示模拟侧**按校准类型**附加 Raw ADC/DAC 字段（见 FastStatus 可选字段）。
  - 0x26 `SoftReset`：S3↔G431，软复位请求/确认；当前固件已实现 v0，使用同一 ID 配合 `FLAG_ACK_REQ/FLAG_IS_ACK` 区分请求与 ACK。
  - 0x27 `PdSinkRequest`：S3→G431，USB‑PD Sink 目标电压请求（固定 5V/20V；PPS 预留）；规划中。
  - 0x30 `CalWrite`：S3→G431，标定写入；用于**多块**下发用户校准点/曲线，G431 收齐并校验后加载本地校准并置位 `CAL_READY`。
  - 0x31 `CalRead`：G431→S3，标定读回；尚未实现，未来用于上行 `CAL_CHUNK`/EEPROM 校验。
  - 0x40+ 调试/诊断（如 `ADC_CAPTURE`、FOTA 等）：尚未实现，仅在下文表格中用于容量评估。

- 角色
  - ESP32‑S3：主控（UI/日志/标定/桥接），下发控制命令，轮询/订阅遥测。
  - STM32G431：实时控制与保护（独立安全），周期上报遥测/故障，关键故障本地强制失能。

### SetMode（0x21）原子控制帧（v1 冻结）

`SetMode` 是 v1 冻结的“原子 Active Control”消息：数字侧每次更新 active preset / 输出开关 / 模式与目标值时，都用 **同一帧** 完整下发，避免多帧更新导致两侧状态不一致。

- 方向：ESP32‑S3 → STM32G431
- 消息：`MSG_SET_MODE = 0x21`
- 可靠性：请求帧必须置位 `FLAG_ACK_REQ`；模拟侧成功解析并（按实现策略）应用后回 `FLAG_IS_ACK`（可选 `FLAG_IS_NACK` 用于解析失败）。
- `mode` 数值（冻结）：
  - `1` = CC
  - `2` = CV
  - 其他值保留（v1 不使用）

Payload（CBOR map，字段编号与 `loadlynx-protocol` 一致）：

| 字段 | 类型 | 单位 | 语义 |
| --- | --- | --- | --- |
| `preset_id` | `u8` | — | 1..=5（active preset 槽位） |
| `output_enabled` | `bool` | — | 用户输出开关；**应用 preset / 切换模式等高层操作必须强制置 `false`**，用户需手动开输出 |
| `mode` | `u8` | — | `1=CC` / `2=CV` |
| `target_i_ma` | `i32` | mA | CC 目标电流；为保持 wire 稳定，CV 模式下仍存在但忽略 |
| `target_v_mv` | `i32` | mV | CV 目标电压；为保持 wire 稳定，CC 模式下仍存在但忽略 |
| `min_v_mv` | `i32` | mV | 欠压阈值（用于欠压锁存）；触发后退流并锁存，需用户“关→开”解除（见 `state_flags[UV_LATCHED]`） |
| `max_i_ma_total` | `i32` | mA | 总电流上限（软件限值，与硬限制共同 clamp） |
| `max_p_mw` | `u32` | mW | 总功率上限（软件限值，与硬限制共同 clamp） |

- 波特率与节拍（建议）
  - 波特率：当前固件使用 115200 baud、8N1；在后续硬件与信号质量验证通过后，可按本节带宽估算提升到 460800–1M（结合布线与时钟抖动评估，最终以硬件定型为准）。
  - 遥测与控制频率：规划为空闲 10 Hz、工作 50–100 Hz；当前 v0 实际采用约 20 Hz `FAST_STATUS` + 10 Hz `SET_POINT`，其余消息类型仍处于规划阶段。

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
| `FAST_STATUS` (0x10) | 物理量字段：`uptime_ms`、`mode`、`state_flags`、`enable`、`target_value`、`i_local_ma`、`i_remote_ma`、`v_local_mv`、`v_remote_mv`、`calc_p_mw`、`dac_headroom_mv`、`loop_error`、`sink_core_temp_mc`、`sink_exhaust_temp_mc`、`mcu_temp_mc`、`fault_flags`；**校准模式下额外可选 Raw 字段**：`cal_kind`、`raw_v_nr_100uv`、`raw_v_rmt_100uv`（电压校准）、`raw_cur_100uv`、`raw_dac_code`（电流校准单通道） | ≈46 B（正常）/≈54–58 B（校准） | 当前固件：20 Hz；规划：UI 刷新 <60 Hz 时可提升到 50–60 Hz | 正常 2.8 kB/s；校准时增加 ≤0.5 kB/s | 高速遥测：正常工作仅发送物理量；当收到 `CalMode` 且进入校准时，模拟侧按类型只附加必要 Raw 数据以降低带宽 |
| `SLOW_HOUSEKEEPING` (0x12) | `vin_mv`、`vref_mv`、`board_temp`、`cal_state`、`diag_counters`、预留 | ≈16 B | 5 Hz | 80 B/s ≈ 0.64 kbps | 提供供电、校准、累计计数等慢变化信息；当前固件尚未实现，仅用于协议规划与带宽估算 |
| `PD_STATUS` (0x13) | `attached`、`contract_mv`、`contract_ma`、`fixed_pdos[[mv,max_ma]...]`、`pps_pdos[[min_mv,max_mv,max_ma]...]` | ≈32–120 B（按 PDO 数） | 0–2 Hz（按 Attach/协商事件触发） | ≤240 B/s ≈ 1.92 kbps | USB‑PD 状态与能力摘要：用于 UI 展示“可选档位/最大电流/当前合同”；协商成功与否仍可用 `v_local_mv` 做冗余验证；规划中 |
| `FAULT_EVENT` (0x11) | `timestamp_ms`、`fault_bits`、`fault_code`、`latched`、`extra` | ≈12 B | 按事件触发（预计 <5 Hz 峰值） | ≤60 B/s ≈ 0.48 kbps | 故障瞬时上报，附带锁存状态与附加参数；当前版本尚未启用独立 `FAULT_EVENT` 帧，故障状态通过 `FAST_STATUS.fault_flags` 传输 |
| `CAL_CHUNK` (0x30) | `offset_index`、`payload[32]`、`crc` | ≈48 B | 0.5–1 Hz，仅在标定模式 | ≤48 B/s ≈ 0.38 kbps | 标定阶段使用多块 `CalWrite` 下发校准点（见 `docs/dev-notes/user-calibration.md`）；上行 `CAL_CHUNK` 仍为预留 |
| `ADC_CAPTURE` (0x40) | `sample_rate`、`count`、`samples[128×u16]`、`checksum` | ≈260 B | ≤5 Hz（诊断时短时开启） | ≤1.3 kB/s ≈ 10.4 kbps | 供调试/上位机抓波使用，默认不发；当前固件尚未实现该数据块，保留作为诊断扩展 |

**典型带宽**（不含诊断）：`FAST_STATUS + SLOW_HK + 心跳/ACK 开销` ≈ 2.6 kB/s（≈ 20.8 kbps，按完整规划消息集估算；当前 v0 仅发送 `FAST_STATUS`，实际带宽显著更低）。<br>
**最坏情况**（100 Hz 遥测 + ADC 抓取）≈ 5.7 kB/s（≈ 45.6 kbps），仍低于 460800 baud 的 10% 左右；当前固件尚未启用 `ADC_CAPTURE` 等高带宽路径。

### ESP32‑S3 → STM32G431（控制、配置、维护）

| 数据块 | 字段概要 | 单帧字节 | 更新频率 | 估算带宽 | 备注 |
| --- | --- | --- | --- | --- | --- |
| `SET_POINT` (0x22) | `seq`、`target_i_ma`（mA，两通道合计 CC 设定值） | ≈18 B | 当前固件：10 Hz（编码器驱动）；规划：50–100 Hz | 0.9 kB/s ≈ 7.2 kbps（按 50 Hz 规划估算；当前 10 Hz 实际带宽约为其 1/5） | 当前固件已实现 v0：总电流恒流设定，全部要求 ACK；G431 将 `target_i_ma` clamp 到 `[0,5_000]` mA，并按 `<2 A 单通道、≥2 A 双通道近似均分` 在 CH1/CH2 间拆分目标电流；数字侧在 `setpoint_tx_task` 中实现 ACK 等待、退避重传与“最新值优先”，模拟侧应用后回 `FLAG_IS_ACK` 空载帧，并在 FastStatus 中回显 `target_value`（即总目标电流） |
| `SET_MODE` (0x21) | `preset_id`、`output_enabled`、`mode`、`target_i_ma`（mA）、`target_v_mv`（mV）、`min_v_mv`（mV）、`max_i_ma_total`（mA）、`max_p_mw`（mW） | ≈30–40 B | 0–10 Hz（按 UI/HTTP 操作触发） | ≤400 B/s ≈ 3.2 kbps | **原子 Active Control（v1 冻结）**：一次下发 active preset + 输出开关 + 模式/目标/限值；应用 preset 必须强制 `output_enabled=false`；需 ACK |
| `LIMIT_PROFILE` (0x23) | `max_i`、`max_p`、`ovp_mv`、`temp_trip`、`thermal_derate`、预留 | ≈20 B | 0.2–1 Hz（用户修改时） | ≤20 B/s ≈ 0.16 kbps | 每次下发表征最大允许功率/电流的参数，并附 ESP 根据风扇控制计算出的 `thermal_derate`，便于版本化；当前固件尚未实现此帧，风扇控制仅在文档与协议层预留 |
| `CONTROL_CMD` (0x20/0x24/0x25 等) | `SetEnable`、`ModeSwitch`、`GetStatus`、`FaultClear` 等短指令 | 8–12 B | 0–20 Hz（按键/脚本触发） | ≤160 B/s ≈ 1.3 kbps | 均带 ACK_REQ，失败可按 5/10/20 ms 退避重试；当前固件仅实际使用 `SetEnable(0x20)`，其余命令仍在规划中 |
| `CAL_MODE` (0x25) | `kind`（0=off,1=voltage,2=current_ch1,3=current_ch2） | ≈10 B | 仅在进入/退出校准 Tab 或切换通道时发送（<1 Hz） | ≈10 B/s | 用于让模拟侧按校准类型附加 Raw ADC/DAC 字段；正常工作保持 off |
| `SOFT_RESET` (0x26) | `reason`（u8，0=manual、1=fw_update、2=ui_recover、3=link_recover）、`timestamp_ms` | 6 B | 上电后一次；或 UI/脚本按需触发（<0.2 Hz） | ≈1.2 B/s | 数字侧通过 `SoftReset` 请求模拟侧软复位：G431 进入安全态并清空状态，然后以同 ID、带 `FLAG_IS_ACK` 的帧确认；当前固件已实现 v0 版本，数字侧在 ACK 缺失时给出警告但仍继续后续握手 |
| `PD_SINK_REQUEST` (0x27) | `mode`（u8，0=fixed、1=pps 预留）、`target_mv` | ≈14–20 B | 0–2 Hz（按 UI 点击/Attach 触发） | ≤40 B/s ≈ 0.32 kbps | USB‑PD Sink 策略请求：数字侧下发目标电压（本轮 5V/20V），模拟侧记录策略并在 Attach 时自动应用；电流请求策略固定为 3A 上限且不超过 PDO 能力；请求带 ACK_REQ，ACK 仅表示“接收/记录成功”，协商成败由 `v_local_mv` 等遥测侧推断；规划中 |
| `CAL_RW` (0x30/0x31) | `index`、`payload[32]`、`crc` | ≈48 B | 0.5 Hz（标定/量产） | ≤24 B/s ≈ 0.19 kbps | `CalWrite` 多块下发、`CalRead` 读回仍为预留；校准数据主存于 ESP EEPROM，模拟侧只缓存并执行校准 |
| `PING/HEARTBEAT` (0x02) | `timestamp`、`nonce` | 6 B | 10 Hz | 60 B/s ≈ 0.48 kbps | 空闲期保持链路活跃，>300 ms 无回应即判为降级；当前固件未实现独立 `PING` 帧，心跳由 `FAST_STATUS` 与控制帧隐式承担 |
| `RESERVED_FOTA` (0x50+) | （暂未定义——需后续 bootstub/升级协议落地） | 0 B | 0 Hz | 0 | 当前项目未实现固件块传输；仅保留 ID 以免未来扩展时与现有消息冲突 |

**典型带宽**：`SET_POINT + HEARTBEAT + 零星命令` ≈ 1.0 kB/s（≈ 8.0 kbps，按规划频率估算；当前 v0 仅使用其中子集，实际带宽更低）。<br>
**最坏情况**（含固件升级）≈ 11.2 kB/s（≈ 89.6 kbps）。

### 带宽结论与后续事项

- 典型双向总吞吐 ≈ 30 kbps，预留 >10× 余量供 SLIP/CBOR、重传与未来字段扩展。
- 遥测与设定频率允许在运行时调节：UI 刷新 <30 Hz 时可将 `FAST_STATUS` 降至 50 Hz，以降低平均带宽与功耗。
- 诊断/固件升级帧仅在专用模式启用；启用时需对 UI 提示“高带宽模式”，同时降低或暂停 UI 级 streaming。
- 下一步：在 `libs/` 中固化上述字段结构（Rust `struct` + CBOR map），并同步生成上位机/测试脚本使用的 schema。

### 心跳与失联保护

- **当前实现（v0）**
  - STM32G431（模拟侧）：
    - 不发送独立 `PING` 帧，链路健康完全基于接收到的控制类帧（`SetPoint`/`SoftReset`/`SetEnable`）的时间戳 `LAST_RX_GOOD_MS`。
    - 若自最后一次有效控制帧起超过 300 ms 未再收到任何帧，则视为链路异常：板载 LED1 以约 2 Hz 闪烁，并在 `FAST_STATUS.state_flags` 中清除 `STATE_FLAG_LINK_GOOD`。
    - 链路异常不会直接关闭 DAC 输出，但会在 UI 上与故障/enable gating 一起提示风险。
  - ESP32‑S3（数字侧）：
    - 不发送独立 `PING` 帧，链路健康基于最近一次成功解析的 `HELLO`/`FAST_STATUS`/ACK 的时间戳 `LAST_GOOD_FRAME_MS`。
    - `stats_task` 每秒检查一次该时间戳；若 300 ms 内无任何有效帧，则将 `LINK_UP=false`，`setpoint_tx_task` 在 `LINK_UP=false` 时 gate 掉新的 `SetPoint` 命令，并限频打印 `SetPoint TX gated (link_up=false, ...)` 日志。
    - 一旦再次接收合法帧，`LINK_UP` 被重新置 true，允许新的控制命令。
- **规划中的理想心跳机制（未实现）**
  - 预留 0x02 作为显式 `PING/HEARTBEAT` 消息，未来可以：
    - 在空闲时以 10 Hz 发送双向 PING 并测量 RTT；
    - 在 300 ms 无 `FAST_STATUS`/`FAULT_EVENT`/`PING` 时触发 `LINK_LOST` 降级状态，按本节早期草案中描述的方式（发送 `FAULT_EVENT(fault_code=LINK_LOST)`、UI 禁止控制等）进行处理。
  - 本节前文带宽估算与“5 Hz `FAULT_EVENT` 报告 LINK_LOST”等设计仍作为中长期规划保留，但尚未落地代码。
- **恢复流程（当前与规划共用）**
  - 任意一侧在失联后再次收到合法帧（CRC 正确、`ver` 兼容）即退出降级。
  - G431 在退出时仍要求重新确认 `enable`（必须等待新的 `SetEnable=1`），避免误导出力；
  - S3 在 UI 中同步清除告警，并允许新的控制命令。

### 软复位序列（数字侧触发、无需掉电）

- **目的**：在开发/调试阶段保持持续供电时，避免模拟侧残留旧状态（积分、限值、故障锁存等）导致行为不一致。
- **触发**：
  - 当前固件在启动 SetPoint 发送任务前，通过 `SoftReset(reason=fw_update, timestamp_ms=now)` 主动向模拟侧发起一次软复位握手；
  - UI/上位机后续可通过脚本/按钮按需再发（推荐限速 <0.2 Hz）。
- **模拟侧动作（G431，当前实现）**：收到请求即刻进入安全态：
  1. 通过内部逻辑拉低负载使能、清零 DAC 输出与目标电流；
  2. 清空故障锁存；控制环内部状态如积分器在当前实现中主要由硬件/简单环路承担；
  3. 记录 `reset_reason`，以同 ID、带 `FLAG_IS_ACK` 的帧回复 SoftReset ACK；
  4. 随后重新发送 `HELLO`，供数字侧确认版本并重新建立握手；
  5. 在收到新的 `SetEnable(true)` 与 `SetPoint` 之前，保持输出 gating。
- **数字侧动作（S3，当前实现）**：
  - 使用 `encode_soft_reset_frame(is_ack=false)` 发送请求，并在 150 ms 间隔内最多重试 3 次；
  - 若在重试窗口内收到带 `FLAG_IS_ACK` 的响应，则在日志中记录成功并继续后续流程；
  - 若最终未收到 ACK，则打印警告（例如 “soft_reset ack not received; proceed with caution”），但仍在 300 ms 静默后继续 CalWrite/SetEnable/SetPoint 流；推荐上层 UI 将此视为“软复位可能未完成”的降级状态。
- **幂等性与降级策略**：
  - 模拟侧将重复请求视为幂等的重新进入安全态操作，连续触发不会破坏状态机；`reason` 字段仅用于日志/诊断。
  - 未来理想设计中，如软复位后 300 ms 内未重新收到 `HELLO`，数字侧应在 UI 中标记“等待重握手”并禁止控制命令；当前实现仅通过 `LINK_UP`/FastStatus 缺失来反映这一状态。

### 近/远端电压与双电流采样

- **采样对**：STM32G431 的 ADC 同时采集本地 Kelvin 点与远端 Sense 线的一对电压（`v_local_mv`、`v_remote_mv`），以及两路电流（`i_local_ma`、`i_remote_ma`）。当前 v4.2 硬件装配下：
  - `i_local` 来源于 CH1 主功率分流器，覆盖 0–6 A 区间，对应功率通道 1；
  - `i_remote` 来源于 CH2 分流器，对应功率通道 2；
  - 总电流 `i_total_ma = i_local_ma + i_remote_ma`，用于功率估算与闭环误差计算。
- **远端电压判定**：远端 Sense 线可能未接入，被动浮空时读数不可用。G431 按以下规则决定是否采用远端值：
  1. 监测 `sense_remote_present`（硬件通过 FPC 检测或差分防呆）：仅当该信号有效时才尝试使用远端值；
  2. 验证远端差分在允许范围内（0.5–55 V）且绝对值未饱和（ADC 原码不接近 0 或满量程）；
  3. 连续 3 帧满足条件即置位 `state_flags[REMOTE_SENSE_ACTIVE]`，闭环输出、电压显示均以 `v_remote` 为准；
  4. 任一条件失败则在 2 帧内退出远端模式，回退至 `v_local` 并清除标志。
-- **字段含义**：
  - `v_local_mv` 始终反映板载 Kelvin 测点电压，供日志/诊断使用；
  - `v_remote_mv` 仅在 `REMOTE_SENSE_ACTIVE=1` 时用于控制，否则 UI 仍可显示（但会叠加“未连接”提示）；
  - `i_local_ma`/`i_remote_ma` 分别反映 CH1/CH2 实测电流：在 `<2 A` 总目标区间内，预期 `i_remote_ma≈0`；在 `≥2 A` 区间，两路电流预期近似均分，总电流约为二者之和。
- **UI 映射**：ESP32‑S3 根据 `state_flags` 决定右侧“REMOTE/LOCAL” 卡片的强调态；若远端失效，则 REMOTE 显示 `--.--` 并提示用户检查 Sense 线。

### FastStatus.mode 与 state_flags（v1 冻结）

`FAST_STATUS.mode` 与 `SetMode.mode` 使用相同数值：

| mode 值 | 含义 |
| --- | --- |
| `1` | CC |
| `2` | CV |

`FAST_STATUS.state_flags` 为 `u32` 位掩码（**不得复用已有 bit**）：

| bit | 常量名 | 含义 |
| --- | --- | --- |
| 0 | `STATE_FLAG_REMOTE_ACTIVE` | 远端 sense 生效（控制/显示以 `v_remote_mv` 为主） |
| 1 | `STATE_FLAG_LINK_GOOD` | 模拟侧认为链路健康（超时/错误时清零） |
| 2 | `STATE_FLAG_ENABLED` | 模拟侧输出 gating 为真（具体验证以固件实现为准） |
| 3 | `STATE_FLAG_UV_LATCHED` | 欠压锁存：触发后强制退流并锁存；仅能通过用户 `output_enabled` 关→开边沿清除 |
| 4 | `STATE_FLAG_POWER_LIMITED` |（建议）因功率上限进入限功率态 |
| 5 | `STATE_FLAG_CURRENT_LIMITED` |（建议）因电流上限进入限流态 |

### 散热片温度传感器布点

- **Tag1 — `sink_core_temp_mc`**：靠近 MOSFET / 散热片热点一侧的 10 k NTC（实物板为 TS2 / R40，贴附在两颗 MOSFET 之间的铝散热片中心，处于风洞内部但远离直接出风位置），用于捕捉 FET 热源附近的最高温度。闭环降额、温度保护以此为主依据。
- **Tag2 — `sink_exhaust_temp_mc`**：位于散热片外侧/出风口附近的同规格 NTC（实物板为 TS1 / R39，固定在散热片侧面、靠近出风口但避开风直接吹拂，位于“吹不到风”的壳体侧壁），反映整体热质量/外壳温度，辅助判断风扇异常或环境散热不佳。
- **遥测语义**：`FAST_STATUS` 同时上报两个通道，ESP 可以：
  - 对 `sink_core_temp_mc` 设快速阈值（如 90 °C 降额、100 °C 触发故障）。
  - 对 `sink_exhaust_temp_mc` 设较慢积分阈值，检测风扇停转或壳体散热恶化（若核心温度正常但侧温持续上升，提示风道堵塞）。
- **标定**：量产时需在 25 °C 环境下测量两个传感器的原始 ADC 与实际温度，写入 ESP EEPROM；运行中 G431 使用校准后的 m°C 数值。
- **MCU 内部温度**：`mcu_temp_mc` 取自 STM32G431 的片上温度传感器，频繁上报以提供 PCB 参考温度。它用于：
  - 佐证 MOSFET 引脚与铜箔的热耦合情况（若散热片温度低但 MCU 温度高，说明 PCB 积热或外壳隔热问题）。
  - 在风扇/散热片被更换时，校验 PCB 与散热体之间的导热垫或螺丝是否压紧。
  - 作为最后的热保护冗余：当前固件在 `mcu_temp_mc` 超过约 110 °C 时通过 `fault_flags` 触发 `FAULT_MCU_OVER_TEMP` bit；未来可进一步扩展为独立 `FAULT_EVENT(fault_code=PCB_OVERHEAT)`，用于更丰富的故障事件记录。

### 风扇控制职责（ESP 主导）

- **硬件归属**：PWM（`FAN_PWM`）与转速反馈（`FAN_TACH`）均接在 ESP32‑S3（`GPIO39/40`，参考 `docs/interfaces/pinmaps/esp32-s3.md:81-82`），STM32G431 仅通过温度/功率遥测提供输入。
- **控制路径**：
  1. G431 在 `FAST_STATUS` 内上报 `sink_core_temp_mc`、`sink_exhaust_temp_mc`、功率与负载信息。
  2. S3 根据这些数据执行风扇曲线（或闭环 PID），直接生成 PWM、占空比与 `thermal_derate`（0–100%，写入 `LIMIT_PROFILE`）并保存在自身 EEPROM。其中风扇控制主要依据 CORE NTC（Tag1 / TS2 / R40，对应 `sink_core_temp_mc`）；EXHAUST NTC（Tag2 / TS1 / R39，对应 `sink_exhaust_temp_mc`）用于判断风道/壳体散热情况（例如风道堵塞或风扇异常等扩展逻辑）。
  3. 当散热不足时，S3 降低 `thermal_derate` → 通过 `LIMIT_PROFILE` 或即时 `CONTROL_CMD` 告知 G431：最大允许功率/电流需要降额。
- **遥测协同**：
  - STM32 不再上报 `fan_pwm`/`fan_rpm` 字段；此类数据由 ESP32 本地记录，可通过 UI 或上位机直接读取 ESP 端日志。
  - `SLOW_HOUSEKEEPING` 中亦不包含风扇配置 ID，相关配置完全由 ESP 侧管理。
- **容错**：ESP 负责风扇供电/报警逻辑（例如检测 Tach 丢失并提示用户），G431 若检测到高温仍会触发本地热保护并失能功率级，形成双重保护（当前实现通过 `fault_flags` 反映相关状态，独立 `FAULT_EVENT` 帧尚未启用）。
- **当前固件状态**：截至 v0，ESP32‑S3 固件尚未在代码中初始化 `FAN_PWM/FAN_TACH` 引脚，也未实现风扇调速/转速监测任务；本小节描述的风扇控制职责与 `LIMIT_PROFILE/thermal_derate` 协议字段仍处于预留阶段。

### 标定数据主存与上电同步

- **持久化所在**：当前硬件未为 STM32G431 配置 EEPROM/Flash 分区；所有校准参数（增益/偏置/温补/风扇曲线等）原则上由 ESP32‑S3 通过其本地 EEPROM/Flash 保存，视作“唯一可信源”。
- **当前固件实现（v0）**：
  - 数字侧在链路建立后按 `CalWrite` 多块协议下发三条校准曲线（电流/近端电压/远端电压）；
  - 模拟侧收齐并校验后加载点数组，运行时自行插值/反插值并置 `CAL_READY=true`；
  - `CalRead`/上行 `CAL_CHUNK` 仍为规划中的扩展，当前不依赖模拟侧持久化。
- **规划中的完整握手**：
  1. G431 上电即发送 `HELLO` 并置 `CAL_REQ` 标志，控制环保持保守安全值（仅允许 Idle）。
  2. S3 完成自检后，从本地 EEPROM 读取标定块，逐帧下发 `CAL_RW`（0x30/0x31）。
  3. G431 收齐所有块并验证 CRC 后，回复确认并清除 `CAL_REQ`，此后才允许 `SetEnable`/`SetPoint`。
- **版本/回退**：ESP 端需要维护 `CAL_VERSION` 与 `HW_REV`，若板子更换传感器或参数失配，可通过 `CAL_CHUNK` 上行把运行中的实时值回传到 ESP/上位机，以便重新烧录；若数据损坏，G431 仍能用“默认限幅”运行但输出被限制。
- **量产流程与容错建议**（规划）：在治具写入阶段，ESP 作为控制端将新的标定结果通过 `CAL_RW` 推送给 G431，同时写入自身 EEPROM；流程完成后立刻做一次断电重启，验证“冷启动 → ESP 下发 → G431 恢复”这条链路。若在约 300 ms 内未能完成首次 `CAL_RW`，UI 应提示“标定未加载”，G431 保持失能状态，防止使用未匹配的补偿系数。

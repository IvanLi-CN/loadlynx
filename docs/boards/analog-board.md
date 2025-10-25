# 模拟板（功率板 + STM32G431）

本文件合并了“模拟板外设清单（已知）”与“功率级/环路设计笔记”。仅包含仓库现有文档与已提供情报中“已出现/已确认”的内容，不臆测未出现的器件与参数。

## 外设清单（已知）
- 计算与控制：
  - MCU：STM32G431（执行快速闭环与保护；对应 `firmware/analog/`）。
  - 闭环：硬件 CC（运放闭环），结合 ADC 采样与固件保护策略。
- 采样与设定：
  - 电流采样：分流电阻 Kelvin 接入 ADC。
  - 电压采样：分压 + RC 接入 ADC。
  - 温度：NTC 用于降额/关断。
  - 设定路径：模拟设定（DAC）或数字控制路径（二选一，具体以原理图定稿为准）。
- 负载功率级：
  - 负载器件：N‑MOSFET 在线性区作可变负载。
  - 放大器：环路运放（低漂/轨到轨，带宽满足环路指标）。
  - 分流电阻：低阻、高功率、开尔文焊盘。
  - 散热：散热器 + 风扇（热设计/曲线在项目文档有描述）。
  - 保护：过温、过压（TVS/串联）、SOA 降额；（可选）浪涌/软启动。
- 板间与外部接口：
  - 与控制板的板间连接：短、低阻路径；默认使用 UART 进行通信（见 `docs/uart-link.md`）。
  - 外部接口（项目文档整体描述）：USB‑C（供电/数据）、触发输入、AUX（SWD/UART）。
- 电源与时序：
  - 电源轨：12 V（风扇）、5 V（逻辑/USB）、3.3 V（MCU）。
  - 上电时序：先 3.3 V 内核，再使能模拟前端。

## 设计笔记（功率级/环路/热与保护）

Scope
- Power path for constant current (CC) / constant resistance (CR) modes
- Sensing (current/voltage), thermal management, protections

Requirements (initial)
- Input voltage range: TBD (e.g., 0–60 V)
- Max current: TBD (e.g., 0–20 A)
- Max power (continuous/peak): TBD (e.g., 200 W cont)
- Transient response target: TBD (e.g., < 50 µs to 10–90%)

Proposed Topology
- N‑MOSFET linear region as variable load
- Source shunt resistor for current sense; op‑amp closes loop to CC
- Optional CR via digital control setpoint mapping

Key Components (candidates)
- Power MOSFETs: see `docs/load_mosfet_candidates.md`
- Sense resistor: low‑ohmic, high‑power, Kelvin sense footprints
- Op‑amps: rail‑to‑rail, low‑offset, bandwidth adequate for loop target
- Heatsink + fan: see `docs/fan_heatsink_integration.md`

Protections
- Over‑temperature (NTC/PT100/PT1000 + threshold)
- Over‑voltage (TVS/series protection)
- Safe‑operating‑area derating (silicon + thermal)
- Inrush limiting and soft‑start path (optional)

Control & Dynamics
- Loop compensation notes and Bode targets
- Slew limiting for setpoint steps to avoid overshoot
- Stability across input V/I, temperature, and PCB tolerances

Open Items
- Finalize operating envelope
- MOSFET selection and parallel strategy
- Sense resistor power/thermal sizing
- Loop compensation prototype and validation plan

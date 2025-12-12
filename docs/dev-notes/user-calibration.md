# 用户手动校准功能（Web + 固件）需求与功能设计

本文件描述 LoadLynx DIY 电子负载的**用户手动校准**功能。校准由用户在 Web 控制台中按向导操作完成，用户使用外部万用表/电压电流表作为参考。设备默认使用出厂/已有校准数据工作，只有在用户明确“应用/保存”新校准后才切换到新参数。

> 说明：本设计采用**多点校准 + 分段线性插值**（piecewise linear），不考虑温度漂移与温补。

---

## 1. 目标与范围

### 1.1 目标

1. 让 DIY 用户在不依赖产线治具的情况下，通过浏览器完成电压/电流的基础校准。
2. 校准过程中提供“预览读数”，让用户在未应用新参数前评估效果。
3. 校准数据持久化在数字板（ESP32‑S3）外部 EEPROM，并在每次上电时自动下发给模拟板（STM32G431）。
4. 未加载合法校准数据时，模拟板保持失能（沿用现有 `CAL_READY` gating）。

### 1.2 范围（DIY 基础校准）

**包含：**
- 电压测量多点校准：`v_local_mv` 与 `v_remote_mv`（0–40 V 重点），用户采集 2–5 点，运行时按点集插值修正。
- 电流测量多点校准：总电流 `i_total_ma`（0.5–5 A 重点），用户采集 1–3 点，运行时插值修正。
- 校准点数组与候选/生效 profile 的保存、预览、应用与回退（默认/用户）。

**不包含：**
- 温度传感器标定与温补（NTC 与 MCU die）。
- 高阶拟合/复杂非线性模型（仅做分段线性）。

---

## 2. 术语与数据模型

### 2.1 Raw / Active / Preview 三类读数

- **Raw**：STM32 侧在校准模式下附加的**原始采样电压/设定码**（无用户校准），用于采点与前端预览输入；单位设计为便于短帧传输：  
  - ADC Raw 电压：**100 µV/LSB 的 i16**（字段名 `*_100uv`，量程覆盖 0–2.9 V）。  
  - DAC Raw 码：12‑bit DAC 码值（0–4095，`u16`）。  
  - Raw 字段由 `CalMode` 控制，只在校准时出现，且按 Tab 类型选择性发送：  
    - **电压校准**：额外上报 `raw_v_nr_100uv`（近端 ADC 电压）与 `raw_v_rmt_100uv`（远端 ADC 电压）。  
    - **电流校准（单通道）**：额外上报 `raw_cur_100uv`（当前通道 ADC 电压）与 `raw_dac_code`（当前通道 DAC 码）。  
- **Active‑calibrated**：STM32 基于本地点数组插值后的物理量（mA/mV/mW），设备运行与保护均以此为准。
- **Preview‑calibrated**：前端使用“候选点数组”对 Raw 再插值的预览值，**不影响硬件**，用于用户确认。

### 2.2 校准模型（多点分段线性）

用户在每个 Tab 中采集校准点数组：

```
points = [{raw_100uv, raw_dac?, meas_physical}, ...]
```

- `raw_100uv` 为 STM32 上报的 ADC Raw 电压（100 µV/LSB）。  
- `raw_dac` 仅在电流校准时存在（12‑bit DAC 码），电压校准时为 0/省略。  
- `meas_physical` 为用户表计读数（电流 mA 或电压 mV）。

运行时对 Raw 读数做**分段线性插值**得到校准值：

1. 按 `raw` 升序排序并去重。
2. 对任一 `Q_raw`：
   - 若 `Q_raw` 介于相邻两点 `raw_i ≤ Q_raw ≤ raw_{i+1}`，则线性插值：
     ```
     t = (Q_raw - raw_i) / (raw_{i+1} - raw_i)
     Q_cal = meas_i + t * (meas_{i+1} - meas_i)
     ```
   - 若 `Q_raw` 小于最小点，使用首段斜率外推或直接 clamp 到 `meas_0`（默认外推）。
   - 若 `Q_raw` 大于最大点，使用末段斜率外推。

**退化规则：**
- 仅 1 点：退化为比例校准 `Q_cal = Q_raw * (meas/raw)`。
- 2 点：退化为一次直线（等价于 scale+offset）。

### 2.3 校准数据集合（CalibrationProfile + Points）

分开保存四组点数组（Tab 分离，电流按通道）：
- `current_ch1_points[]`：CH1 电流 `(raw_cur_100uv, raw_dac_code, i_meas_ma)`。
- `current_ch2_points[]`：CH2 电流 `(raw_cur_100uv, raw_dac_code, i_meas_ma)`。
- `v_local_points[]`：近端电压 `(raw_v_nr_100uv, v_meas_mv)`。
- `v_remote_points[]`：远端电压 `(raw_v_rmt_100uv, v_meas_mv)`。

每组点数组各自独立“预览/应用/保存/恢复默认”。运行时的 Active 校准由各自点数组派生。

---

## 3. 硬件/固件约束（决定校准实现）

1. **电流测量链路（v4.2）**  
   Rsense=50 mΩ，OPA2365 G≈10，固件中当前近似：  
   `i_chx_ma ≈ 2 * curx_sns_mv`，`i_total_ma = i_ch1_ma + i_ch2_ma`。  
   校准需在该基础上做线性修正。

2. **电压测量链路**  
   OPA2365 差分网络理想比例：  
   `V_load = (124/10) * V_SP`。  
   校准对 `v_local_mv`/`v_remote_mv` 分别做线性修正。

3. **校准数据主存**  
   模拟板无本地 EEPROM/Flash 分区；校准参数由数字板 EEPROM 持久化并通过 UART `CalWrite` 下发。  
   模拟板只在收到**合法**校准后置位 `CAL_READY`。

4. **校准应用职责分配（满足“STM32 自己校准、ESP 不修订”）**
   
   - **数字板（ESP32‑S3）是校准数据管理端**：  
     ESP 负责从 Web 收集并持久化点数组（EEPROM），必要时将点数组**预处理为 STM32 方便消费的固定格式**并通过 UART 下发。  
     ESP **不在运行时对遥测/设定做修订**：不会把 Raw 转成校准值再上报给 UI，也不会用校准曲线去反算 SetPoint。

   - **模拟板（STM32G431）是校准执行端**：  
     G431 接收由 ESP 下发的点/段数据，建立本地分段线性曲线，并在运行时：
       1) 将 ADC Raw（按理想比例换算得到的物理量）→ 校准后的物理量；  
       2) 将来自 ESP 的**物理量目标值**（mA/mV）→ 反向插值成 Raw 目标 → DAC 码；  
       3) FastStatus 上报的 `i_*_ma / v_*_mv / p_*_mw` 均为**校准后的物理量**。  
     因此两 MCU 间通信始终基于物理量，ESP 不承担校准计算。

   - **G431 侧反算（Inverse mapping）算法**：  
     对于目标 `Q_des`（mA 或 mV），G431 在本地点数组 `{(raw_i, meas_i)}` 上做反向插值：
     1) meas 必须单调递增（ESP/UI 侧在下发前验证）。  
     2) 找到 `meas_i ≤ Q_des ≤ meas_{i+1}`：
        ```
        t = (Q_des - meas_i) / (meas_{i+1} - meas_i)
        raw_des = raw_i + t * (raw_{i+1} - raw_i)
        ```
     3) 两端按首段/末段外推或 clamp。  
     4) `raw_des` 作为 CC 环比较电压目标，经现有 `0.5A→DAC_code` 标定点映射到 DAC 码。

---

## 4. 用户校准流程

Web 控制台提供“校准”入口，包含两个 Tab：**电压校准** 与 **电流校准**。每个 Tab 是一个向导式流程。

### 4.1 电压校准 Tab（用户主导设定）

**前提：** 用户有可调电源 + 万用表。

流程（建议 2–5 点）：
1. 用户把电源接到负载输入端，负载先保持低电流/关闭输出。
2. UI 提示用户将电源调到一个推荐点（如 12 V / 24 V / 36 V）。
3. 用户用表读实际电压 `V_meas` 并输入。
4. UI 在同一时刻锁定 Raw ADC 电压：`raw_v_nr_100uv` 与 `raw_v_rmt_100uv`。
5. 重复 2–4 采集多点。
6. UI 基于点数组计算候选分段线性曲线（并给出等价直线摘要用于展示），展示 Preview。
7. 用户确认后点击：
   - **应用并保存**：写 EEPROM + CalWrite 下发；
   - **仅临时应用**：只下发不写 EEPROM；
   - **取消**：不改变 active profile。

### 4.2 电流校准 Tab（方案 A：设备主导设定点）

**前提：** 用户有电流表/带电流显示的电源。

流程（推荐 1–3 点）：
1. UI 提示用户将电源设定到安全电压（如 12 V）并限流，连接电流表。
2. 用户先选择要校准的通道：CH1 或 CH2。系统将**只启用该通道**、关闭另一通道，并通过 `CalMode(kind=current_chX)` 让 STM32 仅附加该通道 Raw。
3. 用户在 UI 选择一个推荐电流点（如 1 A / 3 A / 4 A）；该值被视为“当前通道目标电流”。
4. Web 调用控制 API 下发对应 CC SetPoint，等待稳定（UI 显示倒计时/稳定提示）。
5. 用户读表得到实际电流 `I_meas` 并输入。
6. UI 锁定同一时刻 Raw：`raw_cur_100uv` 与 `raw_dac_code`（仅当前通道）。
7. 如需更好线性，可再选择一个点重复 3–6。
8. UI 基于点数组生成候选分段线性曲线（点少时退化为比例/直线），展示 Preview。
9. 用户确认并选择应用方式（同电压 Tab）。

完成 CH1 后，用户切换到 CH2 重复上述流程，形成 `current_ch1_points[] / current_ch2_points[]`。

**理由：** 设备主导推荐点更安全、流程更简单，符合 DIY 场景。

---

## 5. Web 前端设计

### 5.1 页面结构

- 顶部公共状态栏：
  - `analog_state`、`link_up`、故障提示；
  - 当前 Active profile 类型：`factory-default` 或 `user-calibrated`。
- Tabs：
  - 电压校准
  - 电流校准

### 5.2 关键 UI 元素

每个 Tab 内包含：
1. **步骤说明卡片**：接线/安全提示/推荐点。
2. **实时读数区**：显示 Raw / Active / Preview 三类读数：
   - Active 由 G431 侧校准后上报的物理量；
   - Raw 为 G431 侧 ADC 原始采样电压（100 µV/LSB 的 i16）与必要的 DAC 码（校准模式下额外上报），仅用于采点与预览；
   - Preview 由前端本地用候选点数组对 Raw 插值得到。
   - 电压 Tab：`raw_v_nr_100uv / v_local_active / v_local_preview`；远端同理（`raw_v_rmt_100uv`）。
   - 电流 Tab：`raw_cur_100uv / raw_dac_code / i_chx_active / i_chx_preview`（按当前校准通道 CH1/CH2 显示）。
3. **用户输入框**：`V_meas` 或 `I_meas`。
4. **采集按钮**：锁定一条标定点（raw + meas）。
5. **标定点列表**：可删除最近点；显示每点的 raw、meas、点位编号。
6. **候选曲线摘要与预览**：展示点数组、分段线性摘要（首/末段斜率、关键点误差）与“应用前后差异”。
7. **应用/保存/恢复按钮**：
   - Apply (RAM)
   - Commit (EEPROM)
   - Reset to default

### 5.3 前端预览计算

前端维护每个 Tab 的 `candidate_points[]`。对于每次 `/api/v1/status` 得到的 Raw 值：

```
preview = piecewise_linear(candidate_points, raw)
```

其中 `piecewise_linear` 与 2.2 小节算法一致。Active 与 Preview 并排显示，便于用户判断候选参数是否合理。

---

## 6. HTTP API 设计（ESP32‑S3）

遵循 `docs/interfaces/network-http-api.md` 的风格与错误码。

### 6.1 读取当前校准点与来源

`GET /api/v1/calibration/profile`

响应（示例：返回点数组）：
```jsonc
{
  "active": {
    "source": "factory-default" | "user-calibrated",
    "fmt_version": 1,
    "hw_rev": 42
  },
  "current_ch1_points": [
    { "raw_100uv": 25000, "raw_dac_code": 1800, "meas_ma": 3050 }
  ],
  "current_ch2_points": [],
  "v_local_points": [
    { "raw_100uv": 19400, "meas_mv": 24120 }
  ],
  "v_remote_points": [
    { "raw_100uv": 19300, "meas_mv": 24120 }
  ]
}
```

### 6.2 应用候选校准点（不持久化）

`POST /api/v1/calibration/apply`

请求：
```jsonc
{
  "kind": "current_ch1" | "current_ch2" | "v_local" | "v_remote",
  "points": [
    { "raw_100uv": 19400, "meas_mv": 12080 }
  ]
}
```

行为：
- 仅更新对应 `kind` 的 RAM 内 active points；
- 将该点数组预处理为 G431 可直接加载的分段数据（排序/去重/限点/定点化）；
- 通过 UART 以**多块 `CalWrite`** 下发该曲线数据给 G431（见第 7 节）；
- 下发完成且 G431 验证通过后，G431 置 `CAL_READY=true`，Active 物理量立即生效；
- 成功后返回 200。

错误：
- `LINK_DOWN` / `ANALOG_NOT_READY` / `ANALOG_FAULTED` / `INVALID_REQUEST`。

### 6.3 持久化候选校准点

`POST /api/v1/calibration/commit`

请求同 `apply`。  
行为：写 EEPROM → 更新 RAM active points → 预处理并下发多块 CalWrite。  
返回：200 或错误。

### 6.4 恢复默认校准

`POST /api/v1/calibration/reset`

请求：
```jsonc
{ "kind": "current_ch1" | "current_ch2" | "v_local" | "v_remote" | "all" }
```

行为：
- 清空对应点数组（回退到 factory‑default 行为）；
- 更新 active points；
- 下发恢复默认后的曲线（多块 CalWrite）。

---

## 7. CalWrite 多块下发（曲线同步）

`CalWrite` 由 ESP 向 G431 下发校准曲线数据。由于单帧 payload 仅 32 B，采用**多块分段**方式。  
每次 apply/commit/reset 都会完整下发对应曲线的全部块。

### 7.1 CalWrite payload 固定头 + 点数据

payload 结构（小端）：

| 偏移 | 长度 | 字段 | 类型 | 说明 |
| --- | --- | --- | --- | --- |
| 0 | 1 | `fmt_version` | u8 | 结构版本（当前=1） |
| 1 | 1 | `hw_rev` | u8 | 硬件版本标识 |
| 2 | 1 | `kind` | u8 | 0=v_local, 1=v_remote, 2=current_ch1, 3=current_ch2 |
| 3 | 1 | `chunk_index` | u8 | 当前块序号（从 0 开始） |
| 4 | 1 | `total_chunks` | u8 | 该曲线总块数 |
| 5 | 1 | `total_points` | u8 | 该曲线总点数（≤5） |
| 6 | 1 | `flags` | u8 | 预留 |
| 7 | 1 | `reserved0` | u8 | 预留 |
| 8 | 24 | `points` | bytes | 最多 3 个点（见下） |

点格式（每点 8 B，统一格式）：

| 偏移(相对点) | 长度 | 字段 | 类型 |
| --- | --- | --- | --- |
| 0 | 2 | `raw_100uv` | i16 |
| 2 | 2 | `raw_dac_code` | u16 |
| 4 | 4 | `meas_physical` | i32 |

- `raw_100uv` 为 ADC 原始采样电压（100 µV/LSB）；`raw_dac_code` 为 DAC 码（12‑bit）。  
- 字段语义随 `kind`：
  - v_local / v_remote：`raw_100uv`=对应电压 sense ADC 电压；`raw_dac_code`=0；`meas_physical`=mV。
  - current_ch1 / current_ch2：`raw_100uv`=对应电流 sense ADC 电压；`raw_dac_code`=对应通道 DAC 码；`meas_physical`=mA。
- 每块最多携带 3 点，不足部分用 0 填充。
- `CalWrite.crc`（结构体字段）为 inner CRC16，保护 `index+payload`。

### 7.2 G431 侧接收与完成条件

1. 按 `kind` 建立四组点数组缓存（RAM）。  
2. 每收到一块：
   - 验 inner CRC / `fmt_version/hw_rev`；
   - 将该块的点拷贝到 `points[kind]` 的对应位置；
   - 统计已收点数与块数。
3. 当某 `kind` 的 `total_chunks` 块全部收到且点数齐全：
   - 按 `raw` 升序排序、去重；
   - 校验 `meas` 单调递增（不满足则拒绝该曲线）。
4. 四条曲线都完成且合法后，置 `CAL_READY=true`。

### 7.3 ESP 侧下发顺序（推荐）

每次同步按顺序发送：
1. current_ch1 曲线全部块  
2. current_ch2 曲线全部块  
3. v_local 曲线全部块  
4. v_remote 曲线全部块

如遇链路异常或 G431 未进入 ready 状态，ESP 端应重发整套曲线。

---

## 8. EEPROM 存储与上电同步

### 8.1 EEPROM 内容

数字板 EEPROM 持久化四组点数组：
- `current_ch1_points[] / current_ch2_points[] / v_local_points[] / v_remote_points[]`
- `fmt_version`、`hw_rev`、`crc32`
可选：缓存一份“预处理后的 chunk”以加速冷启动下发。

### 8.2 启动流程

1. ESP 上电读取 EEPROM：
   - 校验 `fmt_version/hw_rev/crc` 通过 → 作为 Active profile；
   - 否则 → 使用固件默认 profile（factory‑default）。
2. UART 链路建立后 ESP 按第 7 节完整下发四条曲线（多块 CalWrite）：
   - G431 校验 inner CRC 与版本，收齐并验证曲线后置 `CAL_READY=true`。

---

## 9. 模拟板（G431）侧行为

1. `CalWrite` 多块解包：
   - 逐块计算 `crc16_ccitt_false(index + payload)` 与 `CalWrite.crc` 比较；
   - 校验 `fmt_version/hw_rev/kind/total_chunks`；
   - 收齐四条曲线并验证合法后置 `CAL_READY=true`。
2. 运行时应用：
   - 基于点数组对 ADC Raw 物理量做分段线性插值得到 calibrated 值；
   - FastStatus 上报 calibrated 物理量；
   - SetPoint（物理量目标）在 G431 侧反向插值得到 Raw 目标，再映射到 DAC 码；
   - 保护与 enable gating 使用 calibrated 值。

---

## 10. 安全与异常处理

- 校准期间：
  - 若 `analog_state != ready` 或故障存在，禁止进入向导并提示原因。
  - 设备输出始终受软限与硬限保护；推荐点不超过额定电流/功率。
- 参数合法性：
  - 对每条曲线计算各段斜率 `k = Δmeas/Δraw`，要求落在合理范围（例如 0.8–1.2）且全段同号；异常则拒绝 apply/commit。
  - EEPROM 校验失败时自动回退 factory‑default，并在 UI 显示“未用户校准”。

---

## 11. 可选扩展（非本期必需）

- 增加每通道电流（CH1/CH2）独立点数组与插值（当前仅总电流）。
- 支持每曲线更多点（>5），并扩展 CalWrite chunk 规则。
- 增加 CalWrite ACK/读回，用于更严格的下载完成确认。
- UI 中增加“自动建议下一校准点”与更详细误差统计。

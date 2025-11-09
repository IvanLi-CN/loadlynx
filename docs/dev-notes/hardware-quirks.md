# 硬件设计约定与易混点（Hardware Quirks & Conventions）

目的：记录本项目在跨板命名、接口方向、上电行为等方面的“约定/偏好/易混点”，便于设计、调试与协作时统一口径。

## 1. 跨板命名对照（FPC 连接）

- +5V 轨使能
  - 数字板（ESP32‑S3）命名：`ALG_EN`
  - 模拟板（STM32G431）命名：`5V_EN`
  - 方向/电平：有效高；数字侧固件上电延时 10 ms 拉高。
  - 参考：
    - 数字板网表：`docs/power/netlists/digital-board-netlist.enet:1681`（FPC1 第 3 脚 `ALG_EN`）。
    - 模拟板网表：`docs/power/netlists/analog-board-netlist.enet:5092`（FPC1 第 14 脚 `5V_EN`）。
    - 固件：`firmware/digital/src/main.rs:257`, `firmware/digital/src/main.rs:258`, `firmware/digital/src/main.rs:322`。

- 串口链路（ESP ↔ STM32）
  - 数字板（FPC 侧）命名：`RX` / `TX`。
  - 模拟板（FPC 侧）命名：`ESP_RX` / `ESP_TX`（经隔离后转为本板 `RX` / `TX`）。
  - 固件引脚：
    - ESP32‑S3：`GPIO17=U1TXD`、`GPIO18=U1RXD`（`firmware/digital/src/main.rs:333`, `:342`, `:343`）。
    - STM32G431：`USART3` → `PC10=TX`、`PC11=RX`（`loadlynx.ioc:204-206`，`firmware/analog/src/main.rs:34-45`）。
  - 参考：
    - 模拟板网表示例：隔离器 U13 `pins 6/7 → ESP_RX/ESP_TX`，FPC1 `pins 15/16 → ESP_TX/ESP_RX`（`docs/power/netlists/analog-board-netlist.enet:5032-5094`）。
    - 数字板网表 FPC1：`docs/power/netlists/digital-board-netlist.enet:1640-1687`（`RX`/`TX`）。

> 提示：FPC 连接器在两板上的朝向不同，针脚编号不对号（例：数字侧 `3=ALG_EN`，模拟侧 `14=5V_EN`）。以“网络名”对齐连线，不以“针号”映射。

## 2. ESP32‑S3 启动相关易混点

- PAD‑JTAG 复用：`MTCK/GPIO39` 与 `MTDO/GPIO40` 默认占用，若用作风扇 `FAN_PWM/TACH` 需在启动早期禁用 PAD‑JTAG。
  - 参考：`docs/interfaces/pinmaps/esp32-s3.md:93` 起（风扇引脚与 JTAG 提示）。

- FSPI 组上电脉冲：`GPIO8–GPIO13` 等在上电数十微秒内会有低脉冲；作为复位脚（如 `CTP_RST/TFT_RST`）需评估容忍度或增加 RC 延时。
  - 参考：`docs/interfaces/pinmaps/esp32-s3.md`（引脚注意事项）。

- USB DP/DM 编号易错：文档中 26/27 号引脚容易偏移一位，已在 pin map 文档更正。

## 3. 供电/测量约定

- +5V 轨（TPS82130SILR）：由 `ALG_EN/5V_EN` 控制，数字侧固件默认开机 10 ms 后置高；如需禁用，构建时关闭默认特性。
  - 构建禁用方式：`cargo +esp build --no-default-features`（或移除 `enable_analog_5v_on_boot`）。

- VBUS 检测：`GPIO4=VBUS_SENSE` 分压（Rp=75 kΩ、Rd=10 kΩ，并联 C≈10 nF），USB 5 V 时分压点约 0.588 V。
  - 参考：`docs/interfaces/pinmaps/esp32-s3.md:150` 起。

## 4. 命名偏好

- 跨板控制线：数字侧沿用 `ALG_EN`，模拟侧沿用 `5V_EN`；文档中并列标注（示例：`ALG_EN/5V_EN`）。
- 串口方向：在跨板网络上统一使用 `ESP_TX/ESP_RX` 表示“相对于 ESP 的方向”。

## 5. 联调小抄（Checklist）

- 5V 轨：ESP 上电日志应出现“enabling TPS82130… enabled”字样；万用表确认 +5 V 存在。
- UART 回环：
  - 快速本地：短接 ESP `GPIO17↔GPIO18`、或 STM32 `PA9↔PA10`，观察回环计数。
  - 跨板：ESP 发送 `PING\n`，应在 ESP 端周期看到 `uart rx … bytes`；STM32 侧当前仅 Echo，不打印统计属正常。

—— 若本文与原理图/网表有出入，以最新原理图/网表为准，并在此处同步更正。

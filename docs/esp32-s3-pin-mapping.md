# ESP32-S3FH4R2 控制板引脚分配说明

> 资料来源：`docs/other-datasheets/esp32-s3.md`（Espressif《ESP32-S3 Series Datasheet v2.0》）  
> 适用硬件：LoadLynx 控制板（最新版原理图截图 & 文本清单）  

## 1. 芯片型号确认

- 原理图中使用裸片形式的 ESP32-S3，内部 Flash/PSRAM 通过 `VDD_SPI` 取电且未布线到外部，同时 33～38 号 GPIO 被自由分配。  
- 这与 **ESP32-S3FH4R2**（4 MB Quad-SPI Flash + 2 MB Quad-SPI PSRAM）特性一致：仅占用 30～36 号 Flash 引脚，GPIO33～GPIO38 对外可用。  
- 因此本文以 ESP32-S3FH4R2 为准；若实际改用 Octal PSRAM 的 R8/R16 系列，请重新校验 33～38 号引脚。

## 2. 用户版引脚清单与数据手册对照

| 项目 | 用户描述 | 数据手册核对 | 结论 / 建议 |
| --- | --- | --- | --- |
| UART 引脚编号 | 文档称 “引脚 48 → U0TXD / 49 → U0RXD” | Table 2‑1：Pin 49 = U0TXD、Pin 50 = U0RXD | **编号错位**，实际连线请确认焊盘 49/50。 |
| USB DP/DM 引脚编号 | 文档写 “引脚 26 → GPIO19 (DM)、27 → GPIO20 (DP)” | Pin 25 = GPIO19、Pin 26 = GPIO20、Pin 27 = GPIO21 | **整体偏移一脚**；需确认焊盘 25/26/27 的接线。 |
| JTAG 引脚复用 | `MTCK`/`MTDO` 驱动风扇 EN/PWM | 上电默认启用 PAD-JTAG（Table 3‑5） | 若保持 PAD-JTAG，风扇不会被固件接管。需在启动流程禁用 PAD-JTAG 或改用其它 GPIO。 |
| GPIO0 处理 | 作为 “BOOT” 信号 | GPIO0 为启动模式 Strapping（Table 3‑1） | 允许重用，但必须保证上电时保持高电平以进入 SPI Boot；若由按钮拉低需配合 CHIP_PU 复位。 |
| GPIO8/9/10/11/12/13/14 | 用作 I²C / SPI / LCD 控制 | 文档明确这些管脚在上电 60 µs 内会输出低脉冲（Table 2‑2）且属于 FSPI/SUBSPI 组 | 作为复位脚（CTP_RST/TFT_RST/RS 等）时，需确认外围能够容忍短暂低脉冲；如需避免，可加上电 RC 延时。 |
| GPIO19/20 (USB) | 直连 USB 差分 | Datasheet 提醒上电阶段出现两次 60 µs 高电平毛刺（Table 2‑2 注2） | 对外接的 Type-C PHY/开关一般安全，但若接到其它逻辑，需要确保毛刺不会误触发。 |

其余描述与数据手册一致；下文在修订版中统一更正脚位编号。

## 3. 更新后的功能分组

### 3.1 电源 / 时钟 / 控制

| Pin | 引脚名 | 板上网络 | 说明 |
| --- | --- | --- | --- |
| 1 | LNA_IN | LNA_IN | 射频低噪声放大器输入，仅供射频匹配。 |
| 2, 3 | VDD3P3 | 3V3 | 数字 IO & RF 3.3 V 供电。 |
| 4 | CHIP_PU | ESP_EN | 芯片主使能，高电平上电。 |
| 5 | GPIO0 | BOOT | Strapping 引脚，默认上拉=SPI Boot；下载模式需拉低 + 复位。 |
| 20 | VDD3P3_RTC | 3V3 | RTC 域供电。 |
| 29 | VDD_SPI | 3V3 | 内部 Flash/PSRAM 供电，保持 3.3 V。 |
| 46 | VDD3P3_CPU | 3V3 | 数字核供电。 |
| 55, 56 | VDDA | 3V3 | 模拟域供电。 |
| 53, 54 | XTAL_N / XTAL_P | 40 MHz 晶体 | 系统主时钟。 |
| 57 | GND / EPAD | GND | 大地与散热焊盘。 |

### 3.2 通信接口

| Pin | 引脚名 | 网络 | 说明 |
| --- | --- | --- | --- |
| 49 | U0TXD / GPIO43 | TX1 | 主 UART0 TX；确保连到排针或隔离器 A→G431。 |
| 50 | U0RXD / GPIO44 | RX1 | 主 UART0 RX；G431 → S3。 |
| 13 | GPIO8 | SDA | I²C 数据。 |
| 14 | GPIO9 | SCL | I²C 时钟。 |
| 12 | GPIO7 | INT | 触摸/显示中断输入。 |
| 16 | GPIO11 | MOSI | SPI 主数据输出（驱动显示/外设）。 |
| 17 | GPIO12 | SCLK | SPI 时钟。 |
| 18 | GPIO13 | CS | SPI 片选。 |
| 23 | GPIO17 | U1TXD | **与 STM32 通信的 ESP→STM32 串口 TX。** |
| 24 | GPIO18 | U1RXD | **与 STM32 通信的 STM32→ESP 串口 RX。** |
| 26 | GPIO20 | ESP_DP | USB D+，串联 22 Ω。 |
| 25 | GPIO19 | ESP_DM | USB D−，串联 22 Ω。 |
| 27 | GPIO21 | USB2_PG | USB 电源良好检测/开关反馈。 |

### 3.3 外设控制

| Pin | 引脚名 | 网络 | 说明 |
| --- | --- | --- | --- |
| 5 | GPIO0 | ENC_SW | 编码器按键输入（低有效）；[STRAP] 上电需保持高电平避免进入下载模式。 |
| 6 | GPIO1 | ENC_A | 旋转编码器相位 A（建议上拉/RC 去抖）。 |
| 7 | GPIO2 | ENC_B | 旋转编码器相位 B（建议上拉/RC 去抖）。 |
| 10 | GPIO5 | CTP_RST | 电容触摸控制器复位。 |
| 11 | GPIO6 | TFT_RST | TFT 模块复位。 |
| 15 | GPIO10 | DC | TFT Data/Command 选择。 |
| 19 | GPIO14 | RS | 兼容 DC/寄存器选择信号，注意上电毛刺。 |
| 39 | GPIO34 | 5V_EN | 5 V 电源开关使能输出（默认低，需按电源芯片要求配置上拉/下拉）。 |
| 43 | GPIO38 | BUZZER | 驱动蜂鸣器；需要禁用 PAD-JTAG 后可用。 |
| 44 | MTCK (GPIO39) | FAN_EN | 风扇使能，默认为 JTAG TCK。 |
| 45 | MTDO (GPIO40) | FAN_PWM | 风扇 PWM，默认为 JTAG TDO。 |

> **JTAG 复用提醒**：若要使用 MTCK/MTDO 作为普通 GPIO，需要在早期固件中调用 `esp_apptrace_jtag_disable()` 或烧录 `EFUSE_DIS_PAD_JTAG`；否则 PAD-JTAG 将占用这些引脚。

## 4. 全引脚索引（按编号排序）

标记：`[STRAP]` Strapping 引脚、`[FLASH]` Flash/PSRAM 总线、`[USB]` USB 专用、`[RF]` 射频、`[RESV]` 官方不建议挪用。

| Pin | ESP32-S3 引脚 | 项目网络 | 状态 | 备注 |
| --- | --- | --- | --- | --- |
| 1 | LNA_IN | LNA_IN | 已用 | [RF] 天线前端，仅射频用途。 |
| 2 | VDD3P3 | 3V3 | 已用 | 3.3 V 供电。 |
| 3 | VDD3P3 | 3V3 | 已用 | 3.3 V 供电。 |
| 4 | CHIP_PU | ESP_EN | 已用 | 芯片主使能脚，带 100 nF 去耦。 |
| 5 | GPIO0 | ENC_SW | 已用 | [STRAP] 按键需确保上电未按（高电平）。 |
| 6 | GPIO1 | ENC_A | 已用 | 编码器相位 A（建议上拉/RC 去抖）。 |
| 7 | GPIO2 | ENC_B | 已用 | 编码器相位 B（建议上拉/RC 去抖）。 |
| 8 | GPIO3 | — | 空 | [STRAP] (JTAG 选择)；保持浮空或固定电平。 |
| 9 | GPIO4 | RESET# | 已用 | 外部复位输入。 |
| 10 | GPIO5 | CTP_RST | 已用 | 上电会短暂低电平；外设需容忍。 |
| 11 | GPIO6 | TFT_RST | 已用 | 同上。 |
| 12 | GPIO7 | INT | 已用 | 触摸/显示中断输入。 |
| 13 | GPIO8 | SDA | 已用 | 上电 60 µs 低脉冲，请添加上拉。 |
| 14 | GPIO9 | SCL | 已用 | 同上。 |
| 15 | GPIO10 | DC | 已用 | 同上。 |
| 16 | GPIO11 | MOSI | 已用 | 同上。 |
| 17 | GPIO12 | SCLK | 已用 | 同上。 |
| 18 | GPIO13 | CS | 已用 | 同上。 |
| 19 | GPIO14 | RS | 已用 | 显示寄存器选择。 |
| 20 | VDD3P3_RTC | 3V3 | 已用 | RTC 供电。 |
| 21 | GPIO15 | BLK | 已用 | 背光使能/PWM。 |
| 22 | GPIO16 | — | 空 | 预留 IO。 |
| 23 | GPIO17 | U1TXD | 已用 | UART1 TX → STM32 RX。 |
| 24 | GPIO18 | U1RXD | 已用 | UART1 RX → STM32 TX。 |
| 25 | GPIO19 | ESP_DM | 已用 | [USB] D−（串 22 Ω）。 |
| 26 | GPIO20 | ESP_DP | 已用 | [USB] D+（串 22 Ω）。 |
| 27 | GPIO21 | USB2_PG | 已用 | [USB] 电源良好检测/反馈。 |
| 28 | SPICS1 | — | 保留 | [FLASH] 内置 PSRAM CS，不建议复用。 |
| 29 | VDD_SPI | 3V3 | 已用 | Flash/PSRAM 供电。 |
| 30 | SPIHD | — | 保留 | [FLASH]。 |
| 31 | SPIWP | — | 保留 | [FLASH]。 |
| 32 | SPICS0 | — | 保留 | [FLASH]。 |
| 33 | SPICLK | — | 保留 | [FLASH]。 |
| 34 | SPIQ | — | 保留 | [FLASH]。 |
| 35 | SPID | — | 保留 | [FLASH]。 |
| 36 | SPICLK_N | — | 保留 | [FLASH] 差分。 |
| 37 | SPICLK_P | — | 保留 | [FLASH] 差分。 |
| 38 | GPIO33 | — | 空 | 可用 IO；未被内置 Flash/PSRAM 占用（本板 ESP32‑S3FH4R2，Quad‑SPI）。 |
| 39 | GPIO34 | 5V_EN | 已用 | 5 V 电源开关使能；位于 29–42 范围内；未被内置 Flash/PSRAM 占用（本板 ESP32‑S3FH4R2，Quad‑SPI）。 |
| 40 | GPIO35 | — | 空 | 可用 IO；未被内置 Flash/PSRAM 占用（本板 ESP32‑S3FH4R2，Quad‑SPI）。 |
| 41 | GPIO36 | — | 空 | 可用 IO；未被内置 Flash/PSRAM 占用（本板 ESP32‑S3FH4R2，Quad‑SPI）。 |
| 42 | GPIO37 | — | 空 | 可用 IO；未被内置 Flash/PSRAM 占用（本板 ESP32‑S3FH4R2，Quad‑SPI）。 |
| 43 | GPIO38 | BUZZER | 已用 | 需禁用 PAD-JTAG。 |
| 44 | MTCK / GPIO39 | FAN_EN | 已用 | 默认 JTAG TCK。 |
| 45 | MTDO / GPIO40 | FAN_PWM | 已用 | 默认 JTAG TDO。 |
| 46 | VDD3P3_CPU | 3V3 | 已用 | 数字核供电。 |
| 47 | MTDI / GPIO41 | — | 空 | 默认 JTAG TDI，可作 IO。 |
| 48 | MTMS / GPIO42 | — | 空 | 默认 JTAG TMS，可作 IO。 |
| 49 | U0TXD / GPIO43 | TX1 | 已用 | UART0 TX。 |
| 50 | U0RXD / GPIO44 | RX1 | 已用 | UART0 RX。 |
| 51 | GPIO45 | — | 空 | [STRAP] 默认下拉；勿悬空。 |
| 52 | GPIO46 | — | 空 | [STRAP] 默认下拉；勿悬空。 |
| 53 | XTAL_N | 40 MHz | 已用 | 晶体负端。 |
| 54 | XTAL_P | 40 MHz | 已用 | 晶体正端。 |
| 55 | VDDA | 3V3 | 已用 | 模拟供电。 |
| 56 | VDDA | 3V3 | 已用 | 模拟供电。 |
| 57 | GND / EPAD | GND | 已用 | 焊盘加密接地/散热。 |

## 5. 进一步建议

1. **JTAG 与风扇控制**：上电阶段若南桥仍在 JTAG 模式，风扇可能停转；建议在 ROM 初始化后立即切换到 USB-JTAG 或彻底禁用 Pad-JTAG。  
2. **上电毛刺缓冲**：GPIO8～GPIO18 在 60 µs 内低电平，若外设对低电平敏感，可增加 RC 延时或用 PNP/PMOS 隔离。  
3. **USB 差分线**：保持 90 Ω 差分阻抗，布线旁边的 `ESP_DM/ESP_DP` 需远离高速噪声，并预留 ESD 防护器件。  

喵～以上就是新版完整 pin map，如有新原理图再更新一遍就好啦。***

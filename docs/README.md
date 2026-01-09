# 文档索引

- 板级总览（Boards）
  - `boards/analog-board.md`
  - `boards/control-board.md`

- 接口（Interfaces）
  - `interfaces/uart-link.md`
  - `interfaces/network-control.md`
  - `interfaces/network-http-api.md`
  - `interfaces/pinmaps/esp32-s3.md`

- 器件与选型（Components）
  - MOSFET：`components/mosfets/selection.md`
  - 运放：`components/opamps/selection.md`
  - 运放速查：`components/opamps/ti/*`, `components/opamps/sgmicro/*`
  - 二极管：`components/diodes/mbr30100ct.md`

- 热设计（Thermal）
  - NTC：`thermal/ntc-temperature-sensing.md`
  - 过温保护规范：`thermal/over-temperature-protection.md`
  - 风扇与散热片：`thermal/fans/*`, `thermal/heatsinks/*`

- 电源与保护（Power）
  - 负载开关：`power/tps22810-power-switch.md`

- 软件开发笔记
  - ESP32-S3 启动流程：`dev-notes/software.md`
  - 硬件设计约定与易混点：`dev-notes/hardware-quirks.md`
  - MCU Agent 服务设计：`dev-notes/mcu-agentd.md`
  - 用户手动校准功能：`dev-notes/user-calibration.md`
  - 蜂鸣器提示音管理器：`plan/0007:prompt-tone-manager/PLAN.md`

- 外部数据手册（Other Datasheets，MinerU 转换）
  - ESP32‑S3：`other-datasheets/esp32-s3.md`
  - 触控：`other-datasheets/d-ft6336u-datasheet-v1-1.md`
  - MOSFET：`other-datasheets/irfp*.md`
  - 其他：`other-datasheets/*`

图像资源按文档归档于 `assets/<document-name>/`，引用请使用相对路径，例如：

```
![](../assets/tps22810/figure.jpg)
```

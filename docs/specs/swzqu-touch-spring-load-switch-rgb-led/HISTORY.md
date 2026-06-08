# History

## Origin

- Migrated from legacy planning docs into the canonical specs taxonomy.

## Key Decisions

- Preserve the legacy spec ID `0021` and slug `touch-spring-load-switch-rgb-led` for traceability.
- Keep the original planning scope traceable while assigning long-lived requirements to `SPEC.md` and implementation/history records to companion documents.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

### 变更记录（Change log）

- 2026-01-19: 创建规格 #swzqu
- 2026-01-19: 补齐硬件约束：亚克力 0.8–1.0mm、弹簧顶到背面、RGB 共阳端接 3V3
- 2026-01-19: 冻结范围与验收：状态映射（绿/红/黄）、异常判定与闪烁优先级
- 2026-01-21: 扩充语音播放（MAX98357A / I²S），并按“连续封装引脚”重排 GPIO（AMP_SD_MODE=GPIO34；I²S=GPIO35/36/37；RGB=GPIO38/39/40）
- 2026-02-01: HIL：触摸弹簧 + RGB 已跑通；新增 I²S/MAX98357A 输出任务与启动自检音（用于后续语音片段落地）
- 2026-02-01: HIL：降低“未触碰即触发”的误触风险（提高阈值 + 连续采样判定）
- 2026-02-02: 数字板：扬声器启动自检改为 WAV(PCM16LE mono, 8kHz) 资产播放（I²S=8kHz, mono→stereo duplication；固件侧 PCM digital gain +6dB）；boot playlist: 440/554/659/880 + test melody；PC 可直接预览 `firmware/digital/assets/audio/*.wav`
- 2026-02-03: 数字板：扬声器播放尾部等待改为按 playlist 时长（避免 UI 音效额外阻塞多秒）

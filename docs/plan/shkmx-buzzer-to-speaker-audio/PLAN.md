# 音频迁移：蜂鸣器 → 扬声器（#shkmx）

## 状态

- Status: 部分完成（2/3）
- Created: 2026-02-01
- Last: 2026-02-03

## 背景 / 问题陈述

- 当前数字板固件的提示音/告警音输出依赖 `GPIO21=BUZZER`（LEDC PWM；`firmware/digital/src/prompt_tone.rs`）。
- 本计划前提：**蜂鸣器相关器件不贴装**（蜂鸣器本体及其驱动链路 DNP），导致提示音/告警音在实际设备上“无声”，影响可用性与安全提示。仓库内的 `BUZZER` 网络名/引脚分配属于原理图/连线层信息，不能推导“实际已装配”。
- 术语：本文所称“当前硬件版本”指本计划覆盖的装配版本（不做 `hw_rev` 区分）。
- 数字板原理图/网表已包含 I²S 数字音频功放 `MAX98357AETE+T`（`docs/power/netlists/digital-board-netlist.enet`：`U6`），可驱动扬声器；因此需要把所有“原本通过蜂鸣器输出的音频”迁移到扬声器上。

## 目标 / 非目标

### Goals

- 在当前硬件版本上，通过扬声器提供与现有 `prompt_tone` 语义一致的 UI 反馈音与连续告警音。
- 统一“音频输出后端”，让 `prompt_tone` 不再依赖蜂鸣器 PWM。
- 文档明确声明：当前硬件版本不贴装蜂鸣器相关器件，并更新控制板外设说明（不更改引脚分配）。

### Non-goals

- 不在本计划中实现语音播报的“内容体系/资源管线”（仅保证能发声并覆盖现有提示音/告警音语义）。
- 不新增静音/音量调节 UI 或远程接口。
- 不改变现有告警策略（优先级、抑制、清除需本地确认等），除非为扬声器输出所必需且经确认。

## 范围（Scope）

### In scope

- 数字侧固件 `firmware/digital/`：
  - 扬声器音频输出路径（I²S + MAX98357A），支持短促提示音与连续告警音的“可控播放/停止/静音”能力。
  - 将 `prompt_tone` 从蜂鸣器输出迁移为扬声器输出（保持既有语义与节奏区分）。
  - 不再支持蜂鸣器输出（仅扬声器）。
- 文档 `docs/`：
  - 更新控制板外设清单等说明文档，明确当前硬件无蜂鸣器贴装。
  - 不更改既有引脚分配（pinmap 保持不变）。

### Out of scope

- 模拟板（STM32G431）控制策略变更。
- 新增或修改 HTTP API / UART 协议。
- 音频压缩格式（MP3/Opus）与复杂混音/优先级系统（只覆盖现有提示音/告警音）。
- 不包含 Plan #0021 的语音播放功能测试（#0021 为独立功能测试项）。

## 需求（Requirements）

### MUST

- 当前硬件版本上：
  - 本地交互（触摸/旋钮 detent/按键）产生可辨识的低音量反馈音（等价于现有 `UiOk/UiFail/UiTick`）。
  - 连续告警音策略与现有 `prompt_tone` 语义一致：Primary 覆盖 Secondary；告警期间抑制其它声音；告警清除后仍需下一次本地交互确认才停止。
- 声音输出不得导致数字板主 UI 明显卡顿或导致 UART 链路掉线（可用性优先）。
- 需要有“硬静音”能力：进入错误路径/重启/任务退出时能确保功放/扬声器停止输出（避免卡在持续鸣叫）。
- 文档必须明确声明：当前装配默认不贴装蜂鸣器相关器件（蜂鸣器链路 DNP）；同时说明 pinmap/网表的 `BUZZER` 仅代表“网络与引脚分配”，不代表实际装配。

## 接口契约（Interfaces & Contracts）

None

## 验收标准（Acceptance Criteria）

- Given 当前硬件版本（不贴装蜂鸣器相关器件）并连接扬声器
  When 用户在屏幕上触摸一次
  Then 扬声器播放一次短促 `UiOk` 类反馈音，且 UI 仍可正常刷新/交互。
- Given 当前硬件版本并连接扬声器
  When 旋钮产生 detent（单步与快速连续）
  Then 扬声器对每个 detent 输出 `UiTick` 类短音（允许排队但不允许“长时间延迟后集中爆发”）。
- Given 进入 Primary/Secondary/Trip 任一连续告警状态
  When 告警仍处于 active
  Then 扬声器持续播放对应告警音并抑制其它提示音。
- Given 连续告警条件消失
  When 未发生本地交互
  Then 告警音继续保持；When 发生下一次本地交互 Then 告警音停止且该次交互的反馈音可正常播放。
- Given 构建产物与文档
  When 查阅 `docs/boards/control-board.md`
  Then 明确说明当前硬件版本不贴装蜂鸣器相关器件，且音频输出使用扬声器（MAX98357A/I²S）。

## 实现前置条件（Definition of Ready / Preconditions）

- 已冻结：仅支持扬声器输出；不做硬件版本区分；不引入蜂鸣器回退路径。
- 引脚分配为既定事实：不得改动既有引脚分配与网络命名；实现只按现有分配使用 `I2S_*` 与 `AMP_SD_MODE`，且不得试图复用/重映射 `GPIO21=BUZZER`。

## 非功能性验收 / 质量门槛（Quality Gates）

### Testing

- Unit tests: 若可行，将“提示音/告警音调度策略”抽出为纯逻辑单元并添加 host-side 单测，覆盖优先级与 ack 语义。
- Integration tests: `just d-build`（或等价命令）可通过，且不引入新的警告级错误（按仓库既有约定）。
- HIL: 在当前硬件上 `just agentd flash digital` + `just agentd monitor digital` 验证可听到 UI 反馈音与告警音（记录日志片段与结论）。

### Quality checks

- `just fmt`（或 `cargo fmt --all`）
- （如仓库已有）`cargo clippy ...` 不新增 warnings

## 文档更新（Docs to Update）

说明：计划阶段不修改 `docs/` 下非 `docs/plan/` 的文档；以下更新在实现阶段随代码一起落地并作为验收的一部分。

- `docs/boards/control-board.md`: 明确声明当前硬件版本已经没贴装蜂鸣器相关器件；音频提示走扬声器（MAX98357A/I²S）。

## 计划资产（Plan assets）

- None

## 资产晋升（Asset promotion）

None

## 实现里程碑（Milestones）

- [x] M1: 数字板：新增扬声器音频输出后端（I²S + MAX98357A），提供 play/stop/mute 的最小接口
- [x] M2: 数字板：`prompt_tone` 输出从蜂鸣器迁移到扬声器（保持既有告警/ack 语义）
- [ ] M3: HIL：在当前硬件验证可听到 UI 反馈音与告警音，并记录日志/结论

## 方案概述（Approach, high-level）

- 使用 I²S (TX master) 向 MAX98357A 输出 PCM；提示音用简单正弦/方波合成或预置 PCM 片段实现；告警音用可循环片段实现。
- 将 `prompt_tone` 的“调度/语义”与“实际音频输出”解耦：调度层只决定“播放哪种 sound”，输出层负责把 sound 渲染成 PCM 并发送到 I²S。
- Pop/静音：利用 `AMP_SD_MODE` 或“先送零样本再开声”的策略，减少启停 pop；并保证任何错误路径最终能静音。

## 风险 / 开放问题 / 假设（Risks, Open Questions, Assumptions）

- 风险：
  - I²S DMA/播放任务可能与 UI/网络/串口争用 CPU；需控制 buffer 大小与优先级。
  - 功放启停 pop/底噪；需要明确 `AMP_SD_MODE` 的电气语义与推荐时序。
- 需要决策的问题：None
- 假设（需主人确认）：None

## 变更记录（Change log）

- 2026-02-01: 创建计划 #shkmx
- 2026-02-03: digital: 将 `prompt_tone` 从蜂鸣器迁移到扬声器（MAX98357A/I²S），并更新控制板文档说明（待 HIL 声音验收）。

## 参考（References）

- `firmware/digital/src/prompt_tone.rs`
- `docs/power/netlists/digital-board-netlist.enet`（U6=MAX98357A）
- `docs/plan/0007:prompt-tone-manager/PLAN.md`
- `docs/plan/0021:touch-spring-load-switch-rgb-led/PLAN.md`

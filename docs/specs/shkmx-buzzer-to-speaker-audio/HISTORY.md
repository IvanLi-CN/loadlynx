# History

## Origin

- Migrated from legacy planning docs into the canonical specs taxonomy.

## Key Decisions

- Preserve the legacy spec ID `shkmx` and slug `buzzer-to-speaker-audio` for traceability.
- Keep the original planning scope traceable while assigning long-lived requirements to `SPEC.md` and implementation/history records to companion documents.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

### 变更记录（Change log）

- 2026-02-01: 创建规格 #shkmx
- 2026-02-03: digital: 将 `prompt_tone` 从蜂鸣器迁移到扬声器（MAX98357A/I²S），并更新控制板文档说明（待 HIL 声音验收）。
- 2026-02-05: HIL: 设备实测确认触摸/旋钮反馈音与连续告警音均可正常从扬声器输出；固件通过 `just d-build` + `just agentd flash/monitor digital` 验证。

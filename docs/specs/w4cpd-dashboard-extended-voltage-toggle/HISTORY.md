# History

## Origin

- Companion history initialized during docs/specs catalog migration.

## Key Decisions

- Preserve the existing spec ID `w4cpd` and canonical spec directory.

## Documentation Model

`SPEC.md` is the active topic contract. Historical rationale, evolution notes, and records moved out of the topic contract are kept here.

### 变更记录（Change log）

- 2026-03-09: 创建新 spec，冻结“左侧扩展电压开关 + 右侧 PD 设置入口”的初版设计边界。
- 2026-03-09: 根据主人反馈，右侧设置入口改为无蓝色外边框的深色圆底 + 白色滑杆图标。
- 2026-03-09: 根据主人反馈，左侧按钮恢复为原有 `PD` 顶部文案，不再额外引入 `EXT` / `FIXED` 之类的新标签。
- 2026-03-09: 主人确认当前设计稿，可进入实现阶段。
- 2026-03-10: 数字侧已完成 `allow_extended_voltage` 持久化、Dashboard 语义重排、Safe5V 门控与 `/api/v1/pd` 最小对齐；本地 `cargo fmt` 与 `just d-build` 已通过。
- 2026-03-10: 根据 review 修复 Safe5V 门控的电流语义：关闭“允许扩展电压”时仍保留保存的 `i_req_ma`，并按 5V PDO `max_ma` 做 clamp，避免回退到 3A 默认值。
- 2026-03-10: Web 端 `USB‑PD Settings` 在 `allow_extended_voltage=false` 时明确提示 Safe5V 锁定语义，避免 “Apply succeeded” 与实际合同停留 5V 的误解。
- 2026-03-10: 修复 Web mock PD 初始化的类型标注问题，确保 `bun run build` 的类型检查通过。
- 2026-03-10: UART link-down 被视为新 PD 会话开始时，同步清除扩展电压失败锁存，避免红态跨会话残留。

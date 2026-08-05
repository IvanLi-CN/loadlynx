# LoadLynx 跨规格实现一致性实现状态

> 当前有效规范仍以 `./SPEC.md` 为准；这里记录实现覆盖与交付进度。

## Current Status

- Implementation: 已完成
- Lifecycle: active
- Last updated: 2026-08-05
- Catalog note: 八项跨规格冲突已修复并由 Rust、Web 与 Storybook 回归覆盖。

## Coverage / rollout summary

- 已建立跨规格聚合边界，并保留各 canonical Spec 的领域所有权。
- LAN WiFi 写入对 direct/saved HTTP fail closed；CLI 不再暴露 insecure override。
- Calibration mode 写入统一进入按设备串行化的 coordinator；LAN scan 仅从当前私网 IPv4 派生。
- Storybook i18n 初始化不读取持久化 locale；PWA takeover 仅由 migration build flag 启用。
- 模拟板 ACK 前按 live capability、object position、类型、范围和功率校验 PD 请求。
- 默认中文 CC route 与版本链接完成文案/token 收口，并有稳定 Storybook states。
- 已确认 200 W CP 满量程与 EPR 28 V persistence/application 决策不属于待修复项。

## Remaining Gaps

- None for the eight tracked conflicts.

## Related Changes

- Current implementation is tracked by the local git history for this Spec topic.

## References

- `./SPEC.md`
- `./HISTORY.md`

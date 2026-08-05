# LoadLynx 跨规格实现一致性演进历史

> 这里记录影响本主题边界的关键原因；规范正文仍以 `./SPEC.md` 为准。

## Decision Trace

- 新增 umbrella Spec 作为八项已确认实现冲突的共同验收入口，不复制或替代各领域 canonical Specs。
- 将 200 W CP 满量程和 EPR 仅允许持久化/应用 28 V 记录为不可漂移约束；36 V/48 V capability 保持只读。
- Initiative、Issue、分支与 PR 编排不属于本文的长期产品契约。
- 2026-08-05 完成八项冲突修复；migration-only PWA takeover 由显式 build flag 控制，PD ACK 以最近 live capability 为准。

## Key Reasons / Replacements

- 多项冲突跨越 CLI/devd、Web、PWA 和模拟板固件，需要一个稳定 topic 连接领域规范、实现覆盖和最终一致性验证。
- 本规格没有 successor，也不 supersede 任何被引用 Spec。

## References

- `./SPEC.md`
- `./IMPLEMENTATION.md`

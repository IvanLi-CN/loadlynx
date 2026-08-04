# LoadLynx 跨规格实现一致性实现状态

> 当前有效规范仍以 `./SPEC.md` 为准；这里记录实现覆盖与交付进度。

## Current Status

- Implementation: 未开始
- Lifecycle: active
- Catalog note: 八项已确认的 implementation-versus-Spec 冲突待修复。

## Coverage / rollout summary

- 已建立跨规格聚合边界，并保留各 canonical Spec 的领域所有权。
- 已确认 200 W CP 满量程与 EPR 28 V persistence/application 决策不属于待修复项。

## Remaining Gaps

- LAN WiFi 写入仍存在 insecure override。
- Calibration route/layout 仍有 coordinator 外的 mode writers。
- LAN scan 仍接受任意 IPv4 seed 并扫描其 `/24`。
- Storybook 应用初始化仍读取 `localStorage`。
- 模拟板仍可能 ACK 无效 live PDO/APDO 请求后回退 Safe5V。
- PWA migration takeover 仍作用于普通 production build，且无 waiting worker 时 refresh 可能不发生。
- 默认中文 CC route 仍有硬编码可见英文。
- live source 仍有 daisyUI `link link-hover` token。

## Related Changes

- None

## References

- `./SPEC.md`
- `./HISTORY.md`

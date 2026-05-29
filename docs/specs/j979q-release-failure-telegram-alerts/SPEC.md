# j979q · Release 失败 Telegram 告警接入

## Summary
- 为 `Release (LoadLynx)` 接入统一的 repo-local Telegram notifier wrapper。
- 保留 repo-local `workflow_dispatch` smoke test，用于告警链路自检与 secret 轮换验证。
- 保持 Telegram 通知范围聚焦在发布失败，不覆盖普通 PR CI 失败。

## Scope
- 新增 `.github/workflows/notify-release-failure.yml`。
- 监听统一发布 workflow 的失败结果。
- 提供一个无输入的手动 smoke test 入口。

## Acceptance
- `Release (LoadLynx)` 失败时，wrapper 会发送 Telegram 告警。
- 告警首行必须是 Emoji + 状态 + 项目名。
- `workflow_dispatch` smoke test 能在默认分支成功触发 Telegram 通知。
- wrapper 的 `workflows:` 列表必须包含统一发布 workflow 名称。

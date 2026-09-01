# Release 失败 Telegram 告警接入

## Summary
- 为 `Release (LoadLynx)` 接入 Oidrune 的统一 workflow notifier。
- 调用方使用 OIDC 默认网关，不传 gateway override、OIDC audience 或 Telegram secret。
- 保留 repo-local `workflow_dispatch` smoke test，用于告警链路自检。
- 保持 Telegram 通知范围聚焦在发布失败，不覆盖普通 PR CI 失败。

## Scope
- 维护 `.github/workflows/notify-release-failure.yml`。
- 监听统一发布 workflow 的失败结果。
- 提供一个无输入的手动 smoke test 入口。
- 通过 `id-token: write` 授权调用方完成 OIDC workflow handoff。
- 固定调用 `IvanLi-CN/oidrune/.github/workflows/notify.yml` 到受信任的完整 commit SHA。

## Notification Contract

- Trusted notifier ref: `IvanLi-CN/oidrune/.github/workflows/notify.yml@e48822f99c6402a753ed86557ea029754cbab20b` (Oidrune `v0.1.14`).
- Both notification jobs pass `outcome: failure` and a caller-generated `summary`.
- Each summary includes `Project`, `Status`, `Result`, `Target SHA`, `Run URL`, and exactly one context title: `Failure title` for release failures or `Smoke title` for manual dispatch.
- The automatic job is triggered only by a completed `Release (LoadLynx)` run on `main` whose conclusion is `failure`.
- The manual job is triggered only by repo-local `workflow_dispatch`; validation does not dispatch it and therefore does not send a real notification.
- The caller does not pass `gateway_url`, `oidc_audience`, Telegram secrets, or any other secret mapping.

## Acceptance
- `Release (LoadLynx)` 失败时，wrapper 会发送 Telegram 告警。
- 调用方摘要包含项目名、状态/结果、目标 SHA、run URL 和失败标题。
- `workflow_dispatch` smoke test 保持可触发，并使用相同的 Oidrune notifier 合同。
- wrapper 的 `workflows:` 列表只包含统一发布 workflow 名称，不覆盖普通 PR CI 或其他 workflow。
- workflow contract test 检查完整 SHA、OIDC 权限、无 secret/override 传递、触发过滤和摘要字段。

## Related ADRs

- None

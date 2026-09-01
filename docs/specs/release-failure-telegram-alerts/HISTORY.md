# History

- Legacy compatibility identity: `#j979q`.

## Origin

- Companion history initialized during docs/specs catalog migration.

## Key Decisions

- Preserve the existing spec ID `j979q` and canonical spec directory.
- Keep Telegram failure notifications release-scoped after the release workflow was unified under `Release (LoadLynx)`.
- Replace the legacy `github-workflows` Telegram wrapper with Oidrune `notify.yml` pinned to the trusted `v0.1.14` commit `e48822f99c6402a753ed86557ea029754cbab20b`.
- Keep notification context owned by the caller so the project, status/result, target SHA, run URL, and failure/smoke title are explicit in every handoff.
- Use the default Oidrune gateway with caller-provided `id-token: write`; Telegram secrets and gateway overrides are not passed through the repository workflow.

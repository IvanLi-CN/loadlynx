# History

## Origin

- Created to repair the PR label and GitHub Actions release flow after the first official release showed that labels did not drive version updates and package-visible versions stayed disconnected from release tags.

## Key Decisions

- PR labels are the release intent source of truth.
- Version injection is performed at release build time; package manifests are not rewritten for each release.
- Development releases are channel-driven instead of created for every `main` push.
- Telegram remains scoped to release failures to avoid PR CI notification noise.
- The protected default branch stays PR-only with `0` required approvals; `Label Gate` and signed commits enforce merge eligibility instead of mandatory human review counts.
- The quality-gates declaration must stay bound to the actual workflow and branch-protection job names exposed by GitHub Actions, so local validation treats missing declared workflow/job bindings as a contract failure instead of only checking policy scalars.
- The quality-gates checker itself needs regression coverage, not only a live-repo smoke check, so contract parsing and drift detection can evolve safely as workflows are refactored.

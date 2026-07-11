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
- Workflow hygiene rules such as explicit `permissions` and bounded job runtime must also be enforced by repo-local validation, otherwise they regress too easily during unrelated workflow edits.
- Once `Code Check` owned the digital firmware formatting/build path, keeping a second standalone `Digital Check` workflow added duplicate runtime and a weaker parallel definition of success; the CI contract should prefer one authoritative digital gate.
- Owner-facing operation skills are part of the released user contract, not merely internal documentation. A docs/skill-only PR that changes released operation guidance must receive `type:patch` or higher, because users can install skills directly from the repository and release notes/assets are the traceable stable boundary.
- A merged PR with the wrong no-release label can be corrected by changing the source PR labels and dispatching `Release (LoadLynx)` with `pr_number=<PR>`. PR #100 used that path to publish `v0.5.2` after its released CLI operation docs were reclassified from `type:none` to `type:patch`.
- Host-tools version output is part of the released artifact contract. Release-time version injection is not complete unless owner-facing `loadlynx` and `loadlynx-devd` version commands report the same injected release tag that appears in the GitHub Release.
- Web version provenance follows the same rule: GitHub Pages must serve the exact validated release tarball, and a Pages deployment failure prevents the release from being created.

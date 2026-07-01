# Storybook framework runtime bundle budgets

## Problem

When Storybook uses the Vite builder plus `storybook/test`, static builds can emit a large framework-owned runtime file such as `vite-inject-mocker-entry.js`. Generic Vite chunk warnings do not distinguish that runtime from application-controlled preview chunks, so teams can end up chasing the wrong problem.

## Approach

- Keep explicit manual chunking for application-controlled code.
- Enforce repository-owned budgets from emitted artifacts instead of relying on builder warnings alone.
- Separate Storybook preview chunk budgets from framework-managed runtime budgets.
- Let CI fail on the explicit budget script, not on subjective log reading.

## LoadLynx policy

- `dist/assets/*.js`: app-controlled JS chunks, each capped at `250 kB`.
- `storybook-static/assets/*.js`: Storybook preview JS chunks, each capped at `250 kB`.
- `storybook-static/vite-inject-mocker-entry.js`: Storybook framework mocker runtime, capped separately at `1200 kB`.
- Storybook's Vite `chunkSizeWarningLimit` is raised to match this policy because the repository now has a more precise budget gate.
- Product-only Vite plugins such as PWA generation must be disabled for Storybook builds. If product code imports a Vite virtual module, Storybook should alias it to a no-op mock instead of generating product service workers for Storybook.
- Storybook browser tests may need `test.fileParallelism=false` when route stories intermittently render an empty canvas under the Vitest browser runner. This keeps route/app-shell stories deterministic without changing unit-test parallelism.

## Why this works

- Application regressions still fail fast when any project-controlled JS chunk grows past the agreed budget.
- Storybook framework noise is not ignored; it is tracked under a dedicated cap with an explicit label.
- The rule is stable across local builds and CI because it reads actual emitted files.
- Keeping Storybook out of product PWA generation avoids false precache failures on Storybook manager assets and keeps PWA evidence scoped to the actual Web Console.

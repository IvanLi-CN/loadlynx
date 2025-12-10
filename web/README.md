# LoadLynx Web Console (Scaffold)

This `web/` directory hosts the LoadLynx network control web console. At this
stage it only provides a minimal React + Vite + TypeScript shell aligned with
the `paste-preset` project conventions.

## Tech stack

- React 19 + TypeScript + Vite 7
- Bun (>= 1.0) as runtime and package manager
- Biome for linting/formatting
- Playwright for end-to-end tests
- Lefthook for local Git hooks

## Usage

From the `web/` directory:

```bash
bun install
bun run dev
```

Core scripts:

- `bun run dev` – start the Vite development server.
- `bun run build` – type-check and build for production.
- `bun run preview` – preview the built app.
- `bun run lint` – run `biome lint .`.
- `bun run format` – run `biome format --write .`.
- `bun run check` – run `biome check .`.
- `bun run test:e2e` – run Playwright E2E tests.

## CI versioning

- GitHub Actions calls `.github/scripts/compute-version.sh` to emit `APP_EFFECTIVE_VERSION` (from `APP_BASE_VERSION` or `package.json` plus git metadata).
- `scripts/write-version.mjs` consumes `APP_EFFECTIVE_VERSION` during `bun run build` to write `dist/public/version.json`.

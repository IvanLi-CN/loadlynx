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

## USB‑PD Settings

- Entry point: `Status` → `USB‑PD` card → `Open PD settings` (route: `/$deviceId/pd`).
- Required device endpoints (see `docs/interfaces/network-http-api.md`):
  - `GET /api/v1/pd` — read attach/contract/capabilities/saved config
  - `POST /api/v1/pd` — apply config; Web uses `POST` + `Content-Type: text/plain` with a JSON string body to avoid private-network preflight issues.

## Simulation devices & mock backend

- `mock://` devices are in-memory simulation devices handled entirely in the web app. They are helpful when you do not have hardware on hand, or when you want fast UI demos and automated tests. Real devices use `http://` or `https://` base URLs backed by the LoadLynx HTTP API.
- Empty list UX: when no devices are stored, the Devices page shows a primary prompt to add a real LoadLynx device and also offers an “Add simulation device” button. Clicking it creates a `mock://…` device so you can open CC, Status, Settings, etc., with mocked data.
- Developer mock devtools: the “Add demo device” banner is developer/test tooling controlled by `VITE_ENABLE_MOCK_DEVTOOLS` (runtime constant `ENABLE_MOCK_DEVTOOLS`). When enabled, it adds a quick “Add demo device” button that inserts a `mock://` entry during local development or QA.
- Environment flags (current implementation):
  - `VITE_ENABLE_MOCK_BACKEND` — allows creation of mock/simulation devices via the UI. Default (unset) is allowed. Set to `"false"` to reject new mock devices; previously stored `mock://` entries in `localStorage` still load and work.
  - `VITE_ENABLE_MOCK_DEVTOOLS` — controls visibility of developer-facing mock controls. `"true"`: always show; `"false"`: always hide; unset: enabled in `DEV` builds, disabled otherwise.
  - The mock backend itself is always available for `mock://` URLs; these flags only gate whether the UI lets users create new mock entries and whether the devtools banner is shown.
- Recommended setups:
  - Local development: leave both vars unset or set them to `"true"` → simulation devices plus devtools available.
  - CI / Playwright E2E: default dev config or explicitly set both vars to `"true"` → tests can freely create `mock://` devices.
  - Production (allow user exploration without hardware): `VITE_ENABLE_MOCK_BACKEND="true"`, `VITE_ENABLE_MOCK_DEVTOOLS="false"` → empty-state “Add simulation device” is visible, devtools banner is hidden.
  - Production (disable all new mock entries): `VITE_ENABLE_MOCK_BACKEND="false"`, `VITE_ENABLE_MOCK_DEVTOOLS="false"` → UI rejects creating new mock devices; previously stored `mock://` entries (if any) still load.

## CI versioning

- GitHub Actions calls `.github/scripts/compute-version.sh` to emit `APP_EFFECTIVE_VERSION` (from `APP_BASE_VERSION` or `package.json` plus git metadata).
- `scripts/write-version.mjs` consumes `APP_EFFECTIVE_VERSION` during `bun run build` to write `dist/public/version.json`.

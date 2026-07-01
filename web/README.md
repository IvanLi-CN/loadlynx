# LoadLynx Web Console

This `web/` directory hosts the LoadLynx network control web console. At this
stage it provides the browser console for device discovery, CC control, status,
USB-PD settings, firmware dry-runs, Web Serial ESP32-S3 flashing, calibration
and settings.

## Tech stack

- React 19 + TypeScript + Vite 7
- Tailwind CSS 4 with local OKLCH design tokens
- Local shadcn-style `ll-*` component classes and primitives
- Radix primitives for accessible component foundations
- lucide-react for icons
- i18next + react-i18next for `zh-CN` default UI and `en` fallback
- TanStack Router + TanStack Query
- Storybook for component/route evidence
- vite-plugin-pwa + Workbox for production app-shell caching and controlled updates
- Bun 1.3.14 (pinned in the repo-root `.bun-version`) as runtime and package manager
- Biome for linting/formatting
- Vitest for unit tests and Storybook story tests
- Playwright for end-to-end tests
- Lefthook for local Git hooks
- esptool-js for browser Web Serial ESP32-S3 flashing

The Web UI deliberately does not use daisyUI. Do not add `daisyui`, `@plugin "daisyui"` or daisyUI semantic classes such as `btn`, `card`, `input`, `select`, `badge`, `alert`, `table`, `menu`, `modal`, `tabs`, `mockup-code` or `loading`. Use local `ll-*` component classes or local primitives from `src/components/ui/`.

## i18n

- Default locale: `zh-CN`.
- Fallback locale: `en`.
- Users can switch language from the console top bar.
- Keep domain terms in English when they are technical labels: CC, CV, CP, USB-PD, PPS, PDO, APDO, Firmware, dry-run, lease, devd, UART.

## Demo mode

The production app includes a pure frontend demo mode on the normal console routes. It does not read real hardware, devd, saved real-device state, or backend services while demo mode is active.

Demo mode is remembered in `localStorage` under `loadlynx.demoMode`.
Use `?demo=true` to enter and remember demo mode, or `?demo=false` to exit and remember the normal console mode.
When the query parameter is absent, the app uses the remembered value.
The query parameter is only a switch; after it is applied, the app removes it from the URL and keeps the normal route.

Useful direct routes:

- `/devices?demo=true`
- `/mock-001/cc`
- `/mock-001/status`
- `/mock-001/pd`
- `/mock-001/settings`
- `/mock-001/firmware`
- `/mock-001/calibration`

## Lockfile policy

- Single source of truth: `web/bun.lock` (must be committed).
- Do not use or commit `web/package-lock.json` (npm lockfile). If it exists locally, delete it.
- After changing dependencies in `web/package.json`, run `bun install` and commit the updated `web/bun.lock`.
- CI uses `bun ci` and will fail if the lockfile is out of date.

## Usage

From the `web/` directory:

```bash
bun install
bun run dev
```

For local hardware-backed Web development, start the HTTP bridge separately and
point Vite at it:

```bash
just devd-bridge-http --bind 127.0.0.1:30180 --allow-dev-cors
VITE_LOADLYNX_DEVD_URL=http://127.0.0.1:30180 bun run dev
```

`loadlynx-devd serve` is the IPC daemon for CLI workflows. It uses a Unix
socket on macOS/Linux and a named pipe on Windows by default; Web/browser paths
use `loadlynx-devd bridge-http` or Web Serial. The HTTP bridge must stay on
loopback.

Core scripts:

- `bun run dev` – start the Vite development server.
- `bun run build` – type-check and build for production.
- `bun run check:bundle:app` – verify built app JS chunks stay within the app bundle budget.
- `bun run check:bundle:storybook` – verify Storybook preview chunks stay within budget and separately cap the framework mocker runtime.
- `bun run preview` – preview the built app.
- `bun run test:preview-smoke` – serve the built `dist` bundle with `vite preview` and fail on uncaught runtime errors or console errors during first paint.
- `bun run lint` – run `biome lint .`.
- `bun run format` – run `biome format --write .`.
- `bun run check` – run `biome check .`.
- `bun run test:unit` – run Vitest unit tests.
- `bun run test:e2e` – run Playwright E2E tests.
- `bun run storybook` – start the Storybook dev server.
- `bun run test:storybook` – run Storybook tests through the official Vitest addon.
- `bun run test:storybook:ci` – build Storybook, enforce Storybook bundle budgets, then run Storybook tests through the official Vitest addon.

## Bundle budgets

- App build budget: every emitted `dist/assets/*.js` chunk must stay at or below `250 kB`.
- Production preview smoke: after `bun run build`, `bun run test:preview-smoke` must prove the built `dist` bundle mounts without uncaught `pageerror` or `console error`.
- Storybook preview budget: every emitted `storybook-static/assets/*.js` chunk must stay at or below `250 kB`.
- Storybook framework runtime budget: `storybook-static/vite-inject-mocker-entry.js` is tracked separately with a `1200 kB` cap because it is injected by Storybook's Vitest mocker runtime rather than by LoadLynx application code.
- `bun run test:storybook:ci` enforces the Storybook budget automatically after `build-storybook`.
- Vite's generic chunk-size warning is not the source of truth for Storybook in this repository; the bundle budget scripts above are.

## PWA offline shell and updates

Production Web builds are real PWAs. `bun run build` writes `public/version.json` before Vite runs, then emits `dist/manifest.webmanifest`, `dist/sw.js` and `dist/workbox-*.js`.

PWA behavior:

- The app shell and built static assets are precached after the first successful visit.
- Offline reload can reopen the console shell even when the frontend server is unreachable.
- Device HTTP APIs, devd endpoints, firmware artifacts and Web Serial sessions are not runtime-cached.
- `/version.json` is generated before build for the current server artifact but is not precached by the service worker.
- App updates use prompt mode: a new service worker can cache the next build in the background, and the UI refreshes only after the user clicks `升级` / `Upgrade`.
- Storybook uses a no-op PWA registration mock. Storybook is not itself built as a PWA target.

Validation:

- `bun run test:preview-smoke` includes an offline reload scenario and verifies API and `/version.json` fetches are not returned from cache while offline.
- `bun run test:storybook:ci` covers the update-ready, offline-ready, registration-error and hidden prompt states.

## Ports

To avoid common default ports and accidental “port drift”, LoadLynx uses fixed high ports by default and
fails fast on conflicts (no automatic port fallback).

Default ports (override via env vars):

- Vite dev server: `LOADLYNX_WEB_DEV_PORT` (default: `25219`)
- Vite preview server: `LOADLYNX_WEB_PREVIEW_PORT` (default: `22848`)
- Storybook dev server: `LOADLYNX_STORYBOOK_PORT` (default: `32931`)
- Storybook test static server: `LOADLYNX_STORYBOOK_TEST_PORT` (default: `34033`)

Examples:

- `LOADLYNX_WEB_DEV_PORT=39999 bun run dev`
- `LOADLYNX_STORYBOOK_PORT=39998 bun run storybook`

## USB‑PD Settings

- Entry point: `Status` → `USB‑PD` card → `Open PD settings` (route: `/$deviceId/pd`).
- Required device endpoints (see `docs/interfaces/network-http-api.md`):
  - `GET /api/v1/pd` — read attach/contract/capabilities/saved config
  - `POST /api/v1/pd` — apply config; Web uses `POST` + `Content-Type: text/plain` with a JSON string body to avoid private-network preflight issues.

## Web Serial

- GitHub Pages and release Web bundles are formal human UI paths for browsers
  that expose `navigator.serial`.
- Web Serial firmware flashing requires a release firmware catalog JSON, the
  matching firmware file, SHA-256 verification, explicit `yes` confirmation,
  non-project firmware acknowledgement when applicable, and post-flash identity
  capture.
- Web Serial stores only device identity/profile metadata. It reconnects through
  browser-granted ports from `navigator.serial.getPorts()` and does not persist
  OS serial port paths.
- Browsers without Web Serial should use Chrome/Edge or the released
  CLI/devd host tools.

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
- `scripts/write-version.mjs` consumes `APP_EFFECTIVE_VERSION` during `bun run build` to write `public/version.json` before Vite copies it into `dist/version.json`.
- Official GitHub Releases bypass the package base version and inject the release workflow's computed version/tag directly through `APP_EFFECTIVE_VERSION`, `VITE_APP_VERSION`, and `VITE_APP_GIT_TAG`.

## UI version + GitHub source link

The main Console UI surfaces build metadata directly from build-time injected Vite env vars (compiled into the client bundle via `import.meta.env`), so it does not rely on runtime `fetch("/version.json")`.

- Display: `VITE_APP_VERSION`
- GitHub target (clickable):
  - Prefer stable tag (`VITE_APP_GIT_TAG` matching `v*`) → `https://github.com/<repo>/tree/<tag>`
  - Fallback to commit (`VITE_APP_GIT_SHA`) → `https://github.com/<repo>/commit/<sha>`
- Repo base: `VITE_GITHUB_REPO` (`Owner/Repo`, defaults to `IvanLi-CN/loadlynx` when unset)

CI injects these vars in `.github/workflows/web-pages.yml` and `.github/workflows/web-check.yml`. For local builds you can set them manually (optional):

```bash
VITE_APP_VERSION="$(../.github/scripts/compute-version.sh | cut -d= -f2)" \
VITE_APP_GIT_SHA="$(git rev-parse HEAD)" \
VITE_GITHUB_REPO="IvanLi-CN/loadlynx" \
bun run build
```

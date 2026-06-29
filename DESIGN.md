# LoadLynx Web Design System

## Visual Direction

The Web Console uses a product-grade cyberpunk neon direction: dark instrument surfaces, cyan signal lines, magenta risk accents, amber warnings and green success. The style should feel like a bench instrument HUD, not a game menu.

Scene sentence: a developer uses LoadLynx at a workbench in a dim lab, watching live electrical readbacks while preparing safe hardware writes.

## Color Strategy

Full palette with controlled use:

- Base: near-black tinted blue neutrals, never pure black or pure white.
- Primary: cyan for focus, active navigation, primary actions and live signals.
- Secondary: magenta for high-attention actions and cyberpunk identity moments.
- Warning: amber for recoverable risk and pending states.
- Success: green for healthy links and completed operations.
- Error: red/pink for destructive operations, faults and blocked safety states.

Colors must be expressed with OKLCH tokens in Tailwind/CSS where practical. Neon glow is a state signal, not background decoration everywhere.

## Typography

- Use system UI for interface text.
- Use tabular numbers and monospace accents for measurements, URLs, IDs, logs and protocol payloads.
- Keep fixed type sizes. Do not scale font size with viewport width.
- Use uppercase tracking only for compact labels and status chips.

## Components

The Web UI must not use daisyUI. Component vocabulary is local and shadcn-style:

- Button: square-ish `rounded-lg`, uppercase/mono for command buttons where appropriate, visible focus ring, disabled and loading states.
- Card/panel: `rounded-lg`, 1px cyan-tinted border, low-opacity neon glow, no nested decorative cards.
- Input/select/textarea: dark filled surface, cyan border on focus, visible invalid state, full-width on mobile.
- Dialog/sheet: Radix-backed primitives or equivalent accessible local wrappers; desktop device switching uses a right-side sheet, while mobile routing falls back to the Overview page instead of a navigation drawer.
- Table: responsive wrappers and stacked summaries where narrow screens need it.
- Badge/status chip: semantic color plus text label; never color-only.

## Layout

- App shell uses a sticky top header: brand at left, primary navigation in the center, language and current-device entry at right.
- The current-device entry in the header owns the primary device-availability signal. It uses a single status light: green for connected, pulsing amber for connecting/reconnecting, red for fault/error, and gray for no active connection.
- Primary navigation is always `总览 / 仪表盘 / 系统`; no left sidebar, icon rail or navigation drawer remains in the product shell.
- Device switching is a separate interaction surface from navigation:
  - desktop opens a right-side device sheet,
  - mobile routes to the Overview page and returns to the previous device page when possible.
- Main pages use responsive grids: one column on mobile, two or three only when labels and controls remain readable.
- The Overview page is a multi-device board; it shows device identity and five comparable live metrics (`voltage / current / power / resistance / mode`) as co-primary content, plus secondary link/protection summaries and entry actions without exposing dangerous write shortcuts.
- Overview cards are summary objects, not nested mini dashboards. They must not re-explain Dashboard/System/USB-PD responsibilities or mirror the current-device detail layout inside each card. The metric row is for horizontal comparison across devices, not for narrative copy.
- Overview cards must not expose raw transport URIs such as internal `baseUrl`, `mock://...`, devd bridge paths or other implementation-level endpoints in owner-facing UI.
- Owner-facing Overview and device-switching UI must not show environment markers such as `Mock`, `Demo transport`, raw protocol URIs or other development-only provenance tags.
- The Dashboard stays the high-density current-device work surface. It favors live readback and safety state first, then setpoints, USB-PD controls and write actions.
- The Dashboard status bar must not repeat current-device identity, IP, firmware or other shell-level identity metadata already shown in the header device entry.
- The System domain groups current-device management pages under a dedicated System page navigation. On desktop, that navigation lives as a left-side vertical rail within the System workspace; on mobile, it collapses back to a horizontal scroll row above the content.
- Calibration is the only System page that currently exposes a true second level. `Calibration` itself is a non-clickable group label; its `Voltage / Current channel 1 / Current channel 2` children live inside the System page navigation, use lighter text treatment instead of indentation, and replace any duplicate in-content mode tabs.
- Mobile views must keep primary actions reachable with 44px-class tap targets, and the primary nav should remain visible as a horizontal row within the header area.

## Responsive Contract

The Web Console follows a centered workspace model. Header shell, main content and footer all share the same horizontal padding rhythm and remain visually centered within an explicit page-width envelope rather than stretching edge-to-edge on large desktop screens.

Dashboard optimization targets are defined in **browser content viewport sizes**, not raw hardware panel resolutions. For the Dashboard, viewport height is as important as width because the product goal is to keep common monitoring and control interactions within one screen whenever practical.

### Breakpoints

- `0-767px`: mobile
  - Supported widths: `320-767px`
  - Primary nav remains inside the header as a horizontally scrollable row.
  - Pages collapse to a single content column.
  - Device switching routes to Overview selection mode instead of opening a sheet.
- `768-1023px`: tablet / small laptop
  - Supported widths: `768-1023px`
  - Header still wraps when needed, but keeps the same left/right padding.
  - Dense control grids reduce to fewer columns before text truncation becomes ambiguous.
- `1024-1439px`: standard desktop
  - Supported widths: `1024-1439px`
  - Desktop device sheet is enabled.
  - Overview and dashboard use the full workspace width budget while keeping equal outer gutters.
- `1440-1728px`: wide desktop
  - Supported widths: `1440-1728px`
  - Layout remains centered inside a capped workspace container; extra width becomes balanced side gutter, not asymmetric empty space.
  - Overview cards and management panels may widen, but they do not exceed the workspace cap.
- `1729px+`: ultra-wide fallback
  - Supported as a centered desktop presentation.
  - The shell and page containers stop growing at the documented max widths.
  - Extra viewport width is intentionally left as equal left/right breathing room so scan paths stay readable on 21:9 and larger monitors.

### Width Tokens

- Default reading pages use `--ll-page-max-default: 80rem` (`1280px`).
- Workspace-heavy pages such as `总览` use `--ll-page-max-workspace: 106.5rem` (`1704px`).
- Instrument surfaces such as `仪表盘` may use full-width page containers, but any inner frame that carries dense control UI must still declare its own max width and center itself.

### Compatibility Targets

- Mobile baseline: `375x800`
- Tablet baseline: `768x1024`
- Small laptop baseline: `900x800`
- Desktop baseline: `1200x800`
- Wide desktop baseline: `1440x900`
- Ultra-wide verification baseline: `1728x1117`

These baselines are mirrored in Storybook viewports so every shell-affecting change can be checked against the same screen classes.

### Dashboard Benchmark Viewports

These are the **primary optimization targets** for the Dashboard work surface:

- `1280x800`
  - Compact modern laptop baseline.
  - Goal: common read / set / mode / preset interactions fit without full-page vertical scroll.
- `1440x900`
  - Primary laptop baseline.
  - Goal: the preferred “one-screen dashboard” composition.
- `1536x864`
  - Mainstream modern Windows desktop / laptop baseline.
  - Goal: preserve the same left-monitor / right-control hierarchy without sparse dead space.

Larger desktop widths such as `1728x960` and `1920x1080` remain verification sizes, not separate optimization baselines. They must preserve the same hierarchy without introducing horizontal scrolling, stretched scan paths or excessive dead space.

### Dashboard Overflow Policy

- At and above `1280x800` viewport:
  - no horizontal scrollbars;
  - the Dashboard should avoid full-page vertical scrolling for common workflows;
  - if a rare advanced section must overflow, it should do so below the primary monitoring and control areas.
- From `1024x768` up to below `1280x800`:
  - no horizontal scrollbars;
  - vertical page scrolling is acceptable, but the primary readouts and core controls should remain near the top of the page.
- Below `1024px` width or below `768px` height:
  - task completion remains required;
  - vertical scrolling is expected;
  - the layout may stack more aggressively, but must not introduce horizontal scrolling.

## Motion

Use short 150-220 ms transitions for hover, focus, drawer reveal and state feedback. Respect reduced motion. Do not animate layout-heavy properties for live data.

## i18n

Default locale is `zh-CN`; English is the fallback and second supported language. Technical tokens may stay English inside Chinese copy when that is clearer for hardware users.

## Bans

- No `daisyUI` package, plugin, semantic class names or component dependency.
- No `@iconify/*` icon stack.
- No `bg-white`, pale admin surfaces, generic `shadow-*` card shadows, gradient text or glassmorphism as the default.
- No hidden safety states. Faults, destructive operations and write modes need explicit labels and accessible names.

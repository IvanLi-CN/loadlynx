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
- Dialog/sheet: Radix-backed primitives or equivalent accessible local wrappers; mobile navigation uses a sheet-style drawer.
- Table: responsive wrappers and stacked summaries where narrow screens need it.
- Badge/status chip: semantic color plus text label; never color-only.

## Layout

- App shell keeps a persistent desktop sidebar, a medium icon rail and a mobile drawer.
- Main pages use responsive grids: one column on mobile, two or three only when labels and controls remain readable.
- Control pages favor live readback and safety state first, then setpoints and write actions.
- Mobile views must keep primary actions reachable with 44px-class tap targets.

## Motion

Use short 150-220 ms transitions for hover, focus, drawer reveal and state feedback. Respect reduced motion. Do not animate layout-heavy properties for live data.

## i18n

Default locale is `zh-CN`; English is the fallback and second supported language. Technical tokens may stay English inside Chinese copy when that is clearer for hardware users.

## Bans

- No `daisyUI` package, plugin, semantic class names or component dependency.
- No `@iconify/*` icon stack.
- No `bg-white`, pale admin surfaces, generic `shadow-*` card shadows, gradient text or glassmorphism as the default.
- No hidden safety states. Faults, destructive operations and write modes need explicit labels and accessible names.

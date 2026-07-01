# Web PWA Offline Shell and Update Prompt History

## Initial PWA contract

The Web Console needs a cached app shell because bench operators may reopen it after the frontend static server or network path is unavailable. The contract deliberately limits offline support to the shell: real device state and write operations remain network-only to avoid unsafe stale hardware feedback.

The update flow uses a prompt instead of automatic refresh because control and calibration pages may be open during bench work. New assets can cache in the background, but the operator chooses when to refresh into them.

## Storybook and bundle guardrails

Storybook is not a product PWA target. Its Vite build aliases the PWA virtual registration module to a no-op mock and disables product PWA generation, so Storybook static output does not try to precache Storybook manager assets.

`workbox-window` is split into `pwa-vendor` rather than loosening bundle budgets. React remains on the normal vendor path to avoid reintroducing the production chunk-cycle class documented by `n5nwv`.

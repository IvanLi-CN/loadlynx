# LoadLynx MCU Agentd (legacy)

This in-repo crate (`tools/mcu-agentd/`) is deprecated.

LoadLynx now uses the external upgraded `mcu-agentd` repo (expected sibling checkout at `../mcu-agentd`), invoked via the root `Justfile` recipes and configured by `mcu-agentd.toml`.

- Run via `just agentd …` (or `just agentd-start/status/stop`)
- Configure targets in `mcu-agentd.toml`
- Selector cache files: `.esp32-port` and `.stm32-port`
- Runtime state: `.mcu-agentd/`

See the upstream docs:

- `../mcu-agentd/docs/usage/mcu-agentd.md`
- `../mcu-agentd/docs/design/config.md`

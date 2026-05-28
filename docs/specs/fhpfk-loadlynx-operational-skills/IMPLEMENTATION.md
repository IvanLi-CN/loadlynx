# LoadLynx operational skills packaging and workflow boundary implementation

## Current coverage

- `loadlynx-user-operations` now assumes an end-user machine and covers released host-tools installation, USB-first CLI access, HTTP fallback, GitHub Release firmware download, released CLI flash workflows, CLI hardware memory, and CLI WiFi capability checks without requiring a source checkout.
- `loadlynx-developer-operations` now requires project checkout detection before repo commands, allows cloning `https://github.com/IvanLi-CN/loadlynx.git` only when source work is needed, and keeps Just/source/HIL workflows behind developer context and hardware approval gates.
- Both skill folders include `SKILL.md` and `agents/openai.yaml`.
- `AGENTS.md` routes released CLI-only user hardware operations to the user skill and source/Just/devd/firmware/HIL work to the developer skill.
- Official and development release workflows build platform host-tools archives before creating a GitHub Release. Each archive includes `loadlynx-devd`, `loadlynx`, and a short package README.
- The user skill and project README now point normal USB bridge setup at released host tools instead of source builds.
- The user skill includes owner-facing host-tools installation steps: choose the platform Release asset, extract to a user-local bin directory, configure user PATH, verify both binaries, and start `loadlynx-devd serve` for USB operation.
- `loadlynx` implements user-level hardware memory through `loadlynx hardware available/recent/path/list/save/forget`, `loadlynx status --hardware <id>`, and best-effort automatic updates after successful `status --device` or `status --url`.
- CLI hardware memory is stored in the user's OS config directory, not in the repository checkout: macOS `~/Library/Application Support/LoadLynx/devices.json`, Linux `${XDG_CONFIG_HOME:-~/.config}/loadlynx/devices.json`, Windows `%APPDATA%\LoadLynx\devices.json`; `LOADLYNX_HOME` overrides the directory for tests or advanced setups.
- The user skill explicitly treats missing `loadlynx wifi ...` support as a stop condition because the current CLI command surface does not implement that complete user capability.

## Verification

- `quick_validate.py` passes for both skill directories.
- `npx skills add . --list` discovers both skills.
- Temporary-directory `npx skills add <repo-url> --skill ...` installs both skills and copies `SKILL.md` plus `agents/openai.yaml`; the published command uses `https://github.com/IvanLi-CN/loadlynx`.
- Release workflow validation includes local host-tools release build and YAML/diff checks.
- `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml --locked` covers CLI hardware memory parsing, path selection, registry round-trip behavior, and generated IDs.

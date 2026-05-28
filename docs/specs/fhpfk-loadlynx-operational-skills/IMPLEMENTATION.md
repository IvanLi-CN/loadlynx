# LoadLynx operational skills packaging and workflow boundary implementation

## Current coverage

- `loadlynx-user-operations` now assumes an end-user machine and covers released host-tools installation, USB-first CLI access, HTTP fallback, GitHub Release firmware download, released CLI flash workflows, and CLI WiFi/hardware-memory capability checks without requiring a source checkout.
- `loadlynx-developer-operations` now requires project checkout detection before repo commands, allows cloning `https://github.com/IvanLi-CN/loadlynx.git` only when source work is needed, and keeps Just/source/HIL workflows behind developer context and hardware approval gates.
- Both skill folders include `SKILL.md` and `agents/openai.yaml`.
- `AGENTS.md` routes released CLI-only user hardware operations to the user skill and source/Just/devd/firmware/HIL work to the developer skill.
- Official and development release workflows build platform host-tools archives before creating a GitHub Release. Each archive includes `loadlynx-devd`, `loadlynx`, and a short package README.
- The user skill and project README now point normal USB bridge setup at released host tools instead of source builds.
- The user skill includes owner-facing host-tools installation steps: choose the platform Release asset, extract to a user-local bin directory, configure user PATH, verify both binaries, and start `loadlynx-devd serve` for USB operation.
- The user skill explicitly treats missing `loadlynx wifi ...` support and missing CLI saved-hardware support as stop conditions because the current CLI command surface does not implement those complete user capabilities.

## Verification

- `quick_validate.py` passes for both skill directories.
- `npx skills add . --list` discovers both skills.
- Temporary-directory `npx skills add <repo-url> --skill ...` installs both skills and copies `SKILL.md` plus `agents/openai.yaml`; the published command uses `https://github.com/IvanLi-CN/loadlynx`.
- Release workflow validation includes local host-tools release build and YAML/diff checks.

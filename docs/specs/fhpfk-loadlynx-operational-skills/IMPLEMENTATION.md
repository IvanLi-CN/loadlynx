# LoadLynx operational skills packaging and workflow boundary implementation

## Current coverage

- `loadlynx-user-operations` now assumes an end-user machine and covers released host-tools installation, USB-first CLI access, HTTP fallback, GitHub Release firmware download, released CLI flash workflows, CLI hardware memory, and CLI WiFi capability checks without requiring a source checkout.
- `loadlynx-developer-operations` now requires project checkout detection before repo commands, allows cloning `https://github.com/IvanLi-CN/loadlynx.git` only when source work is needed, and keeps Just/source/HIL workflows behind developer context and hardware approval gates.
- Both skill folders include `SKILL.md` and `agents/openai.yaml`.
- `AGENTS.md` routes released CLI-only user hardware operations to the user skill and source/Just/devd/firmware/HIL work to the developer skill.
- Official release workflows build platform host-tools archives before creating a GitHub Release. Each archive includes `loadlynx-devd`, `loadlynx`, and a short package README. The release also publishes installer scripts, firmware catalog/assets, Web bundle, and `SHA256SUMS` covering every release asset.
- The user skill and project README now point normal USB bridge setup at released host tools installed by `install-loadlynx-host.sh` / `install-loadlynx-host.ps1` or manually verified against `SHA256SUMS`, instead of source builds.
- The released CLI/devd boundary is native IPC-first. `loadlynx` exposes `--ipc` as a Unix socket / Windows named pipe endpoint override and can auto-start sibling `loadlynx-devd serve`; `loadlynx-devd bridge-http` is reserved for browser/Web/debug bridge usage and must stay loopback-only.
- The user skill treats pre-IPC HTTP-devd host tools as obsolete for skill-driven hardware operation. If `loadlynx --help` exposes `--devd`, or `loadlynx-devd --help` lacks `bridge-http`, the agent must stop and require an IPC-capable stable host-tools release instead of using a compatibility path.
- The v0.3.0 GitHub Release has been backfilled with `SHA256SUMS` for installation recovery only; its host tools remain obsolete for the user skill because they expose the old `--devd` CLI surface.
- The user skill includes owner-facing host-tools installation steps: choose the platform Release asset, extract to a user-local bin directory, configure user PATH, verify both binaries, and start `loadlynx-devd serve` for USB operation.
- The Windows host-tools installer honors its `-Force` option before replacing an existing `loadlynx.exe`. Without `-Force`, an already-installed matching requested version exits cleanly and other existing installations are left untouched with an explicit replacement message.
- `loadlynx` implements user-level hardware memory through `loadlynx hardware available/recent/path/list/save/forget`, `loadlynx status --hardware <id>`, and best-effort automatic updates after successful `status --device` or `status --url`.
- CLI hardware memory is stored in the user's OS config directory, not in the repository checkout: macOS `~/Library/Application Support/LoadLynx/devices.json`, Linux `${XDG_CONFIG_HOME:-~/.config}/loadlynx/devices.json`, Windows `%APPDATA%\LoadLynx\devices.json`; `LOADLYNX_HOME` overrides the directory for tests or advanced setups.
- The user skill explicitly treats missing `loadlynx wifi ...` support as a stop condition because the current CLI command surface does not implement that complete user capability.
- The user and developer skills require first-flash/non-project firmware gates for real ESP32-S3 flash: artifact/hash/target evidence, explicit owner confirmation, explicit non-project acknowledgement when applicable, and post-flash identity capture. Owner confirmation can be natural language and must not require a fixed typed phrase.
- Web Serial is documented as the formal human browser path for GitHub Pages and release Web bundles. It uses release catalog/assets and browser-granted ports, stores identity/profile memory only, and does not save OS port paths.

## Verification

- `quick_validate.py` passes for both skill directories.
- `npx skills add . --list` discovers both skills.
- Temporary-directory `npx skills add <repo-url> --skill ...` installs both skills and copies `SKILL.md` plus `agents/openai.yaml`; the published command uses `https://github.com/IvanLi-CN/loadlynx`.
- Release workflow validation includes local host-tools release build and YAML/diff checks.
- `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml --locked` covers CLI hardware memory parsing, path selection, registry round-trip behavior, generated IDs, IPC/CLI behavior, lease handling and flash gates.
- Review convergence validation passed with `cargo fmt --manifest-path tools/loadlynx-devd/Cargo.toml --all`, `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml`, `git diff --check`, and `tools/loadlynx-devd/install/install-loadlynx-host.sh --dry-run`. PowerShell parser validation was not available in the local environment.

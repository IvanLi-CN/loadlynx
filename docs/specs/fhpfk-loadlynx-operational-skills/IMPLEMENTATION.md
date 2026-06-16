# LoadLynx operational skills packaging and workflow boundary implementation

## Current coverage

- `loadlynx-user-operations` now assumes an end-user machine and covers released host-tools installation, saved-device-first CLI access, USB/devd preference, HTTP fallback, GitHub Release firmware download, released CLI flash workflows, CLI device memory, released WiFi commands, and external USB-C source validation without requiring a source checkout.
- `loadlynx-developer-operations` now requires project checkout detection before repo commands, allows cloning `https://github.com/IvanLi-CN/loadlynx.git` only when source work is needed, and keeps Just/source/HIL workflows behind developer context and hardware approval gates.
- Both skill folders include `SKILL.md` and `agents/openai.yaml`.
- `AGENTS.md` routes released CLI-only user hardware operations to the user skill and source/Just/devd/firmware/HIL work to the developer skill.
- Official release workflows build platform host-tools archives before creating a GitHub Release. Each archive includes `loadlynx-devd`, `loadlynx`, and a short package README. The release also publishes installer scripts, firmware catalog/assets, Web bundle, and `SHA256SUMS` covering every release asset.
- The user skill and project README now point normal USB bridge setup at released host tools installed by `install-loadlynx-host.sh` / `install-loadlynx-host.ps1` or manually verified against `SHA256SUMS`, instead of source builds.
- The released CLI/devd boundary is native IPC-first. `loadlynx` can auto-start sibling `loadlynx-devd serve` on the default Unix socket / Windows named pipe, while `--ipc` remains an optional endpoint override rather than a normal user argument; `loadlynx-devd bridge-http` is reserved for browser/Web/debug bridge usage and must stay loopback-only.
- The user skill treats pre-IPC HTTP-devd host tools as obsolete for skill-driven hardware operation. If `loadlynx --help` exposes `--devd`, or `loadlynx-devd --help` lacks `bridge-http`, the agent must stop and require an IPC-capable stable host-tools release instead of using a compatibility path.
- The v0.3.0 GitHub Release has been backfilled with `SHA256SUMS` for installation recovery only; its host tools remain obsolete for the user skill because they expose the old `--devd` CLI surface.
- The user skill includes owner-facing host-tools installation steps: choose the platform Release asset, extract to a user-local bin directory, configure user PATH, verify both binaries, and start `loadlynx-devd serve` for USB operation.
- The Windows host-tools installer honors its `-Force` option before replacing an existing `loadlynx.exe`. Without `-Force`, an already-installed matching requested version exits cleanly and other existing installations are left untouched with an explicit replacement message.
- `loadlynx` implements user-level device memory through registry schema 2: `loadlynx devices`, `loadlynx device list|add|use|remove`, `loadlynx status`, and `loadlynx status --device <id>`. Registry keys are stable firmware `identity.device_id` values, each device record may contain both USB and HTTP transport locators, and `last_transport` records which transport to use by default.
- Temporary USB/devd candidate IDs are not operation targets. They are valid only inside the interactive `loadlynx device add` USB bind flow; bind reads USB identity and rejects devices that do not expose a stable `loadlynx-<short-id>` device ID. `status --device` and other direct candidate operations return bind-first errors.
- `loadlynx status` uses the selection chain `--device > nearest ancestor .loadlynx > global default > interactive saved-device selection`, and JSON automation receives a structured no-selection error if no remembered device exists. Saved USB operations can recover from a fresh devd device table by scanning, leasing, and confirming `identity.device_id` before issuing the operation.
- CLI device memory is stored in the user's OS config directory, not in the repository checkout: macOS `~/Library/Application Support/LoadLynx/devices.json`, Linux `${XDG_CONFIG_HOME:-~/.config}/loadlynx/devices.json`, Windows `%APPDATA%\LoadLynx\devices.json`; `LOADLYNX_HOME` overrides the directory for tests or advanced setups.
- The user skill documents the current released `loadlynx wifi show|set|clear` commands while preserving the installed-help gate: if a user's installed binary lacks those commands, the agent must stop instead of inventing commands or falling back to raw HTTP.
- The user skill now includes a generic external USB-C source validation path: use a saved LoadLynx target, prefer USB/devd transport for writes, use `loadlynx pd set` as the PD sink stimulus, use `loadlynx cv <target_v_mv>` as voltage-clamp load stimulus, and treat the external DUT diagnostics as the primary current-limit verdict while using LoadLynx telemetry as auxiliary evidence.
- The user and developer skills require first-flash/non-project firmware gates for real ESP32-S3 flash: artifact/hash/target evidence, explicit owner confirmation, explicit non-project acknowledgement when applicable, and post-flash identity capture. Owner confirmation can be natural language and must not require a fixed typed phrase.
- Web Serial is documented as the formal human browser path for GitHub Pages and release Web bundles. It uses release catalog/assets and browser-granted ports, stores identity/profile memory only, and does not save OS port paths.

## Verification

- `quick_validate.py` passes for both skill directories.
- `npx skills add . --list` discovers both skills.
- Temporary-directory `npx skills add <repo-url> --skill ...` installs both skills and copies `SKILL.md` plus `agents/openai.yaml`; the published command uses `https://github.com/IvanLi-CN/loadlynx`.
- Release workflow validation includes local host-tools release build and YAML/diff checks.
- `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml --locked` covers CLI hardware registry parsing, schema migration, default hardware, transport selection, bind-probe lease restriction, saved USB identity confirmation, fresh devd scan-before-lease retry, path selection, IPC/CLI behavior, lease handling and flash gates.
- `npm run test:workflow-hygiene` covers documentation drift guards for the released CLI/user skill surface: WiFi, `pd set`, `cv`, `loadlynx-devd serve`, `bridge-http`, stale ordinary daemon URL wording, and banned project-specific external DUT names.
- Review convergence validation passed with `cargo fmt --manifest-path tools/loadlynx-devd/Cargo.toml --all`, `cargo test --manifest-path tools/loadlynx-devd/Cargo.toml`, `git diff --check`, and `tools/loadlynx-devd/install/install-loadlynx-host.sh --dry-run`. PowerShell parser validation was not available in the local environment.

## Specification Companion Notes

`SPEC.md` owns the long-lived topic contract. Implementation progress, rollout records, documentation maintenance notes, and prior catalog state live in this companion document.

### Catalog Context
- Prior catalog status: 已更新
- Prior catalog timestamp: 2026-05-28
- Prior catalog implementation note: PR #78；用户/开发者 skill 场景拆分、CLI-only 硬件操作、USB 优先、硬件记忆门禁、vercel-labs/skills 安装验证

### 状态

- Status: 已更新；用户侧硬件操作仅允许 released CLI。CLI 硬件记忆已实现为用户级配置，当前 stable release 公开 `loadlynx wifi show|set|clear`，skill 仍要求先验证 installed help。

### 文档更新

- 更新 `AGENTS.md` 中的 skill 路由。
- 更新 `README.md` 的 released host-tools、用户/开发路径与 CLI 能力边界。
- 新增本规格，并在 `docs/specs/README.md` 登记。

### 实现里程碑

- [x] M1: 保持两个 skill，改为用户版 / 开发者版。
- [x] M2: 用户版写入 released host-tools 安装、saved device / USB 优先 / HTTP fallback、GitHub 固件下载、CLI 烧录、CLI WiFi 与 CLI 硬件记忆流程。
- [x] M3: 开发者版写入 checkout 检测、必要时 clone、`just` 本地 devd/CLI/固件工作流。
- [x] M4: 补齐 `agents/openai.yaml` 与 `vercel-labs/skills` 安装验证。
- [x] M5: 用户 CLI WiFi 配置已通过 `loadlynx wifi show|set|clear`、devd/firmware协议与持久化发布；skill 仍按 installed help 做能力门禁。
- [x] M6: 实现 CLI 用户级硬件记忆：保存、列出可连接设备、列出最近连接设备、列出已记住设备、选择、更新、遗忘 USB 与 HTTP 设备，并保存到用户配置目录。
- [x] M7: 增加通用外部 USB-C source validation 操作路径，使用 `pd set` + `cv` 刺激并把外部 DUT diagnostics 作为主判定。

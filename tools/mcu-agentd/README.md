# LoadLynx MCU Agentd

`loadlynx-agentd` is the single-instance daemon and CLI used by LoadLynx to manage both MCUs:

- **digital** — ESP32‑S3 host firmware (`firmware/digital/`)
- **analog** — STM32G431 analog firmware (`firmware/analog/`)

It provides:

- Port/probe caching for each MCU
- Flash and reset helpers
- Background monitoring of defmt/serial logs
- Structured meta/session logs for later inspection

The crate lives in `tools/mcu-agentd/` and is usually driven through the project‑level `just` recipes.

- Development: `cd tools/mcu-agentd && cargo run -- <subcommand>`
- Recommended: `just agentd …` (passthrough wrapper that runs `cargo run --release -- …`)

---

## 1. MCU Mapping & Port Cache

**MCU names**

- `digital` → ESP32‑S3 board (host / display / bridge)
- `analog` → STM32G431 board (power / control loop)

**Cache files (repository root)**

- Digital (ESP32‑S3): `.esp32-port`
  - Example content: `/dev/cu.usbserial-xxxx`
- Analog (STM32G431): `.stm32-port`
  - Example content: `0483:3748:SERIAL`

The cached values are used by all flash/reset/monitor operations. If the cache is missing, the daemon falls back to the existing helper scripts:

- Digital: `scripts/ensure_esp32_port.sh`
- Analog: `scripts/ensure_stm32_probe.sh`

Those scripts reuse the same selection rules as the older workflow and may also migrate a legacy `.stm32-probe` file into `.stm32-port` (read once, write `.stm32-port`, then delete `.stm32-probe`).

**Managing the cache (recommended)**

Set cached port/probe:

- `just agentd set-port digital /dev/cu.usbserial-xxxx`
- `just agentd set-port analog 0483:3748:SERIAL`

Read cached value:

- `just agentd-get-port digital`
- `just agentd-get-port analog`

Internally these wrappers call:

- `cargo run --release -- set-port <mcu> [PATH]`
- `cargo run --release -- get-port <mcu>`

---

## 2. Daemon Lifecycle

The daemon is single-instance. A Unix socket and lock file under `logs/agentd/` ensure that only one `loadlynx-agentd` process is active.

**Direct CLI**

```bash
cd tools/mcu-agentd
cargo run --release -- start    # start daemon (no-op if already running)
cargo run --release -- status   # show PID and socket path
cargo run --release -- stop     # stop daemon and clean stale lock/socket
```

**Just wrappers (preferred)**

- `just agentd-start` → `loadlynx-agentd start`
- `just agentd-status` → `loadlynx-agentd status`
- `just agentd-stop` → `loadlynx-agentd stop`

`status` returns a JSON payload on success. If the socket is missing or unreachable, the CLI prints a more descriptive error and suggests restarting via `just agentd-start`.

Any user-facing subcommand (`set-port`, `flash`, `reset`, `monitor`, `logs`) will auto-start the daemon on first use if it is not already running.

---

## 3. Core Subcommands

All examples assume you are in the repository root and use the `just agentd …` passthrough. You can always replace `just agentd` with `cd tools/mcu-agentd && cargo run --release --`.

### 3.1 Ports / Probes

**SetPort**

```bash
just agentd set-port <mcu> [PATH]
```

- `mcu` ∈ `{digital, analog}`
- With `PATH`: write the cache file directly.
- Without `PATH`: start an interactive picker:
  - `digital`: uses `serialport` + `espflash list-ports`, filters to likely Espressif `/dev/cu.*` ports.
  - `analog`: uses `probe-rs list` and shows a filtered list (ST‑LINK / CMSIS‑DAP / common STM32 probes).

On success the daemon hot‑restarts the background monitor for that MCU so the new port/probe is used immediately.

**GetPort**

```bash
just agentd get-port <mcu>
```

Prints the current cached selector (or `null` if unset).

**ListPorts**

```bash
just agentd list-ports <mcu>
```

- `digital`: returns a list of serial device paths derived from `espflash list-ports` (e.g. `/dev/cu.usbserial-xxxx`).
- `analog`: returns `probe-rs list` selectors such as `0483:3748:SERIAL`.

### 3.2 Flash

```bash
just agentd flash <mcu> [ELF] [--after {no-reset,hard-reset}]
```

- `mcu` ∈ `{digital, analog}` (positional)
- `ELF` (optional positional):
  - If provided, that path is used.
  - If omitted, the daemon expects the default release ELF to exist:
    - Digital: `firmware/digital/target/xtensa-esp32s3-none-elf/release/digital`
    - Analog: `firmware/analog/target/thumbv7em-none-eabihf/release/analog`
  - If the default ELF is missing, the command fails with `default ELF missing; provide --elf`. Build with `make d-build` / `make a-build` (or the equivalent `just` recipes) or pass an explicit `ELF`.
- `--after` (digital only, optional):
  - `no-reset` (default): leave the ESP32‑S3 running without an extra hard reset.
  - `hard-reset`: request a hard reset after flashing is complete.
  - Analog ignores this option.

Under the hood:

- Digital: `espflash flash <elf> --chip esp32s3 --port <cached> --after {no-reset|hard-reset} --ignore_app_descriptor --non-interactive --skip-update-check`
- Analog: `probe-rs download --chip STM32G431CB --probe <cached> <elf>` (with a short retry loop for transient “interfaces are claimed” errors)

On success, the daemon writes a `flash` entry into the corresponding meta log and restarts monitoring for that MCU if a cache is set.

### 3.3 Reset

```bash
just agentd reset <mcu>
```

- `mcu` ∈ `{digital, analog}`
- Digital: `espflash reset --chip esp32s3 --port <cached>`
- Analog: `probe-rs reset --chip STM32G431CB --probe <cached>` with a small retry loop on USB/probe busy errors.

Analog reset treats some “interfaces are claimed” failures as a soft success: it logs a warning, records a `reset` event, and then allows the monitor to restart so the MCU still ends up running.

Reset always writes a `reset` meta event. If the underlying command ultimately fails, the CLI returns an error and points to the session log path.

### 3.4 Monitor

```bash
just agentd monitor <mcu> [ELF] [--reset] [--duration DUR] [--lines N]
```

- `mcu` ∈ `{digital, analog}`
- `ELF` (optional positional): reserved for future use; the daemon currently tails the latest log file for the selected MCU and does not require an explicit ELF path for normal workflows.
- `--reset`:
  - Perform a `reset` first.
  - Wait (up to 5s) for a new session or monitor log file to appear.
  - Start streaming from the **beginning** of the new file.
- Without `--reset`:
  - Attach to the latest existing session/monitor log file.
  - Seek to the **end** and only stream new lines.
- `--duration DUR` (default `0`): stop after the given wall‑clock duration (`30s`, `2m`, `1h`, …). `0` means “no time limit”.
- `--lines N` (default `0`): stop after printing `N` lines. `0` means “no line limit”.

The CLI prints plain text log lines while running. When it exits due to duration/line limits or EOF, it stops without killing the daemon.

### 3.5 Logs

```bash
just agentd logs <mcu|all> [--since RFC3339] [--until RFC3339] [--tail N] [--sessions]
```

- `mcu` ∈ `{digital, analog, all}`
- `--since` / `--until`:
  - Inclusive RFC3339 timestamps used to filter log entries (e.g. `2025-11-23T14:05:00-08:00`).
- `--tail N`:
  - Limit the number of meta entries after filtering (default is `200` from the daemon config).
  - `--tail 0` returns no meta entries.
- `--sessions`:
  - If `false` (default): only return meta entries.
  - If `true`: also attach the tail of each corresponding session file.

The command returns structured JSON:

- `payload.meta` — array of meta entries (see section 4)
- `payload.sessions` — array of `{ "session": "<path>", "lines": [ "...", ... ] }` objects when `--sessions` is enabled

Example:

- Recent digital activity with session tails:

```bash
just agentd logs digital --tail 50 --sessions
```

---

## 4. Logs & On‑Disk Layout

All paths below are relative to the repository root.

**Daemon state**

- `logs/agentd/agentd.sock` — Unix socket used by the CLI
- `logs/agentd/agentd.lock` — lock file enforcing a single daemon instance
- `logs/agentd/agentd.log` — daemon stdout/stderr (spawned via `spawn_background`)

**Meta logs**

- `logs/agentd/digital.meta.log`
- `logs/agentd/analog.meta.log`

Each line is JSON (NDJSON) with at least:

- `ts` — wall‑clock timestamp in RFC3339
- `mono_ms` — monotonic milliseconds since daemon start
- `mcu` — `"digital"` or `"analog"`
- `event` — high‑level event name (`"flash"`, `"reset"`, `"monitor-start"`, `"monitor-stop"`, …)
- `status` — exit code from the underlying command (0 on success)
- `duration_ms` — duration in milliseconds
- `session` — path to the associated session or monitor log file

**Session & monitor logs**

- Session logs (per action):
  - `logs/agentd/digital/YYYYMMDD_HHMMSS.session.log`
  - `logs/agentd/analog/YYYYMMDD_HHMMSS.session.log`
- Long‑running monitor logs:
  - `logs/agentd/digital/monitor/YYYYMMDD_HHMMSS.mon.log`
  - `logs/agentd/analog/monitor/YYYYMMDD_HHMMSS.mon.log`

Each line in these files is JSON:

```json
{"ts":"2025-11-23T14:05:31.842-08:00","mcu":"digital","src":"stdout","text":"..."}
```

- `src` — `"stdout"` or `"stderr"` from the underlying tool (`espflash`, `probe-rs`, etc.)
- `text` — one decoded line of tool or MCU output

The `monitor` CLI subcommand reads and prints the `text` field, so you usually do not need to parse the JSON manually unless you are building tooling.

---

## 5. Relationship to Scripts & Justfile

**Recommended entry points**

- For day‑to‑day work, use the top‑level `just` recipes:
  - `just agentd-start` / `just agentd-status` / `just agentd-stop`
  - `just agentd set-port …` / `just agentd-get-port …`
  - `just agentd flash …` / `just agentd reset …` / `just agentd monitor …` / `just agentd logs …`
- Only drop into `cd tools/mcu-agentd && cargo run -- …` when:
  - You are developing or debugging the agentd itself, or
  - You want to run a one‑off subcommand without installing `just`.

**Legacy scripts**

- `scripts/agent_verify_analog.sh`
- `scripts/agent_verify_digital.sh`
- `scripts/agent_dual_monitor.sh`

These scripts now delegate most of their work to `loadlynx-agentd` and are kept as thin wrappers for special cases or historical workflows. For new work, prefer the agentd CLI and `just` recipes.

---

## 6. Typical Workflows

Below are some copy‑paste friendly sequences for common tasks.

**First‑time setup**

```bash
# Build both firmwares (release profile by default)
make a-build
make d-build

# Start the daemon
just agentd-start

# Select and cache ports/probes (one‑time per machine)
just agentd set-port digital /dev/cu.usbserial-xxxx
just agentd set-port analog 0483:3748:SERIAL
```

**Flash and monitor the digital board**

```bash
just agentd flash digital --after hard-reset
just agentd monitor digital --duration 30s --lines 200
```

**Inspect recent activity across both MCUs**

```bash
just agentd logs all --tail 200 --sessions
```

From here you can follow the paths printed in the JSON to open specific session or monitor log files under `logs/agentd/` if you need deep debugging.


# LoadLynx CLI/devd hardware operations

LoadLynx hardware operations are owned by the released `loadlynx` CLI and the local `loadlynx-devd` daemon. This includes device discovery, saved device memory, firmware flashing, reset, monitor, bounded logs, firmware artifact verification, USB CDC ownership, and post-operation identity/status evidence.

## Responsibilities

- `loadlynx` is the human and automation command surface.
- `loadlynx-devd serve` owns local IPC for CLI workflows.
- `loadlynx-devd bridge-http` owns loopback browser/debug access only.
- `loadlynx-devd` owns USB CDC sessions, leases, flash/reset exclusivity, monitor sessions, and bounded logs.
- Firmware artifacts are selected from the firmware catalog and verified by hash before real flash.
- Device memory is stored through the CLI registry, keyed by stable `identity.device_id`.
- Local directory selection uses the nearest ancestor `.loadlynx` file containing one saved device id.

External MCU daemons are not a LoadLynx hardware-operation path. If a flash, reset, monitor, or log workflow is missing from `loadlynx`/`loadlynx-devd`, implement it there.

## Development Entry Points

```sh
just devd-build
just devd-test
just loadlynx devices
just loadlynx device add
just loadlynx device use <saved-id>
just loadlynx status --device <saved-id>
just loadlynx flash digital --device <saved-id> --artifact <artifact-id>
just loadlynx monitor digital --device <saved-id>
```

`loadlynx` auto-starts sibling `loadlynx-devd serve` on the default IPC endpoint when a command needs devd. Use `--ipc` and a matching `loadlynx-devd serve --endpoint ...` only for deliberate multi-instance or debugging overrides.

## Target Selection

- Do not guess or silently switch hardware targets.
- Set the approved ESP32-S3 digital USB CDC port only after owner authorization:

```sh
just loadlynx usb-port set digital <path>
```

- Agents must not use interactive port selection to bypass explicit owner authorization.
- Agents must not edit `.esp32-port`, `.stm32-port`, device registry files, or local `.loadlynx` files unless the owner explicitly asks for that mutation.
- Before flash/reset/digital monitor, echo the saved device id, selected transport, physical port/probe evidence when available, artifact id, and dry-run/real mode.

## Firmware Flows

- Digital ESP32-S3 real flash uses devd's direct `espflash` backend against the approved `.esp32-port` target.
- ELF artifacts use `espflash flash`; raw image artifacts require `flash_address` and use `espflash write-bin`.
- Analog STM32G431 flash/reset must be exposed as `loadlynx` CLI + `loadlynx-devd` operations. Analog RTT/defmt monitor is a separate devd backend gap; until implemented, `loadlynx monitor analog` must reject explicitly instead of routing through the digital USB session or any external MCU daemon.
- Dry-run validates target resolution, artifact presence, and hashes without touching hardware.
- Real flash requires artifact/hash/target evidence, explicit confirmation, and post-flash identity/status capture.

## Logs And Monitor

`loadlynx-devd` should provide bounded monitor/session logs through CLI-visible operations. A successful validation should include one or more of:

- decoded USB CDC `hello`
- successful `get_identity`
- successful `get_status`
- monitor log lines containing the expected firmware version
- post-flash identity matching the selected artifact

When validating freshly built firmware, compare boot/version evidence with:

```sh
tmp/analog-fw-version.txt
tmp/digital-fw-version.txt
```

## Safety Rules

- No hardware-changing operation may run against a temporary scan candidate.
- Saved device identity must match the active hardware before writes.
- USB CDC writes must hold a valid devd lease.
- Flash/reset must reserve the port exclusively.
- Missing CLI/devd capability is a product gap, not a reason to use another hardware daemon.

# LoadLynx devd control plane history

## Initial Design

The specification was created after reviewing `mains-aegis` devd, Web management UI specs and LoadLynx's existing mDNS/LAN discovery plan. The main design decision is to reuse the local daemon + explicit lease model while changing the domain model from a single ESP32 UPS to a dual-target LoadLynx device with separate ESP32-S3 and STM32G431 flash/control paths.

## First Implementation

The first implementation keeps the daemon and CLI in one Rust crate with two binaries. Web support starts from the existing Devices route and adds a Firmware route instead of introducing a separate fleet app shell. Firmware changes are limited to stable identity/DNS-SD contract fields and build metadata; the analog board remains represented by artifact/probe provenance until a direct identity read path is designed.

## Serial Owner Concurrency

The USB CDC bridge moved from per-request serial opens to lease-scoped per-port ownership. This keeps devd as the only process inside the control plane that touches the serial port, lets multiple CLI/Web clients share the same device through internal leases, and separates authorization from physical access ordering. Flash/reset remain exclusive because vendor tools need the OS port directly, so devd closes the serial owner before invoking them and rejects same-port JSONL work while the exclusive operation is active.

Review convergence kept two safety-compatible legacy behaviors intact: PD GET falls back to the cached PD view only for missing or mismatched serial responses, and real USB lease creation refuses to touch hardware unless the selected port matches the approved `.esp32-port` default.

Real CLI HIL verification exposed that owner-facing commands must cover the full safe-control lifecycle, not just enable. The CLI now includes PD writes, CC current targets and explicit output disable, while devd handles USB Serial/JTAG log-noise fragmentation without treating stale or mismatched request IDs as success.

## Control Plane Completion

The bridge contract was expanded from identity/status/PD/output operations to the full owner-facing control surface: control, presets, calibration, WiFi status/config requests, soft reset and diagnostics. The accepted model keeps USB as a compact dedicated protocol while devd maps Web/CLI/LAN-compatible HTTP calls to firmware ops. Firmware remains the final authority for validation and safety, and devd only prevalidates selection, lease ownership, request matching and sensitive-field redaction.

Calibration profile reads use a compact firmware response instead of the verbose HTTP shape on USB. This keeps full 24-point curve profiles inside the single JSONL frame budget while preserving the public HTTP/Web shape through devd expansion.

Lease semantics were clarified: multiple clients may hold leases for the same device/port, ordinary JSONL writes are queued through the per-port owner, and flash/reset are exclusive windows that fail same-port JSONL work fast instead of letting it wait behind long vendor-tool operations.

Real completion HIL showed that ESP32-S3 USB Serial/JTAG log noise can corrupt not only status/output-control responses but also identity, control, presets, compact calibration profile and WiFi status responses. The implementation now keeps a larger non-protocol buffer and performs operation-scoped recovery from post-transmit frames or text fragments. Diagnostics already returned a complete matched response; preset list recovery can still be partial when the first array element is lost to noise, so the CLI evidence pairs it with control/status for the active preset.

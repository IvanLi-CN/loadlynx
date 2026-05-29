# LoadLynx devd control plane history

## Initial Design

The specification was created after reviewing `mains-aegis` devd, Web management UI specs and LoadLynx's existing mDNS/LAN discovery plan. The main design decision is to reuse the local daemon + explicit lease model while changing the domain model from a single ESP32 UPS to a dual-target LoadLynx device with separate ESP32-S3 and STM32G431 flash/control paths.

## First Implementation

The first implementation keeps the daemon and CLI in one Rust crate with two binaries. Web support starts from the existing Devices route and adds a Firmware route instead of introducing a separate fleet app shell. Firmware changes are limited to stable identity/DNS-SD contract fields and build metadata; the analog board remains represented by artifact/probe provenance until a direct identity read path is designed.

## Serial Owner Concurrency

The USB CDC bridge moved from per-request serial opens to lease-scoped per-port ownership. This keeps devd as the only process inside the control plane that touches the serial port, lets multiple CLI/Web clients share the same device through internal leases, and separates authorization from physical access ordering. Flash/reset remain exclusive because vendor tools need the OS port directly, so devd closes the serial owner before invoking them and rejects same-port JSONL work while the exclusive operation is active.

Review convergence kept two safety-compatible legacy behaviors intact: PD GET falls back to the cached PD view only for missing or mismatched serial responses, and real USB lease creation refuses to touch hardware unless the selected port matches the approved `.esp32-port` default.

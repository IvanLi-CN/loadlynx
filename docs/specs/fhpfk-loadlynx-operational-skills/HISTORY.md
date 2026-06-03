# LoadLynx operational skills packaging and workflow boundary history

## Creation

The spec was created after review feedback showed that the initial skill split was too coupled to the repository checkout. The revised boundary treats users and developers as different runtime contexts:

- Users may only have network access, USB, released host tools, and released firmware assets from GitHub Releases.
- Developers may have the repository and hardware tooling, but must prove that context before running project commands, or clone the canonical repository only when source work is required.

The design also records `vercel-labs/skills` compatibility as an explicit packaging requirement instead of relying on local validator success alone.

## Release asset requirement

The user workflow depends on a local USB bridge as the preferred hardware path, so the project cannot leave that bridge as a source-only developer artifact. Official and development GitHub Releases publish platform host-tools archives containing `loadlynx-devd` and `loadlynx`; user-facing USB instructions can now point to those release assets while developer-only source builds remain in the developer skill.

## CLI capability gate

The user skill now includes released CLI firmware flash, WiFi configuration, and saved hardware as user-facing workflow areas, but requires command-surface verification before giving steps. This is intentional: agents must report missing commands instead of inventing commands or falling back to Web UI operation.

## CLI hardware memory

Hardware memory is a `loadlynx` CLI feature, not a Web UI or project-local feature. The CLI stores saved and successfully connected USB/HTTP hardware in the user's OS config directory, with `LOADLYNX_HOME` as an override for tests or explicit advanced setup. USB records sort before HTTP records so normal user workflows try the local USB/devd path before HTTP fallback. Dedicated `hardware available` and `hardware recent` commands expose currently connectable devices and recent devices without overloading the remembered-device list.

## CLI-only hardware operation

Skill-driven user hardware operation is CLI-only. USB/devd access is preferred first; HTTP is a fallback when USB is unavailable, explicitly not desired, or selected from saved hardware. Web UI can remain a product/developer surface, but it is not the operation path for these skills.

## Installer and IPC boundary

Released host tools now include installer scripts and `SHA256SUMS` verification as the primary user install path. The CLI/devd skill boundary changed with the host tools: CLI hardware operation uses IPC and sibling auto-start, while HTTP bridge usage is limited to loopback browser/debug paths. The skills explicitly treat Web Serial as a formal human browser path, not the agent-operated hardware path.

Real ESP32-S3 flash instructions now require first-flash/non-project gates across CLI, devd bridge and Web Serial: artifact/hash/target evidence, explicit owner confirmation, explicit non-project acknowledgement when applicable, and post-flash identity capture. Owner confirmation can be natural language and must not require a fixed typed phrase.

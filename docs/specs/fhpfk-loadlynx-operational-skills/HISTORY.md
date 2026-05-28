# LoadLynx operational skills packaging and workflow boundary history

## Creation

The spec was created after review feedback showed that the initial skill split was too coupled to the repository checkout. The revised boundary treats users and developers as different runtime contexts:

- Users may only have network access, USB, released host tools, and released firmware assets from GitHub Releases.
- Developers may have the repository and hardware tooling, but must prove that context before running project commands, or clone the canonical repository only when source work is required.

The design also records `vercel-labs/skills` compatibility as an explicit packaging requirement instead of relying on local validator success alone.

## Release asset requirement

The user workflow depends on a local USB bridge as the preferred hardware path, so the project cannot leave that bridge as a source-only developer artifact. Official and development GitHub Releases publish platform host-tools archives containing `loadlynx-devd` and `loadlynx`; user-facing USB instructions can now point to those release assets while developer-only source builds remain in the developer skill.

## CLI capability gate

The user skill now includes released CLI firmware flash, WiFi configuration, and saved hardware as user-facing workflow areas, but requires command-surface verification before giving steps. This is intentional: the current `loadlynx` CLI exposes flash/reset/monitor/status/output/USB-port commands, but does not expose WiFi configuration or complete user-level saved hardware commands. Agents must report those gaps instead of inventing commands or falling back to Web UI operation.

## CLI-only hardware operation

Skill-driven user hardware operation is CLI-only. USB/devd access is preferred first; HTTP is a fallback when USB is unavailable, explicitly not desired, or selected from saved hardware. Web UI can remain a product/developer surface, but it is not the operation path for these skills.

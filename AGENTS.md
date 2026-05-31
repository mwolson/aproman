# Agent Instructions

## Project overview

aproman (audio-profile-manager) is a Rust daemon that fixes HDMI audio after
suspend/resume on Linux systems running PipeWire + WirePlumber. It monitors
D-Bus for wake signals and cycles the card profile to force a node rebuild.

## Planning

Prefer to write plans in the `plans/` directory.

## Conventions

- Single-binary Rust crate with focused modules.
- Minimal dependencies. No async runtime and no D-Bus library; shell out to
  `pactl`, `pw-dump`, and `dbus-monitor`.
- Keep code comments minimal.
- When making changes to data in existing code, try to keep things in
  alphabetical order when it's reasonable to do so.
- Prefer top-down control flow: caller first, then callee.
- When writing bash scripts: `#!/bin/bash`, 4 spaces for indentation, fail-fast
  dependency checks.

## Key files

- `src/main.rs` -- clap parse and subcommand dispatch
- `src/cli.rs` -- CLI structs
- `src/config.rs` -- `~/.config/aproman.conf` parser and default writes
- `src/daemon.rs` -- daemon loop, IPC, sleep, and PipeWire event handling
- `src/pactl.rs` -- card/profile parsing and profile switching
- `src/service/` -- install/uninstall dispatch by init system
- `systemd/aproman.service` -- systemd user service
- `openrc-user/aproman` -- OpenRC 0.60+ user init script
- `openrc-system/aproman` -- OpenRC pre-0.60 system init script
- `install.sh` -- convenience installer for source builds

## Dev loop tools

```sh
bun run test
bun run test:integration
bun run test:all
bun run hooks:check
```

## Releasing

Before releasing, run `git fetch --tags` and `bun run hooks:check`. Version
bumps update `Cargo.toml` and `package.json`, then run `cargo update -p aproman`
to refresh `Cargo.lock`. Commit the version bump by itself with message
`chore: bump version to <version>`.

For GitHub release notes, review commits since the previous tag and write notes
from the user's point of view. Start with a short summary, group related changes
under descriptive headings, and avoid a single generic `## Changes` section when
the release has multiple themes. Put user-visible changes first, keep
maintenance details secondary, and keep the "Full Changelog" link when GitHub
generated one. Omit routine verification sections or check-command lists; report
validation in the chat handoff instead.

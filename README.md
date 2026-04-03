# aproman

Fix HDMI audio after suspend/resume on Linux systems running PipeWire +
WirePlumber.

## The Problem

When a Linux system resumes from suspend, HDMI audio devices often lose their
connection. WirePlumber tries to link to stale node proxies, resulting in
silence. The only manual fix is to open your audio settings and switch the card
profile away (for example to `off`) and back, forcing a full teardown and
rebuild of the audio nodes.

## How It Works

`aproman` runs as a user systemd service and:

1. Auto-detects your HDMI audio card, or uses the one saved in the config file
2. Monitors D-Bus for `PrepareForSleep` signals from systemd-logind
3. On wake, waits briefly for HDMI to renegotiate, then cycles the card profile
   off and back on

This forces PipeWire and WirePlumber to rebuild fresh nodes, restoring audio
without manual intervention.

## Requirements

- PipeWire with WirePlumber, or PulseAudio compatibility via PipeWire
- `pactl`
- `dbus-monitor`
- `systemctl`
- A systemd-based Linux distribution

## Installation

### Recommended: uv

```bash
uv tool install aproman
```

This installs `aproman` to `~/.local/bin/`.

Then install and start the systemd service:

```bash
git clone https://github.com/mwolson/aproman.git
cd aproman
mkdir -p ~/.config/systemd/user
cp systemd/aproman.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now aproman.service
```

### Alternative: install.sh

```bash
git clone https://github.com/mwolson/aproman.git
cd aproman
./install.sh
systemctl --user start aproman.service
```

This copies `aproman` to `~/.local/bin/` and installs and enables the user
service.

### Optional: set defaults

After installing, you can optionally save your preferred card and profile so
that aproman uses them instead of auto-detecting:

```bash
aproman list-cards
aproman set-default-card alsa_card.pci-0000_01_00.1

aproman list-profiles
aproman set-default-profile pro-audio
```

These write to `~/.config/aproman.conf` and signal the running daemon to pick up
the changes. Without defaults, aproman auto-detects the first HDMI card and uses
its active profile at startup.

## Usage

The service runs automatically. To check status:

```bash
systemctl --user status aproman.service
journalctl --user -u aproman.service -f
```

### Commands

aproman uses subcommands for one-off operations. With no subcommand, it runs as
a daemon.

```text
aproman                              Run as a daemon (default)
aproman cycle                        Cycle the card profile off and back on
aproman get-default-card             Print the default card from the config file
aproman get-default-profile          Print the default profile from the config file
aproman list-cards                   List available audio cards
aproman list-profiles                List available profiles for the card
aproman set-default-card CARD        Save default card and signal the daemon
aproman set-default-profile PROFILE  Save default profile and signal the daemon
```

### Daemon options

These flags apply to the daemon and to `cycle`:

```text
--card CARD            PipeWire/PulseAudio card name (default: config file, then auto-detect HDMI)
--profile PROFILE      Desired audio profile (default: config file, then active profile)
--wake-delay SECONDS   Seconds to wait after wake before cycling (default: 3.0)
```

### Configuration File

`aproman` reads defaults from `~/.config/aproman.conf` (or
`$XDG_CONFIG_HOME/aproman.conf`). The file uses one flag per line:

```text
--card=alsa_card.pci-0000_01_00.1
--profile=pro-audio
```

Only `--card` and `--profile` are supported. Unrecognized flags cause an error
at startup. Command-line arguments always take precedence over the config file.

When the daemon receives a SIGHUP (sent automatically by `set-default-card` and
`set-default-profile`, or manually via `kill -HUP`), it reloads the config file
and updates the card and profile for future suspend/resume cycles.

### One-Shot Fix

If audio breaks and the daemon missed the resume event (for example, after a
service restart), you can manually trigger a single profile cycle:

```bash
aproman cycle
```

This sends a cycle request to the running daemon via its Unix socket. If the
daemon is unavailable, it falls back to running the cycle directly.

When the card is stuck in the `off` state, `cycle` automatically selects the
highest-priority available profile. You can override with `--profile`:

```bash
aproman --profile pro-audio cycle
```

### Example: Custom Card and Profile

```bash
aproman set-default-card alsa_card.pci-0000_01_00.1
aproman set-default-profile output:hdmi-stereo
```

## Uninstall

```bash
systemctl --user disable --now aproman.service
uv tool uninstall aproman  # or: rm ~/.local/bin/aproman
rm ~/.config/systemd/user/aproman.service
rm -f ~/.config/aproman.conf
systemctl --user daemon-reload
```

## Testing

```bash
python3 -m unittest discover -s tests -v
```

## Hooks

```bash
lefthook install
lefthook run pre-commit --all-files
```

The pre-commit hook runs `uvx ruff check`, `uvx ty check`, and the unit test
suite.

## License

MIT

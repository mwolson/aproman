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

1. Auto-detects your HDMI audio card, or accepts one via `--card`
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

```bash
git clone https://github.com/mwolson/aproman.git
cd aproman
./install.sh
```

This copies `aproman` to `~/.local/bin/` and installs and enables the user
service.

To start immediately without logging out:

```bash
systemctl --user start aproman.service
```

## Usage

The service runs automatically. To check status:

```bash
systemctl --user status aproman.service
journalctl --user -u aproman.service -f
```

### Command-Line Options

You can customize behavior by editing the `ExecStart` line in the service file:

```text
--card CARD_NAME       PipeWire/PulseAudio card name (default: auto-detect HDMI card)
--profile PROFILE      Desired audio profile (default: active profile on startup)
--wake-delay SECONDS   Seconds to wait after wake before cycling (default: 3.0)
```

To find your card name:

```bash
pactl list cards short
```

### Example: Custom Card and Profile

Edit `~/.config/systemd/user/aproman.service`:

```ini
ExecStart=%h/.local/bin/aproman --card alsa_card.pci-0000_01_00.1 --profile output:hdmi-stereo
```

Then reload and restart:

```bash
systemctl --user daemon-reload
systemctl --user restart aproman.service
```

## Uninstall

```bash
systemctl --user disable --now aproman.service
rm ~/.local/bin/aproman
rm ~/.config/systemd/user/aproman.service
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

# audio-profile-manager

Fix HDMI audio after suspend/resume on Linux systems running PipeWire + WirePlumber.

## The Problem

When a Linux system resumes from suspend, HDMI audio devices often lose their
connection. WirePlumber tries to link to stale node proxies, resulting in
silence. The only manual fix is to open your audio settings and switch the card
profile away (e.g. to "Off") and back, forcing a full teardown and rebuild of
the audio nodes.

This is a long-standing issue across multiple kernel versions and desktop
environments, affecting AMD and Intel HDMI/DisplayPort audio. Kernel-level fixes
have been attempted (such as the conditional ALSA snooping patch for AMD in
Linux 6.11) but were reverted due to regressions on other hardware.

## How It Works

`audio-profile-manager` runs as a user systemd service and:

1. Auto-detects your HDMI audio card (or accepts one via `--card`)
2. Monitors D-Bus for `PrepareForSleep` signals from systemd-logind
3. On wake, waits briefly for HDMI to renegotiate, then cycles the card profile
   off and back on

This forces PipeWire/WirePlumber to tear down stale nodes and rebuild fresh
ones, restoring audio without manual intervention.

## Requirements

- PipeWire with WirePlumber (or PulseAudio)
- `pactl` (from `pipewire-pulse` or `pulseaudio-utils`)
- `dbus-monitor` (from `dbus` or `dbus-tools`)
- A systemd-based Linux distribution

## Installation

```bash
git clone https://github.com/mwolson/audio-profile-manager.git
cd audio-profile-manager
```

Then run:

```bash
./install.sh
```

This copies `audio-profile-manager` to `~/.local/bin/` and installs + enables
the systemd user service.

To start immediately without logging out:

```bash
systemctl --user start audio-profile-manager.service
```

## Usage

The service runs automatically. To check status:

```bash
systemctl --user status audio-profile-manager.service
journalctl --user -u audio-profile-manager.service -f
```

### Command-Line Options

You can customize behavior by editing the `ExecStart` line in the service file:

```
--card CARD_NAME       PipeWire/PulseAudio card name (default: auto-detect HDMI card)
--profile PROFILE      Desired audio profile (default: auto-detect active profile)
--wake-delay SECONDS   Seconds to wait after wake before cycling (default: 3)
```

To find your card name:

```bash
pactl list cards short
```

### Example: Custom Card and Profile

Edit `~/.config/systemd/user/audio-profile-manager.service`:

```ini
ExecStart=%h/.local/bin/audio-profile-manager --card alsa_card.pci-0000_01_00.1 --profile output:hdmi-stereo
```

Then reload and restart:

```bash
systemctl --user daemon-reload
systemctl --user restart audio-profile-manager.service
```

## Uninstall

```bash
systemctl --user disable --now audio-profile-manager.service
rm ~/.local/bin/audio-profile-manager
rm ~/.config/systemd/user/audio-profile-manager.service
systemctl --user daemon-reload
```

## License

MIT

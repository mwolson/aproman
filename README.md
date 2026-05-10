# aproman

Fix HDMI audio after suspend/resume on Linux systems running PipeWire +
WirePlumber.

## The Problem

When a Linux system resumes from suspend, HDMI audio devices often lose their
connection. WirePlumber can link to stale node proxies, resulting in silence.
Switching the card profile away and back forces a full audio node rebuild.

## How It Works

`aproman` runs as a service (systemd or OpenRC) and:

1. Auto-detects your HDMI audio card, or uses the one saved in the config file.
2. Monitors D-Bus for `PrepareForSleep` signals from systemd-logind or elogind.
3. On wake, waits briefly for HDMI to renegotiate, then cycles the card profile
   off and back on.
4. Monitors PipeWire through `pw-dump --monitor --no-colors` for nodes entering
   an error state. If one is detected, it restarts PipeWire with a cooldown.

## Requirements

- PipeWire with WirePlumber, or PulseAudio compatibility via PipeWire
- `pactl`
- `dbus-monitor`
- `pw-dump` (optional, for node error monitoring)
- A Linux distribution with systemd or OpenRC (elogind for OpenRC)

## Installation

### cargo

```bash
cargo install aproman
aproman install-service
```

Start the service:

```bash
systemctl --user start aproman.service          # systemd
rc-service --user aproman start                 # OpenRC 0.60+
sudo rc-service aproman start                   # older OpenRC
```

### install.sh (systemd, source build)

```bash
git clone https://github.com/mwolson/aproman.git
cd aproman
./install.sh
systemctl --user start aproman.service
```

## Usage

```text
aproman                              Run as a daemon (default)
aproman cycle                        Cycle the card profile off and back on
aproman toggle                       Alias for cycle
aproman get-default-card             Print the default card from the config file
aproman get-default-profile          Print the default profile from the config file
aproman install-service              Install and enable the service (systemd or OpenRC)
aproman list-cards                   List available audio cards
aproman list-profiles                List available profiles for the card
aproman set-default-card CARD        Save default card and signal the daemon
aproman set-default-profile PROFILE  Save default profile and signal the daemon
aproman uninstall-service            Disable and remove the service (systemd or OpenRC)
```

Daemon options:

```text
--card CARD            PipeWire/PulseAudio card name
--profile PROFILE      Desired audio profile
--wake-delay SECONDS   Seconds to wait after wake before cycling (default: 3.0)
```

`aproman` reads defaults from `~/.config/aproman.conf` (or
`$XDG_CONFIG_HOME/aproman.conf`). The file uses one flag per line:

```text
--card=alsa_card.pci-0000_01_00.1
--profile=pro-audio
```

## Uninstall

```bash
aproman uninstall-service
cargo uninstall aproman       # or: rm ~/.local/bin/aproman
rm -f ~/.config/aproman.conf
```

## License

MIT

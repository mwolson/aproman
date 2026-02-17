#!/bin/bash
#
# Install audio-profile-manager and its systemd user service.
#
# This script:
#   1. Copies audio-profile-manager to ~/.local/bin/
#   2. Copies the systemd service to ~/.config/systemd/user/
#   3. Enables (but does not start) the service
#
# Before running, edit systemd/audio-profile-manager.service to uncomment
# the After= and WantedBy= lines matching your desktop environment.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BIN_DIR="$HOME/.local/bin"
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"

echo "Installing audio-profile-manager..."

mkdir -p "$BIN_DIR"
cp "$SCRIPT_DIR/audio-profile-manager" "$BIN_DIR/audio-profile-manager"
chmod +x "$BIN_DIR/audio-profile-manager"
echo "  Installed $BIN_DIR/audio-profile-manager"

mkdir -p "$SYSTEMD_USER_DIR"
cp "$SCRIPT_DIR/systemd/audio-profile-manager.service" "$SYSTEMD_USER_DIR/"
echo "  Installed $SYSTEMD_USER_DIR/audio-profile-manager.service"

systemctl --user daemon-reload
systemctl --user enable audio-profile-manager.service
echo "  Enabled audio-profile-manager.service"

echo ""
echo "Done. To start immediately:"
echo "  systemctl --user start audio-profile-manager.service"
echo ""
echo "To check status:"
echo "  systemctl --user status audio-profile-manager.service"
echo "  journalctl --user -u audio-profile-manager.service -f"

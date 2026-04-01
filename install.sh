#!/bin/bash

set -euo pipefail

if ! command -v systemctl >/dev/null 2>&1; then
    echo "Error: 'systemctl' is required but not found in PATH." >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BIN_DIR="$HOME/.local/bin"
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"

echo "Installing aproman..."

mkdir -p "$BIN_DIR"
cp "$SCRIPT_DIR/aproman.py" "$BIN_DIR/aproman"
chmod +x "$BIN_DIR/aproman"
echo "  Installed $BIN_DIR/aproman"

mkdir -p "$SYSTEMD_USER_DIR"
cp "$SCRIPT_DIR/systemd/aproman.service" "$SYSTEMD_USER_DIR/"
echo "  Installed $SYSTEMD_USER_DIR/aproman.service"

systemctl --user daemon-reload
systemctl --user enable aproman.service
echo "  Enabled aproman.service"

echo ""
echo "Done. To start immediately:"
echo "  systemctl --user start aproman.service"
echo ""
echo "To check status:"
echo "  systemctl --user status aproman.service"
echo "  journalctl --user -u aproman.service -f"

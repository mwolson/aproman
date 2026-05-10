#!/bin/bash

set -euo pipefail

for cmd in docker; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Error: '$cmd' is required but not found in PATH." >&2
        exit 1
    fi
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

docker compose build
docker compose run --rm systemd
docker compose run --rm openrc-user
docker compose run --rm openrc-system

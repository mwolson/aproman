#!/bin/bash

set -euo pipefail

export PATH="$HOME/.local/bin:$PATH"

aproman --version
aproman --help >/dev/null

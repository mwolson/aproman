#!/bin/bash
set -euo pipefail

# If running as root, set up environment and re-exec as testuser
if [[ "$(id -u)" == "0" ]]; then
    export XDG_RUNTIME_DIR=/run/user/1000
    mkdir -p "$XDG_RUNTIME_DIR"
    chown testuser:testuser "$XDG_RUNTIME_DIR"
    exec su testuser -s /bin/bash "$0"
fi

# From here on, running as testuser
export PATH="$HOME/.local/bin:$PATH"
export XDG_RUNTIME_DIR=/run/user/1000

pass=0
fail=0
errors=""

run_test() {
    local name="$1"
    shift
    printf "  %-50s " "$name"
    local output
    if output=$(eval "$*" 2>&1); then
        echo "ok"
        pass=$((pass + 1))
    else
        echo "FAIL"
        fail=$((fail + 1))
        errors="${errors}  - ${name}\n"
        if [[ -n "$output" ]]; then
            printf "    %s\n" "$output"
        fi
    fi
}

echo "=== OpenRC User Integration Tests ==="
echo ""

# Initialize user OpenRC runtime
mkdir -p "$XDG_RUNTIME_DIR/openrc"
touch "$XDG_RUNTIME_DIR/openrc/softlevel"
openrc --user 2>/dev/null || true

# Create user default runlevel and conf.d for logging
mkdir -p "$HOME/.config/rc/runlevels/default"
mkdir -p "$HOME/.config/rc/conf.d"
cat > "$HOME/.config/rc/conf.d/aproman" << CONF
output_log="$HOME/aproman.log"
error_log="$HOME/aproman.log"
CONF

echo "Install service:"
run_test "install-service succeeds" \
    'aproman install-service'
run_test "init script installed" \
    "test -x $HOME/.config/rc/init.d/aproman"
run_test "init script has openrc-run shebang" \
    "head -1 $HOME/.config/rc/init.d/aproman | grep -q openrc-run"
run_test "listed in default runlevel" \
    "rc-update --user show default 2>&1 | grep -q aproman"

echo ""
echo "Service lifecycle:"
run_test "start service" \
    "rc-service --user aproman start"
run_test "status reports started" \
    "rc-service --user aproman status 2>&1 | grep -q started"
run_test "aproman process is running" \
    "pgrep -f aproman >/dev/null"
run_test "stop service" \
    "rc-service --user aproman stop"
run_test "status reports stopped" \
    "(rc-service --user aproman status 2>&1 || true) | grep -q stopped"
run_test "aproman process is not running" \
    "! pgrep -f /usr/local/bin/aproman >/dev/null"
run_test "restart service" \
    "rc-service --user aproman start"
run_test "status reports started after restart" \
    "rc-service --user aproman status 2>&1 | grep -q started"
run_test "stop after restart" \
    "rc-service --user aproman stop"

echo ""
echo "Config reload via socket:"
run_test "start service for reload test" \
    "rc-service --user aproman start"
run_test "daemon socket is ready" \
    'for i in $(seq 1 5); do test -S $XDG_RUNTIME_DIR/aproman.sock && exit 0; sleep 1; done; exit 1'
run_test "set-default-card succeeds" \
    "aproman set-default-card test_card"
run_test "daemon reloaded config" \
    'for i in $(seq 1 5); do grep -q "Reloading config" '"$HOME/aproman.log"' 2>/dev/null && exit 0; sleep 1; done; exit 1'
run_test "config file written" \
    "grep -q 'card=test_card' $HOME/.config/aproman.conf"
run_test "daemon still running after reload" \
    "pgrep -f aproman >/dev/null"
run_test "stop after reload test" \
    "rc-service --user aproman stop"

echo ""
echo "Uninstall service:"
run_test "uninstall-service succeeds" \
    "aproman uninstall-service"
run_test "init script removed" \
    "! test -f $HOME/.config/rc/init.d/aproman"
run_test "not listed in default runlevel" \
    "! rc-update --user show default 2>&1 | grep -q aproman"

echo ""
echo "Results: ${pass} passed, ${fail} failed"
if [[ -n "$errors" ]]; then
    echo ""
    echo "Failures:"
    printf "$errors"
    exit 1
fi

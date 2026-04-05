#!/bin/bash
set -euo pipefail

# If running as root, set up environment and re-exec as testuser
if [[ "$(id -u)" == "0" ]]; then
    export XDG_RUNTIME_DIR=/run/user/1000
    mkdir -p "$XDG_RUNTIME_DIR"
    chown testuser:testuser "$XDG_RUNTIME_DIR"

    systemctl start user@1000.service 2>/dev/null || true
    echo "Waiting for user systemd instance..."
    for i in $(seq 1 15); do
        if su testuser -c "XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR systemctl --user daemon-reload" 2>/dev/null; then
            break
        fi
        sleep 1
    done

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

echo "=== systemd Integration Tests ==="
echo ""

run_test "shipped service file exists" \
    'test -f /build/systemd/aproman.service'

service_file="$HOME/.config/systemd/user/aproman.service"

echo ""
echo "Install service:"
run_test "install-service succeeds" \
    'aproman install-service'
run_test "unit file installed" \
    'test -f '"$service_file"
run_test "installed file has [Service] section" \
    'grep -q "^\[Service\]" '"$service_file"
run_test "systemd-analyze verify passes" \
    'systemd-analyze --user verify '"$service_file"
run_test "service is enabled" \
    'systemctl --user is-enabled aproman.service'

echo ""
echo "Service lifecycle:"
run_test "start service" \
    'systemctl --user start aproman.service'

sleep 1

run_test "service is active" \
    'systemctl --user is-active aproman.service'
run_test "aproman process is running" \
    'pgrep -f "aproman" >/dev/null'
run_test "stop service" \
    'systemctl --user stop aproman.service'
run_test "service is inactive after stop" \
    '! systemctl --user is-active --quiet aproman.service'
run_test "restart service" \
    'systemctl --user restart aproman.service'

sleep 1

run_test "service is active after restart" \
    'systemctl --user is-active aproman.service'
run_test "stop after restart" \
    'systemctl --user stop aproman.service'

echo ""
echo "Config reload via socket:"
run_test "start service for reload test" \
    'systemctl --user start aproman.service'
sleep 1
run_test "set-default-card succeeds" \
    'aproman set-default-card test_card'
run_test "daemon reloaded config" \
    'for i in $(seq 1 5); do journalctl --user -u aproman.service --no-pager 2>&1 | grep -q "Reloading config" && exit 0; sleep 1; done; exit 1'
run_test "config file written" \
    'grep -q "card=test_card" ~/.config/aproman.conf'
run_test "daemon still running after reload" \
    'systemctl --user is-active aproman.service'
run_test "stop after reload test" \
    'systemctl --user stop aproman.service'

echo ""
echo "Uninstall service:"
run_test "uninstall-service succeeds" \
    'aproman uninstall-service'
run_test "unit file removed" \
    '! test -f '"$service_file"

echo ""
echo "Results: ${pass} passed, ${fail} failed"
if [[ -n "$errors" ]]; then
    echo ""
    echo "Failures:"
    printf "$errors"
    exit 1
fi

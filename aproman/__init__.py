#!/usr/bin/env python3

import argparse
import datetime as dt
import os
import re
import select
import shutil
import signal
import socket
import subprocess
import sys
import time

VERSION = "0.2.0"

CONF_PATH = os.path.join(
    os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config"),
    "aproman.conf",
)
ALLOWED_CONF_FLAGS = {"--card", "--profile"}
SYSTEMD_USER_DIR = os.path.join(
    os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config"),
    "systemd",
    "user",
)


def main():
    file_args = load_conf()
    args = parse_args(file_args)

    command = getattr(args, "command", None)
    if command == "cycle":
        require_commands(["pactl", "systemctl"])
        run_cycle(args)
    elif command == "get-default-card":
        run_get_default("card")
    elif command == "get-default-profile":
        run_get_default("profile")
    elif command == "install-service":
        require_commands(["systemctl"])
        run_install_service()
    elif command == "list-cards":
        require_commands(["pactl"])
        run_list_cards(args)
    elif command == "list-profiles":
        require_commands(["pactl"])
        run_list_profiles(args)
    elif command == "set-default-card":
        run_set_default("card", args.value)
    elif command == "set-default-profile":
        run_set_default("profile", args.value)
    elif command == "uninstall-service":
        require_commands(["systemctl"])
        run_uninstall_service()
    else:
        require_commands(["dbus-monitor", "pactl", "systemctl"])
        run_daemon(args)


def run_list_cards(args):
    cards = list(iter_card_blocks())
    if not cards:
        warn("Error: No audio cards found.")
        sys.exit(1)
    selected = args.card or detect_hdmi_card_name()
    for name, block in cards:
        card_label = None
        active = None
        has_hdmi = False
        for line in block:
            match = re.match(r'\s*alsa\.card_name\s*=\s*"(.+)"', line)
            if match:
                card_label = match.group(1)
            match = re.match(r"\s*Active Profile:\s*(.+)", line)
            if match:
                active = match.group(1).strip()
            if 'port.type = "hdmi"' in line:
                has_hdmi = True
        parts = [name]
        if card_label:
            parts.append(f"({card_label})")
        if active:
            parts.append(f"profile: {active}")
        if has_hdmi:
            parts.append("hdmi")
        if name == selected:
            parts.append("*")
        print("  ".join(parts))


def run_list_profiles(args):
    card_name = args.card or detect_hdmi_card()
    active = get_active_profile(card_name)
    profiles = get_profiles(card_name)
    if not profiles:
        warn(f"Error: No profiles found for card '{card_name}'.")
        sys.exit(1)
    for name, priority, available in profiles:
        marker = " *" if name == active else ""
        print(f"{name}  (priority: {priority}, available: {available}){marker}")


def run_get_default(key):
    flag = f"--{key}"
    hint = f"Use 'aproman list-{key}s' to see available options, then 'aproman set-default-{key}' to set one."
    hdmi_note = "Without a default, aproman auto-detects the first HDMI card." if key == "card" else ""

    if not os.path.exists(CONF_PATH):
        warn(f"No config file found at {CONF_PATH}")
        if hdmi_note:
            warn(hdmi_note)
        warn(hint)
        sys.exit(1)

    for conf_flag, value in iter_conf_entries(CONF_PATH):
        if conf_flag == flag:
            print(value)
            return

    warn(f"No {flag} entry found in {CONF_PATH}")
    if hdmi_note:
        warn(hdmi_note)
    warn(hint)
    sys.exit(1)


def run_set_default(key, value):
    require_commands(["systemctl"])
    flag = f"--{key}"
    flag_prefix = f"{flag}="
    lines = []
    replaced = False

    if os.path.exists(CONF_PATH):
        with open(CONF_PATH) as f:
            for line in f:
                if line.strip().startswith(flag_prefix):
                    lines.append(f"{flag_prefix}{value}\n")
                    replaced = True
                else:
                    lines.append(line)

    if not replaced:
        lines.append(f"{flag_prefix}{value}\n")

    os.makedirs(os.path.dirname(CONF_PATH), exist_ok=True)
    with open(CONF_PATH, "w") as f:
        f.writelines(lines)

    log(f"Wrote {flag_prefix}{value} to {CONF_PATH}")
    signal_daemon()


def signal_daemon():
    try:
        output = subprocess.check_output(
            ["systemctl", "--user", "show", "aproman.service", "-p", "MainPID"],
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError:
        return

    match = re.match(r"MainPID=(\d+)", output.strip())
    if not match or match.group(1) == "0":
        return

    pid = int(match.group(1))
    try:
        os.kill(pid, signal.SIGHUP)
        log(f"Sent SIGHUP to aproman daemon (PID {pid})")
    except OSError as exc:
        warn(f"Warning: Could not signal daemon (PID {pid}): {exc}")


def run_install_service():
    content = get_service_source()
    service_path = os.path.join(SYSTEMD_USER_DIR, "aproman.service")

    os.makedirs(SYSTEMD_USER_DIR, exist_ok=True)
    with open(service_path, "w") as f:
        f.write(content)
    log(f"Installed {service_path}")

    subprocess.run(["systemctl", "--user", "daemon-reload"], check=True)
    subprocess.run(["systemctl", "--user", "enable", "aproman.service"], check=True)
    log("Enabled aproman.service")

    log("")
    log("To start immediately:")
    log("  systemctl --user start aproman.service")
    log("")
    log("To check status:")
    log("  systemctl --user status aproman.service")
    log("  journalctl --user -u aproman.service -f")


def get_service_source():
    try:
        from importlib.resources import files

        return (files("aproman") / "systemd" / "aproman.service").read_text()
    except (FileNotFoundError, TypeError, ModuleNotFoundError):
        pass

    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    path = os.path.join(root, "systemd", "aproman.service")
    with open(path) as f:
        return f.read()


def run_uninstall_service():
    service_path = os.path.join(SYSTEMD_USER_DIR, "aproman.service")

    subprocess.run(
        ["systemctl", "--user", "disable", "--now", "aproman.service"],
        check=False,
    )

    try:
        os.remove(service_path)
        log(f"Removed {service_path}")
    except FileNotFoundError:
        log(f"No service file at {service_path}")

    subprocess.run(["systemctl", "--user", "daemon-reload"], check=True)
    log("Uninstalled aproman.service")


def run_cycle(args):
    card_name = args.card or detect_hdmi_card()
    profile = resolve_cycle_profile(args, card_name)

    try:
        send_command(f"cycle {profile}")
        log(f"Sent cycle request to daemon (profile: {profile})")
    except OSError as exc:
        warn(f"Warning: daemon unavailable, running cycle directly ({exc}).")
        log(f"Cycling profile on {card_name}: -> off -> {profile}")
        cycle_profile(card_name, profile)
        log("Done.")


def resolve_cycle_profile(args, card_name):
    if args.profile:
        return args.profile

    current_profile = get_active_profile(card_name)
    if current_profile and current_profile != "off":
        return current_profile

    best = detect_best_profile(card_name)
    if not best:
        warn(f"Error: Card '{card_name}' has no available profiles. Use --profile to specify one.")
        sys.exit(1)
    log(f"Active profile is 'off', selected best available: {best}")
    return best


def run_daemon(args):
    card_name = args.card or detect_hdmi_card()
    current_profile = get_active_profile(card_name)
    if not current_profile:
        warn(f"Error: Card '{card_name}' not found or has no active profile.")
        sys.exit(1)

    profile = args.profile or current_profile

    if not args.card:
        log("No --card specified, auto-detecting HDMI audio card...")
        log(f"Detected: {card_name}")
    if not args.profile:
        log(f"No --profile specified, using active profile: {profile}")

    log(f"Card: {card_name}")
    log(f"Current profile: {current_profile}")
    log(f"Target profile: {profile}")

    state = {
        "card_name": card_name,
        "cli_card": args.cli_card,
        "cli_profile": args.cli_profile,
        "profile": profile,
    }

    def handle_sighup(_signum, _frame):
        reload_conf(state)

    signal.signal(signal.SIGHUP, handle_sighup)

    socket_path = get_socket_path()
    cleanup_socket(socket_path)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
    server.bind(socket_path)

    def handle_exit(_signum, _frame):
        cleanup_socket(socket_path)
        sys.exit(0)

    signal.signal(signal.SIGINT, handle_exit)
    signal.signal(signal.SIGTERM, handle_exit)

    log(f"Listening on {socket_path}")
    log("Monitoring for suspend/resume events...")

    try:
        monitor_loop(args.wake_delay, state, server)
    finally:
        server.close()
        cleanup_socket(socket_path)


def reload_conf(state):
    log(f"{timestamp()} Received SIGHUP, reloading {CONF_PATH}...")
    try:
        file_args = load_conf()
    except SystemExit:
        warn(f"{timestamp()} Warning: Failed to reload config, keeping current settings.")
        return

    changed = False

    new_card = file_args.get("card")
    if new_card and new_card != state["card_name"]:
        if state["cli_card"]:
            log(f"{timestamp()} CLI --card takes precedence, keeping: {state['card_name']}")
        else:
            log(f"{timestamp()} Updated card: {state['card_name']} -> {new_card}")
            state["card_name"] = new_card
            changed = True

    new_profile = file_args.get("profile")
    if new_profile and new_profile != state["profile"]:
        if state["cli_profile"]:
            log(f"{timestamp()} CLI --profile takes precedence, keeping: {state['profile']}")
        else:
            log(f"{timestamp()} Updated target profile: {state['profile']} -> {new_profile}")
            state["profile"] = new_profile
            changed = True

    if not changed:
        log(f"{timestamp()} Config reloaded, no changes.")


def parse_args(file_args):
    parser = argparse.ArgumentParser(
        prog="aproman",
        description=(
            "Fix HDMI audio after suspend/resume on PipeWire + WirePlumber. "
            "With no command, runs as a daemon monitoring D-Bus for suspend/resume events."
        ),
    )
    parser.add_argument("--version", action="version", version=f"%(prog)s {VERSION}")
    parser.add_argument("--card", help="PipeWire/PulseAudio card name")
    parser.add_argument("--profile", help="desired audio profile")
    parser.add_argument(
        "--wake-delay",
        type=positive_float,
        default=3.0,
        help="seconds to wait after wake before cycling (default: %(default)s)",
    )

    sub = parser.add_subparsers(dest="command", title="commands")
    sub.add_parser("cycle", help="cycle the card profile off and back on once, then exit")
    sub.add_parser("get-default-card", help="print the default card from the config file")
    sub.add_parser("get-default-profile", help="print the default profile from the config file")
    sub.add_parser("install-service", help="install and enable the systemd user service")
    sub.add_parser("list-cards", help="list available audio cards")
    sub.add_parser("list-profiles", help="list available profiles for the card")
    p = sub.add_parser("set-default-card", help="set the default card and signal the daemon")
    p.add_argument("value", metavar="CARD")
    p = sub.add_parser("set-default-profile", help="set the default profile and signal the daemon")
    p.add_argument("value", metavar="PROFILE")
    sub.add_parser("uninstall-service", help="disable and remove the systemd user service")

    args = parser.parse_args()

    args.cli_card = args.card
    args.cli_profile = args.profile
    if not args.card and "card" in file_args:
        args.card = file_args["card"]
    if not args.profile and "profile" in file_args:
        args.profile = file_args["profile"]

    return args


def load_conf():
    if not os.path.exists(CONF_PATH):
        return {}

    result = {}
    for flag, value in iter_conf_entries(CONF_PATH):
        if flag not in ALLOWED_CONF_FLAGS:
            warn(f"Error: Unsupported flag '{flag}' in {CONF_PATH}")
            sys.exit(1)
        if flag == "--card":
            result["card"] = value
        elif flag == "--profile":
            result["profile"] = value
    return result


def iter_conf_entries(path):
    with open(path) as f:
        for line_num, raw_line in enumerate(f, 1):
            line = raw_line.strip()
            if not line or line.startswith("#"):
                continue
            match = re.match(r"^(--[a-z][a-z0-9-]*)=(.+)$", line)
            if not match:
                warn(f"Error: Malformed line {line_num} in {path}: {line}")
                sys.exit(1)
            yield match.group(1), match.group(2)


def positive_float(value):
    try:
        parsed = float(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(f"invalid positive number: {value}") from exc
    if parsed <= 0:
        raise argparse.ArgumentTypeError(f"invalid positive number: {value}")
    return parsed


def require_commands(commands):
    missing = [command for command in commands if not shutil.which(command)]
    if missing:
        for command in missing:
            warn(f"Error: '{command}' is required but not found in PATH.")
        sys.exit(1)


def detect_hdmi_card_name():
    for name, block in iter_card_blocks():
        if any('port.type = "hdmi"' in line for line in block):
            return name
    return None


def detect_hdmi_card():
    name = detect_hdmi_card_name()
    if name:
        return name
    warn("Error: No HDMI audio card detected. Use --card to specify one manually.")
    sys.exit(1)


def get_active_profile(card_name):
    for name, block in iter_card_blocks():
        if name != card_name:
            continue
        for line in block:
            match = re.match(r"\s*Active Profile:\s*(.+)", line)
            if match:
                return match.group(1).strip()
        return None
    return None


def get_profiles(card_name):
    profiles = []
    for name, block in iter_card_blocks():
        if name != card_name:
            continue
        for line in block:
            match = re.match(
                r"\s+(\S+):.*priority:\s*(\d+),\s*available:\s*(yes|no)",
                line,
            )
            if match:
                profiles.append((match.group(1), int(match.group(2)), match.group(3)))
        break
    profiles.sort(key=lambda p: p[1], reverse=True)
    return profiles


def detect_best_profile(card_name):
    best_name = None
    best_priority = -1
    for name, block in iter_card_blocks():
        if name != card_name:
            continue
        for line in block:
            match = re.match(
                r"\s+(\S+):.*priority:\s*(\d+),\s*available:\s*yes",
                line,
            )
            if match:
                profile_name = match.group(1)
                priority = int(match.group(2))
                if profile_name != "off" and priority > best_priority:
                    best_name = profile_name
                    best_priority = priority
        break
    return best_name


def iter_card_blocks():
    try:
        output = subprocess.check_output(["pactl", "list", "cards"], text=True, stderr=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        return
    card_name = None
    block = []
    for line in output.splitlines():
        if line.startswith("Card #"):
            if card_name:
                yield card_name, block
            card_name = None
            block = []
            continue
        block.append(line)
        match = re.match(r"\s*Name:\s*(\S+)", line)
        if match:
            card_name = match.group(1)
    if card_name:
        yield card_name, block


def handle_resume(state, wake_delay):
    card_name = state["card_name"]
    profile = state["profile"]
    log(f"{timestamp()} Waking from sleep, waiting {wake_delay:g}s for HDMI to renegotiate...")
    time.sleep(wake_delay)
    cycle_profile(card_name, profile)


def handle_cycle_command(state, command):
    parts = command.split(None, 1)
    profile = parts[1] if len(parts) > 1 else state["profile"]
    card_name = state["card_name"]
    log(f"{timestamp()} Received cycle command (profile: {profile})")
    cycle_profile(card_name, profile)


def cycle_profile(card_name, target_profile):
    current_profile = get_active_profile(card_name)
    if not current_profile:
        warn(f"Warning: Could not determine active profile for {card_name}. Skipping cycle.")
        return

    log("Restarting pipewire and pipewire-pulse")
    subprocess.run(["systemctl", "--user", "restart", "pipewire.service"], check=True)
    subprocess.run(["systemctl", "--user", "restart", "pipewire-pulse.service"], check=True)

    log(f"Cycling profile on {card_name}: {current_profile} -> off -> {target_profile}")
    if not set_card_profile(card_name, "off", attempts=20, retry_delay=0.25):
        warn("Warning: Failed to set profile to 'off'. Will retry next cycle.")
        return

    time.sleep(1.0)

    if not set_card_profile(card_name, target_profile, attempts=20, retry_delay=0.25):
        warn(f"Warning: Failed to restore profile to '{target_profile}'. Will retry next cycle.")
        return

    log(f"Profile restored to {target_profile}.")


def set_card_profile(card_name, profile, attempts=1, retry_delay=0.25):
    for attempt in range(attempts):
        result = subprocess.run(
            ["pactl", "set-card-profile", card_name, profile],
            check=False,
            text=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if result.returncode == 0:
            return True
        if attempt + 1 < attempts:
            time.sleep(retry_delay)
    return False


def get_socket_path():
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR") or f"/run/user/{os.getuid()}"
    return os.path.join(runtime_dir, "aproman.sock")


def cleanup_socket(socket_path):
    try:
        if os.path.exists(socket_path):
            os.unlink(socket_path)
    except FileNotFoundError:
        pass


def send_command(command):
    client = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
    try:
        client.sendto(command.encode(), get_socket_path())
    finally:
        client.close()


def decode_command(data):
    command = data.decode().strip()
    if not command.startswith("cycle"):
        raise ValueError(f"Unsupported command: {command}")
    return command


def monitor_loop(wake_delay, state, server):
    dbus_filter = (
        "interface=org.freedesktop.login1.Manager,"
        "sender=org.freedesktop.login1,"
        "member=PrepareForSleep"
    )
    process = subprocess.Popen(
        ["dbus-monitor", "--system", "--monitor", dbus_filter],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )

    assert process.stdout is not None
    dbus_fd = process.stdout.fileno()
    sock_fd = server.fileno()
    expect_state = False
    dbus_buffer = ""

    try:
        while True:
            try:
                readable, _, _ = select.select([dbus_fd, sock_fd], [], [])
            except InterruptedError:
                continue

            for fd in readable:
                if fd == sock_fd:
                    try:
                        data = server.recv(256)
                        command = decode_command(data)
                        handle_cycle_command(state, command)
                    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
                        warn(f"Warning: {exc}")
                elif fd == dbus_fd:
                    chunk = os.read(dbus_fd, 4096)
                    if not chunk:
                        return
                    dbus_buffer += chunk.decode()
                    while "\n" in dbus_buffer:
                        raw_line, dbus_buffer = dbus_buffer.split("\n", 1)
                        line = raw_line.strip()
                        if "member=PrepareForSleep" in line:
                            expect_state = True
                            continue
                        if not expect_state or "boolean " not in line:
                            continue

                        expect_state = False
                        if "boolean true" in line:
                            log(f"{timestamp()} Going to sleep.")
                        elif "boolean false" in line:
                            handle_resume(state, wake_delay)
                        else:
                            log(f"{timestamp()} Unknown state after PrepareForSleep: {line}")
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)


def timestamp():
    return dt.datetime.now().strftime("%Y-%m-%d %H:%M:%S")


def log(message):
    print(message, flush=True)


def warn(message):
    print(message, file=sys.stderr, flush=True)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3

import argparse
import datetime as dt
import re
import shutil
import subprocess
import sys
import time

VERSION = "0.1.1"


def main():
    args = parse_args()
    require_commands(["dbus-monitor", "pactl", "systemctl"])

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
    log("Monitoring for suspend/resume events...")

    monitor_suspend_resume(args.wake_delay, lambda: handle_resume(card_name, profile, args.wake_delay))


def parse_args():
    parser = argparse.ArgumentParser(
        prog="aproman",
        description="Fix HDMI audio after suspend/resume on PipeWire + WirePlumber.",
    )
    parser.add_argument("--version", action="version", version=f"%(prog)s {VERSION}")
    parser.add_argument("--card", help="PipeWire/PulseAudio card name")
    parser.add_argument("--profile", help="Desired audio profile")
    parser.add_argument(
        "--wake-delay",
        type=positive_float,
        default=3.0,
        help="Seconds to wait after wake before cycling (default: %(default)s)",
    )
    return parser.parse_args()


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


def detect_hdmi_card():
    for name, block in iter_card_blocks():
        if any('port.type = "hdmi"' in line for line in block):
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


def handle_resume(card_name, profile, wake_delay):
    log(f"{timestamp()} Waking from sleep, waiting {wake_delay:g}s for HDMI to renegotiate...")
    time.sleep(wake_delay)
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


def monitor_suspend_resume(wake_delay, on_resume):
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

    expect_state = False
    try:
        assert process.stdout is not None
        for raw_line in process.stdout:
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
                on_resume()
            else:
                log(f"{timestamp()} Unknown state after PrepareForSleep event: {line}")
    except KeyboardInterrupt:
        process.terminate()
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

use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
use std::process::Child;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use crossbeam_channel::{after, select};
use tracing::{info, warn};

use crate::cli::Overrides;
use crate::config::{self, Config};
use crate::events::{SignalKind, SleepTransition};
use crate::pactl;
use crate::pipewire::RestartState;
use crate::pw_monitor::PwMonitor;
use crate::signals::Handles;
use crate::sleep_monitor::SleepMonitor;

pub fn run(mut config: Config, overrides: Overrides, sigs: Handles) -> Result<()> {
    let mut card = pactl::resolve_card(config.card.as_deref())?;
    let profile = pactl::resolve_cycle_profile(&card, config.profile.as_deref())?;
    if config.card.is_none() {
        info!("No --card specified, auto-detecting HDMI audio card...");
        info!("Detected: {}", card.name);
    }
    if config.profile.is_none() {
        info!("No --profile specified, using active profile: {profile}");
    }
    config.card = Some(card.name.clone());
    config.profile = Some(profile);
    info!("Card: {}", card.name);
    info!(
        "Current profile: {}",
        card.active_profile.take().unwrap_or_default()
    );
    info!(
        "Target profile: {}",
        config.profile.as_deref().unwrap_or_default()
    );

    let socket_path = socket_path();
    cleanup_socket(&socket_path);
    let socket = UnixDatagram::bind(&socket_path)
        .with_context(|| format!("binding {}", socket_path.display()))?;
    socket.set_nonblocking(true)?;
    info!("Listening on {}", socket_path.display());
    info!("Monitoring for suspend/resume events...");

    let mut sleep_monitor = SleepMonitor::spawn()?;
    let mut pw_monitor = PwMonitor::spawn();
    let sleep_rx = sleep_monitor.rx();
    let pw_rx = pw_monitor.as_ref().map(PwMonitor::rx);
    let mut restart_state = RestartState::default();

    while !sigs.stop_requested() {
        if sigs.take_reload() {
            reload_config(&mut config, &overrides);
        }

        let timer = after(Duration::from_millis(250));
        select! {
            recv(sleep_rx) -> msg => match msg {
                Ok(SleepTransition::Sleeping) => info!("Going to sleep."),
                Ok(SleepTransition::Waking) => handle_resume(&config),
                Err(_) => break,
            },
            recv(sigs.rx) -> msg => match msg {
                Ok(SignalKind::Stop) => break,
                Ok(SignalKind::Reload) => reload_config(&mut config, &overrides),
                Err(_) => {}
            },
            recv(timer) -> _ => {
                drain_socket(&socket, &mut config, &overrides);
                if let Some(rx) = &pw_rx {
                    while let Ok(desc) = rx.try_recv() {
                        if let Err(err) = restart_state.handle_node_error(&desc) {
                            warn!("PipeWire restart failed: {:#}", err);
                        }
                    }
                }
            }
        }
    }

    stop_child(sleep_monitor.child_mut());
    if let Some(monitor) = pw_monitor.as_mut() {
        stop_child(monitor.child_mut());
    }
    cleanup_socket(&socket_path);
    Ok(())
}

pub fn send_command(command: &str) -> Result<()> {
    let client = UnixDatagram::unbound()?;
    client.send_to(command.as_bytes(), socket_path())?;
    Ok(())
}

pub fn decode_command(data: &[u8]) -> Result<Command> {
    let command = std::str::from_utf8(data)?.trim();
    if command == "reload" {
        return Ok(Command::Reload);
    }
    if command == "cycle" || command == "toggle" {
        return Ok(Command::Cycle(None));
    }
    if let Some(profile) = command.strip_prefix("cycle ") {
        return Ok(Command::Cycle(Some(profile.to_string())));
    }
    if let Some(profile) = command.strip_prefix("toggle ") {
        return Ok(Command::Cycle(Some(profile.to_string())));
    }
    anyhow::bail!("Unsupported command: {command}")
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Cycle(Option<String>),
    Reload,
}

fn handle_resume(config: &Config) {
    let card_name = config.card.as_deref().unwrap_or_default();
    let profile = config.profile.as_deref().unwrap_or_default();
    info!(
        "Waking from sleep, waiting {}s for HDMI to renegotiate...",
        config.wake_delay
    );
    thread::sleep(Duration::from_secs_f64(config.wake_delay));
    if let Err(err) = pactl::cycle_profile(card_name, profile) {
        warn!("profile cycle failed: {:#}", err);
    }
}

fn drain_socket(socket: &UnixDatagram, config: &mut Config, overrides: &Overrides) {
    let mut buf = [0_u8; 256];
    loop {
        match socket.recv(&mut buf) {
            Ok(n) => match decode_command(&buf[..n]) {
                Ok(Command::Reload) => reload_config(config, overrides),
                Ok(Command::Cycle(profile)) => {
                    let card_name = config.card.as_deref().unwrap_or_default();
                    let profile = profile
                        .as_deref()
                        .or(config.profile.as_deref())
                        .unwrap_or_default();
                    info!("Received cycle command (profile: {profile})");
                    if let Err(err) = pactl::cycle_profile(card_name, profile) {
                        warn!("profile cycle failed: {:#}", err);
                    }
                }
                Err(err) => warn!("{:#}", err),
            },
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => return,
            Err(err) => {
                warn!("socket read failed: {err}");
                return;
            }
        }
    }
}

fn reload_config(config: &mut Config, overrides: &Overrides) {
    match Config::build(overrides, config::default_conf_path().as_deref()) {
        Ok(new) => {
            if !config.cli_card {
                config.card = new.card;
            }
            if !config.cli_profile {
                config.profile = new.profile;
            }
            info!("Config reloaded");
        }
        Err(err) => warn!(
            "Failed to reload config, keeping current settings: {:#}",
            err
        ),
    }
}

fn socket_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", effective_uid().unwrap_or(0))))
        .join("aproman.sock")
}

fn cleanup_socket(socket_path: &PathBuf) {
    let _ = std::fs::remove_file(socket_path);
}

fn effective_uid() -> Option<u32> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let mut parts = rest.split_whitespace();
            let _real = parts.next()?;
            return parts.next()?.parse().ok();
        }
    }
    None
}

fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

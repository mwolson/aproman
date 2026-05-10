use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use tracing::{info, warn};

use crate::deps;

pub const RESTART_COOLDOWN: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
pub struct RestartState {
    last_restart: Option<Instant>,
}

impl RestartState {
    pub fn handle_node_error(&mut self, desc: &str) -> Result<()> {
        if self
            .last_restart
            .is_some_and(|last| last.elapsed() < RESTART_COOLDOWN)
        {
            info!("Node error detected but within cooldown, skipping restart.");
            return Ok(());
        }
        info!("PipeWire node error detected: {desc}");
        info!("Restarting PipeWire to recover...");
        self.last_restart = Some(Instant::now());
        restart_pipewire()
    }
}

pub fn restart_pipewire() -> Result<()> {
    if deps::which("systemctl").is_some() {
        run(&["systemctl", "--user", "restart", "pipewire.service"])?;
        run(&["systemctl", "--user", "restart", "pipewire-pulse.service"])?;
        return Ok(());
    }
    if deps::which("rc-service").is_some() {
        run_soft(&["rc-service", "--user", "pipewire", "restart"]);
        run_soft(&["rc-service", "pipewire", "restart"]);
        run_soft(&["rc-service", "--user", "pipewire-pulse", "restart"]);
        run_soft(&["rc-service", "pipewire-pulse", "restart"]);
        return Ok(());
    }
    warn!("No service manager found to restart PipeWire. Proceeding with profile cycle only.");
    Ok(())
}

fn run(args: &[&str]) -> Result<()> {
    let status = Command::new(args[0]).args(&args[1..]).status()?;
    if !status.success() {
        bail!("{} failed with {}", args.join(" "), status);
    }
    Ok(())
}

fn run_soft(args: &[&str]) {
    if let Err(err) = run(args) {
        warn!("{} failed: {:#}", args.join(" "), err);
    }
}

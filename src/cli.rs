use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "aproman",
    version,
    about = "Fix HDMI audio after suspend/resume on PipeWire + WirePlumber"
)]
pub struct Cli {
    /// PipeWire/PulseAudio card name
    #[arg(long, value_name = "CARD")]
    pub card: Option<String>,

    /// Desired audio profile
    #[arg(long, value_name = "PROFILE")]
    pub profile: Option<String>,

    /// Seconds to wait after wake before cycling
    #[arg(long = "wake-delay", value_name = "SECONDS", default_value_t = 3.0, value_parser = parse_positive_f64)]
    pub wake_delay: f64,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Cycle the card profile off and back on once
    Cycle,
    /// Print the default card from the config file
    GetDefaultCard,
    /// Print the default profile from the config file
    GetDefaultProfile,
    /// Install and enable the service (systemd or OpenRC)
    InstallService,
    /// List available audio cards
    ListCards,
    /// List available profiles for the card
    ListProfiles,
    /// Set the default card and signal the daemon
    SetDefaultCard { value: String },
    /// Set the default profile and signal the daemon
    SetDefaultProfile { value: String },
    /// Disable and remove the service (systemd or OpenRC)
    UninstallService,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Overrides {
    pub card: Option<String>,
    pub profile: Option<String>,
    pub wake_delay: Option<f64>,
}

pub fn overrides(cli: &Cli) -> Overrides {
    Overrides {
        card: cli.card.clone(),
        profile: cli.profile.clone(),
        wake_delay: Some(cli.wake_delay),
    }
}

fn parse_positive_f64(value: &str) -> Result<f64, String> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| format!("invalid positive number: {value}"))?;
    if parsed <= 0.0 {
        return Err(format!("invalid positive number: {value}"));
    }
    Ok(parsed)
}

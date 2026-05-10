use anyhow::Result;
use aproman::{cli, config, daemon, deps, logging, pactl, service, signals};
use clap::Parser;
use tracing::error;

fn main() {
    logging::init();
    let parsed = cli::Cli::parse();
    if let Err(err) = dispatch(parsed) {
        error!("{:#}", err);
        std::process::exit(1);
    }
}

fn dispatch(parsed: cli::Cli) -> Result<()> {
    match &parsed.command {
        Some(cli::Command::Cycle) => run_cycle(&parsed),
        Some(cli::Command::GetDefaultCard) => config::print_default("card"),
        Some(cli::Command::GetDefaultProfile) => config::print_default("profile"),
        Some(cli::Command::InstallService) => service::install(),
        Some(cli::Command::ListCards) => run_list_cards(&parsed),
        Some(cli::Command::ListProfiles) => run_list_profiles(&parsed),
        Some(cli::Command::SetDefaultCard { value }) => config::set_default("card", value),
        Some(cli::Command::SetDefaultProfile { value }) => config::set_default("profile", value),
        Some(cli::Command::UninstallService) => service::uninstall(),
        None => run_daemon(parsed),
    }
}

fn run_daemon(parsed: cli::Cli) -> Result<()> {
    deps::check_required(&["dbus-monitor", "pactl"])?;
    let overrides = cli::overrides(&parsed);
    let config = config::Config::build(&overrides, config::default_conf_path().as_deref())?;
    let sigs = signals::install()?;
    daemon::run(config, overrides, sigs)
}

fn run_cycle(parsed: &cli::Cli) -> Result<()> {
    deps::check_required(&["pactl"])?;
    let overrides = cli::overrides(parsed);
    let config = config::Config::build(&overrides, config::default_conf_path().as_deref())?;
    let card = pactl::resolve_card(config.card.as_deref())?;
    let profile = pactl::resolve_cycle_profile(&card, config.profile.as_deref())?;
    if daemon::send_command(&format!("cycle {profile}")).is_ok() {
        println!("Sent cycle request to daemon (profile: {profile})");
        return Ok(());
    }
    tracing::warn!("daemon unavailable, running cycle directly");
    pactl::cycle_profile(&card.name, &profile)
}

fn run_list_cards(parsed: &cli::Cli) -> Result<()> {
    deps::check_required(&["pactl"])?;
    let cards = pactl::list_cards()?;
    let selected = parsed
        .card
        .clone()
        .or_else(|| pactl::detect_hdmi_card_name().ok());
    for card in cards {
        let mut parts = vec![card.name.clone()];
        if let Some(label) = card.label {
            parts.push(format!("({label})"));
        }
        if let Some(active) = card.active_profile {
            parts.push(format!("profile: {active}"));
        }
        if card.has_hdmi {
            parts.push("hdmi".to_string());
        }
        if selected.as_deref() == Some(card.name.as_str()) {
            parts.push("*".to_string());
        }
        println!("{}", parts.join("  "));
    }
    Ok(())
}

fn run_list_profiles(parsed: &cli::Cli) -> Result<()> {
    deps::check_required(&["pactl"])?;
    let card = pactl::resolve_card(parsed.card.as_deref())?;
    for profile in card.sorted_profiles() {
        let marker = if Some(profile.name.as_str()) == card.active_profile.as_deref() {
            " *"
        } else {
            ""
        };
        println!(
            "{}  (priority: {}, available: {}){}",
            profile.name, profile.priority, profile.available, marker
        );
    }
    Ok(())
}

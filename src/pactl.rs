use std::cmp::Reverse;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use regex::Regex;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub active_profile: Option<String>,
    pub has_hdmi: bool,
    pub label: Option<String>,
    pub name: String,
    pub profiles: Vec<Profile>,
}

impl Card {
    pub fn sorted_profiles(&self) -> Vec<Profile> {
        let mut profiles = self.profiles.clone();
        profiles.sort_by_key(|profile| Reverse(profile.priority));
        profiles
    }

    pub fn best_available_profile(&self) -> Option<String> {
        self.sorted_profiles()
            .into_iter()
            .filter(|profile| profile.available == "yes" && profile.name != "off")
            .max_by_key(|profile| profile.priority)
            .map(|profile| profile.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub available: String,
    pub name: String,
    pub priority: u32,
}

pub fn resolve_card(explicit: Option<&str>) -> Result<Card> {
    if let Some(name) = explicit {
        return find_card(name)?.with_context(|| format!("Card '{name}' not found"));
    }
    find_hdmi_card()?.context("No HDMI audio card detected. Use --card to specify one manually.")
}

pub fn resolve_cycle_profile(card: &Card, explicit: Option<&str>) -> Result<String> {
    if let Some(profile) = explicit {
        return Ok(profile.to_string());
    }
    match card.active_profile.as_deref() {
        Some(active) if active != "off" => Ok(active.to_string()),
        _ => card
            .best_available_profile()
            .context("card has no available profiles; use --profile to specify one"),
    }
}

pub fn detect_hdmi_card_name() -> Result<String> {
    find_hdmi_card()?
        .map(|card| card.name)
        .context("No HDMI audio card detected. Use --card to specify one manually.")
}

pub fn find_card(name: &str) -> Result<Option<Card>> {
    Ok(list_cards()?.into_iter().find(|card| card.name == name))
}

pub fn find_hdmi_card() -> Result<Option<Card>> {
    Ok(list_cards()?.into_iter().find(|card| card.has_hdmi))
}

pub fn list_cards() -> Result<Vec<Card>> {
    let output = Command::new("pactl")
        .args(["list", "cards"])
        .output()
        .context("running pactl list cards")?;
    if !output.status.success() {
        bail!("pactl list cards failed with {}", output.status);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_cards(&text))
}

pub fn parse_cards(text: &str) -> Vec<Card> {
    let name_re = Regex::new(r#"^\s*Name:\s*(\S+)"#).expect("name regex");
    let active_re = Regex::new(r#"^\s*Active Profile:\s*(.+)"#).expect("active regex");
    let label_re = Regex::new(r#"^\s*alsa\.card_name\s*=\s*"(.+)""#).expect("label regex");
    let profile_re = Regex::new(r#"^\s+(\S+):.*priority:\s*(\d+),\s*available:\s*(yes|no)"#)
        .expect("profile regex");
    let mut cards = Vec::new();
    let mut block = Vec::new();
    for line in text.lines() {
        if line.starts_with("Card #") {
            if !block.is_empty() {
                if let Some(card) =
                    parse_card_block(&block, &name_re, &active_re, &label_re, &profile_re)
                {
                    cards.push(card);
                }
                block.clear();
            }
            continue;
        }
        block.push(line.to_string());
    }
    if !block.is_empty() {
        if let Some(card) = parse_card_block(&block, &name_re, &active_re, &label_re, &profile_re) {
            cards.push(card);
        }
    }
    cards
}

pub fn cycle_profile(card_name: &str, target_profile: &str) -> Result<()> {
    let card = resolve_card(Some(card_name))?;
    let current_profile = card
        .active_profile
        .context("could not determine active profile; skipping cycle")?;
    info!("Cycling profile on {card_name}: {current_profile} -> off -> {target_profile}");
    if !set_card_profile(card_name, "off", 20, Duration::from_millis(250)) {
        warn!("Failed to set profile to 'off'. Will retry next cycle.");
        return Ok(());
    }
    thread::sleep(Duration::from_secs(1));
    if !set_card_profile(card_name, target_profile, 20, Duration::from_millis(250)) {
        warn!("Failed to restore profile to '{target_profile}'. Will retry next cycle.");
        return Ok(());
    }
    info!("Profile restored to {target_profile}.");
    Ok(())
}

pub fn set_card_profile(
    card_name: &str,
    profile: &str,
    attempts: usize,
    retry_delay: Duration,
) -> bool {
    for attempt in 0..attempts {
        let status = Command::new("pactl")
            .args(["set-card-profile", card_name, profile])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if matches!(status, Ok(status) if status.success()) {
            return true;
        }
        if attempt + 1 < attempts {
            thread::sleep(retry_delay);
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn parse_card_block(
    block: &[String],
    name_re: &Regex,
    active_re: &Regex,
    label_re: &Regex,
    profile_re: &Regex,
) -> Option<Card> {
    let mut active_profile = None;
    let mut has_hdmi = false;
    let mut label = None;
    let mut name = None;
    let mut profiles = Vec::new();
    for line in block {
        if let Some(captures) = name_re.captures(line) {
            name = Some(captures[1].to_string());
        }
        if let Some(captures) = active_re.captures(line) {
            active_profile = Some(captures[1].trim().to_string());
        }
        if let Some(captures) = label_re.captures(line) {
            label = Some(captures[1].to_string());
        }
        if line.contains(r#"port.type = "hdmi""#) {
            has_hdmi = true;
        }
        if let Some(captures) = profile_re.captures(line) {
            profiles.push(Profile {
                name: captures[1].to_string(),
                priority: captures[2].parse().unwrap_or(0),
                available: captures[3].to_string(),
            });
        }
    }
    Some(Card {
        active_profile,
        has_hdmi,
        label,
        name: name?,
        profiles,
    })
}

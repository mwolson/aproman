use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;

use crate::cli::Overrides;
use crate::daemon;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub card: Option<String>,
    pub cli_card: bool,
    pub cli_profile: bool,
    pub profile: Option<String>,
    pub wake_delay: f64,
}

impl Config {
    pub fn build(overrides: &Overrides, path: Option<&Path>) -> Result<Self> {
        let mut file = FileConfig::default();
        if let Some(path) = path {
            if path.exists() {
                file = FileConfig::load(path)?;
            }
        }
        Ok(Self {
            card: overrides.card.clone().or(file.card),
            cli_card: overrides.card.is_some(),
            cli_profile: overrides.profile.is_some(),
            profile: overrides.profile.clone().or(file.profile),
            wake_delay: overrides.wake_delay.unwrap_or(3.0),
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct FileConfig {
    card: Option<String>,
    profile: Option<String>,
}

impl FileConfig {
    fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config file {}", path.display()))?;
        let mut result = Self::default();
        for (flag, value) in parse_conf(&text, path)? {
            match flag.as_str() {
                "--card" => result.card = Some(value),
                "--profile" => result.profile = Some(value),
                _ => bail!("unsupported flag '{}' in {}", flag, path.display()),
            }
        }
        Ok(result)
    }
}

pub fn print_default(key: &str) -> Result<()> {
    let path = default_conf_path().context("HOME env var not set")?;
    if !path.exists() {
        bail!(
            "No config file found at {}\n{}",
            path.display(),
            default_hint(key)
        );
    }
    let flag = format!("--{key}");
    for (entry_flag, value) in parse_conf(&std::fs::read_to_string(&path)?, &path)? {
        if entry_flag == flag {
            println!("{value}");
            return Ok(());
        }
    }
    bail!(
        "No {flag} entry found in {}\n{}",
        path.display(),
        default_hint(key)
    )
}

pub fn set_default(key: &str, value: &str) -> Result<()> {
    let path = default_conf_path().context("HOME env var not set")?;
    let flag = format!("--{key}");
    let flag_prefix = format!("{flag}=");
    let mut lines = Vec::new();
    let mut replaced = false;

    if path.exists() {
        let text = std::fs::read_to_string(&path)?;
        for line in text.lines() {
            if line.trim_start().starts_with(&flag_prefix) {
                lines.push(format!("{flag_prefix}{value}"));
                replaced = true;
            } else {
                lines.push(line.to_string());
            }
        }
    }
    if !replaced {
        lines.push(format!("{flag_prefix}{value}"));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, format!("{}\n", lines.join("\n")))?;
    println!("Wrote {flag_prefix}{value} to {}", path.display());
    if daemon::send_command("reload").is_ok() {
        println!("Signaled daemon to reload config");
    }
    Ok(())
}

pub fn default_conf_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("aproman.conf"))
}

pub fn parse_conf(text: &str, path: &Path) -> Result<Vec<(String, String)>> {
    let line_re = Regex::new(r"^(--[a-z][a-z0-9-]*)=(.+)$").expect("conf regex");
    let mut entries = Vec::new();
    for (line_num, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let captures = line_re.captures(line).ok_or_else(|| {
            anyhow!(
                "malformed line {} in {}: {}",
                line_num + 1,
                path.display(),
                line
            )
        })?;
        entries.push((captures[1].to_string(), captures[2].to_string()));
    }
    Ok(entries)
}

fn default_hint(key: &str) -> String {
    let mut parts = Vec::new();
    if key == "card" {
        parts.push("Without a default, aproman auto-detects the first HDMI card.");
    }
    parts.push("Use 'aproman list-cards' or 'aproman list-profiles' to see available options.");
    parts.join("\n")
}

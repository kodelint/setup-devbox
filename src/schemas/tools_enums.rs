use serde::{Deserialize, Serialize};
use std::fmt;
use chrono::Duration;

// ============================================================================
// DURATION PARSING
// ============================================================================

/// A wrapper around `chrono::Duration` that supports custom string serialization (e.g., "7d", "1h").
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct SdbDuration(pub Duration);

impl Default for SdbDuration {
    fn default() -> Self {
        Self(Duration::zero())
    }
}

impl<'de> Deserialize<'de> for SdbDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_duration(&s)
            .map(SdbDuration)
            .map_err(serde::de::Error::custom)
    }
}

impl Serialize for SdbDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let d = self.0;
        let s = if d.num_days() > 0 && d.num_days() * 24 == d.num_hours() {
            format!("{}d", d.num_days())
        } else if d.num_hours() > 0 && d.num_hours() * 60 == d.num_minutes() {
            format!("{}h", d.num_hours())
        } else if d.num_minutes() > 0 && d.num_minutes() * 60 == d.num_seconds() {
            format!("{}m", d.num_minutes())
        } else {
            format!("{}s", d.num_seconds())
        };
        serializer.serialize_str(&s)
    }
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Duration::zero());
    }
    let unit = s.chars().last().ok_or("Missing unit")?;
    let value_str = &s[..s.len() - 1];
    let value: i64 = value_str.parse().map_err(|_| "Invalid number in duration")?;

    match unit {
        's' => Ok(Duration::seconds(value)),
        'm' => Ok(Duration::minutes(value)),
        'h' => Ok(Duration::hours(value)),
        'd' => Ok(Duration::days(value)),
        _ => Err(format!("Unknown duration unit '{unit}'. Use s, m, h, or d.")),
    }
}

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Debug)]
pub enum InstallerError {
    MissingCommand(String),
}

impl fmt::Display for InstallerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand(cmd) => write!(f, "Installer command '{}' not found", cmd),
        }
    }
}

impl std::error::Error for InstallerError {}

#[derive(Debug)]
pub enum ToolEntryError {
    MissingField(&'static str),
}

impl fmt::Display for ToolEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {field}"),
        }
    }
}

impl std::error::Error for ToolEntryError {}

// =========================================================================== //
//                           PROCESSING RESULT TYPES                           //
// =========================================================================== //

#[derive(Debug)]
pub enum ToolProcessingResult {
    Installed,
    Updated,
    ConfigurationUpdated,
    Skipped(String),
    ConfigurationSkipped(String),
    Failed(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ToolAction {
    Install,
    Update,
    UpdateConfigurationOnly,
    Skip(String),
    SkipConfigurationOnly(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum VersionAction {
    Update,
    Skip(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigurationAction {
    Update,
    Skip(String),
}

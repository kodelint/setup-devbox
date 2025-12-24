use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

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
    let value: i64 = value_str
        .parse()
        .map_err(|_| "Invalid number in duration")?;

    match unit {
        's' => Ok(Duration::seconds(value)),
        'm' => Ok(Duration::minutes(value)),
        'h' => Ok(Duration::hours(value)),
        'd' => Ok(Duration::days(value)),
        _ => Err(format!(
            "Unknown duration unit '{unit}'. Use s, m, h, or d."
        )),
    }
}

// ============================================================================
// ENUMS
// ============================================================================

/// Defines the set of valid installation/source methods for tools.
/// Each variant corresponds to a different installation backend or package manager.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Brew,   // Homebrew package manager (macOS/Linux)
    Cargo,  // Rust package manager
    Github, // GitHub releases and repositories
    Go,     // Go language tooling
    Rustup, // Rust toolchain manager
    Url,    // Direct URL downloads
    Uv,     // Python package manager
    Pip,    // Python package installer
}

/// Implementation of string parsing for SourceType enum.
/// Allows converting string arguments to strongly-typed SourceType values.
impl FromStr for SourceType {
    type Err = String;

    /// Parses a string into a SourceType enum variant.
    ///
    /// # Arguments
    /// * `s` - The string to parse (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(SourceType)` if the string matches a valid source type
    /// * `Err(String)` with error message if no match found
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "brew" => Ok(SourceType::Brew),
            "cargo" => Ok(SourceType::Cargo),
            "github" => Ok(SourceType::Github),
            "go" => Ok(SourceType::Go),
            "rustup" => Ok(SourceType::Rustup),
            "url" => Ok(SourceType::Url),
            "uv" => Ok(SourceType::Uv),
            "pip" => Ok(SourceType::Pip),
            _ => {
                let valid_types = [
                    "brew", "cargo", "github", "go", "rustup", "url", "uv", "pip",
                ]
                .join(", ");
                Err(format!(
                    "Invalid source type '{s}'. Must be one of: {valid_types}"
                ))
            }
        }
    }
}

/// Implementation of display formatting for SourceType enum.
/// Provides human-readable string representation for each source type.
impl fmt::Display for SourceType {
    /// Formats the SourceType as a string for display purposes.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SourceType::Brew => write!(f, "brew"),
            SourceType::Cargo => write!(f, "cargo"),
            SourceType::Github => write!(f, "github"),
            SourceType::Go => write!(f, "go"),
            SourceType::Rustup => write!(f, "rustup"),
            SourceType::Url => write!(f, "url"),
            SourceType::Uv => write!(f, "uv"),
            SourceType::Pip => write!(f, "pip"),
        }
    }
}

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Debug, Error)]
pub enum InstallerError {
    #[error("Installer command '{0}' not found")]
    MissingCommand(String),
}

#[derive(Debug, Error)]
pub enum ToolEntryError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
}

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

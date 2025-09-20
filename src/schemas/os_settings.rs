use serde::{Deserialize, Serialize};

/// Configuration schema for `settings.yaml`.
/// Defines the structure for applying system-level settings.
/// This file configures OS-specific system preferences and settings.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsConfig {
    /// OS-specific settings, organized by operating system
    pub settings: OsSpecificSettings,
}

/// Container for operating system-specific settings.
/// Each field contains settings specific to a particular operating system.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OsSpecificSettings {
    /// macOS specific settings (applied using `defaults` command)
    #[serde(default)]
    pub macos: Vec<SettingEntry>,
    // [Future] Other operating systems as needed.
    // Example: linux: Vec<SettingEntry>, windows: Vec<SettingEntry>
}

/// Represents a single system setting to be applied (e.g., via macOS `defaults` command).
/// Each entry defines a specific system preference to configure.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingEntry {
    /// The setting's domain (e.g., "com.apple.finder" for Finder preferences)
    pub domain: String,
    /// The specific key within the domain to modify
    pub key: String,
    /// The value to set for the key
    pub value: String,
    /// Data type of the value (e.g., "bool", "string", "int", "float")
    #[serde(rename = "type")]
    pub value_type: String,
}

//! # Operating System Settings Configuration Schema
//!
//! This module defines the data structures for system-level settings configuration
//! across different operating systems. These structures are used to parse and
//! generate `settings.yaml` configuration files that customize OS-specific
//! preferences and system settings.
//!
//! ## Configuration File Structure
//!
//! The `settings.yaml` file follows this structure:
//! ```yaml
//! settings:
//!   macos:
//!     - domain: "com.apple.finder"
//!       key: "AppleShowAllFiles"
//!       value: "true"
//!       type: "bool"
//!     - domain: "com.apple.dock"
//!       key: "autohide"
//!       value: "true"
//!       type: "bool"
//! ```
//!
//! ## Platform Support
//!
//! Currently, supports macOS settings through the `defaults` command system.
//! The architecture is designed to be extensible for future support of:
//! - Linux (GNOME, KDE, and other desktop environments)
//! - Windows (Registry settings, system preferences)
//! - Other Unix-like systems
use serde::{Deserialize, Serialize};

// ============================================================================
// TOP-LEVEL SETTINGS CONFIGURATION
// ============================================================================

/// Configuration schema for `settings.yaml`.
///
/// Defines the complete structure for applying system-level settings across
/// different operating systems. This file configures OS-specific preferences
/// and system settings using platform-appropriate mechanisms.
///
/// ## File Location and Management
///
/// The state file is typically located at:
/// - `~/.setup-devbox/configs/settings.yaml` (default)
/// - It also supports ENV Variables
///   - `SDB_CONFIG_PATH` -> `$SDB_CONFIG_PATH/configs/settings.yaml`
///
/// ## Platform-Specific Implementation
/// Each operating system uses different mechanisms for applying settings:
/// - **macOS**: `defaults` command for preference domain system
/// - **Linux**: Various methods (gsettings, config files, sysctl)
/// - **Windows**: Registry edits, PowerShell commands
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsConfig {
    /// OS-specific settings, organized by operating system.
    ///
    /// Contains separate vectors of settings for each supported operating system,
    /// enabling platform-specific configuration while maintaining a unified
    /// configuration file format.
    ///
    /// ## Organization Benefits
    /// - Clear separation of platform-specific settings
    /// - Single configuration file for multi-platform environments
    /// - Easy to extend with new operating system support
    /// - Platform-appropriate setting application logic
    pub settings: OsSpecificSettings,
}

// ============================================================================
// OPERATING SYSTEM SPECIFIC SETTINGS
// ============================================================================

/// Container for operating system-specific settings.
///
/// Each field contains settings specific to a particular operating system,
/// enabling a unified configuration format while supporting platform-specific
/// setting mechanisms and preferences.
///
/// ## Extensibility Design
/// The structure is designed to be easily extended with new operating system
/// support by adding new fields for each target platform.
///
/// ## Default Behavior
/// `#[derive(Default)]` ensures that unspecified OS sections are initialized
/// as empty vectors, preventing null pointer issues and simplifying parsing.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OsSpecificSettings {
    /// macOS specific settings (applied using `defaults` command).
    ///
    /// Settings for macOS system preferences, applied using the `defaults`
    /// command-line tool which modifies the macOS preference domain system.
    ///
    /// ## Default Behavior
    /// `#[serde(default)]` ensures that if no macOS settings are specified,
    /// an empty vector is used instead of a null value.
    ///
    /// ## macOS Preference System
    /// macOS uses a domain-based preference system where:
    /// - **Domains**: Typically reverse DNS notation (e.g., "com.apple.finder")
    /// - **Keys**: Specific preference names within the domain
    /// - **Values**: Typed values (boolean, string, integer, float, etc.)
    ///
    /// ## Examples
    /// - Finder preferences: `com.apple.finder` domain
    /// - Dock preferences: `com.apple.dock` domain
    /// - System-wide preferences: `NSGlobalDomain` domain
    ///
    /// ## Safety Notes
    /// macOS settings changes are immediate and can affect system behavior.
    /// Use with caution and test changes in a safe environment first.
    #[serde(default)]
    pub macos: Vec<SettingEntry>,
    // [Future] Other operating systems as needed.
    // Example: linux: Vec<SettingEntry>, windows: Vec<SettingEntry>
    //
    // /// Linux specific settings (applied using appropriate mechanisms).
    // ///
    // /// Settings for Linux desktop environments and system configuration.
    // /// May use various mechanisms depending on the desktop environment:
    // /// - GNOME: `gsettings` command
    // /// - KDE: `kwriteconfig5` or configuration files
    // /// - System-wide: `/etc/sysctl.conf`, `sysctl` command
    // /// - Application-specific: config files in `~/.config`
    // #[serde(default)]
    // pub linux: Vec<SettingEntry>,
    //
    // /// Windows specific settings (applied via Registry or PowerShell).
    // ///
    // /// Settings for Windows system preferences and configuration.
    // /// Typically applied through:
    // /// - Registry edits using `reg` command or PowerShell
    // /// - PowerShell cmdlets for system configuration
    // /// - Group Policy settings (enterprise environments)
    // #[serde(default)]
    // pub windows: Vec<SettingEntry>,
}

// ============================================================================
// INDIVIDUAL SETTING ENTRIES
// ============================================================================

/// Represents a single system setting to be applied (e.g., via macOS `defaults` command).
///
/// Each entry defines a specific system preference to configure, including
/// the domain, key, value, and data type information needed to properly
/// apply the setting using platform-specific mechanisms.
///
/// ## Validation Requirements
/// Settings are validated before application to ensure:
/// - Value types match the specified type field
/// - Domains and keys exist on the target system
/// - Values are within acceptable ranges for the setting
/// - The setting is appropriate for the current OS version
///
/// ## Examples
/// ```yaml
/// settings:
///   ## Settings specifically for macOS
///   macos:
///     # Show hidden files in Finder
///     - domain: com.apple.finder
///       key: AppleShowAllFiles
///       value: "true"
///       type: bool
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingEntry {
    /// The setting's domain (e.g., "com.apple.finder" for Finder preferences).
    ///
    /// The preference domain namespace that contains the specific key to modify.
    /// Domains typically follow reverse DNS notation for organization-specific
    /// preferences and standard domains for system components.
    ///
    /// ## Common macOS Domains
    /// - `"com.apple.finder"`: Finder file manager preferences
    /// - `"com.apple.dock"`: Dock application launcher preferences
    /// - `"com.apple.menuextra.clock"`: Menu bar clock preferences
    /// - `"NSGlobalDomain"`: System-wide global preferences
    /// - `"com.apple.universalaccess"`: Accessibility preferences
    ///
    /// ## Domain Discovery
    /// Existing domains and keys can be discovered using:
    /// - `defaults domains` (list all domains)
    /// - `defaults read <domain>` (read all keys in domain)
    /// - `defaults read <domain> <key>` (read specific key value)
    pub domain: String,

    /// The specific key within the domain to modify.
    ///
    /// The individual preference setting name within the specified domain.
    /// Keys are specific to each domain and control particular aspects of
    /// system or application behavior.
    ///
    /// ## Common macOS Keys
    /// - `"AppleShowAllFiles"`: Show hidden files in Finder (bool)
    /// - `"autohide"`: Auto-hide the Dock (bool)
    /// - `"ShowSeconds"`: Show seconds in menu bar clock (bool)
    /// - `"AppleHighlightColor"`: Finder highlight color (string)
    /// - `"NSToolbarTitleViewRolloverDelay"`: Toolbar delay (float)
    ///
    /// ## Key Validation
    /// The system attempts to validate that specified keys exist in the
    /// domain before applying changes to prevent configuration errors.
    pub key: String,

    /// The value to set for the key.
    ///
    /// The new value to apply to the specified key in the domain.
    /// The value is provided as a string but will be converted to the
    /// appropriate type based on the `value_type` field.
    ///
    /// ## Value Formatting
    /// Values must be formatted appropriately for their type:
    /// - **bool**: `"true"`, `"false"` (case-insensitive)
    /// - **string**: Any string value, quoted if containing spaces
    /// - **int**: Integer numbers (`"0"`, `"1"`, `"42"`)
    /// - **float**: Floating-point numbers (`"0.5"`, `"1.0"`, `"3.14"`)
    ///
    /// ## Examples by Type
    /// - Boolean: `"true"`, `"false"`, `"YES"`, `"NO"`
    /// - String: `"Blue"`, `"RGB(0,0,255)"`, `"~/Documents"`
    /// - Integer: `"0"`, `"1"`, `"100"`, `"-1"`
    /// - Float: `"0.5"`, `"1.0"`, `"2.718"`, `"3.14159"`
    pub value: String,

    /// Data type of the value (e.g., "bool", "string", "int", "float").
    ///
    /// Determines how the string `value` should be interpreted and converted
    /// when applying the setting to the system. This ensures type-safe
    /// setting application and prevents type mismatch errors.
    ///
    /// ## Supported Types
    /// - `"bool"`: Boolean values (true/false)
    /// - `"string"`: String values (text)
    /// - `"int"`: Integer numbers (whole numbers)
    /// - `"float"`: Floating-point numbers (decimal numbers)
    ///
    /// ## Serialization Note
    /// `#[serde(rename = "type")]` allows using the reserved word `type` in
    /// YAML/JSON while using `value_type` as the Rust field name, avoiding
    /// Rust keyword conflicts while maintaining clean configuration syntax.
    ///
    /// ## Type Validation
    /// The system validates that the string `value` can be properly converted
    /// to the specified type before attempting to apply the setting.
    #[serde(rename = "type")]
    pub value_type: String,
}

//! # Configuration Management System
//!
//! This module defines the data structures and components for managing tool configuration files.
//! It provides a system for automatically synchronizing configuration files between a centralized
//! source location and individual tool configuration destinations, enabling consistent tool
//! configuration across different environments and machines.
//!
//! ## Core Functionality
//!
//! The configuration management system handles:
//! - **Source File Discovery**: Finding configuration source files in designated directories
//! - **Format Conversion**: Converting TOML source files to various target formats (JSON, YAML, TOML, KEY=VALUE)
//! - **Change Detection**: Using SHA-256 hashes to detect changes and avoid unnecessary updates
//! - **Path Expansion**: Supporting `~`, `$HOME`, and other environment variables in paths
//! - **State Tracking**: Remembering file hashes to optimize future update checks
//!
//! ## Configuration Source Hierarchy
//!
//! The system searches for source configuration files in this priority order:
//! 1. `SDB_TOOLS_SOURCE_CONFIG_PATH` environment variable and looks for `{tool_name}.toml`
//! 2. `SDB_CONFIG_PATH` environment variable and builds `$SDB_CONFIG_PATH/configs/tools`
//! 3. `~/.setup-devbox/configs/tools` (default and fallback)
//!
//! ## Output Format Detection
//!
//! Target format is determined by the file extension in `tools_configuration_path`:
//! - `.json` → Pretty-printed JSON
//! - `.yaml` or `.yml` → YAML format
//! - `.toml` → Pretty-printed TOML
//! - Other extensions → KEY=VALUE pairs with smart quoting

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// CONFIGURATION MANAGEMENT SETTINGS
// ============================================================================

/// Configuration settings for tool configuration file management.
///
/// This struct defines whether configuration management is enabled for a tool
/// and specifies the path where the tool's configuration should be stored.
/// It is used in tool definitions to enable automatic configuration synchronization.
///
/// ## Usage in Tool Definitions
/// ```yaml
/// - name: "starship"
///   source: "brew"
///   configuration_manager:
///     enabled: true
///     tools_configuration_path: "~/.config/starship/starship.toml"
/// ```
///
/// ## Enablement Behavior
/// When `enabled` is `true`, the system will automatically manage the configuration
/// file at the specified path, synchronizing it with the source configuration.
/// When `false`, configuration management is disabled for this tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationManager {
    /// Whether configuration management is enabled for this tool.
    ///
    /// When `true`, the system will automatically synchronize the tool's
    /// configuration file with the source configuration. When `false`,
    /// configuration management is disabled and no files will be managed.
    ///
    /// ## Default Behavior
    /// Typically defaults to `false` in tool definitions, requiring explicit
    /// enablement for configuration management.
    pub enabled: bool,

    /// File system path where the tool's configuration should be stored.
    ///
    /// This path specifies the destination for the synchronized configuration file.
    /// The path supports environment variable expansion and tilde expansion for
    /// user home directory references.
    ///
    /// ## Path Expansion Support
    /// - `~` expands to the user's home directory
    /// - `$HOME` expands to the user's home directory
    /// - Other environment variables are expanded if defined
    ///
    /// ## Format Detection
    /// The file extension determines the output format:
    /// - `.toml` → TOML format (pretty-printed)
    /// - `.json` → JSON format (pretty-printed)
    /// - `.yaml`/`.yml` → YAML format
    /// - Other → KEY=VALUE format with smart quoting
    ///
    /// ## Examples
    /// - `"~/.config/starship/starship.toml"`
    /// - `"$HOME/.config/helix/config.toml"`
    /// - `"/etc/myapp/config.json"`
    /// - `"./local-config.yaml"`
    pub tools_configuration_path: String,
}

// ============================================================================
// CONFIGURATION MANAGEMENT STATE
// ============================================================================

/// Persistent state information for configuration management.
///
/// This struct tracks the current state of configuration file synchronization
/// for each tool, including SHA-256 hashes of both source and destination files.
/// It is stored in the global state file to enable change detection and avoid
/// unnecessary file operations.
///
/// ## Change Detection Logic
/// Configuration updates are triggered when:
/// - Source file content changes (SHA-256 mismatch)
/// - Destination file is missing or was deleted
/// - Destination file was modified externally (SHA-256 mismatch)
/// - Configuration path in tool definition changes
/// - Configuration management is newly enabled for a tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationManagerState {
    /// Whether configuration management is currently enabled for this tool.
    ///
    /// This mirrors the `enabled` field from `ConfigurationManager` but is
    /// stored in the state to track the current management status.
    pub enabled: bool,

    /// The configured path where the tool's configuration is stored.
    ///
    /// This is the expanded and resolved path after environment variable
    /// and tilde expansion. Stored to detect path changes that would
    /// require re-synchronization.
    pub tools_configuration_path: String,

    /// SHA-256 hash of the source configuration file content.
    ///
    /// Used to detect changes in the source configuration file. When the
    /// source file changes (different SHA-256), the destination file will
    /// be updated to match the new source content.
    ///
    /// ## Hash Calculation
    /// The hash is computed from the raw file content after reading from
    /// the source location but before any format conversion.
    pub source_configuration_sha: String,

    /// SHA-256 hash of the destination configuration file content.
    ///
    /// Used to detect external modifications to the destination file.
    /// If the destination file's hash doesn't match what's expected
    /// (indicating manual edits), the system may need to handle conflicts
    /// or re-synchronize the file.
    ///
    /// ## Conflict Detection
    /// Mismatches between expected and actual destination hashes may
    /// indicate that the user manually edited the file, requiring
    /// special handling to avoid overwriting user changes.
    pub destination_configuration_sha: String,
}

// ============================================================================
// CONFIGURATION PROCESSOR
// ============================================================================

/// Processor responsible for managing configuration file synchronization.
///
/// This struct handles the actual work of discovering source configuration files,
/// converting formats, and synchronizing them to their destination paths.
/// It maintains the base path for configuration source files and provides
/// methods for processing configuration updates.
///
/// ## Responsibilities
/// - Discovering source configuration files in configured directories
/// - Reading and parsing source configuration files
/// - Converting between configuration formats (TOML → JSON/YAML/KEY=VALUE)
/// - Writing configuration files to their destination paths
/// - Calculating and verifying SHA-256 hashes for change detection
/// - Handling environment variable expansion in paths
/// - Managing file permissions and ownership
pub struct ConfigurationManagerProcessor {
    /// Base directory path for searching source configuration files.
    ///
    /// This path is the root directory where the system looks for tool
    /// configuration source files. Source files are expected to be named
    /// `{tool_name}.toml` within this directory or its subdirectories.
    ///
    /// ## Search Hierarchy
    /// The processor searches for source files in this order:
    /// 1. Environment-specific configured paths
    /// 2. The `config_base_path` specified here
    /// 3. Default fallback locations
    ///
    /// ## Typical Locations
    /// - `~/.setup-devbox/configs/tools/` (user-specific)
    /// - `/etc/setup-devbox/configs/tools/` (system-wide)
    /// - `./.setup-devbox/configs/tools/` (project-specific)
    pub(crate) config_base_path: PathBuf,
}

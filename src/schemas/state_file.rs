//! # Application State File Schema (`state.json`)
//!
//! This module defines the structure and serialization format for `setup-devbox`'s internal
//! state file, which tracks installed tools, applied system settings, and installed fonts.
//! The state file enables the system to manage updates, reinstallations, and cleanup operations
//! by maintaining a persistent record of what has been installed and configured.
//!
//! ## File Location and Management
//!
//! The state file is typically located at:
//! - `~/.setup-devbox/state.json` (user-specific state)
//! - It also supports ENV Variables
//!   - `SDB_CONFIG_PATH` -> `$SDB_CONFIG_PATH/state.json`
//!   - `SDB_STATE_FILE_PATH` -> `$SDB_STATE_FILE_PATH/state.json`
//!
//! ## Automatic Management
//!
//! This file is automatically managed by the application and should not be manually edited.
//! Manual modifications may cause inconsistencies in tool management and update detection.
//!
//! ## State File Purpose
//!
//! The state file serves as the system's "persistent memory" by tracking:
//! - **Installed Tools**: Version, installation method, paths, and configuration status
//! - **System Settings**: Applied configuration changes and their current values
//! - **Installed Fonts**: Font families, files, and source information
//!
//! ## Update Detection Logic
//!
//! The state file enables intelligent update detection by comparing:
//! - Current tool configurations vs. previously installed versions
//! - Configuration file changes vs. stored SHA-256 hashes
//! - System setting current values vs. desired states
//!
//! ## Data Integrity
//!
//! - Uses SHA-256 hashing for configuration file change detection
//! - Maintains timestamps for update frequency control
//! - Tracks installation sources for proper update methods
//! - Stores original parameters for reinstallation scenarios

use crate::libs::configuration_manager::ConfigurationManagerState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// TOP-LEVEL STATE STRUCTURE
// ============================================================================

/// The complete structure of `state.json`, representing `setup-devbox`'s persistent memory.
///
/// This structure encapsulates all state information tracked by the application,
/// enabling comprehensive management of the development environment across sessions.
/// It serves as the authoritative source for what tools, settings, and fonts are
/// currently installed and configured.
///
/// ## Serialization Format
/// The state is serialized as JSON with pretty-printing for human readability
/// while maintaining machine-parsable structure for programmatic access.
///
/// ## State Persistence
/// The state is automatically saved to disk after successful operations and
/// loaded at application startup to resume management from the previous state.
///
/// ## Example State File
/// ```json
/// {
///   "tools": {
///     "starship": {
///       "version": "1.17.1",
///       "install_path": "/usr/local/bin/starship",
///       "installed_by_devbox": true,
///       "install_method": "github-release",
///       "package_type": "binary",
///       "repo": "starship/starship",
///       "tag": "v1.17.1",
///       "last_updated": "2024-01-15T10:30:45Z",
///       "configuration_manager": {
///         "enabled": true,
///         "tools_configuration_path": "$HOME/.config/starship.toml",
///         "source_configuration_sha": "58b78d994e8...",
///         "destination_configuration_sha": "4e3a615841a..."
///       }
///     }
///   },
///   "settings": {
///     "com.apple.finder:AppleShowAllFiles": {
///       "domain": "com.apple.finder",
///       "key": "AppleShowAllFiles",
///       "value": "true",
///       "value_type": "bool"
///     }
///   },
///   "fonts": {
///     "Fira Code": {
///       "name": "Fira Code",
///       "url": "https://github.com/tonsky/FiraCode/releases/download/6.2/Fira_Code_v6.2.zip",
///       "files": ["FiraCode-Regular.ttf", "FiraCode-Bold.ttf"],
///       "version": "6.2"
///     }
///   }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DevBoxState {
    /// Records information about installed tools, keyed by tool name.
    ///
    /// This map tracks all tools managed by `setup-devbox`, including:
    /// - Installation details (method, path, version)
    /// - Source information (GitHub repo, tags, URLs)
    /// - Configuration management state
    /// - Update timestamps and installation metadata
    ///
    /// ## Key Structure
    /// Uses the tool's canonical name (as specified in the configuration)
    /// regardless of any `rename_to` operations during installation.
    pub tools: HashMap<String, ToolState>,

    /// Records applied system settings, keyed by setting domain:key combination.
    ///
    /// This map tracks system configuration changes made by the application,
    /// enabling management of preferences, defaults, and system-wide settings.
    ///
    /// ## Key Format
    /// Uses `"{domain}:{key}"` format for unique identification of settings.
    /// Example: `"com.apple.finder:AppleShowAllFiles"`
    pub settings: HashMap<String, SettingState>,

    /// Stores information about installed fonts, keyed by font name.
    ///
    /// This map tracks fonts installed through the system, including:
    /// - Font family names and file lists
    /// - Source URLs and version information
    /// - GitHub repository details for font packages
    pub fonts: HashMap<String, FontState>,
}

// ============================================================================
// TOOL STATE MANAGEMENT
// ============================================================================

/// Stores detailed information about each installed tool.
///
/// This information is used for updates, re-installs, and cleanup operations.
/// It provides the necessary context to determine what actions are needed
/// when the configuration changes or updates are available.
///
/// ## Version Management
/// The `version` field is critical for update detection - it stores the
/// exact version that was installed, enabling comparison with desired versions.
///
/// ## Installation Source Tracking
/// Source information (`repo`, `tag`, `url`) ensures that updates use the
/// same source and method as the original installation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolState {
    /// Exact version of the installed tool.
    ///
    /// This should match the version that was actually installed, which may
    /// differ from the requested version if version constraints or "latest"
    /// was specified.
    ///
    /// ## Format
    /// Version string in the format provided by the installer or source.
    /// Examples: "1.2.3", "v1.2.3", "2024.01.15"
    pub version: String,

    /// File system path where the tool is installed.
    ///
    /// This is the full path to the executable or installation directory.
    /// Used for verification, updates, and potential uninstallation.
    ///
    /// ## Path Examples
    /// - `/usr/local/bin/tool-name` (brew installations)
    /// - `$HOME/.cargo/bin/tool-name` (cargo installations)
    /// - `/opt/tool-name/tool-name` (manual installations)
    pub install_path: String,

    /// True if managed by `setup-devbox` (vs pre-installed or manually installed).
    ///
    /// This flag distinguishes between tools that were installed by this system
    /// and tools that were already present on the system. It affects update
    /// behavior and potential cleanup operations.
    ///
    /// ## Behavior
    /// - `true`: Tool was installed by this system and can be managed/updated
    /// - `false`: Tool was pre-existing; only configuration may be managed
    pub installed_by_devbox: bool,

    /// Method used for installation (e.g., "github-release", "brew", "cargo").
    ///
    /// Determines how updates and reinstallations should be performed.
    /// Must match one of the supported installation methods.
    ///
    /// ## Supported Methods
    /// - `"github-release"`: GitHub release downloads
    /// - `"brew"`: Homebrew package manager
    /// - `"cargo"`: Rust cargo installations
    /// - `"go"`: Go module installations
    /// - `"url"`: Direct URL downloads
    /// - `"pip"`: Python pip installations
    /// - `"uv"`: UV Python package manager
    pub install_method: String,

    /// New name if the executable was renamed during installation.
    ///
    /// When a tool is installed with a `rename_to` option, this field stores
    /// the new name while the original tool name remains the map key.
    ///
    /// ## Example
    /// Original tool name: "helix-editor"
    /// `rename_to`: "hx"
    /// Key: "helix-editor", `renamed_to`: Some("hx")
    pub renamed_to: Option<String>,

    /// Type of package (e.g., "binary", "go-module", "rust-binary").
    ///
    /// Categorizes the installation type for proper update and management logic.
    /// Different package types may require different update strategies.
    ///
    /// ## Common Types
    /// - `"binary"`: Pre-compiled executable
    /// - `"go-module"`: Go language module
    /// - `"rust-binary"`: Rust compiled binary
    /// - `"python-package"`: Python package
    pub package_type: String,

    /// GitHub repository if applicable (for GitHub source installations).
    ///
    /// Format: `"owner/repository"`
    /// Used for checking updates and re-downloading from the same source.
    ///
    /// ## Examples
    /// - `"starship/starship"`
    /// - `"helix-editor/helix"`
    /// - `"sharkdp/bat"`
    pub repo: Option<String>,

    /// Specific GitHub tag/release if applicable (for GitHub source installations).
    ///
    /// The exact release tag that was downloaded and installed.
    /// Used for version tracking and update detection.
    ///
    /// ## Format
    /// Typically starts with "v": `"v1.2.3"`, `"v23.10"`
    /// May also be commit hashes or other release identifiers.
    pub tag: Option<String>,

    /// Options passed to the installer during installation.
    ///
    /// Stores the original installation options for consistent reinstallation
    /// and update behavior. Options are source-specific.
    ///
    /// ## Examples by Source
    /// - **Brew**: `["--HEAD"]` for development versions
    /// - **Cargo**: `["--locked"]` for exact dependency versions
    /// - **Pip**: `["--user"]` for user-local installation
    ///
    /// ## Default Behavior
    /// `#[serde(default)]` ensures empty options are stored as `None`.
    #[serde(default)]
    pub options: Option<Vec<String>>,

    /// Original download URL for direct URL installations.
    ///
    /// Used for re-downloading and updating tools installed from direct URLs.
    /// Must be a valid HTTP/HTTPS URL.
    ///
    /// ## URL Types
    /// - Direct binary downloads
    /// - Archive downloads (.tar.gz, .zip)
    /// - Installation scripts
    ///
    /// ## Default Behavior
    /// `#[serde(default)]` ensures empty URLs are stored as `None`.
    #[serde(default)]
    pub url: Option<String>,

    /// Timestamp of when the tool was last updated.
    ///
    /// Used for update frequency control, particularly for tools with
    /// `version: "latest"` configurations.
    ///
    /// ## Format
    /// ISO 8601 timestamp: `"2024-01-15T10:30:45Z"`
    /// Timezone should be UTC for consistency.
    pub last_updated: Option<String>,

    /// Path to executable within extracted archive, relative to `install_path`.
    ///
    /// For archive-based installations, this specifies where the actual
    /// executable is located within the extracted contents.
    ///
    /// ## Examples
    /// - `"bin/tool-name"` (executable in bin subdirectory)
    /// - `"tool-name"` (executable at archive root)
    /// - `"release/x86_64-apple-darwin/tool"` (platform-specific binary)
    pub executable_path_after_extract: Option<String>,

    /// Records any additional commands that were executed during installation.
    ///
    /// This is stored for reference and potential cleanup/uninstall operations.
    /// Commands are executed in the order specified during installation.
    ///
    /// ## Use Cases
    /// - Post-installation setup scripts
    /// - Configuration file copying
    /// - Directory creation and permission setting
    /// - Symlink creation
    ///
    /// ## Default Behavior
    /// `#[serde(default)]` ensures empty command lists are stored as `None`.
    #[serde(default)]
    pub executed_post_installation_hooks: Option<Vec<String>>,

    /// Configuration management state for this tool.
    ///
    /// Tracks the status of configuration file synchronization, including
    /// file hashes, paths, and synchronization timestamps.
    ///
    /// ## Serialization Behavior
    /// `#[serde(skip_serializing_if = "Option::is_none")]` omits this field
    /// from serialization when `None`, reducing state file size for tools
    /// without configuration management.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_manager: Option<ConfigurationManagerState>,
}

// ============================================================================
// SYSTEM SETTINGS STATE
// ============================================================================

/// Records the state of an applied system setting.
///
/// This information is used to track what settings have been configured
/// and their current values, enabling management and potential reversion.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingState {
    /// Setting's domain (e.g., "com.apple.finder").
    ///
    /// The namespace or application domain for the setting.
    /// Typically, follows reverse DNS notation for system settings.
    ///
    /// ## Examples
    /// - `"com.apple.finder"` (macOS Finder settings)
    /// - `"com.apple.dock"` (macOS Dock settings)
    /// - `"org.gnome.desktop.interface"` (GNOME desktop settings)
    pub domain: String,

    /// Setting's key within the domain.
    ///
    /// The specific setting name within the domain namespace.
    ///
    /// ## Examples
    /// - `"AppleShowAllFiles"` (show hidden files in Finder)
    /// - `"autohide"` (auto-hide the Dock)
    /// - `"clock-format"` (clock format in GNOME)
    pub key: String,

    /// Value that was set for this setting.
    ///
    /// The current value that was applied to the system.
    /// Stored as a string but represents various data types.
    ///
    /// ## Value Interpretation
    /// The actual data type is determined by `value_type` field.
    /// String representation enables consistent serialization.
    pub value: String,

    /// Type of the value (e.g., "bool", "string").
    ///
    /// Determines how the `value` field should be interpreted and
    /// how the setting should be applied to the system.
    ///
    /// ## Supported Types
    /// - `"bool"`: Boolean values (`"true"`, `"false"`)
    /// - `"string"`: String values
    /// - `"int"`: Integer values
    /// - `"float"`: Floating-point values
    pub value_type: String,
}

// ============================================================================
// FONT MANAGEMENT STATE
// ============================================================================

/// Records the state of an installed font.
///
/// This information is used for font management and updates,
/// ensuring that fonts can be properly maintained and tracked.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontState {
    /// Name of the font family.
    ///
    /// The canonical name of the font as used by applications and the system.
    ///
    /// ## Examples
    /// - `"Fira Code"`
    /// - `"JetBrains Mono"`
    /// - `"Source Code Pro"`
    pub name: String,

    /// Original download URL for the font.
    ///
    /// Used for re-downloading and updating fonts when new versions are available.
    /// Typically points to direct download links or GitHub release assets.
    pub url: String,

    /// List of installed font files (filenames).
    ///
    /// The actual font files that were installed and registered with the system.
    /// Used for cleanup, verification, and management operations.
    ///
    /// ## File Locations
    /// Font files are typically installed to system or user font directories:
    /// - macOS: `~/Library/Fonts/` or `/Library/Fonts/`
    /// - Linux: `~/.local/share/fonts/` or `/usr/share/fonts/`
    pub files: Vec<String>,

    /// Installed version of the font.
    ///
    /// The version string for the font package, used for update detection
    /// and version management.
    ///
    /// ## Format
    /// Typically follows semantic versioning: `"1.2.3"`, `"6.2"`
    pub version: String,

    /// GitHub repository if applicable (for GitHub source installations).
    ///
    /// Format: `"owner/repository"`
    /// Used for checking font updates from GitHub releases.
    ///
    /// ## Default Behavior
    /// `#[serde(default)]` ensures empty values are stored as `None`.
    #[serde(default)]
    pub repo: Option<String>,

    /// Specific GitHub tag/release if applicable (for GitHub source installations).
    ///
    /// The exact release tag that was downloaded for the font installation.
    /// Used for version tracking and update detection.
    ///
    /// ## Default Behavior
    /// `#[serde(default)]` ensures empty values are stored as `None`.
    #[serde(default)]
    pub tag: Option<String>,
}

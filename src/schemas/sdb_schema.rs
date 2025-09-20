// Defines the data structures (schemas) for configuration files and the application's internal state.
// Serde traits for serialization and deserialization.
use serde::{Deserialize, Serialize};
// Used for key-value pair storage.
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Represents a downloadable asset attached to a GitHub release.
/// This is used when parsing GitHub API responses for release assets.
#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    // The asset's filename (e.g., "tool-v1.0.0-linux-x86_64.tar.gz").
    pub(crate) name: String,
    // Direct download URL for the asset.
    pub(crate) browser_download_url: String,
}

/// Captures details of a single GitHub release, including its assets.
/// This is used when fetching release information from GitHub API.
#[derive(Debug, Deserialize)]
pub struct Release {
    // List of downloadable assets for this release.
    pub(crate) assets: Vec<ReleaseAsset>,
}

// User Configuration File Schemas (e.g., for YAML files)
// Defines the structure for user-defined configuration files.

/// Errors related to external installer command availability.
/// These errors occur when required command-line tools are not found in the system PATH.
#[derive(Debug)]
pub enum InstallerError {
    /// Indicates a required command-line tool was not found in PATH.
    MissingCommand(String),
}

/// Custom error types for schema validation during tool entry processing.
/// These errors help users identify configuration issues in their tools.yaml file.
#[derive(Debug)]
pub enum ToolEntryError {
    /// Required field is missing from the tool configuration
    MissingField(&'static str),
    /// Invalid source type specified for the tool
    InvalidSource(String),
    /// Conflicting fields that shouldn't be used together for a given source type
    ConflictingFields(String),
}

/// Defines the possible installation methods for a tool.
/// Each variant represents a different package manager or installation source.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ToolInstallMethod {
    Brew,   // Homebrew package manager (macOS)
    Cargo,  // Rust package manager
    Go,     // Go language tools
    GitHub, // GitHub releases
    Url,    // Direct URL download
    Rustup, // Rust toolchain installer
    Pip,    // Python package installer
    System, // For tools managed externally or expected to be pre-installed.
}

/// Configuration schema for `tools.yaml`.
/// Defines the top-level structure for managing software tools.
/// This is the main configuration file for tool installation management.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Optional setting to control when to update tools to latest versions
    pub update_latest_only_after: Option<String>,
    /// List of individual tool entries to be installed/managed
    pub tools: Vec<ToolEntry>,
}

/// Represents a single tool entry defined by the user in `tools.yaml`.
/// Each entry defines how a specific tool should be installed and configured.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolEntry {
    /// Name of the tool (used for identification and state tracking)
    pub name: String,
    /// Desired version of the tool (optional - latest used if not specified)
    pub version: Option<String>,
    /// Installation source (e.g., "GitHub", "brew", "cargo", "url")
    pub source: String,
    /// Direct download URL (required if source is "url")
    pub url: Option<String>,
    /// GitHub repository in "owner/repo_name" format (required for GitHub source)
    pub repo: Option<String>,
    /// Specific GitHub release tag to download from (required for github source)
    pub tag: Option<String>,
    /// Optional new name for the executable after installation (useful for renaming binaries)
    pub rename_to: Option<String>,
    /// Additional installer-specific options/flags
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// Path to executable within extracted archive (relative to extraction directory)
    #[serde(default)]
    pub executable_path_after_extract: Option<String>,

    /// Additional commands to execute after successful tool installation.
    /// These commands are executed in the context of the extracted archive directory
    /// (for archive-based installations) or download directory (for binary installations).
    ///
    /// Commands can reference:
    /// - Environment variables like $HOME, $USER, etc.
    /// - Relative paths within the extracted/downloaded content
    /// - Shell built-ins and system commands
    ///
    /// Example usage:
    /// ```yaml
    /// additional_cmd:
    ///   - cp -r runtime $HOME/.config/helix/
    ///   - mkdir -p $HOME/.local/share/helix
    ///   - ln -sf $(pwd)/themes $HOME/.config/helix/themes
    /// ```
    ///
    /// Important notes:
    /// - Commands are executed sequentially in the order specified
    /// - If any command fails, the entire installation is considered failed
    /// - Commands are executed with the working directory set to the extraction/download location
    /// - Use absolute paths when referencing locations outside the tool's directory
    #[serde(default)]
    pub additional_cmd: Option<Vec<String>>,
    #[serde(default)]
    pub configuration_manager: ConfigurationManager,
}

/// Represents Shell run commands configuration with section-based organization
/// This structure defines how shell commands should be organized in the RC file
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellRunCommands {
    /// Type of shell (e.g., "bash", "zsh") - determines which RC file to use
    pub shell: String,
    /// List of run command entries organized by section
    pub run_commands: Vec<RunCommandEntry>,
}

/// Represents a single run command entry with section information
/// Each entry defines a shell command and which section it belongs to
#[derive(Debug, Serialize, Deserialize)]
pub struct RunCommandEntry {
    /// The actual shell command to be added to the RC file
    pub command: String,
    /// Which section this command belongs to (organizes commands in the RC file)
    pub section: ConfigSection,
}

/// Configuration schema for `shellac.yaml`.
/// Defines the structure for shell environment customization (shell run commands and aliases).
/// This file configures the user's shell environment with custom commands and aliases.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Configuration for shell run commands (exports, evals, paths, etc.)
    pub run_commands: ShellRunCommands,
    /// List of shell aliases to be created
    pub aliases: Vec<AliasEntry>,
}

/// Defines the possible sections for organizing shell run commands
/// Each section groups related types of shell commands for better organization
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ConfigSection {
    Exports,   // Environment variable exports (export VAR=value)
    Aliases,   // Shell command aliases (alias ll='ls -la')
    Evals,     // Commands that need to be evaluated (eval "$(starship init bash)")
    Functions, // Shell function definitions
    Paths,     // PATH modifications (export PATH=$PATH:/new/path)
    Other,     // Miscellaneous commands that don't fit other categories
}

/// Represents a single command alias entry in `shellac.yaml`.
/// Defines a shell alias that maps a short name to a longer command
#[derive(Debug, Serialize, Deserialize)]
pub struct AliasEntry {
    /// The alias name (what the user types in the shell)
    pub name: String,
    /// The command the alias expands to (what gets executed)
    pub value: String,
}

/// Configuration schema for `fonts.yaml`.
/// Defines the structure for managing and installing custom fonts.
/// This file configures font installation from various sources.
#[derive(Debug, Serialize, Deserialize)]
pub struct FontConfig {
    /// List of individual font entries to be installed
    pub fonts: Vec<FontEntry>,
}

/// Represents a single font entry defined by the user in `fonts.yaml`.
/// Each entry defines how a specific font should be downloaded and installed.
#[derive(Debug, Serialize, Deserialize)]
pub struct FontEntry {
    /// Name of the font (for identification and state tracking)
    pub name: String,
    /// Desired font version (optional)
    pub version: Option<String>,
    /// Source for the font (e.g., "GitHub", "nerd-fonts")
    pub source: String,
    /// GitHub repository for the font (if source is GitHub)
    pub repo: Option<String>,
    /// Specific GitHub tag/release (if source is GitHub)
    pub tag: Option<String>,
    /// Optional list of keywords for filtering specific font files to install
    /// This allows installing only certain font weights/styles from a font family
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_only: Option<Vec<String>>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationManager {
    pub enabled: bool,
    pub tools_configuration_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationManagerState {
    pub enabled: bool,
    pub tools_configuration_path: String,
    pub source_configuration_sha: String,
    pub destination_configuration_sha: String,
}

pub struct ConfigurationManagerProcessor {
    pub(crate) config_base_path: PathBuf,
}

// Application State File Schema (`state.json`)
// Defines the structure of `setup-devbox`'s internal state file,
// used to track installed tools and applied configurations.
// This file is automatically managed by the application and should not be manually edited.

/// The complete structure of `state.json`, representing `setup-devbox`'s persistent memory.
/// This file tracks what has been installed and configured to enable updates and management.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DevBoxState {
    /// Records information about installed tools (keyed by tool name)
    pub tools: HashMap<String, ToolState>,
    /// Records applied system settings (keyed by setting domain:key)
    pub settings: HashMap<String, SettingState>,
    /// Stores information about installed fonts (keyed by font name)
    pub fonts: HashMap<String, FontState>,
}

/// Stores detailed information about each installed tool.
/// This information is used for updates, reinstalls, and cleanup operations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolState {
    /// Exact version of the installed tool
    pub version: String,
    /// File system path where the tool is installed
    pub install_path: String,
    /// True if managed by `setup-devbox` (vs pre-installed or manually installed)
    pub installed_by_devbox: bool,
    /// Method used for installation (e.g., "github-release", "brew", "cargo")
    pub install_method: String,
    /// New name if the executable was renamed during installation
    pub renamed_to: Option<String>,
    /// Type of package (e.g., "binary", "go-module", "rust-binary")
    pub package_type: String,
    /// GitHub repository if applicable (for GitHub source installations)
    pub repo: Option<String>,
    /// Specific GitHub tag/release if applicable (for GitHub source installations)
    pub tag: Option<String>,
    /// Options passed to the installer during installation
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// Original download URL for direct URL installations
    #[serde(default)]
    pub url: Option<String>,
    /// Timestamp of when the tool was last updated
    pub last_updated: Option<String>,
    /// Path to executable within extracted archive, relative to `install_path`
    pub executable_path_after_extract: Option<String>,
    /// Records any additional commands that were executed during installation.
    /// This is stored for reference and potential cleanup/uninstall operations.
    #[serde(default)]
    pub additional_cmd_executed: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_manager: Option<ConfigurationManagerState>,
}

impl ToolState {
    // Helper method to update configuration manager state
    pub fn set_configuration_manager(&mut self, config_state: ConfigurationManagerState) {
        self.configuration_manager = Some(config_state);
    }

    // Helper method to get configuration manager state
    pub fn get_configuration_manager(&self) -> Option<&ConfigurationManagerState> {
        self.configuration_manager.as_ref()
    }
}

/// Records the state of an applied system setting.
/// This information is used to track what settings have been configured.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingState {
    /// Setting's domain (e.g., "com.apple.finder")
    pub domain: String,
    /// Setting's key within the domain
    pub key: String,
    /// Value that was set for this setting
    pub value: String,
    /// Type of the value (e.g., "bool", "string")
    pub value_type: String,
}

/// Records the state of an installed font.
/// This information is used for font management and updates.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontState {
    /// Name of the font family
    pub name: String,
    /// Original download URL for the font
    pub url: String,
    /// List of installed font files (filenames)
    pub files: Vec<String>,
    /// Installed version of the font
    pub version: String,
    /// GitHub repository if applicable (for GitHub source installations)
    #[serde(default)]
    pub repo: Option<String>,
    /// Specific GitHub tag/release if applicable (for GitHub source installations)
    #[serde(default)]
    pub tag: Option<String>,
}

// Main Application Configuration

/// Defines the main application configuration file,
/// pointing to paths for other detailed configuration files.
/// This is the entry point configuration that references all other config files.
#[derive(Debug, Serialize, Deserialize)]
pub struct MainConfig {
    /// Optional path to `tools.yaml` configuration file
    pub tools: Option<String>,
    /// Optional path to `settings.yaml` configuration file
    pub settings: Option<String>,
    /// Optional path to `shellac.yaml` configuration file
    pub shellrc: Option<String>,
    /// Optional path to `fonts.yaml` configuration file
    pub fonts: Option<String>,
}

/// Structure representing the main config.yaml file that contains paths to other config files
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigPaths {
    pub(crate) tools: String,
    pub(crate) settings: String,
    pub(crate) shellrc: String,
    pub(crate) fonts: String,
}

// Implementations for error types
/// Implementation of Display trait for ToolEntryError to provide user-friendly error messages
impl fmt::Display for ToolEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolEntryError::MissingField(field) => write!(f, "Missing required field: {}", field),
            ToolEntryError::InvalidSource(source) => {
                write!(f, "Invalid tool source: '{}'", source)
            }
            ToolEntryError::ConflictingFields(msg) => write!(f, "Conflicting fields: {}", msg),
        }
    }
}

/// Mark ToolEntryError as a standard error type
impl std::error::Error for ToolEntryError {}

/// Implementation of validation methods for ToolEntry
impl ToolEntry {
    // Validates the ToolEntry based on its specified source.
    // This ensures that the configuration has all required fields and no conflicting fields.
    pub fn validate(&self) -> Result<(), ToolEntryError> {
        let supported_sources = [
            "github", "brew", "cargo", "rustup", "pip", "go", "url", "uv",
        ];
        let source_lower = self.source.to_lowercase();

        if !supported_sources.contains(&source_lower.as_str()) {
            return Err(ToolEntryError::InvalidSource(self.source.clone()));
        }

        match source_lower.as_str() {
            "github" => {
                if self.repo.is_none() {
                    return Err(ToolEntryError::MissingField("repo (for GitHub source)"));
                }
                if self.tag.is_none() {
                    return Err(ToolEntryError::MissingField("tag (for GitHub source)"));
                }
                // Ensure URL-specific fields are NOT present.
                if self.url.is_some() || self.executable_path_after_extract.is_some() {
                    return Err(ToolEntryError::ConflictingFields(
                        "url or executable_path_after_extract should not be present for GitHub source".to_string(),
                    ));
                }
            }
            "url" => {
                if self.url.is_none() {
                    return Err(ToolEntryError::MissingField("url (for URL source)"));
                }
                // Ensure GitHub-specific fields are NOT present.
                if self.repo.is_some() || self.tag.is_some() {
                    return Err(ToolEntryError::ConflictingFields(
                        "repo or tag should not be present for URL source".to_string(),
                    ));
                }
                // options, rename_to, and additional_cmd are general and can be present.
            }
            // For other sources, ensure GitHub/URL specific fields are NOT present.
            "brew" | "cargo" | "rustup" | "pip" => {
                if self.repo.is_some()
                    || self.tag.is_some()
                    || self.url.is_some()
                    || self.executable_path_after_extract.is_some()
                {
                    return Err(ToolEntryError::ConflictingFields(format!(
                        "repo, tag, url, or executable_path_after_extract should not be present for '{}' source",
                        self.source
                    )));
                }
                // additional_cmd is allowed for all sources as it provides post-install flexibility
            }
            _ => { /* Handled by supported_sources check. */ }
        }
        Ok(())
    }
}

/// Implementation of Display trait for InstallerError to provide user-friendly error messages
impl fmt::Display for InstallerError {
    // Implements the `Display` trait for `InstallerError`, allowing it to be formatted for user-facing messages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Formats the `MissingCommand` variant to indicate a missing command-line tool.
            InstallerError::MissingCommand(cmd) => {
                write!(
                    f,
                    "Installer command '{}' not found in your system's PATH.",
                    cmd
                )
            }
        }
    }
}

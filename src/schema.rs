// Defines the data structures (schemas) for configuration files and the application's internal state.

use serde::{Deserialize, Serialize}; // Serde traits for serialization and deserialization.
use std::collections::HashMap; // Used for key-value pair storage.
use std::fmt;

/// Represents a downloadable asset attached to a GitHub release.
#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    pub(crate) name: String,                 // The asset's filename.
    pub(crate) browser_download_url: String, // Direct download URL for the asset.
}

/// Captures details of a single GitHub release, including its assets.
#[derive(Debug, Deserialize)]
pub struct Release {
    pub(crate) assets: Vec<ReleaseAsset>, // List of downloadable assets for this release.
}

// User Configuration File Schemas (e.g., for YAML files)
// Defines the structure for user-defined configuration files.

/// Errors related to external installer command availability.
#[derive(Debug)]
pub enum InstallerError {
    /// Indicates a required command-line tool was not found in PATH.
    MissingCommand(String),
}

/// Custom error types for schema validation during tool entry processing.
#[derive(Debug)]
pub enum ToolEntryError {
    MissingField(&'static str),
    InvalidSource(String),
    ConflictingFields(String),
}

/// Defines the possible installation methods for a tool.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ToolInstallMethod {
    Brew,
    Cargo,
    Go,
    GitHub,
    Url,
    Rustup,
    Pip,
    System, // For tools managed externally or expected to be pre-installed.
}

/// Configuration schema for `tools.yaml`.
/// Defines the top-level structure for managing software tools.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    pub tools: Vec<ToolEntry>, // List of individual tool entries.
}

/// Represents a single tool entry defined by the user in `tools.yaml`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolEntry {
    pub name: String,
    pub version: Option<String>,
    pub source: String,            // Installation source (e.g., "github", "brew").
    pub url: Option<String>,       // Direct download URL (required if source is "URL").
    pub repo: Option<String>,      // GitHub repository in "owner/repo_name" format.
    pub tag: Option<String>,       // Specific GitHub release tag to download from.
    pub rename_to: Option<String>, // Optional new name for the executable after installation.
    #[serde(default)]
    pub options: Option<Vec<String>>, // Additional installer-specific options/flags.
    #[serde(default)]
    pub executable_path_after_extract: Option<String>, // Path to executable within extracted archive.

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
}

/// Configuration schema for `shellac.yaml`.
/// Defines the structure for shell environment customization (shellrc and aliases).
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    pub shellrc: ShellRc,         // Core shell configuration.
    pub aliases: Vec<AliasEntry>, // List of custom command aliases.
}

/// Represents the `shellrc` block within `shellac.yaml`.
/// Contains fundamental shell settings.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellRc {
    pub shell: String,            // Type of shell (e.g., "bash", "zsh").
    pub raw_configs: Vec<String>, // Raw commands to be appended/sourced into shell config.
}

/// Represents a single command alias entry in `shellac.yaml`.
#[derive(Debug, Serialize, Deserialize)]
pub struct AliasEntry {
    pub name: String,  // The alias name.
    pub value: String, // The command the alias expands to.
}

/// Configuration schema for `fonts.yaml`.
/// Defines the structure for managing and installing custom fonts.
#[derive(Debug, Serialize, Deserialize)]
pub struct FontConfig {
    pub fonts: Vec<FontEntry>, // List of individual font entries.
}

/// Represents a single font entry defined by the user in `fonts.yaml`.
#[derive(Debug, Serialize, Deserialize)]
pub struct FontEntry {
    pub name: String,
    pub version: Option<String>, // Desired font version.
    pub source: String,          // Source for the font (e.g., "github", "nerdfonts").
    pub repo: Option<String>,    // GitHub repository for the font.
    pub tag: Option<String>,     // Specific GitHub tag/release.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_only: Option<Vec<String>>, // Optional list of keywords for filtering specific font files to install.
}

/// Configuration schema for `settings.yaml`.
/// Defines the structure for applying system-level settings.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsConfig {
    pub settings: OsSpecificSettings, // OS-specific settings, keyed by OS name.
}

/// Container for operating system-specific settings.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OsSpecificSettings {
    #[serde(default)]
    pub macos: Vec<SettingEntry>, // macOS specific settings.
                                  // Add fields for other operating systems as needed.
}

/// Represents a single system setting to be applied (e.g., via macOS `defaults` command).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingEntry {
    pub domain: String, // The setting's domain (e.g., "com.apple.finder").
    pub key: String,    // The specific key within the domain.
    pub value: String,  // The value to set for the key.
    #[serde(rename = "type")]
    pub value_type: String, // Data type of the value (e.g., "bool", "string").
}

// Application State File Schema (`state.json`)
// Defines the structure of `setup-devbox`'s internal state file,
// used to track installed tools and applied configurations.

/// The complete structure of `state.json`, representing `setup-devbox`'s persistent memory.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DevBoxState {
    pub tools: HashMap<String, ToolState>, // Records information about installed tools.
    pub settings: HashMap<String, SettingState>, // Records applied system settings.
    pub fonts: HashMap<String, FontState>, // Stores information about installed fonts.
}

/// Stores detailed information about each installed tool.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolState {
    pub version: String,            // Exact version of the installed tool.
    pub install_path: String,       // File system path where the tool is installed.
    pub installed_by_devbox: bool,  // True if managed by `setup-devbox`.
    pub install_method: String, // Method used for installation (e.g., "github-release", "brew").
    pub renamed_to: Option<String>, // New name if the executable was renamed.
    pub package_type: String,   // Type of package (e.g., "binary", "go-module").
    pub repo: Option<String>,   // GitHub repository if applicable.
    pub tag: Option<String>,    // Specific GitHub tag/release if applicable.
    #[serde(default)]
    pub options: Option<Vec<String>>, // Options passed to the installer during installation.
    #[serde(default)]
    pub url: Option<String>, // Original download URL for direct URL installations.
    pub executable_path_after_extract: Option<String>, // Path to executable within extracted archive, relative to `install_path`.

    /// Records any additional commands that were executed during installation.
    /// This is stored for reference and potential cleanup/uninstall operations.
    #[serde(default)]
    pub additional_cmd_executed: Option<Vec<String>>,
}

/// Records the state of an applied system setting.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingState {
    pub domain: String,     // Setting's domain.
    pub key: String,        // Setting's key.
    pub value: String,      // Value that was set.
    pub value_type: String, // Type of the value.
}

/// Records the state of an installed font.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontState {
    pub name: String,       // Name of the font.
    pub url: String,        // Original download URL.
    pub files: Vec<String>, // List of installed font files.
    pub version: String,    // Installed version of the font.
    #[serde(default)]
    pub repo: Option<String>, // GitHub repository if applicable.
    #[serde(default)]
    pub tag: Option<String>, // Specific GitHub tag/release if applicable.
}

// Main Application Configuration

/// Defines the main application configuration file,
/// pointing to paths for other detailed configuration files.
#[derive(Debug, Serialize, Deserialize)]
pub struct MainConfig {
    pub tools: Option<String>,    // Optional path to `tools.yaml`.
    pub settings: Option<String>, // Optional path to `settings.yaml`.
    pub shellrc: Option<String>,  // Optional path to `shellac.yaml`.
    pub fonts: Option<String>,    // Optional path to `fonts.yaml`.
}

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
impl std::error::Error for ToolEntryError {}

impl ToolEntry {
    // Validates the ToolEntry based on its specified source.
    pub fn validate(&self) -> Result<(), ToolEntryError> {
        let supported_sources = ["github", "brew", "cargo", "rustup", "pip", "go", "url"];
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
            "brew" | "cargo" | "rustup" | "pip" | "go" => {
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

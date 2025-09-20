use crate::libs::configuration_manager::{ConfigurationManager, ConfigurationManagerProcessor};
use crate::schemas::state_file::DevBoxState;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::fmt;

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

/// ### Enums and Supporting Structs
/// Represents the possible outcomes after processing a single tool.
/// These results are used to build the final installation summary.
#[derive(Debug)]
pub enum ToolProcessingResult {
    Installed,
    Updated,
    ConfigurationUpdated,
    Skipped(String),
    ConfigurationSkipped(String),
    Failed(String),
}

/// Represents the high-level action to be taken for a tool.
/// This enum simplifies the decision-making process within the orchestrator.
#[derive(Debug, PartialEq)]
pub enum ToolAction {
    Install,
    Update,
    UpdateConfigurationOnly,
    Skip(String),
    SkipConfigurationOnly(String),
}

/// A sub-action related to a tool's version.
#[derive(Debug, PartialEq)]
pub enum VersionAction {
    Update,
    Skip(String),
}

/// A sub-action related to a tool's configuration.
#[derive(Debug, PartialEq)]
pub enum ConfigurationAction {
    Update,
    Skip(String),
}

/// ### Main Orchestrator Logic
/// The main orchestrator for the tool installation pipeline.
/// It holds the shared state and configuration processor.
pub struct ToolInstallationOrchestrator<'a> {
    // A mutable reference to the shared state, allowing updates to be persisted.
    pub(crate) state: &'a mut DevBoxState,
    // A reference to the installation configuration parameters.
    pub(crate) configuration: &'a InstallationConfiguration,
    // The processor responsible for managing tool configurations.
    pub(crate) config_processor: ConfigurationManagerProcessor,
}

/// Holds the parameters for the installation process, primarily the update policy.
#[derive(Debug)]
pub struct InstallationConfiguration {
    pub(crate) update_threshold_duration: Duration,
    pub(crate) force_update_enabled: bool,
}

/// ### Installation Summary
/// A struct to hold a summary of the installation results.
/// This is used to present a clean, organized report to the user at the end.
pub struct InstallationSummary {
    pub(crate) installed_tools: Vec<String>,
    pub(crate) updated_tools: Vec<String>,
    pub(crate) configuration_updated_tools: Vec<String>,
    pub(crate) skipped_tools: Vec<(String, String)>,
    pub(crate) configuration_skipped_tools: Vec<(String, String)>,
    pub(crate) failed_tools: Vec<(String, String)>,
}

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

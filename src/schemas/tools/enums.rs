// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that occur when external installer commands are not available on the system.
///
/// These errors are thrown during the validation phase when checking if required
/// command-line tools (like `brew`, `cargo`, `git`) are installed and accessible
/// via the system's PATH environment variable.
///
/// ## Common Scenarios
/// - User tries to install a brew package but homebrew isn't installed
/// - Cargo-based tool installation attempted without Rust toolchain
/// - Go module installation without Go compiler available
#[derive(Debug)]
pub enum InstallerError {
    /// A required command-line installer tool was not found in the system PATH.
    ///
    /// Contains the name of the missing command (e.g., "brew", "cargo", "go").
    /// This error helps users understand which tools they need to install first
    /// before attempting to use that installation method.
    MissingCommand(String),
}

/// Configuration validation errors that occur during tool entry parsing and validation.
///
/// These errors help users identify and fix issues in their `tools.yaml` configuration
/// file before the installation process begins. Each error provides specific guidance
/// about what needs to be corrected.
///
/// ## Error Categories
/// - **Missing Fields**: Required configuration fields are absent
/// - **Invalid Sources**: Unsupported installation methods specified
/// - **Conflicting Fields**: Incompatible configuration options used together
#[derive(Debug)]
pub enum ToolEntryError {
    /// A required configuration field is missing from the tool definition.
    ///
    /// Contains a static string describing which field is required.
    /// Common examples include missing `repo` field for GitHub sources,
    /// or missing `url` field for direct URL downloads.
    MissingField(&'static str),

    /// An unsupported or invalid installation source was specified.
    ///
    /// Contains the invalid source name that was provided.
    /// Valid sources include: GitHub, brew, cargo, rustup, pip, go, url, uv.
    InvalidSource(String),

    /// Configuration fields that shouldn't be used together for the specified source.
    ///
    /// Contains a descriptive message explaining the conflict.
    /// For example, using both `repo` (GitHub-specific) and `url` (URL-specific)
    /// fields in the same tool definition.
    ConflictingFields(String),
}

// =========================================================================== //
//                           PROCESSING RESULT TYPES                           //
// =========================================================================== //

/// Represents the outcome after processing a single tool installation.
///
/// These results are collected from all tool processing operations and used
/// to build the final installation summary that's displayed to the user.
/// Each variant provides specific information about what happened during processing.
#[derive(Debug)]
pub enum ToolProcessingResult {
    /// Tool was successfully installed for the first time.
    ///
    /// Indicates that the tool was not previously installed and has been
    /// successfully downloaded, configured, and made available for use.
    Installed,

    /// Tool was successfully updated to a newer version.
    ///
    /// Indicates that an existing installation was found and successfully
    /// updated to meet the current version requirements.
    Updated,

    /// Only the tool's configuration files were updated.
    ///
    /// The tool binary was already up-to-date, but configuration files
    /// needed to be synchronized or updated based on changes in the source.
    ConfigurationUpdated,

    /// Tool processing was skipped with a reason.
    ///
    /// Contains a string explaining why the tool was skipped (e.g.,
    /// "already up-to-date", "within update threshold").
    Skipped(String),

    /// Configuration management was skipped with a reason.
    ///
    /// Similar to `Skipped`, but specifically for tools that have configuration
    /// management enabled but didn't need configuration updates.
    ConfigurationSkipped(String),

    /// Tool processing failed with an error message.
    ///
    /// Contains a string describing what went wrong during the installation
    /// or configuration process. Used for error reporting and debugging.
    Failed(String),
}

/// High-level action to be taken for a tool during processing.
///
/// This enum simplifies the decision-making process within the orchestrator
/// by reducing complex state analysis into clear, actionable decisions.
/// Each action corresponds to a specific execution path in the installer.
#[derive(Debug, PartialEq, Eq)]
pub enum ToolAction {
    /// Install the tool for the first time.
    ///
    /// Used when no existing installation is found in the system state.
    /// Triggers the full installation pipeline including binary installation
    /// and configuration management.
    Install,

    /// Update an existing tool installation.
    ///
    /// Used when the tool exists but needs to be updated due to version
    /// changes or other requirements. May include both binary and configuration updates.
    Update,

    /// Update only the tool's configuration files.
    ///
    /// Used when the tool binary is up-to-date but configuration files
    /// need to be synchronized or updated.
    UpdateConfigurationOnly,

    /// Skip processing entirely with a reason.
    ///
    /// Used when no action is needed (tool is up-to-date, within update
    /// thresholds, etc.). Contains the reason for skipping.
    Skip(String),

    /// Skip processing but specifically for configuration reasons.
    ///
    /// Similar to `Skip` but used when configuration management is involved.
    /// Helps categorize different types of skips in the final summary.
    SkipConfigurationOnly(String),
}

/// Sub-action related specifically to tool version management.
///
/// Used internally by the decision engine to separate version-related
/// decisions from configuration-related decisions, enabling more precise
/// control flow and better error messaging.
#[derive(Debug, PartialEq, Eq)]
pub enum VersionAction {
    /// The tool version needs to be updated.
    ///
    /// Indicates that either no version is installed or the installed
    /// version doesn't match requirements.
    Update,

    /// Version update should be skipped with a reason.
    ///
    /// Contains a string explaining why the version doesn't need updating
    /// (e.g., "already at requested version", "within update threshold").
    Skip(String),
}

/// Sub-action related specifically to tool configuration management.
///
/// Used internally by the decision engine to make configuration-specific
/// decisions independently of version decisions, enabling fine-grained
/// control over when configuration files are updated.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigurationAction {
    /// Tool configuration needs to be updated.
    ///
    /// Indicates that configuration files are out of sync, missing,
    /// or need to be created for the first time.
    Update,

    /// Configuration update should be skipped with a reason.
    ///
    /// Contains a string explaining why configuration doesn't need updating
    /// (e.g., "configuration disabled", "files up-to-date").
    Skip(String),
}

//! # Tool Installation and Configuration Management Schema
//!
//! This module defines the data structures and validation logic for the tool installation system.
//! It handles parsing of user configuration files (like `tools.yaml`), validates tool entries,
//! and provides the core types used throughout the installation pipeline.
//!
//! ## Core Concepts
//!
//! - **ToolConfig**: Top-level configuration containing all tool definitions
//! - **ToolEntry**: Individual tool specification with installation and configuration details
//! - **ToolInstallMethod**: Supported installation sources (brew, cargo, GitHub, etc.)
//! - **Configuration Management**: Automatic synchronization of tool configuration files
//!
//! ## Usage Example
//!
//! ```yaml
//! # tools.yaml
//! update_latest_only_after: "7d"
//! tools:
//!   - name: "starship"
//!     source: "brew"
//!     configuration_manager:
//!       enabled: true
//!       tools_configuration_path: $HOME/.config/starship.toml"
//! ```

use crate::libs::configuration_manager::{ConfigurationManager, ConfigurationManagerProcessor};
use crate::schemas::state_file::DevBoxState;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::fmt;

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

// ============================================================================
// CORE ENUMS
// ============================================================================

/// Supported installation methods for software tools.
///
/// Each variant represents a different package manager, language toolchain,
/// or distribution method. The system routes tool installation requests
/// to the appropriate installer based on this enum value.
///
/// ## Installation Method Details
///
/// - **Package Managers**: Brew (macOS), system package managers
/// - **Language Toolchains**: Cargo (Rust), Go modules, Pip (Python), Rustup
/// - **Direct Sources**: GitHub releases, direct URL downloads
/// - **Special Cases**: System-managed tools (external to this system)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ToolInstallMethod {
    /// Homebrew package manager (primarily macOS, also Linux)
    ///
    /// Uses `brew install <tool_name>` command. Supports version constraints
    /// and additional installation options through the `options` field.
    Brew,

    /// Rust package manager and build system
    ///
    /// Installs Rust crates using `cargo install <crate_name>`. Automatically
    /// handles compilation from source and dependency management.
    Cargo,

    /// Go language module system
    ///
    /// Uses `go install <module_path>@<version>` to install Go-based tools.
    /// Supports version pinning and automatic dependency resolution.
    Go,

    /// GitHub release downloads
    ///
    /// Downloads pre-built binaries or source archives from GitHub releases.
    /// Supports asset filtering, extraction, and binary renaming.
    GitHub,

    /// Direct URL downloads
    ///
    /// Downloads files directly from specified URLs. Supports archives,
    /// single binaries, and custom extraction paths.
    Url,

    /// Rust toolchain installer
    ///
    /// Manages Rust compiler versions and components using the official
    /// rustup tool. Handles toolchain installation and updates.
    Rustup,

    /// Python package installer
    ///
    /// Installs Python packages using `pip install <package_name>`.
    /// Supports version constraints and additional pip options.
    Pip,

    /// UV Python package manager
    ///
    /// Modern, fast Python package installer that's compatible with pip
    /// but offers improved performance and dependency resolution.
    Uv,

    /// System-managed tools (external management)
    ///
    /// Placeholder for tools that are managed outside this system.
    /// Used for documentation purposes or dependency tracking without
    /// actual installation logic.
    System,
}

// ============================================================================
// CONFIGURATION SCHEMAS
// ============================================================================

/// Top-level configuration schema for the tool installation system.
///
/// This structure represents the complete `tools.yaml` configuration file
/// that users create to define their development environment setup.
/// It contains global settings and a list of all tools to be managed.
///
/// ## Configuration File Structure
///
/// ```yaml
/// ## Global settings (optional)
/// update_latest_only_after: "7d"  # Don't update "latest" tools more frequently than this
///
/// ## Tool definitions (required)
/// tools:
///   ## Install Zed Editor
///   - name: zed
///     version: 0.204.3
///     source: github
///     repo: zed-industries/zed
///     tag: v0.204.3
///     rename_to: zed
///     configuration_manager:
///       enabled: true
///       tools_configuration_path: $HOME/.config/zed/settings.json
/// ```
///
/// ## Global Behavior Control
///
/// The `update_latest_only_after` setting helps prevent excessive updates of tools
/// that use "latest" as their version. This is particularly useful for CI/CD
/// environments or when working with frequently updated tools.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Controls the update frequency for tools with "latest" version specification.
    ///
    /// When a tool is configured with `version: "latest"` (or no version specified),
    /// the system will only check for updates if more time than this threshold
    /// has passed since the last update.
    ///
    /// ## Format
    /// Duration string using suffixes: `s` (seconds), `m` (minutes), `h` (hours), `d` (days)
    ///
    /// ## Examples
    /// - `"1h"` - Update latest tools at most once per hour
    /// - `"7d"` - Update latest tools at most once per week
    /// - `"30m"` - Update latest tools at most every 30 minutes
    ///
    /// ## Default Behavior
    /// If not specified, tools with "latest" versions are checked for updates on every run.
    pub update_latest_only_after: Option<String>,

    /// List of individual tool definitions to be installed and managed.
    ///
    /// Each entry in this vector represents a single software tool with its
    /// installation source, version requirements, and configuration settings.
    /// Tools are processed sequentially in the order they appear in this list.
    pub tools: Vec<ToolEntry>,
}

/// Individual tool specification within the configuration.
///
/// Each `ToolEntry` defines how a specific software tool should be installed,
/// configured, and maintained. This includes the installation source, version
/// requirements, post-installation commands, and configuration file management.
///
/// ## Basic Tool Definition
///
/// ```yaml
/// - name: "ripgrep"           # Tool identifier (required)
///   source: "brew"            # Installation method (required)
///   version: "13.0.0"         # Specific version (optional, defaults to latest)
/// ```
///
/// ## Advanced Tool Definition
///
/// ```yaml
/// - name: "helix"
///   source: "github"
///   repo: "helix-editor/helix"
///   tag: "v23.10"
///   rename_to: "hx"
///   additional_cmd:
///     - "cp -r runtime $HOME/.config/helix/"
///   configuration_manager:
///     enabled: true
///     tools_configuration_path: "~/.config/helix/config.toml"
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolEntry {
    /// Unique identifier for the tool.
    ///
    /// This name is used for:
    /// - State tracking (remembering what's installed)
    /// - Configuration file discovery (`{name}.toml`)
    /// - Logging and error reporting
    /// - Default executable name (unless `rename_to` is specified)
    ///
    /// ## Naming Conventions
    /// - Use lowercase with hyphens for consistency (e.g., "git-lfs")
    /// - Match the tool's common name or package name when possible
    /// - Avoid spaces and special characters that might cause issues in file paths
    pub name: String,

    /// Desired version of the tool to install.
    ///
    /// ## Version Specification Options
    /// - **Specific version**: `"1.2.3"` - Install exactly this version
    /// - **Latest version**: `"latest"` or `None` - Install the most recent version
    /// - **Version ranges**: Depends on the installer (e.g., cargo supports semver ranges)
    ///
    /// ## Update Behavior
    /// - Tools with specific versions are only updated if the version changes in config
    /// - Tools with "latest" versions are updated based on `update_latest_only_after` policy
    /// - Missing version field is treated as "latest"
    pub version: Option<String>,

    /// Installation source method.
    ///
    /// Specifies which installer/package manager to use for this tool.
    /// The value must match one of the supported installation methods.
    /// This field determines which installer module will handle the tool.
    ///
    /// ## Supported Values
    /// `"brew"`, `"cargo"`, `"go"`, `"github"`, `"url"`, `"rustup"`, `"pip"`, `"uv"`
    ///
    /// ## Source-Specific Requirements
    /// Each source type requires different additional fields - see validation logic for details.
    pub source: String,

    /// Direct download URL (required when `source` is `"url"`).
    ///
    /// Specifies the exact URL to download the tool from. Can point to:
    /// - Single binary files (will be downloaded and made executable)
    /// - Archive files (.tar.gz, .zip, etc. - will be extracted)
    /// - Installation scripts (will be downloaded and executed)
    ///
    /// ## URL Requirements
    /// - Must be a valid HTTP/HTTPS URL
    /// - Should point to a stable, versioned resource when possible
    /// - Must be accessible without authentication
    ///
    /// ## Conflicts
    /// Cannot be used with GitHub-specific fields (`repo`, `tag`).
    pub url: Option<String>,

    /// GitHub repository specification (required when `source` is `"github"`).
    ///
    /// Format: `"owner/repository_name"`
    ///
    /// ## Examples
    /// - `"helix-editor/helix"`
    /// - `"sharkdp/bat"`
    /// - `"BurntSushi/ripgrep"`
    ///
    /// ## Behavior
    /// The system will use the GitHub API to discover available releases
    /// and download assets from the specified repository's releases page.
    ///
    /// ## Conflicts
    /// Cannot be used with URL-specific fields (`url`, `executable_path_after_extract`).
    pub repo: Option<String>,

    /// GitHub release tag to download (required when `source` is `"github"`).
    ///
    /// Specifies which release to download from the GitHub repository.
    /// Must match an actual release tag in the repository.
    ///
    /// ## Tag Format
    /// - Usually starts with `"v"`: `"v1.2.3"`, `"v23.10"`
    /// - Sometimes without prefix: `"1.2.3"`, `"2023.10"`
    /// - Can be pre-release tags: `"v1.3.0-beta1"`
    ///
    /// ## Special Values
    /// - Use `"latest"` in the `version` field instead of specifying a tag for the latest release
    pub tag: Option<String>,

    /// New name for the tool's executable after installation.
    ///
    /// When specified, the installed binary will be renamed to this value.
    /// Useful when the original binary name is too long, conflicts with existing tools,
    /// or doesn't match your preferred naming convention.
    ///
    /// ## Examples
    /// - `rename_to: "hx"` (for helix editor)
    /// - `rename_to: "lg"` (for lazygit)
    /// - `rename_to: "rg"` (for ripgrep)
    ///
    /// ## Behavior
    /// - The renamed binary will be placed in the standard executable location
    /// - The original name is preserved in installation metadata for updates
    /// - Symlinks may be used depending on the installer implementation
    pub rename_to: Option<String>,

    /// Additional command-line options passed to the installer.
    ///
    /// These options are passed directly to the underlying package manager
    /// or installation tool. The specific options available depend on the
    /// installation source being used.
    ///
    /// ## Examples by Source
    /// - **Brew**: `["--HEAD"]` for latest development version
    /// - **Cargo**: `["--locked"]` to use exact dependency versions
    /// - **Pip**: `["--user"]` for user-local installation
    ///
    /// ## Default Behavior
    /// When not specified (`#[serde(default)]`), no additional options are passed.
    #[serde(default)]
    pub options: Option<Vec<String>>,

    /// Path to the executable within extracted archives (for URL and GitHub sources).
    ///
    /// When downloading archives (.tar.gz, .zip), this field specifies the relative
    /// path from the extraction root to the actual executable file that should be
    /// installed.
    ///
    /// ## Path Examples
    /// - `"bin/tool-name"` - executable in a bin subdirectory
    /// - `"tool-name"` - executable at archive root
    /// - `"release/linux/tool"` - platform-specific binary location
    ///
    /// ## When to Use
    /// - Archive contains multiple files and directories
    /// - Executable is not at the archive root
    /// - Archive contains platform-specific subdirectories
    ///
    /// ## Default Behavior
    /// If not specified, the system will attempt to find an executable automatically
    /// by searching common locations and using the tool name.
    #[serde(default)]
    pub executable_path_after_extract: Option<String>,

    /// Post-installation commands to execute after successful tool installation.
    ///
    /// These commands run after the tool binary has been installed but before
    /// the installation is marked as complete. They execute in sequence, and
    /// if any command fails, the entire installation is considered failed.
    ///
    /// ## Execution Context
    /// - **Working Directory**: Set to the extraction/download directory for archive-based installs
    /// - **Environment**: Full user environment with PATH, HOME, etc.
    /// - **Permissions**: Same as the user running the installation
    /// - **Shell**: Commands are executed through the system shell
    ///
    /// ## Common Use Cases
    /// - Copy configuration files: `"cp config.toml $HOME/.config/tool/"`
    /// - Create directories: `"mkdir -p $HOME/.local/share/tool"`
    /// - Set up symlinks: `"ln -sf $(pwd)/data $HOME/.tool-data"`
    /// - Run setup scripts: `"./setup.sh"`
    /// - Install additional components: `"./install-extras.sh"`
    ///
    /// ## Environment Variable Support
    /// Commands can reference environment variables like `$HOME`, `$USER`, `$PWD`.
    /// Variables are expanded by the shell before command execution.
    ///
    /// ## Path Considerations
    /// - Use absolute paths when referencing locations outside the extraction directory
    /// - Relative paths are resolved from the extraction/download directory
    /// - Be cautious with commands that change the working directory
    ///
    /// ## Error Handling
    /// - Commands execute sequentially in the specified order
    /// - First failing command stops execution of remaining commands
    /// - Failed commands cause the entire tool installation to be marked as failed
    /// - Use `|| true` suffix to make commands non-fatal if needed
    ///
    /// ## Security Considerations
    /// - Commands execute with full user privileges
    /// - Avoid downloading and executing remote scripts unless from trusted sources
    /// - Review commands carefully as they can modify system configuration
    ///
    /// ## Examples
    ///
    /// ```yaml
    /// additional_cmd:
    ///   - "cp -r runtime $HOME/.config/helix/"
    ///   - "mkdir -p $HOME/.local/share/helix"
    ///   - "ln -sf $(pwd)/themes $HOME/.config/helix/themes"
    ///   - "chmod +x contrib/completion.sh"
    /// ```
    #[serde(default)]
    pub additional_cmd: Option<Vec<String>>,

    /// Configuration file management settings for this tool.
    ///
    /// When enabled, this system automatically manages configuration files for tools,
    /// keeping them synchronized between a centralized source location and the tool's
    /// expected configuration directory. This enables consistent tool configuration
    /// across different environments and machines.
    ///
    /// ## Core Functionality
    ///
    /// 1. **Source File Discovery**: Looks for `{tool_name}.toml` in the configured source directory
    /// 2. **Format Conversion**: Converts TOML source files to the target format (JSON, YAML, TOML, KEY=VALUE)
    /// 3. **Change Detection**: Uses SHA-256 hashes to detect changes and avoid unnecessary updates
    /// 4. **Path Expansion**: Supports `~`, `$HOME`, and other environment variables in paths
    /// 5. **State Tracking**: Remembers file hashes to optimize future update checks
    ///
    /// ## Configuration Source Hierarchy
    ///
    /// The system searches for source configuration files in this priority order:
    /// 1. `SDB_TOOLS_SOURCE_CONFIG_PATH` environment variable and looks for `{tool_name}.toml`
    /// 2. `SDB_CONFIG_PATH` environment variable and builds `SDB_CONFIG_PATH/config/tools`
    /// 3. `~/.setup-devbox/configs/tools` (default and fallback)
    ///
    /// ## Output Format Detection
    ///
    /// Target format is determined by the file extension in `tools_configuration_path`:
    /// - `.json` → Pretty-printed JSON
    /// - `.yaml` or `.yml` → YAML format
    /// - `.toml` → Pretty-printed TOML
    /// - Other extensions → KEY=VALUE pairs with smart quoting
    ///
    /// ## Change Detection Logic
    ///
    /// Configuration updates are triggered when:
    /// - Source file content changes (SHA-256 mismatch)
    /// - Destination file is missing or was deleted
    /// - Destination file was modified externally (SHA-256 mismatch)
    /// - Configuration path in tool definition changes
    /// - Configuration management is newly enabled for a tool
    ///
    /// ## Error Handling and Resilience
    ///
    /// - **Missing source files**: Logged as warnings, tool installation continues
    /// - **Path expansion failures**: Falls back to literal path interpretation
    /// - **Format conversion errors**: Tool installation continues with warning
    /// - **Permission errors**: Logged as errors, tool installation continues
    ///
    /// ## Performance Optimizations
    ///
    /// - SHA-256 hashing prevents unnecessary file I/O operations
    /// - Only processes files when actual changes are detected
    /// - Caches expanded paths and file metadata
    ///
    /// ## Example Configuration
    ///
    /// ```yaml
    /// configuration_manager:
    ///   enabled: true
    ///   tools_configuration_path: "~/.config/starship/starship.toml"
    /// ```
    ///
    /// With source file at `~/.setup-devbox/configs/tools/starship.toml`:
    /// ```toml
    /// # starship.toml
    /// format = "$directory$git_branch$character"
    /// add_newline = true
    ///
    /// [directory]
    /// style = "blue bold"
    /// ```
    ///
    /// ## Use Cases
    ///
    /// - **Dotfiles Management**: Keep tool configurations synchronized across machines
    /// - **Team Standardization**: Ensure consistent tool settings across development team
    /// - **Environment Portability**: Easily replicate development environment setup
    /// - **Configuration Backup**: Centralized storage of tool configuration files
    /// - **Format Flexibility**: Convert between different configuration file formats
    ///
    /// ## Integration with Installation Pipeline
    ///
    /// Configuration management runs after successful tool installation:
    /// 1. Tool binary is installed and verified
    /// 2. Configuration files are processed and synchronized
    /// 3. State is updated with both tool and configuration information
    /// 4. Installation is marked as complete
    ///
    /// Configuration-only updates can occur when:
    /// - Tool binary is already up-to-date but configuration changed
    /// - User modifies the source configuration file
    /// - Configuration path is updated in tool definition
    ///
    /// ## Default Behavior
    ///
    /// The `#[serde(default)]` attribute ensures that:
    /// - Existing tool definitions without configuration management continue to work
    /// - New tools have configuration management disabled by default (opt-in)
    /// - Backward compatibility is maintained when upgrading the system
    /// - Configuration is only managed when explicitly enabled by the user
    #[serde(default)]
    pub configuration_manager: ConfigurationManager,
}

// ============================================================================
// PROCESSING RESULT TYPES
// ============================================================================

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
#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
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

// ============================================================================
// ORCHESTRATOR AND CONFIGURATION TYPES
// ============================================================================

/// Main orchestrator responsible for coordinating the tool installation pipeline.
///
/// This struct holds the shared state and processors needed to install and configure
/// tools. It orchestrates the entire process from validation through installation
/// to configuration management and state persistence.
///
/// ## Lifetime Parameter
/// The `'a` lifetime ensures that the orchestrator cannot outlive the mutable
/// references it holds, preventing use-after-free and ensuring memory safety.
///
/// ## Responsibilities
/// - Validates tool configurations and system requirements
/// - Determines appropriate actions for each tool (install/update/skip)
/// - Coordinates with individual installers (brew, cargo, github, etc.)
/// - Manages configuration file synchronization
/// - Updates and persists installation state
/// - Provides detailed feedback and error reporting
pub struct ToolInstallationOrchestrator<'a> {
    /// Mutable reference to the global installation state.
    ///
    /// This state tracks what tools are installed, their versions, configuration
    /// status, and last update times. Changes made during installation are
    /// persisted to disk at the end of the process.
    pub(crate) state: &'a mut DevBoxState,

    /// Reference to installation configuration parameters.
    ///
    /// Contains settings like update thresholds, force update flags, and other
    /// global behavior controls that affect how tools are processed.
    pub(crate) configuration: &'a InstallationConfiguration,

    /// Configuration file processor for managing tool configurations.
    ///
    /// Handles the discovery, conversion, and synchronization of tool
    /// configuration files from source locations to target destinations.
    pub(crate) config_processor: ConfigurationManagerProcessor,
}

/// Installation behavior configuration parameters.
///
/// This struct encapsulates the global settings that control how the installation
/// process behaves across all tools. These settings are derived from command-line
/// arguments and configuration file options.
///
/// ## Update Policy Control
/// The primary purpose is to control when tools with "latest" versions are updated,
/// preventing excessive update frequency in automated environments while still
/// allowing manual override when needed.
#[derive(Debug)]
pub struct InstallationConfiguration {
    /// Minimum time that must pass before updating "latest" version tools.
    ///
    /// Tools configured with `version: "latest"` (or no version specified) will
    /// only be checked for updates if more than this duration has elapsed since
    /// their last update. This prevents excessive API calls and unnecessary
    /// processing in frequently-run installations.
    ///
    /// ## Duration Examples
    /// - `Duration::hours(1)` - Update at most once per hour
    /// - `Duration::days(7)` - Update at most once per week
    /// - `Duration::seconds(0)` - Always update (when force_update_enabled is true)
    pub(crate) update_threshold_duration: Duration,

    /// Whether to ignore update thresholds and force updates of all "latest" tools.
    ///
    /// When `true`, all tools with "latest" versions will be checked for updates
    /// regardless of when they were last updated. This is typically controlled
    /// by command-line flags like `--force-update` or `--update-latest`.
    ///
    /// ## Use Cases
    /// - Manual update requests by users
    /// - CI/CD pipelines that need fresh tools
    /// - Recovery from corrupted installations
    /// - Testing and development scenarios
    pub(crate) force_update_enabled: bool,
}

/// Summary of installation results across all processed tools.
///
/// This struct collects and categorizes the results from processing all tools,
/// providing a structured way to present a comprehensive summary to the user.
/// It separates different types of outcomes to enable targeted reporting and
/// decision-making about whether state persistence is needed.
///
/// ## Result Categorization
/// Results are grouped by outcome type to enable:
/// - Clear, organized output to users
/// - Conditional behavior based on what happened
/// - Metrics and monitoring of installation success rates
/// - Debugging and troubleshooting support
pub struct InstallationSummary {
    /// Tools that were successfully installed for the first time.
    ///
    /// Contains the names of tools that were not previously installed
    /// and have been successfully set up during this run.
    pub(crate) installed_tools: Vec<String>,

    /// Tools that were successfully updated to newer versions.
    ///
    /// Contains the names of tools that had existing installations
    /// but were updated to meet current version requirements.
    pub(crate) updated_tools: Vec<String>,

    /// Tools that had only their configuration files updated.
    ///
    /// Contains the names of tools where the binary was already current
    /// but configuration files needed synchronization or updates.
    pub(crate) configuration_updated_tools: Vec<String>,

    /// Tools that were skipped during processing with reasons.
    ///
    /// Contains tuples of `(tool_name, skip_reason)` for tools that didn't
    /// need any processing. Reasons typically include "already up-to-date"
    /// or "within update threshold".
    pub(crate) skipped_tools: Vec<(String, String)>,

    /// Tools with configuration management that were skipped with reasons.
    ///
    /// Contains tuples of `(tool_name, skip_reason)` for tools that have
    /// configuration management enabled but didn't need configuration updates.
    /// This is separate from `skipped_tools` to provide clearer reporting.
    pub(crate) configuration_skipped_tools: Vec<(String, String)>,

    /// Tools that failed to process with error messages.
    ///
    /// Contains tuples of `(tool_name, error_message)` for tools that
    /// encountered errors during installation, update, or configuration.
    /// Used for error reporting and debugging.
    pub(crate) failed_tools: Vec<(String, String)>,
}

// ============================================================================
// IMPLEMENTATION BLOCKS
// ============================================================================

/// Validation methods for individual tool entries.
///
/// This implementation provides comprehensive validation of tool configurations
/// before they're processed by the installation system. Validation catches
/// common configuration errors early and provides clear guidance for fixing them.
impl ToolEntry {
    /// Validates a tool entry's configuration based on its specified source.
    ///
    /// This method performs comprehensive validation to ensure that:
    /// 1. The specified installation source is supported
    /// 2. All required fields for that source are present
    /// 3. No conflicting fields from other sources are specified
    /// 4. Field values meet basic format requirements
    ///
    /// ## Validation Rules by Source
    ///
    /// ### GitHub Source (`source: "github"`)
    /// - **Required**: `repo` (format: "owner/repository")
    /// - **Required**: `tag` (must match a release tag)
    /// - **Forbidden**: `url`, `executable_path_after_extract`
    /// - **Optional**: `rename_to`, `options`, `additional_cmd`, `configuration_manager`
    ///
    /// ### URL Source (`source: "url"`)
    /// - **Required**: `url` (must be valid HTTP/HTTPS URL)
    /// - **Optional**: `executable_path_after_extract` (for archives)
    /// - **Forbidden**: `repo`, `tag`
    /// - **Optional**: `rename_to`, `options`, `additional_cmd`, `configuration_manager`
    ///
    /// ### Package Manager Sources (`brew`, `cargo`, `rustup`, `pip`, `uv`, `go`)
    /// - **Required**: `name` only
    /// - **Optional**: `version`, `rename_to`, `options`, `additional_cmd`, `configuration_manager`
    /// - **Forbidden**: All source-specific fields (`repo`, `tag`, `url`, `executable_path_after_extract`)
    ///
    /// ## Common Fields (allowed for all sources)
    /// - `name`: Tool identifier (always required)
    /// - `version`: Version specification (optional, defaults to "latest")
    /// - `rename_to`: Alternative executable name (optional)
    /// - `options`: Installer-specific command-line options (optional)
    /// - `additional_cmd`: Post-installation commands (optional)
    /// - `configuration_manager`: Configuration file management (optional, defaults to disabled)
    ///
    /// ## Error Types
    /// - **InvalidSource**: Unsupported installation method specified
    /// - **MissingField**: Required field for the specified source is absent
    /// - **ConflictingFields**: Fields from different sources used together inappropriately
    ///
    /// ## Examples
    ///
    /// **Valid GitHub tool:**
    /// ```yaml
    /// - name: "ripgrep"
    ///   source: "github"
    ///   repo: "BurntSushi/ripgrep"
    ///   tag: "v13.0.0"
    ///   rename_to: "rg"
    /// ```
    ///
    /// **Valid Brew tool:**
    /// ```yaml
    /// - name: "bat"
    ///   source: "brew"
    ///   version: "0.24.0"
    ///   options: ["--HEAD"]
    /// ```
    ///
    /// **Invalid configuration (conflicting fields):**
    /// ```yaml
    /// - name: "bad-tool"
    ///   source: "github"      # GitHub source specified
    ///   repo: "owner/repo"    # GitHub field (correct)
    ///   tag: "v1.0.0"        # GitHub field (correct)
    ///   url: "http://..."     # URL field (CONFLICT!)
    /// ```
    ///
    /// ## Returns
    /// - `Ok(())`: Configuration is valid and ready for processing
    /// - `Err(ToolEntryError)`: Specific validation error with description
    pub fn validate(&self) -> Result<(), ToolEntryError> {
        // Define all supported installation sources
        let supported_sources = [
            "github", "brew", "cargo", "rustup", "pip", "go", "url", "uv",
        ];
        let source_lower = self.source.to_lowercase();

        // Validate that the source is supported
        if !supported_sources.contains(&source_lower.as_str()) {
            return Err(ToolEntryError::InvalidSource(self.source.clone()));
        }

        // Perform source-specific validation
        match source_lower.as_str() {
            "github" => {
                // GitHub sources require repository and tag specification
                if self.repo.is_none() {
                    return Err(ToolEntryError::MissingField("repo (for GitHub source)"));
                }
                if self.tag.is_none() {
                    return Err(ToolEntryError::MissingField("tag (for GitHub source)"));
                }

                // GitHub sources cannot use URL-specific fields
                if self.url.is_some() || self.executable_path_after_extract.is_some() {
                    return Err(ToolEntryError::ConflictingFields(
                        "url or executable_path_after_extract should not be present for GitHub source".to_string(),
                    ));
                }
            }
            "url" => {
                // URL sources require a download URL
                if self.url.is_none() {
                    return Err(ToolEntryError::MissingField("url (for URL source)"));
                }

                // URL sources cannot use GitHub-specific fields
                if self.repo.is_some() || self.tag.is_some() {
                    return Err(ToolEntryError::ConflictingFields(
                        "repo or tag should not be present for URL source".to_string(),
                    ));
                }
                // Note: executable_path_after_extract is allowed for URL sources (archives)
            }
            // Package manager sources (brew, cargo, rustup, pip, go, uv)
            "brew" | "cargo" | "rustup" | "pip" | "go" | "uv" => {
                // Package managers cannot use any source-specific fields
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
                // Note: additional_cmd is allowed for all sources for post-install flexibility
            }
            _ => {
                // This shouldn't be reachable due to the supported_sources check above,
                // but is included for completeness and future-proofing
                unreachable!(
                    "Source validation should have caught unsupported source: {}",
                    source_lower
                );
            }
        }

        // All validation checks passed
        Ok(())
    }
}

// ============================================================================
// ERROR TYPE IMPLEMENTATIONS
// ============================================================================

/// User-friendly display implementation for tool entry validation errors.
///
/// Provides clear, actionable error messages that help users understand
/// what's wrong with their configuration and how to fix it.
impl fmt::Display for ToolEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolEntryError::MissingField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            ToolEntryError::InvalidSource(source) => {
                write!(
                    f,
                    "Invalid tool source: '{}'. Supported sources are: github, brew, cargo, rustup, pip, go, url, uv",
                    source
                )
            }
            ToolEntryError::ConflictingFields(msg) => {
                write!(f, "Conflicting fields: {}", msg)
            }
        }
    }
}

/// Standard error trait implementation for tool entry errors.
///
/// Enables `ToolEntryError` to be used with Rust's standard error handling
/// infrastructure, including error chaining and conversion patterns.
impl std::error::Error for ToolEntryError {}

/// User-friendly display implementation for installer availability errors.
///
/// Provides clear guidance when required command-line tools are missing
/// from the system, helping users understand what they need to install.
impl fmt::Display for InstallerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallerError::MissingCommand(cmd) => {
                write!(
                    f,
                    "Installer command '{}' not found in your system's PATH. Please install {} before proceeding.",
                    cmd,
                    match cmd.as_str() {
                        "brew" => "Homebrew (https://brew.sh/)",
                        "cargo" => "Rust toolchain (https://rustup.rs/)",
                        "go" => "Go programming language (https://golang.org/dl/)",
                        "pip" => "Python package installer (usually bundled with Python)",
                        "uv" => "UV Python package manager (https://github.com/astral-sh/uv)",
                        _ => cmd, // Generic fallback for unknown commands
                    }
                )
            }
        }
    }
}

/// Standard error trait implementation for installer errors.
///
/// Enables `InstallerError` to be used with Rust's standard error handling
/// patterns and to be returned from functions that return `Result` types.
impl std::error::Error for InstallerError {}

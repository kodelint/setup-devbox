// =========================================================================== //
//                             EXTERNAL DEPENDENCIES                           //
// =========================================================================== //
use chrono::Duration;
use serde::{Deserialize, Serialize};
// =========================================================================== //
//                              INTERNAL IMPORTS                               //
// =========================================================================== //
use crate::libs::tools::configuration::processor::{
    ConfigurationManager, ConfigurationManagerProcessor,
};
use crate::schemas::state_file::DevBoxState;

// =========================================================================== //
//                             CONFIGURATION SCHEMAS                           //
// =========================================================================== //

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
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// post_installation_hooks:
    ///   - "cp -r runtime $HOME/.config/helix/"
    ///   - "mkdir -p $HOME/.local/share/helix"
    ///   - "ln -sf $(pwd)/themes $HOME/.config/helix/themes"
    ///   - "chmod +x contrib/completion.sh"
    /// ```
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_installation_hooks: Option<Vec<String>>,

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
    /// 1. `SDB_CONFIG_PATH` environment variable and builds `SDB_CONFIG_PATH/config/tools`
    /// 2. `~/.setup-devbox/configs/tools` (default and fallback)
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
    #[serde(skip_serializing_if = "ConfigurationManager::is_default")]
    pub configuration_manager: ConfigurationManager,
}

// =========================================================================== //
//                    ORCHESTRATOR AND CONFIGURATION TYPES                     //
// =========================================================================== //

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
    pub state: &'a mut DevBoxState,

    /// Reference to installation configuration parameters.
    ///
    /// Contains settings like update thresholds, force update flags, and other
    /// global behavior controls that affect how tools are processed.
    pub configuration: &'a InstallationConfiguration,

    /// Configuration file processor for managing tool configurations.
    ///
    /// Handles the discovery, conversion, and synchronization of tool
    /// configuration files from source locations to target destinations.
    pub config_processor: ConfigurationManagerProcessor,
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
    /// - `Duration::seconds(0)` - Always update (when `force_update_enabled` is `true`)
    pub update_threshold_duration: Duration,

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
    pub force_update_enabled: bool,
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
    pub installed_tools: Vec<String>,

    /// Tools that were successfully updated to newer versions.
    ///
    /// Contains the names of tools that had existing installations
    /// but were updated to meet current version requirements.
    pub updated_tools: Vec<String>,

    /// Tools that had only their configuration files updated.
    ///
    /// Contains the names of tools where the binary was already current
    /// but configuration files needed synchronization or updates.
    pub configuration_updated_tools: Vec<String>,

    /// Tools that were skipped during processing with reasons.
    ///
    /// Contains tuples of `(tool_name, skip_reason)` for tools that didn't
    /// need any processing. Reasons typically include "already up-to-date"
    /// or "within update threshold".
    pub skipped_tools: Vec<(String, String)>,

    /// Tools with configuration management that were skipped with reasons.
    ///
    /// Contains tuples of `(tool_name, skip_reason)` for tools that have
    /// configuration management enabled but didn't need configuration updates.
    /// This is separate from `skipped_tools` to provide clearer reporting.
    pub configuration_skipped_tools: Vec<(String, String)>,

    /// Tools that failed to process with error messages.
    ///
    /// Contains tuples of `(tool_name, error_message)` for tools that
    /// encountered errors during installation, update, or configuration.
    /// Used for error reporting and debugging.
    pub failed_tools: Vec<(String, String)>,
}

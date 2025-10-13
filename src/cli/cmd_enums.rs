use crate::cli::type_enums::{ConfigType, SourceType, ValueType};
use clap::{Parser, Subcommand};

/// Defines the command-line interface (CLI) for 'setup-devbox'.
/// `#[derive(Parser)]` automatically generates argument parsing code via `clap`.
#[derive(Parser)]
#[command(name = "setup-devbox")]
#[command(disable_help_subcommand = true)] // Disables the built-in help subcommand to use custom implementation
#[command(disable_help_flag = true)] // Disables the built-in help flag to use custom implementation
pub struct Cli {
    /// Enables detailed debug output for troubleshooting and development.
    #[arg(short, long)]
    pub(crate) debug: bool,

    /// Defines available subcommands for 'setup-devbox'.
    #[command(subcommand)]
    pub(crate) command: Commands,
}

/// Enumerates all supported subcommands with their specific arguments and options.
/// Each variant represents a distinct functionality of the setup-devbox application.
#[derive(Subcommand)]
pub enum Commands {
    /// Show the current Version of the tool.
    Version,
    /// Installs and Configures Tools, Fonts, OS Settings and Shell Configs.
    /// This is the primary command that executes the full setup process.
    Now {
        /// Optional path to a custom configuration file for tools, fonts, settings, and shell configurations.
        #[arg(long)]
        config: Option<String>,
        /// Optional path to a custom state file for tracking installation status.
        #[arg(long)]
        state: Option<String>,
        /// Force update all tools with version "latest", overriding `update_latest_only_after` policy.
        /// This ensures the latest versions are installed regardless of previous installation timestamps.
        #[arg(long)]
        update_latest: bool,
    },
    /// Generates default configuration files with sensible defaults.
    /// Useful for initial setup or creating template configurations.
    Generate {
        /// Optional path to save the generated configuration files.
        #[arg(long)]
        config: Option<String>,
        /// Optional path to save the generated state file.
        #[arg(long)]
        state: Option<String>,
    },
    /// Synchronizes or generates configurations from a state file.
    /// This allows recreating configuration files from an existing installation state.
    SyncConfig {
        /// Optional path to the state file (defaults to ~/.setup-devbox/state.json).
        #[arg(long)]
        state: Option<String>,
        /// Optional output directory for generated configuration files (defaults to ~/.setup-devbox/configs).
        #[arg(long)]
        output_dir: Option<String>,
    },
    /// Edit configuration files or state file in your preferred editor.
    /// Provides quick access to modify configurations using the system's default editor.
    Edit {
        /// Edit the state file (break glass mechanism - use with caution).
        /// Modifying the state file directly can affect idempotent operations.
        #[arg(long, conflicts_with = "config")]
        state: bool,
        /// Edit a specific configuration file [possible values: tools, fonts, shell, settings].
        #[arg(long, conflicts_with = "state")]
        config: Option<ConfigType>,
    },
    /// Add a new tool, font, setting, or alias to configuration files.
    /// Provides a convenient way to extend configurations without manual file editing.
    Add {
        #[command(subcommand)]
        add_type: AddCommands,
    },
    /// Remove an installed tool, font, alias, or setting
    Remove {
        #[command(subcommand)]
        item: RemoveCommands,
    },
    /// Show detailed help for commands and installers.
    /// Provides comprehensive documentation and usage examples.
    Help {
        /// The command or topic to show help for (e.g., 'now', 'installers').
        topic: Option<String>,
        /// Show detailed information with examples and advanced usage.
        #[arg(long)]
        detailed: bool,
        /// Filter results by installer type or category.
        #[arg(long)]
        filter: Option<String>,
    },
}

/// Enumerates the types of entities that can be added to configuration files.
/// Each variant represents a different configuration category with specific parameters.
#[derive(Subcommand)]
pub enum AddCommands {
    /// Add a new tool to tools.yaml configuration.
    /// Tools can be installed from various sources like Homebrew, GitHub, Cargo, etc.
    Tool {
        /// Name of the tool to add. This is the primary identifier for the tool.
        #[arg(long)]
        name: String,
        /// Version of the tool (e.g., "1.0.0" or "latest"). Use "latest" for the most recent version.
        #[arg(long)]
        version: String,
        /// Source type [brew, github, rustup, cargo, pip, go, url, uv].
        /// Determines which installer will be used and how the tool is fetched.
        #[arg(long)]
        source: SourceType,
        /// Direct URL for downloading the tool (used with 'url' source type).
        #[arg(long)]
        url: Option<String>,
        /// Repository (required for github source, format: owner/repo).
        /// Specifies the GitHub repository where the tool is hosted.
        #[arg(long)]
        repo: Option<String>,
        /// Release tag (required for github source).
        /// Specific version tag to download from the repository.
        #[arg(long)]
        tag: Option<String>,
        /// Rename the binary to a different name (optional).
        /// Useful when the downloaded binary has a different name than expected.
        #[arg(long)]
        rename_to: Option<String>,
        /// Additional Options for installation.
        /// Source-specific options like compilation flags, installation parameters, etc.
        #[arg(long)]
        options: Option<Vec<String>>,
        /// Additional commands to run after installation (can be specified multiple times).
        /// Commands executed in sequence after the main installation completes.
        #[arg(long)]
        executable_path_after_extract: Vec<String>,
        /// Post installation hooks - commands to run after successful installation.
        /// Useful for setup tasks like creating symlinks, generating configurations, etc.
        #[arg(long)]
        post_installation_hooks: Option<Vec<String>>,
        /// Enable configuration manager tracking.
        /// When enabled, the tool's configuration files will be tracked and managed.
        #[arg(long)]
        enable_config_manager: bool,
        /// Configuration file paths to track (can be specified multiple times).
        /// Paths to configuration files that should be managed for this tool.
        #[arg(long, help = "Paths for the configuration files", value_name = "CONFIGURATION_FILE_NAME", num_args(1..))]
        config_paths: Vec<String>,
    },
    /// Add a new font to fonts.yaml configuration.
    /// Fonts are typically downloaded from GitHub releases and installed system-wide.
    Font {
        /// Name of the font to add. This will be used as the identifier in configurations.
        #[arg(long)]
        name: String,
        /// Version of the font (e.g., "3.4.0"). Typically corresponds to the release tag.
        #[arg(long)]
        version: String,
        /// Source type (currently only "github" is supported for fonts).
        #[arg(long, default_value = "github")]
        source: String,
        /// Repository (format: owner/repo).
        /// GitHub repository containing the font files.
        #[arg(long)]
        repo: String,
        /// Release tag.
        /// Specific version tag to download from the repository.
        #[arg(long)]
        tag: String,
        /// Font variants to install (can be specified multiple times, e.g., "regular", "Mono").
        /// Allows selective installation of specific font weights or styles.
        #[arg(long, help = "Only install specific sub-fonts (e.g., 'regular mono bold').", value_name = "SUB_FONT_NAMES", num_args(1..))]
        install_only: Vec<String>,
    },
    /// Add a new setting to settings.yaml configuration (currently macOS only).
    /// System settings are applied using macOS defaults system.
    Setting {
        /// Domain for the setting (e.g., NSGlobalDomain, com.apple.finder).
        /// The preference domain where the setting should be applied.
        #[arg(long)]
        domain: String,
        /// Setting key name.
        /// The specific preference key to modify.
        #[arg(long)]
        key: String,
        /// Setting value.
        /// The value to set for the specified preference key.
        #[arg(long)]
        value: String,
        /// Value type [bool, string, int, float].
        /// Data type of the setting value for proper serialization.
        #[arg(long)]
        value_type: ValueType,
    },
    /// Add a new alias to shellrc.yaml configuration.
    /// Shell aliases are shortcuts for commonly used commands.
    Alias {
        /// Alias name (the command shortcut).
        /// The short name that will be used to invoke the command.
        #[arg(long)]
        name: String,
        /// Alias value (the command it expands to).
        /// The full command that the alias will execute.
        #[arg(long)]
        value: String,
    },
}

#[derive(Subcommand)]
pub enum RemoveCommands {
    /// Remove an installed tool
    Tool {
        /// Name of the tool to remove
        name: String,
    },

    /// Remove an installed font
    Font {
        /// Name of the font to remove
        name: String,
    },

    /// Remove a shell alias
    Alias {
        /// Name of the alias to remove
        name: String,
    },

    /// Remove a system setting
    Setting {
        /// Domain of the setting (e.g., "NSGlobalDomain")
        domain: String,

        /// Key of the setting
        key: String,
    },
}

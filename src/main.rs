//! # Setup DevBox - Main Application Entry Point
//!
//! This is the core of the `setup-devbox` application - a comprehensive toolchain
//! and development environment management system. The application provides a unified
//! interface for installing, configuring, and managing development tools, fonts,
//! system settings, and shell configurations across multiple platforms.
//!
//! ## Core Architecture
//!
//! The application follows a modular architecture with clear separation of concerns:
//!
//! - **Command Dispatch**: Parses CLI arguments and routes to appropriate subcommands
//! - **Configuration Management**: Handles YAML-based configuration files for tools, fonts, settings, and aliases
//! - **Installation System**: Modular installers for different package managers and sources
//! - **State Tracking**: Maintains installation state in JSON format for idempotent operations
//! - **Cross-Platform Support**: Works on macOS, Linux, and supports multiple shell environments
//!
//! ## Key Features
//!
//! - **Unified Tool Management**: Install tools from multiple sources (Homebrew, GitHub, Cargo, Rustup, etc.)
//! - **Font Installation**: Download and install fonts from GitHub releases
//! - **System Configuration**: Apply OS-level settings (macOS focused)
//! - **Shell Customization**: Manage shell aliases and configuration files
//! - **Idempotent Operations**: Smart state tracking prevents unnecessary reinstallations
//! - **Extensible Architecture**: Easy to add new installers and configuration types
//!
//! ## Module Structure
//!
//! - `commands/`: Individual subcommand implementations (now, generate, sync, etc.)
//! - `installers/`: Tool-specific installation logic (brew, cargo, github, rustup, etc.)
//! - `schemas/`: Configuration file structures and validation
//! - `libs/`: Shared utilities and helper functions
//! - `logger/`: Application logging system with debug support
//!
//! ## Configuration Files
//!
//! The application uses YAML configuration files stored in `~/.setup-devbox/`:
//!
//! - `tools.yaml`: Tool definitions with sources, versions, and installation options
//! - `fonts.yaml`: Font specifications from GitHub releases
//! - `settings.yaml`: OS-level configuration settings (macOS domains and keys)
//! - `shellrc.yaml`: Shell aliases and configuration snippets
//! - `state.json`: Installation state tracking for idempotent operations
//!
//! ## Command Line Interface
//!
//! The CLI uses a hierarchical command structure with comprehensive help system:
//!
//! ```text
//! setup-devbox [OPTIONS] <COMMAND>
//!
//! Commands:
//!   now          Installs and Configures Tools, Fonts, OS Settings and Shell Configs
//!   generate     Generates default configuration files
//!   sync         Synchronizes or generates configurations from a state file
//!   edit         Edit configuration files or state file in your preferred editor
//!   add          Add a new tool, font, setting, or alias to configuration files
//!   help         Show detailed help for commands and installers
//!   version      Show the current Version of the tool
//! ```
//!
//! ## Error Handling
//!
//! The application provides detailed error messages with color-coded output:
//! - **Info**: General operation progress
//! - **Debug**: Detailed execution flow (enabled with `--debug` flag)
//! - **Warn**: Non-fatal issues or recommendations
//! - **Error**: Critical failures with specific error context
//!
//! ## Installation Workflow
//!
//! 1. **Configuration**: User defines tools, fonts, and settings in YAML files
//! 2. **Validation**: Configuration files are parsed and validated
//! 3. **State Check**: Current installation state is compared against desired state
//! 4. **Execution**: Only necessary installations and configurations are applied
//! 5. **State Update**: Installation results are recorded for future runs
//! 6. **Verification**: Successful installation is confirmed

// This is the core of the `setup-devbox` application.
// It parses command-line arguments and dispatches to the appropriate subcommand logic.

// Import necessary internal modules.
mod commands; // Handles individual subcommand logic (e.g., 'now', 'generate', 'sync').
mod help_details;
mod installers; // Contains logic for software installation.
mod libs;
mod logger; // Manages application logging.
mod schemas; // Defines configuration file structures.

use colored::Colorize;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

// Import specific `run` functions from the `commands` module.
use crate::commands::{add, edit, help};
// Use 'clap' for command-line argument parsing.
use crate::schemas::path_resolver::PathResolver;
use clap::{Parser, Subcommand};
use commands::{generate, now, sync, version};
// ============================================================================
// Constants and Environment Variables
// ============================================================================

// /// Environment variable name for custom DevBox configuration path
// const ENV_SDB_CONFIG_PATH: &str = "SDB_CONFIG_PATH";
// /// Environment variable name for custom DevBox configuration path
// const ENV_SDB_TOOLS_SOURCE_CONFIG_PATH: &str = "SDB_TOOLS_SOURCE_CONFIG_PATH";
//
// /// Default DevBox directory name (relative to home directory)
// const DEFAULT_DEVBOX_DIR: &str = ".setup-devbox";

/// Defines the command-line interface (CLI) for 'setup-devbox'.
/// `#[derive(Parser)]` automatically generates argument parsing code via `clap`.
#[derive(Parser)]
#[command(name = "setup-devbox")]
#[command(disable_help_subcommand = true)] // Disables the built-in help subcommand to use custom implementation
#[command(disable_help_flag = true)] // Disables the built-in help flag to use custom implementation
struct Cli {
    /// Enables detailed debug output for troubleshooting and development.
    #[arg(short, long)]
    debug: bool,

    /// Defines available subcommands for 'setup-devbox'.
    #[command(subcommand)]
    command: Commands,
}

/// Enumerates all supported subcommands with their specific arguments and options.
/// Each variant represents a distinct functionality of the setup-devbox application.
#[derive(Subcommand)]
enum Commands {
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
        state: Option<PathBuf>,
        /// Optional output directory for generated configuration files (defaults to ~/.setup-devbox/configs).
        #[arg(long)]
        output_dir: Option<PathBuf>,
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
enum AddCommands {
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

/// Defines the set of valid configuration types that can be edited.
/// Each variant corresponds to a specific configuration file.
#[derive(Debug, Clone)]
pub enum ConfigType {
    Tools,    // tools.yaml - Tool definitions and installation specifications
    Fonts,    // fonts.yaml - Font specifications and installation details
    Shell,    // shellrc.yaml - Shell aliases and configuration snippets
    Settings, // settings.yaml - System settings and preferences
}

/// Implementation of string parsing for ConfigType enum.
/// Allows converting string arguments to strongly-typed ConfigType values.
impl FromStr for ConfigType {
    type Err = String;

    /// Parses a string into a ConfigType enum variant.
    ///
    /// # Arguments
    /// * `s` - The string to parse (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(ConfigType)` if the string matches a valid configuration type
    /// * `Err(String)` with error message if no match found
    ///
    /// # Examples
    /// ```
    /// use std::str::FromStr;
    /// use ConfigType;
    ///
    /// let config = ConfigType::from_str("tools").unwrap(); // ConfigType::Tools
    /// let config = ConfigType::from_str("FONTS").unwrap(); // ConfigType::Fonts (case-insensitive)
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tools" => Ok(ConfigType::Tools),
            "fonts" => Ok(ConfigType::Fonts),
            "shell" => Ok(ConfigType::Shell),
            "settings" => Ok(ConfigType::Settings),
            _ => {
                let valid_types = ["tools", "fonts", "shell", "settings"].join(", ");
                Err(format!(
                    "Invalid config type '{s}'. Must be one of: {valid_types}",
                ))
            }
        }
    }
}

/// Implementation of display formatting for ConfigType enum.
/// Provides human-readable string representation for each configuration type.
impl fmt::Display for ConfigType {
    /// Formats the ConfigType as a string for display purposes.
    ///
    /// # Arguments
    /// * `f` - Formatter to write the output
    ///
    /// # Returns
    /// `fmt::Result` indicating success or failure of formatting
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigType::Tools => write!(f, "tools"),
            ConfigType::Fonts => write!(f, "fonts"),
            ConfigType::Shell => write!(f, "shell"),
            ConfigType::Settings => write!(f, "settings"),
        }
    }
}

/// Defines the set of valid installation/source methods for tools.
/// Each variant corresponds to a different installation backend or package manager.
#[derive(Debug, Clone)]
pub enum SourceType {
    Brew,   // Homebrew package manager (macOS/Linux)
    Cargo,  // Rust package manager
    Github, // GitHub releases and repositories
    Go,     // Go language tooling
    Rustup, // Rust toolchain manager
    Url,    // Direct URL downloads
    Uv,     // Python package manager
    Pip,    // Python package installer
}

/// Implementation of string parsing for SourceType enum.
/// Allows converting string arguments to strongly-typed SourceType values.
impl FromStr for SourceType {
    type Err = String;

    /// Parses a string into a SourceType enum variant.
    ///
    /// # Arguments
    /// * `s` - The string to parse (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(SourceType)` if the string matches a valid source type
    /// * `Err(String)` with error message if no match found
    ///
    /// # Examples
    /// ```
    /// use std::str::FromStr;
    /// use SourceType;
    ///
    /// let source = SourceType::from_str("brew").unwrap(); // SourceType::Brew
    /// let source = SourceType::from_str("GITHUB").unwrap(); // SourceType::Github (case-insensitive)
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "brew" => Ok(SourceType::Brew),
            "cargo" => Ok(SourceType::Cargo),
            "github" => Ok(SourceType::Github),
            "go" => Ok(SourceType::Go),
            "rustup" => Ok(SourceType::Rustup),
            "url" => Ok(SourceType::Url),
            "uv" => Ok(SourceType::Uv),
            "pip" => Ok(SourceType::Pip),
            _ => {
                let valid_types = [
                    "brew", "cargo", "github", "go", "rustup", "url", "uv", "pip",
                ]
                .join(", ");
                Err(format!(
                    "Invalid source type '{s}'. Must be one of: {valid_types}"
                ))
            }
        }
    }
}

/// Implementation of display formatting for SourceType enum.
/// Provides human-readable string representation for each source type.
impl fmt::Display for SourceType {
    /// Formats the SourceType as a string for display purposes.
    ///
    /// # Arguments
    /// * `f` - Formatter to write the output
    ///
    /// # Returns
    /// `fmt::Result` indicating success or failure of formatting
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SourceType::Brew => write!(f, "brew"),
            SourceType::Cargo => write!(f, "cargo"),
            SourceType::Github => write!(f, "github"),
            SourceType::Go => write!(f, "go"),
            SourceType::Rustup => write!(f, "rustup"),
            SourceType::Url => write!(f, "url"),
            SourceType::Uv => write!(f, "uv"),
            SourceType::Pip => write!(f, "pip"),
        }
    }
}

/// Defines the set of valid configuration value types for system settings.
/// Used for type-safe serialization and validation of setting values.
#[derive(Debug, Clone)]
pub enum ValueType {
    Bool,   // Boolean values (true/false)
    String, // String values
    Int,    // Integer values
    Float,  // Floating-point values
}

/// Implementation of string parsing for ValueType enum.
/// Allows converting string arguments to strongly-typed ValueType values.
impl FromStr for ValueType {
    type Err = String;

    /// Parses a string into a ValueType enum variant.
    ///
    /// # Arguments
    /// * `s` - The string to parse (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(ValueType)` if the string matches a valid value type
    /// * `Err(String)` with error message if no match found
    ///
    /// # Examples
    /// ```
    /// use std::str::FromStr;
    /// use ValueType;
    ///
    /// let value_type = ValueType::from_str("bool").unwrap(); // ValueType::Bool
    /// let value_type = ValueType::from_str("INT").unwrap(); // ValueType::Int (case-insensitive)
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bool" => Ok(ValueType::Bool),
            "string" => Ok(ValueType::String),
            "int" => Ok(ValueType::Int),
            "float" => Ok(ValueType::Float),
            _ => {
                let valid_types = ["bool", "string", "int", "float"].join(", ");
                Err(format!(
                    "Invalid value type '{s}'. Must be one of: {valid_types}"
                ))
            }
        }
    }
}

/// Implementation of display formatting for ValueType enum.
/// Provides human-readable string representation for each value type.
impl fmt::Display for ValueType {
    /// Formats the ValueType as a string for display purposes.
    ///
    /// # Arguments
    /// * `f` - Formatter to write the output
    ///
    /// # Returns
    /// `fmt::Result` indicating success or failure of formatting
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueType::Bool => write!(f, "bool"),
            ValueType::String => write!(f, "string"),
            ValueType::Int => write!(f, "int"),
            ValueType::Float => write!(f, "float"),
        }
    }
}

/// Main entry point of the application.
///
/// This function serves as the application's starting point and performs the following:
/// 1. Parses command-line arguments
/// 2. Initializes the logging system
/// 3. Routes to the appropriate subcommand handler
/// 4. Handles global error conditions
///
/// # Returns
/// * `Ok(())` if the application completes successfully
/// * `Err(Box<dyn std::error::Error>)` if any error occurs during execution
///
/// # Error Handling
/// The function uses Rust's error propagation with `?` operator and provides
/// comprehensive error messages for different failure scenarios.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect all command-line arguments for processing
    let args: Vec<String> = std::env::args().collect();

    // Check if help flag is anywhere in the arguments
    // This custom help handling allows for more flexible help display than clap's default
    if let Some(help_index) = args.iter().position(|arg| arg == "--help" || arg == "-h") {
        // Initialize logger without debug mode for help display
        logger::init(false);

        // Determine if help is requested for a specific topic
        // This allows commands like: `setup-devbox now --help`
        let topic = if help_index > 1 {
            // Check if there's a subcommand before the help flag
            let potential_topic = &args[help_index - 1];
            if !potential_topic.starts_with('-') && potential_topic != "help" {
                Some(potential_topic.clone())
            } else {
                None
            }
        } else {
            None
        };

        log_debug!("[SDB] Help flag detected, topic: {:?}", topic);
        // Display help and exit immediately
        help::run(topic, false, None);
        std::process::exit(0);
    }

    // Parse command-line arguments into the `Cli` structure using clap
    let cli = Cli::parse();
    // Initialize the logger based on the debug flag from command line
    logger::init(cli.debug);
    log_debug!("[SDB] Command line arguments successfully parsed.");
    log_debug!("[SDB] Debug mode requested: {}", cli.debug);

    // Dispatch control based on the detected subcommand
    // Each branch handles a specific functionality of the application
    match cli.command {
        Commands::Add { add_type } => {
            log_debug!("[SDB] 'Add' subcommand detected.");

            // Handle different types of add operations
            match add_type {
                AddCommands::Tool {
                    name,
                    version,
                    source,
                    url,
                    repo,
                    tag,
                    rename_to,
                    options,
                    executable_path_after_extract,
                    post_installation_hooks,
                    enable_config_manager,
                    config_paths,
                } => {
                    log_debug!("[SDB] 'Add Tool' subcommand detected.");
                    log_debug!("[SDB] Tool name: {}", name);
                    log_debug!("[SDB] Tool version: {}", version);
                    log_debug!("[SDB] Tool source: {:?}", source);
                    log_debug!("[SDB] Tool URL: {:?}", url);
                    log_debug!("[SDB] Tool repo: {:?}", repo);
                    log_debug!("[SDB] Tool tag: {:?}", tag);
                    log_debug!("[SDB] Rename to: {:?}", rename_to);
                    log_debug!("[SDB] Options: {:?}", options);
                    log_debug!(
                        "[SDB] Executable path after extract: {:?}",
                        executable_path_after_extract
                    );
                    log_debug!(
                        "[SDB] Post installation hooks: {:?}",
                        post_installation_hooks
                    );
                    log_debug!("[SDB] Enable config manager: {}", enable_config_manager);
                    log_debug!("[SDB] Config paths: {:?}", config_paths);

                    // Convert SourceType enum to String for the add_tool function
                    let source_string = source.to_string();

                    // Call the add_tool function with all collected parameters
                    add::add_tool(
                        name,
                        version,
                        source_string,
                        url,
                        repo,
                        tag,
                        rename_to,
                        options,
                        None,
                        post_installation_hooks,
                        enable_config_manager,
                        config_paths,
                    );
                }
                AddCommands::Font {
                    name,
                    version,
                    source,
                    repo,
                    tag,
                    install_only,
                } => {
                    log_debug!("[SDB] 'Add Font' subcommand detected.");
                    log_debug!("[SDB] Font name: {}", name);
                    log_debug!("[SDB] Font version: {}", version);
                    log_debug!("[SDB] Font source: {}", source);
                    log_debug!("[SDB] Font repo: {}", repo);
                    log_debug!("[SDB] Font tag: {}", tag);
                    log_debug!("[SDB] Install only: {:?}", install_only);

                    // Call the add_font function with font-specific parameters
                    add::add_font(name, version, source, repo, tag, install_only);
                }
                AddCommands::Setting {
                    domain,
                    key,
                    value,
                    value_type,
                } => {
                    log_debug!("[SDB] 'Add Setting' subcommand detected.");
                    log_debug!("[SDB] Setting domain: {}", domain);
                    log_debug!("[SDB] Setting key: {}", key);
                    log_debug!("[SDB] Setting value: {}", value);
                    log_debug!("[SDB] Setting type: {}", value_type);

                    // Convert ValueType enum to String for the add_setting function
                    let value_type_string = value_type.to_string();

                    // Call the add_setting function with system setting parameters
                    add::add_setting(domain, key, value, value_type_string);
                }
                AddCommands::Alias { name, value } => {
                    log_debug!("[SDB] 'Add Alias' subcommand detected.");
                    log_debug!("[SDB] Alias name: {}", name);
                    log_debug!("[SDB] Alias value: {}", value);

                    // Call the add_alias function with shell alias parameters
                    add::add_alias(name, value);
                }
            }
        }
        Commands::Edit { state, config } => {
            log_debug!("[SDB] 'Edit' subcommand detected.");
            log_debug!("[SDB] Edit state flag: {}", state);
            log_debug!("[SDB] Edit config type: {:?}", config);

            // Ensure either --state or --config is provided, but not both
            // This validation prevents ambiguous command usage
            if !state && config.is_none() {
                eprintln!(
                    "{}",
                    "Error: You must specify either --state or --config <type>".red()
                );
                eprintln!("Usage:");
                eprintln!("  setup-devbox edit --state");
                eprintln!("  setup-devbox edit --config <tools|fonts|shell|settings>");
                std::process::exit(1);
            }

            // Convert ConfigType to String for the edit::run function
            let config_str = config.map(|c| c.to_string());
            // Call the edit function with the specified target
            edit::run(state, config_str);
        }
        Commands::Generate { config, state } => {
            log_debug!("[SDB] 'Generate' subcommand detected.");

            // Initialize path resolver with command overrides for custom file locations
            let paths = PathResolver::new(config, state)?;

            log_debug!(
                "[SDB] 'Generate' subcommand using config dir: {}",
                paths.configs_dir().display()
            );
            log_debug!(
                "[SDB] 'Generate' subcommand using state file: {}",
                paths.state_file().display()
            );

            // Generate default configuration files at the specified locations
            generate::run(paths.configs_dir(), paths.state_file().to_path_buf());
        }
        Commands::Help {
            topic,
            detailed,
            filter,
        } => {
            log_debug!("[main] 'Help' subcommand detected.");
            log_debug!("[main] Help topic: {:?}", topic);
            log_debug!("[main] Detailed mode: {}", detailed);
            log_debug!("[main] Filter: {:?}", filter);
            // Display comprehensive help information
            help::run(topic, detailed, filter);
        }
        Commands::Now {
            config,
            state,
            update_latest,
        } => {
            log_debug!("[SDB] 'Now' subcommand detected.");

            // Initialize path resolver with command overrides for custom file locations
            let paths = PathResolver::new(config, state)?;

            log_debug!(
                "[SDB] 'Now' subcommand using config file: {}",
                paths.config_file().display()
            );
            log_debug!(
                "[SDB] 'Now' subcommand using state file: {}",
                paths.state_file().display()
            );

            // Execute the main installation and configuration process
            // Pass the PathResolver to provide consistent file path resolution
            now::run(&paths, update_latest);
        }
        Commands::SyncConfig { state, output_dir } => {
            log_debug!("[SDB] 'SyncConfig' subcommand detected.");
            // Package arguments for the sync operation
            let args = sync::SyncConfigArgs { state, output_dir };
            // Synchronize configurations from state file
            sync::run(args);
        }
        Commands::Version => {
            log_debug!("[SDB] 'Version' subcommand detected. Calling version::run().");
            // Display application version information
            version::run();
        }
    }

    log_debug!("[SDB] Command execution completed. Exiting application.");
    Ok(())
}

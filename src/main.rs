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
use clap::{Parser, Subcommand};
use commands::{generate, now, sync, version};

/// Defines the command-line interface (CLI) for 'setup-devbox'.
/// `#[derive(Parser)]` automatically generates argument parsing code via `clap`.
#[derive(Parser)]
#[command(name = "setup-devbox")]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
struct Cli {
    /// Enables detailed debug output.
    #[arg(short, long)]
    debug: bool,

    /// Defines available subcommands for 'setup-devbox'.
    #[command(subcommand)]
    command: Commands,
}

/// Enumerates all supported subcommands.
#[derive(Subcommand)]
enum Commands {
    /// Show the current Version of the tool.
    Version,
    /// Installs and Configures Tools, Fonts, OS Settings and Shell Configs.
    Now {
        /// Optional path to a custom configuration file.
        #[arg(long)]
        config: Option<String>,
        /// Optional path to a custom state file.
        #[arg(long)]
        state: Option<String>,
        /// Force update all tools with version "latest", overriding `update_latest_only_after` policy
        #[arg(long)]
        update_latest: bool,
    },
    /// Generates default configuration files.
    Generate {
        /// Optional path to save the generated configuration.
        #[arg(long)]
        config: Option<String>,
        /// Optional path to save the generated state file.
        #[arg(long)]
        state: Option<String>,
    },
    /// Synchronizes or generates configurations from a state file.
    SyncConfig {
        /// Optional path to the state file (defaults to ~/.setup-devbox/state.json).
        #[arg(long)]
        state: Option<PathBuf>,
        /// Optional output directory for generated configuration files (defaults to ~/.setup-devbox/configs).
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
    /// Edit configuration files or state file in your preferred editor.
    Edit {
        /// Edit the state file (break glass mechanism - use with caution).
        #[arg(long, conflicts_with = "config")]
        state: bool,
        /// Edit a specific configuration file [possible values: tools, fonts, shell, settings].
        #[arg(long, conflicts_with = "state")]
        config: Option<ConfigType>,
    },
    /// Add a new tool, font, setting, or alias to configuration files.
    Add {
        #[command(subcommand)]
        add_type: AddCommands,
    },
    /// Show detailed help for commands and installers.
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

#[derive(Subcommand)]
enum AddCommands {
    /// Add a new tool to tools.yaml configuration.
    Tool {
        /// Name of the tool to add.
        #[arg(long)]
        name: String,
        /// Version of the tool (e.g., "1.0.0" or "latest").
        #[arg(long)]
        version: String,
        /// Source type [brew, github, rustup, cargo, pip, go, url, uv].
        #[arg(long)]
        source: SourceType,
        /// URL
        #[arg(long)]
        url: Option<String>,
        /// Repository (required for github source, format: owner/repo).
        #[arg(long)]
        repo: Option<String>,
        /// Release tag (required for github source).
        #[arg(long)]
        tag: Option<String>,
        /// Rename the binary to a different name (optional).
        #[arg(long)]
        rename_to: Option<String>,
        /// Additional Options for installation
        #[arg(long)]
        options: Option<Vec<String>>,
        /// Additional commands to run after installation (can be specified multiple times).
        #[arg(long)]
        executable_path_after_extract: Vec<String>,
        /// Post installation hooks
        #[arg(long)]
        post_installation_hooks: Option<Vec<String>>,
        /// Enable configuration manager tracking.
        #[arg(long)]
        enable_config_manager: bool,
        /// Configuration file paths to track (can be specified multiple times).
        #[arg(long, help = "Paths for the configuration files", value_name = "CONFIGURATION_FILE_NAME", num_args(1..))]
        config_paths: Vec<String>,
    },
    /// Add a new font to fonts.yaml configuration.
    Font {
        /// Name of the font to add.
        #[arg(long)]
        name: String,
        /// Version of the font (e.g., "3.4.0").
        #[arg(long)]
        version: String,
        /// Source type (currently only "github" is supported).
        #[arg(long, default_value = "github")]
        source: String,
        /// Repository (format: owner/repo).
        #[arg(long)]
        repo: String,
        /// Release tag.
        #[arg(long)]
        tag: String,
        /// Font variants to install (can be specified multiple times, e.g., "regular", "Mono").
        #[arg(long, help = "Only install specific sub-fonts (e.g., 'regular mono bold').", value_name = "SUB_FONT_NAMES", num_args(1..))]
        install_only: Vec<String>,
    },
    /// Add a new setting to settings.yaml configuration (currently macOS only).
    Setting {
        /// Domain for the setting (e.g., NSGlobalDomain, com.apple.finder).
        #[arg(long)]
        domain: String,
        /// Setting key name.
        #[arg(long)]
        key: String,
        /// Setting value.
        #[arg(long)]
        value: String,
        /// Value type [bool, string, int, float].
        #[arg(long)]
        value_type: ValueType,
    },
    /// Add a new alias to shellrc.yaml configuration.
    Alias {
        /// Alias name (the command shortcut).
        #[arg(long)]
        name: String,
        /// Alias value (the command it expands to).
        #[arg(long)]
        value: String,
    },
}

/// Defines the set of valid configuration types.
#[derive(Debug, Clone)]
pub enum ConfigType {
    Tools,
    Fonts,
    Shell,
    Settings,
}

impl FromStr for ConfigType {
    type Err = String;

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

impl fmt::Display for ConfigType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigType::Tools => write!(f, "tools"),
            ConfigType::Fonts => write!(f, "fonts"),
            ConfigType::Shell => write!(f, "shell"),
            ConfigType::Settings => write!(f, "settings"),
        }
    }
}

/// Defines the set of valid installation/source methods.
#[derive(Debug, Clone)]
pub enum SourceType {
    Brew,
    Cargo,
    Github,
    Go,
    Rustup,
    Url,
    Uv,
    Pip,
}

impl FromStr for SourceType {
    type Err = String;

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

impl fmt::Display for SourceType {
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

/// Defines the set of valid configuration value types.
#[derive(Debug, Clone)]
pub enum ValueType {
    Bool,
    String,
    Int,
    Float,
}

impl FromStr for ValueType {
    type Err = String;

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

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueType::Bool => write!(f, "bool"),
            ValueType::String => write!(f, "string"),
            ValueType::Int => write!(f, "int"),
            ValueType::Float => write!(f, "float"),
        }
    }
}

// Main entry point of the application.
fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Check if help flag is anywhere in the arguments
    if let Some(help_index) = args.iter().position(|arg| arg == "--help" || arg == "-h") {
        logger::init(false);

        // Determine if help is requested for a specific topic
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
        help::run(topic, false, None);
        std::process::exit(0);
    }

    // Parse command-line arguments into the `Cli` structure.
    let cli = Cli::parse();
    // Initialize the logger based on the debug flag.
    logger::init(cli.debug);
    log_debug!("[SDB] Command line arguments successfully parsed.");
    log_debug!("[SDB] Debug mode requested: {}", cli.debug);

    // Dispatch control based on the detected subcommand.
    match cli.command {
        Commands::Add { add_type } => {
            log_debug!("[SDB] 'Add' subcommand detected.");

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

                    add::add_setting(domain, key, value, value_type_string);
                }
                AddCommands::Alias { name, value } => {
                    log_debug!("[SDB] 'Add Alias' subcommand detected.");
                    log_debug!("[SDB] Alias name: {}", name);
                    log_debug!("[SDB] Alias value: {}", value);

                    add::add_alias(name, value);
                }
            }
        }
        Commands::Edit { state, config } => {
            log_debug!("[SDB] 'Edit' subcommand detected.");
            log_debug!("[SDB] Edit state flag: {}", state);
            log_debug!("[SDB] Edit config type: {:?}", config);

            // Ensure either --state or --config is provided, but not both
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
            edit::run(state, config_str);
        }
        Commands::Generate { config, state } => {
            log_debug!("[SDB] 'Generate' subcommand detected.");
            log_debug!(
                "[SDB] 'Generate' subcommand received config path: {:?}",
                config
            );
            log_debug!(
                "[SDB] 'Generate' subcommand received state path: {:?}",
                state
            );
            generate::run(config, state);
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
            help::run(topic, detailed, filter);
        }
        Commands::Now {
            config,
            state,
            update_latest,
        } => {
            log_debug!("[SDB] 'Now' subcommand detected.");
            log_debug!("[SDB] 'Now' subcommand received config path: {:?}", config);
            log_debug!("[SDB] 'Now' subcommand received state path: {:?}", state);
            now::run(config, state, update_latest);
        }
        Commands::SyncConfig { state, output_dir } => {
            log_debug!("[main] 'SyncConfig' subcommand detected.");
            let args = sync::SyncConfigArgs { state, output_dir };
            sync::run(args);
        }
        Commands::Version => {
            log_debug!("[SDB] 'Version' subcommand detected. Calling version::run().");
            version::run();
        }
    }
    log_debug!("[SDB] Command execution completed. Exiting application.");
}

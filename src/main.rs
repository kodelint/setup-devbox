// This is the core of the `setup-devbox` application.
// It parses command-line arguments and dispatches to the appropriate subcommand logic.

// Import necessary internal modules.
mod commands; // Handles individual subcommand logic (e.g., 'now', 'generate', 'sync').
mod help_details;
mod installers; // Contains logic for software installation.
mod libs;
mod logger; // Manages application logging.
mod schemas;
// Defines configuration file structures. // General utility functions/libraries.

use colored::Colorize;
use std::path::PathBuf; // Used for colored terminal output in logs.
// Standard library module for interacting with environment variables. (This comment was already good)

// Use 'clap' for command-line argument parsing.
use clap::{Parser, Subcommand};
// Import specific `run` functions from the `commands` module.
use crate::commands::{edit, help};
use commands::{generate, now, sync, version};

/// Defines the command-line interface (CLI) for 'setup-devbox'.
/// `#[derive(Parser)]` automatically generates argument parsing code via `clap`.
#[derive(Parser)]
#[command(name = "setup-devbox")]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
struct Cli {
    // Global argument to enable debug logging.
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
        #[arg(long, value_parser = validate_config_type, conflicts_with = "state")]
        config: Option<String>,
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

/// Validates that the config type is one of the allowed values.
fn validate_config_type(s: &str) -> Result<String, String> {
    let valid_types = ["tools", "fonts", "shell", "settings"];
    if valid_types.contains(&s) {
        Ok(s.to_string())
    } else {
        Err(format!(
            "Invalid config type '{}'. Must be one of: {}",
            s,
            valid_types.join(", ")
        ))
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
        Commands::Edit { state, config } => {
            log_debug!("[SDB] 'Edit' subcommand detected.");
            log_debug!("[SDB] Edit state flag: {}", state);
            log_debug!("[SDB] Edit config type: {:?}", config);

            // Ensure either --state or --config is provided, but not both
            if !state && config.is_none() {
                eprintln!("{}", "Error: You must specify either --state or --config <type>".red());
                eprintln!("Usage:");
                eprintln!("  setup-devbox edit --state");
                eprintln!("  setup-devbox edit --config <tools|fonts|shell|settings>");
                std::process::exit(1);
            }

            edit::run(state, config);
        },
        Commands::Generate { config, state } => {
            log_debug!("[SDB] 'Generate' subcommand detected.");
            log_debug!("[SDB] 'Generate' subcommand received config path: {:?}", config);
            log_debug!("[SDB] 'Generate' subcommand received state path: {:?}", state);
            generate::run(config, state);
        },
        Commands::Help { topic, detailed, filter } => {
            log_debug!("[main] 'Help' subcommand detected.");
            log_debug!("[main] Help topic: {:?}", topic);
            log_debug!("[main] Detailed mode: {}", detailed);
            log_debug!("[main] Filter: {:?}", filter);
            help::run(topic, detailed, filter);
        },
        Commands::Now { config, state, update_latest } => {
            log_debug!("[SDB] 'Now' subcommand detected.");
            log_debug!("[SDB] 'Now' subcommand received config path: {:?}", config);
            log_debug!("[SDB] 'Now' subcommand received state path: {:?}", state);
            now::run(config, state, update_latest);
        },
        Commands::SyncConfig { state, output_dir } => {
            log_debug!("[main] 'SyncConfig' subcommand detected.");
            let args = sync::SyncConfigArgs { state, output_dir };
            sync::run(args);
        },
        Commands::Version => {
            log_debug!("[SFB] 'Version' subcommand detected. Calling version::run().");
            version::run();
        },
    }
    log_debug!("[SDB] Command execution completed. Exiting application.");
}

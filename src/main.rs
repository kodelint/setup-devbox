// This is the core of the `setup-devbox` application.
// It parses command-line arguments and dispatches to the appropriate subcommand logic.

// Import necessary internal modules.
mod commands; // Handles individual subcommand logic (e.g., 'now', 'generate', 'sync').
mod installers; // Contains logic for software installation.
mod libs;
mod logger; // Manages application logging.
mod schema; // Defines configuration file structures. // General utility functions/libraries.

use colored::Colorize;
use std::path::PathBuf; // Used for colored terminal output in logs.
// Standard library module for interacting with environment variables. (This comment was already good)

// Use 'clap' for command-line argument parsing.
use clap::{Parser, Subcommand};
// Import specific `run` functions from the `commands` module.
use commands::{generate, now, sync, version};

/// Defines the command-line interface (CLI) for 'setup-devbox'.
/// `#[derive(Parser)]` automatically generates argument parsing code via `clap`.
#[derive(Parser)]
#[command(name = "setup-devbox")]
#[command(about = "Setup development environment with ease", long_about = None)]
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
        /// Force update all tools with version "latest", overriding update_latest_only_after policy
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
}

// Main entry point of the application.
fn main() {
    // Parse command-line arguments into the `Cli` structure.
    let cli = Cli::parse();
    // Initialize the logger based on the debug flag.
    logger::init(cli.debug);
    log_debug!("[main] Command line arguments successfully parsed.");
    log_debug!("[main] Debug mode requested: {}", cli.debug);

    // Dispatch control based on the detected subcommand.
    match cli.command {
        Commands::Version => {
            log_debug!("[main] 'Version' subcommand detected. Calling version::run().");
            version::run();
        }
        Commands::Now {
            config,
            state,
            update_latest,
        } => {
            log_debug!("[main] 'Now' subcommand detected.");
            log_debug!("[main] 'Now' subcommand received config path: {:?}", config);
            log_debug!("[main] 'Now' subcommand received state path: {:?}", state);
            now::run(config, state, update_latest);
        }
        Commands::Generate { config, state } => {
            log_debug!("[main] 'Generate' subcommand detected.");
            log_debug!(
                "[main] 'Generate' subcommand received config path: {:?}",
                config
            );
            log_debug!(
                "[main] 'Generate' subcommand received state path: {:?}",
                state
            );
            generate::run(config, state);
        }
        Commands::SyncConfig { state, output_dir } => {
            log_debug!("[main] 'SyncConfig' subcommand detected.");
            let args = sync::SyncConfigArgs { state, output_dir };
            sync::run(args);
        }
    }
    log_debug!("[main] Command execution completed. Exiting application.");
}

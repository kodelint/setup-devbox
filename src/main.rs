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
//! - `commands/`: Individual subcommand implementations (now, bootstrap, sync, etc.)
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
//!   bootstrap    Bootstraps the development environment by generating default configurations and installing Homebrew
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

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

/// CLI argument structures and type definitions
mod cli;
/// Individual subcommand implementations
mod commands;
// Detailed help text and documentation

// Tool-specific installation logic

mod config;
/// Shared utility functions
mod core;
mod engine;
mod fonts;
mod settings;
mod shell;
mod state;

/// Application logging system
mod logger;
/// Configuration file structures
mod schemas;

// ============================================================================
// EXTERNAL DEPENDENCIES
// ============================================================================

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

// ============================================================================
// INTERNAL IMPORTS
// ============================================================================

use crate::cli::cmd_enums::{Cli, Commands, RemoveCommands};
use crate::commands::{add, bootstrap, check_updates, edit, help, now, reset, sync, version};
use crate::schemas::path_resolver::PathResolver;

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// Main entry point of the application.
///
/// This function serves as the application's starting point and performs:
/// 1. Parses command-line arguments using clap
/// 2. Initializes the logging system
/// 3. Routes to the appropriate subcommand handler
/// 4. Handles global error conditions
///
/// # Returns
/// * `Ok(())` if the application completes successfully
/// * `Err(anyhow::Error)` if any error occurs during execution
fn main() -> Result<()> {
    // ========================================================================
    // STEP 1: PARSE COMMAND-LINE ARGUMENTS
    // ========================================================================
    // Use clap to parse the arguments into our structured Cli type.
    // This handles validation, type conversion, and error messages for
    // malformed arguments.
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("-h")
                || error_str.contains("--help")
                || error_str.contains("help")
            {
                eprintln!(
                    "\n{}",
                    "To see available commands and options, please use:"
                        .bold()
                        .yellow()
                );
                eprintln!("  {} help", "setup-devbox".cyan());
                std::process::exit(0);
            }
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // ========================================================================
    // STEP 2: INITIALIZE LOGGING SYSTEM
    // ========================================================================
    // Set up the logger based on the --debug flag.
    logger::init(cli.debug);
    log_debug!("[SDB] Command line arguments successfully parsed.");
    log_debug!("[SDB] Debug mode requested: {}", cli.debug);

    // ========================================================================
    // STEP 3: COMMAND DISPATCH
    // ========================================================================
    // Route to the appropriate subcommand handler based on parsed command.
    // Each match arm handles a different subcommand with its specific arguments.
    match cli.command {
        // ====================================================================
        // ADD COMMAND - Add items to configuration files
        // ====================================================================
        Commands::Add { add_type } => {
            log_debug!("[SDB] 'Add' subcommand detected.");
            add::run(add_type);
        }
        // ====================================================================
        // REMOVE COMMAND - Remove items from system and configuration
        // ====================================================================
        Commands::Remove { item } => match item {
            RemoveCommands::Tool { name } => {
                crate::commands::remove::remove_tool(name);
            }
            RemoveCommands::Font { name } => {
                crate::commands::remove::remove_font(name);
            }
            RemoveCommands::Alias { name } => {
                crate::commands::remove::remove_alias(name);
            }
            RemoveCommands::Setting { domain, key } => {
                crate::commands::remove::remove_setting(domain, key);
            }
        },
        // ====================================================================
        // EDIT COMMAND - Open configuration files in editor
        // ====================================================================
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
        // ====================================================================
        // BOOTSTRAP COMMAND - Create default configuration files and initial setup
        // ====================================================================
        Commands::Bootstrap { config } => {
            log_debug!("[SDB] 'Bootstrap' subcommand detected.");

            // Initialize path resolver with command overrides for custom file locations
            let paths = PathResolver::new(config, None).map_err(|e| anyhow::anyhow!(e))?;

            log_debug!(
                "[SDB] 'Bootstrap' subcommand using config dir: {}",
                paths.configs_dir().display()
            );

            // Bootstrap default configuration files at the specified locations
            bootstrap::run(paths.configs_dir());
        }
        // ====================================================================
        // HELP COMMAND - Display comprehensive documentation
        // ====================================================================
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

        // ====================================================================
        // NOW COMMAND - Main installation and configuration workflow
        // ====================================================================
        Commands::Now {
            config,
            state,
            update_latest,
            dry_run,
        } => {
            log_debug!("[SDB] 'Now' subcommand detected.");

            // Initialize path resolver with command overrides for custom file locations
            let paths = PathResolver::new(config, state).map_err(|e| anyhow::anyhow!(e))?;

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
            now::run(&paths, update_latest, dry_run);
        }

        // ====================================================================
        // SYNC CONFIG COMMAND - Generate configs from state file or Gist
        // ====================================================================
        Commands::SyncConfig {
            state,
            output_dir,
            gist,
            github_token,
        } => {
            log_debug!("[SDB] 'SyncConfig' subcommand detected.");
            let paths = PathResolver::new(output_dir, state).map_err(|e| anyhow::anyhow!(e))?;
            sync::run(paths, gist, github_token);
        }

        // ====================================================================
        // VERSION COMMAND - Display version information
        // ====================================================================
        Commands::Version => {
            log_debug!("[SDB] 'Version' subcommand detected. Calling version::run().");
            // Display application version information
            version::run();
        }

        // ====================================================================
        // CHECK UPDATES COMMAND - Check for new versions of tools
        // ====================================================================
        Commands::CheckUpdates => {
            log_debug!("[SDB] 'CheckUpdates' subcommand detected.");
            check_updates::run();
        }

        // ====================================================================
        // RESET COMMAND - Reset installation state
        // ====================================================================
        Commands::Reset { tool, all, state } => {
            log_debug!("[SDB] 'Reset' subcommand detected.");
            reset::run(tool, all, state);
        }
    }

    log_debug!("[SDB] Command execution completed. Exiting application.");
    std::process::exit(0);
}

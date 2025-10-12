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

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

/// CLI argument structures and type definitions
mod cli;
/// Individual subcommand implementations
mod commands;
/// Detailed help text and documentation
mod help_details;
/// Tool-specific installation logic
mod installers;
/// Shared utility functions
mod libs;
/// Application logging system
mod logger;
/// Configuration file structures
mod schemas;

// ============================================================================
// EXTERNAL DEPENDENCIES
// ============================================================================

use clap::Parser;
use colored::Colorize;

// ============================================================================
// INTERNAL IMPORTS
// ============================================================================

use crate::cli::cmd_enums::{AddCommands, Cli, Commands, RemoveCommands};
use crate::commands::{add, edit, help};
use crate::schemas::path_resolver::PathResolver;
use commands::{generate, now, sync, version};

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// Main entry point of the application.
///
/// This function serves as the application's starting point and performs:
/// 1. Custom help flag handling for context-aware help
/// 2. Parses command-line arguments
/// 3. Initializes the logging system
/// 4. Routes to the appropriate subcommand handler
/// 5. Handles global error conditions
///
/// # Returns
/// * `Ok(())` if the application completes successfully
/// * `Err(Box<dyn std::error::Error>)` if any error occurs during execution
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ========================================================================
    // STEP 1: CUSTOM HELP FLAG HANDLING
    // ========================================================================
    // Collect all command-line arguments for preprocessing before clap parsing.
    // This allows us to implement custom help handling logic.
    let args: Vec<String> = std::env::args().collect();

    // Check if --help or -h appears anywhere in the arguments.
    // We do this before clap parsing to provide enhanced, context-aware help.
    if let Some(help_index) = args.iter().position(|arg| arg == "--help" || arg == "-h") {
        // Initialize logger without debug mode for help display
        logger::init(false);

        // Determine if help is requested for a specific topic/subcommand.
        // If --help appears after a subcommand name, we show help for that subcommand.
        // Example: `setup-devbox now --help` should show help for the 'now' command.
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

    // ========================================================================
    // STEP 2: PARSE COMMAND-LINE ARGUMENTS
    // ========================================================================
    // Use clap to parse the arguments into our structured Cli type.
    // This handles validation, type conversion, and error messages for
    // malformed arguments. If parsing fails, clap will print an error
    // and exit automatically.
    let cli = Cli::parse();

    // ========================================================================
    // STEP 3: INITIALIZE LOGGING SYSTEM
    // ========================================================================
    // Set up the logger based on the --debug flag.
    logger::init(cli.debug);
    log_debug!("[SDB] Command line arguments successfully parsed.");
    log_debug!("[SDB] Debug mode requested: {}", cli.debug);

    // ========================================================================
    // STEP 4: COMMAND DISPATCH
    // ========================================================================
    // Route to the appropriate subcommand handler based on parsed command.
    // Each match arm handles a different subcommand with its specific arguments.
    match cli.command {
        // ====================================================================
        // ADD COMMAND - Add items to configuration files
        // ====================================================================
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
        // GENERATE COMMAND - Create default configuration files
        // ====================================================================
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

        // ====================================================================
        // SYNC CONFIG COMMAND - Generate configs from state file
        // ====================================================================
        Commands::SyncConfig { state, output_dir } => {
            log_debug!("[SDB] 'SyncConfig' subcommand detected.");
            let paths = PathResolver::new(output_dir, state)?;
            sync::run(paths);
        }

        // ====================================================================
        // VERSION COMMAND - Display version information
        // ====================================================================
        Commands::Version => {
            log_debug!("[SDB] 'Version' subcommand detected. Calling version::run().");
            // Display application version information
            version::run();
        }
    }

    log_debug!("[SDB] Command execution completed. Exiting application.");
    Ok(())
}

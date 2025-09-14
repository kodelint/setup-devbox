use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Structure representing the main config.yaml file that contains paths to other config files
#[derive(Debug, Deserialize, Serialize)]
struct ConfigPaths {
    tools: String,
    settings: String,
    shellrc: String,
    fonts: String,
}

/// Main entry point for the edit command
///
/// # Arguments
/// * `edit_state` - Boolean flag indicating if the state file should be edited
/// * `config_type` - Optional config type to edit (tools, fonts, shell, settings)
pub fn run(edit_state: bool, config_type: Option<String>) {
    log_debug!("[Edit] Starting edit command execution");
    log_debug!("[Edit] Edit state requested: {}", edit_state);
    log_debug!("[Edit] Config type requested: {:?}", config_type);

    if edit_state {
        log_debug!("[Edit] Handling state file edit request");
        handle_state_edit();
    } else if let Some(config_type) = config_type {
        log_debug!(
            "[Edit] Handling config file edit request for type: {}",
            config_type
        );
        handle_config_edit(&config_type);
    } else {
        // This should not happen due to clap validation, but adding as safety
        log_error!("[Edit] Neither state nor config type was specified");
        eprintln!(
            "{}",
            "Error: You must specify either --state or --config <type>".red()
        );
        std::process::exit(1);
    }
}

/// Handles editing the state file
/// Shows warning about this being a "break glass" mechanism
fn handle_state_edit() {
    log_debug!("[Edit] Preparing to edit state file");

    // Show warning about editing state file
    println!(
        "{}",
        "⚠️  WARNING: Editing state file directly".yellow().bold()
    );
    println!(
        "{}",
        "This is a break-glass mechanism and should be used with caution.".yellow()
    );
    println!(
        "{}",
        "Direct state file modifications may lead to inconsistent behavior.".yellow()
    );
    println!(
        "{}",
        "Consider using configuration files instead when possible.".yellow()
    );
    println!();

    let state_file_path = get_state_file_path();
    log_debug!("[Edit] State file path resolved to: {:?}", state_file_path);

    if !state_file_path.exists() {
        log_warn!("[Edit] State file does not exist at: {:?}", state_file_path);
        eprintln!(
            "{}",
            format!(
                "Warning: State file does not exist at: {}",
                state_file_path.display()
            )
            .yellow()
        );
        eprintln!(
            "{}",
            "You may want to run 'setup-devbox now' first to create it.".yellow()
        );
        println!();

        // Ask user if they want to continue
        print!("Do you want to create and edit the state file anyway? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        if !input.trim().to_lowercase().starts_with('y') {
            log_info!("[Edit] User chose not to create state file, exiting");
            println!("{}", "Operation cancelled.".cyan());
            return;
        }

        log_debug!("[Edit] User chose to create state file, creating parent directories");
        // Create parent directories if they don't exist
        if let Some(parent) = state_file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                log_error!("[Edit] Failed to create directories: {:?}", e);
                eprintln!("{}", format!("Error creating directories: {}", e).red());
                std::process::exit(1);
            }
        }
    }

    log_info!("[Edit] Opening state file in editor: {:?}", state_file_path);
    open_file_in_editor(&state_file_path);

    println!("{}", "✅ State file editing completed.".green());
    log_debug!("[Edit] State file editing completed successfully");
}

/// Handles editing configuration files
/// Automatically runs 'now' command after successful edit
///
/// # Arguments
/// * `config_type` - The type of config to edit (tools, fonts, shell, settings)
fn handle_config_edit(config_type: &str) {
    log_debug!("[Edit] Starting config file edit for type: {}", config_type);

    let config_file_path = get_config_file_path(config_type);
    log_debug!(
        "[Edit] Config file path resolved to: {:?}",
        config_file_path
    );

    if !config_file_path.exists() {
        log_warn!(
            "[Edit] Config file does not exist at: {:?}",
            config_file_path
        );
        eprintln!(
            "{}",
            format!(
                "Error: Configuration file for '{}' does not exist at: {}",
                config_type,
                config_file_path.display()
            )
            .red()
        );
        eprintln!("{}", "You may want to run 'setup-devbox generate' first to create default configuration files.".yellow());
        std::process::exit(1);
    }

    println!(
        "{}",
        format!("📝 Opening {} configuration for editing...", config_type)
            .cyan()
            .bold()
    );
    log_info!(
        "[Edit] Opening config file in editor: {:?}",
        config_file_path
    );

    open_file_in_editor(&config_file_path);

    println!(
        "{}",
        format!("✅ {} configuration editing completed.", config_type).green()
    );
    println!();

    // Automatically run 'now' command after config edit
    println!(
        "{}",
        "🚀 Automatically applying changes by running 'setup-devbox now'..."
            .cyan()
            .bold()
    );
    log_info!("[Edit] Auto-running 'now' command to apply configuration changes");

    // Import and call the now command
    crate::commands::now::run(None, None, false);

    println!(
        "{}",
        "✅ Configuration changes have been applied successfully!"
            .green()
            .bold()
    );
    log_debug!("[Edit] Config file editing and application completed successfully");
}

/// Gets the path to the state file based on environment variables or defaults
///
/// # Returns
/// PathBuf pointing to the state file location
fn get_state_file_path() -> PathBuf {
    log_debug!("[Edit] Resolving state file path");

    // Check for SDB_CONFIG_DIR environment variable first
    if let Ok(config_dir) = env::var("SDB_CONFIG_DIR") {
        log_debug!(
            "[Edit] Using SDB_CONFIG_DIR environment variable: {}",
            config_dir
        );
        let mut path = PathBuf::from(config_dir);
        path.push("state.json");
        return path;
    }

    // Fall back to default location: $HOME/.setup-devbox/state.json
    let home_dir = env::var("HOME").unwrap_or_else(|_| {
        log_error!("[Edit] HOME environment variable not found");
        eprintln!("{}", "Error: HOME environment variable not found".red());
        std::process::exit(1);
    });

    log_debug!(
        "[Edit] Using default location in HOME directory: {}",
        home_dir
    );
    let mut path = PathBuf::from(home_dir);
    path.push(".setup-devbox");
    path.push("state.json");

    log_debug!("[Edit] Final state file path: {:?}", path);
    path
}

/// Gets the path to a specific configuration file
///
/// # Arguments
/// * `config_type` - The type of config file to get path for
///
/// # Returns
/// PathBuf pointing to the specific configuration file
fn get_config_file_path(config_type: &str) -> PathBuf {
    log_debug!(
        "[Edit] Resolving config file path for type: {}",
        config_type
    );

    // First, get the main config.yaml file path
    let main_config_path = get_main_config_path();
    log_debug!("[Edit] Main config file path: {:?}", main_config_path);

    // Read and parse the main config file to get paths to individual config files
    match read_config_paths(&main_config_path) {
        Ok(config_paths) => {
            log_debug!("[Edit] Successfully parsed main config file");
            let file_path = match config_type {
                "tools" => &config_paths.tools,
                "fonts" => &config_paths.fonts,
                "shell" => &config_paths.shellrc,
                "settings" => &config_paths.settings,
                _ => {
                    log_error!("[Edit] Invalid config type: {}", config_type);
                    eprintln!(
                        "{}",
                        format!("Error: Invalid config type '{}'", config_type).red()
                    );
                    std::process::exit(1);
                }
            };

            log_debug!("[Edit] Config file path for {}: {}", config_type, file_path);
            PathBuf::from(file_path)
        }
        Err(e) => {
            log_error!("[Edit] Failed to read main config file: {:?}", e);
            eprintln!("{}", format!("Error reading main config file: {}", e).red());
            eprintln!(
                "{}",
                "You may want to run 'setup-devbox generate' first.".yellow()
            );
            std::process::exit(1);
        }
    }
}

/// Gets the path to the main config.yaml file
///
/// # Returns
/// PathBuf pointing to the main configuration file
fn get_main_config_path() -> PathBuf {
    log_debug!("[Edit] Resolving main config file path");

    // Check for SDB_CONFIG_DIR environment variable first
    if let Ok(config_dir) = env::var("SDB_CONFIG_DIR") {
        log_debug!(
            "[Edit] Using SDB_CONFIG_DIR environment variable: {}",
            config_dir
        );
        let mut path = PathBuf::from(config_dir);
        path.push("config");
        path.push("config.yaml");
        return path;
    }

    // Fall back to default location: $HOME/.setup-devbox/config/config.yaml
    let home_dir = env::var("HOME").unwrap_or_else(|_| {
        log_error!("[Edit] HOME environment variable not found");
        eprintln!("{}", "Error: HOME environment variable not found".red());
        std::process::exit(1);
    });

    log_debug!(
        "[Edit] Using default location in HOME directory: {}",
        home_dir
    );
    let mut path = PathBuf::from(home_dir);
    path.push(".setup-devbox");
    path.push("config");
    path.push("config.yaml");

    log_debug!("[Edit] Final main config file path: {:?}", path);
    path
}

/// Reads and parses the main config.yaml file to extract paths to individual config files
///
/// # Arguments
/// * `config_path` - Path to the main config.yaml file
///
/// # Returns
/// Result containing ConfigPaths struct or error
fn read_config_paths(config_path: &PathBuf) -> Result<ConfigPaths, Box<dyn std::error::Error>> {
    log_debug!("[Edit] Reading config paths from: {:?}", config_path);

    let content = fs::read_to_string(config_path)?;
    log_debug!("[Edit] Successfully read config file content");

    let config_paths: ConfigPaths = serde_yaml::from_str(&content)?;
    log_debug!("[Edit] Successfully parsed config paths");

    Ok(config_paths)
}

/// Opens a file in the user's preferred editor
/// Uses the $EDITOR environment variable, falls back to common editors
///
/// # Arguments
/// * `file_path` - Path to the file to open
fn open_file_in_editor(file_path: &PathBuf) {
    log_debug!("[Edit] Opening file in editor: {:?}", file_path);

    // Get the editor from environment variable or use defaults
    let editor = env::var("EDITOR").unwrap_or_else(|_| {
        log_debug!("[Edit] EDITOR environment variable not set, checking for common editors");

        // Try to find a common editor
        let common_editors = ["code", "nano", "vim", "vi"];
        for editor in &common_editors {
            if Command::new("which").arg(editor).output().is_ok() {
                log_debug!("[Edit] Found editor: {}", editor);
                return editor.to_string();
            }
        }

        log_warn!("[Edit] No common editor found, defaulting to vi");
        "vi".to_string()
    });

    log_debug!("[Edit] Using editor: {}", editor);
    println!("{}", format!("Opening file with editor: {}", editor).cyan());

    // Launch the editor
    let mut command = Command::new(&editor);
    command.arg(file_path);

    match command.status() {
        Ok(status) => {
            if status.success() {
                log_info!("[Edit] Editor exited successfully");
            } else {
                log_warn!("[Edit] Editor exited with status: {:?}", status);
                eprintln!(
                    "{}",
                    format!("Editor exited with status: {:?}", status).yellow()
                );
            }
        }
        Err(e) => {
            log_error!("[Edit] Failed to launch editor: {:?}", e);
            eprintln!(
                "{}",
                format!("Error launching editor '{}': {}", editor, e).red()
            );
            eprintln!(
                "{}",
                "Try setting the EDITOR environment variable to your preferred editor.".yellow()
            );
            std::process::exit(1);
        }
    }
}

use crate::schemas::common::ConfigPaths;
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
        log_error!(
            "You must specify either {} or {} <type>",
            "--state".red().italic(),
            "--config".red().italic()
        );
        std::process::exit(1);
    }
}

/// Handles editing the state file
/// Shows warning about this being a "break glass" mechanism
fn handle_state_edit() {
    log_debug!("[Edit] Preparing to edit state file");
    println!();
    // Show warning about editing state file
    println!(
        "            {}",
        "⚠️  WARNING: Editing state file directly"
            .bright_yellow()
            .bold()
    );
    println!(
        "{}",
        "======================================================================"
            .yellow()
            .dimmed()
    );
    println!(
        "  {}",
        "This is a break-glass mechanism and should be used with caution."
            .yellow()
            .dimmed()
    );
    println!(
        "  {}",
        "Direct state file modifications may lead to inconsistent behavior."
            .yellow()
            .dimmed()
    );
    println!(
        "  {}",
        "Consider using configuration files instead when possible."
            .yellow()
            .dimmed()
    );
    println!(
        "{}",
        "======================================================================"
            .yellow()
            .dimmed()
    );
    println!();

    let state_file_path = get_config_file_path_with_fallback("state", |home_dir| {
        let mut path = PathBuf::from(home_dir);
        path.push(".setup-devbox");
        path.push("state.json");
        path
    });

    log_debug!("[Edit] State file path resolved to: {:?}", state_file_path);

    if !state_file_path.exists() {
        log_warn!("[Edit] State file does not exist at: {:?}", state_file_path);
        log_error!(
            "Warning: State file does not exist at: {}",
            state_file_path.display().to_string().yellow()
        );
        log_error!(
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
            log_info!("{}", "Operation cancelled.".cyan());
            return;
        }

        log_debug!("[Edit] User chose to create state file, creating parent directories");
        // Create parent directories if they don't exist
        if let Some(parent) = state_file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                log_error!("[Edit] Failed to create directories: {:?}", e);
                log_error!("Error creating directories: {}", e.to_string().yellow());
                std::process::exit(1);
            }
        }
    }

    // Handle the result properly
    if let Err(e) = open_file_in_editor(&state_file_path) {
        log_error!("[Edit] Failed to edit state file: {:?}", e);
        log_error!("Error editing state file: {}", e.to_string().yellow());
        std::process::exit(1);
    }

    log_debug!("[Edit] State file editing completed successfully");
}

/// Handles editing configuration files
/// Automatically runs 'now' command after successful edit if changes were detected
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
        log_error!(
            "Configuration file for '{}' does not exist at: {}",
            config_type,
            config_file_path.display().to_string().red()
        );
        log_error!(
            "{}",
            "You may want to run 'setup-devbox generate' first to create default \
        configuration files."
                .yellow()
        );
        std::process::exit(1);
    }

    // Get content hash before editing (more reliable than modification time)
    let original_hash = get_file_content_hash(&config_file_path);

    log_info!(
        "{}",
        format!("Opening {config_type} configuration for editing...",)
            .cyan()
            .bold()
    );
    log_info!(
        "[Edit] Opening config file in editor: {:?}",
        config_file_path
    );

    // Open the editor and wait for it to complete
    if let Err(e) = open_file_in_editor(&config_file_path) {
        log_error!("[Edit] Failed to edit file: {:?}", e);
        log_error!("Error editing file: {}", e.to_string().red());
        std::process::exit(1);
    }

    // Get content hash after editing
    let new_hash = get_file_content_hash(&config_file_path);

    // Check if file content was actually modified
    let file_was_modified = match (original_hash, new_hash) {
        (Some(original), Some(new)) => {
            log_debug!("[Edit] Original hash: {}", original);
            log_debug!("[Edit] New hash: {}", new);
            original != new
        }
        _ => {
            log_warn!("[Edit] Could not determine if file was modified, assuming it was");
            true // If we can't determine, assume it was modified
        }
    };

    if !file_was_modified {
        log_info!(
            "[Edit] No changes detected in {} configuration.",
            config_type.blue().bold()
        );
        log_info!("[Edit] No changes detected in config file, skipping 'now' command");
        return;
    }
    log_info!(
        "{}",
        format!("{config_type} configuration editing completed.").green()
    );
    println!();

    // Automatically run 'now' command after config edit if changes were detected
    log_info!(
        "{}",
        "Automatically applying changes by running 'setup-devbox now'..."
            .cyan()
            .bold()
    );
    log_info!("[Edit] Auto-running 'now' command to apply configuration changes");

    // Import and call the now command
    crate::commands::now::run(None, None, false);

    log_info!(
        "{}",
        "Configuration changes have been applied successfully!"
            .green()
            .bold()
    );
    log_debug!("[Edit] Config file editing and application completed successfully");
}

/// Gets the SHA256 hash of a file's content
/// This is more reliable than modification time for detecting actual changes
///
/// # Arguments
/// * `file_path` - Path to the file
///
/// # Returns
/// Option containing the hash string or None if file couldn't be read
fn get_file_content_hash(file_path: &PathBuf) -> Option<String> {
    log_debug!("[Edit] Computing content hash for: {:?}", file_path);

    match fs::read(file_path) {
        Ok(content) => {
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let hash = format!("{:x}", hasher.finalize());
            log_debug!("[Edit] Content hash computed successfully");
            Some(hash)
        }
        Err(e) => {
            log_warn!("[Edit] Failed to read file for hashing: {:?}", e);
            None
        }
    }
}

/// Gets the path to a specific configuration file by reading from config.yaml
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

    // Read and parse the main config file to get paths to individual config files
    let main_config_path = get_main_config_path();
    log_debug!("[Edit] Main config file path: {:?}", main_config_path);

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
                        format!("Error: Invalid config type '{config_type}'").red()
                    );
                    std::process::exit(1);
                }
            };

            log_debug!("[Edit] Config file path for {}: {}", config_type, file_path);
            PathBuf::from(file_path)
        }
        Err(e) => {
            log_error!("[Edit] Failed to read main config file: {:?}", e);
            eprintln!("{}", format!("Error reading main config file: {e}").red());
            eprintln!(
                "{}",
                "You may want to run 'setup-devbox generate' first.".yellow()
            );
            std::process::exit(1);
        }
    }
}

/// Gets the path to a configuration file with fallback logic
/// Used for files that don't have their paths defined in config.yaml
///
/// # Arguments
/// * `file_type` - The type of file to get path for
/// * `fallback_fn` - Function to generate fallback path if SDB_CONFIG_DIR is not set
///
/// # Returns
/// PathBuf pointing to the configuration file
fn get_config_file_path_with_fallback<F>(file_type: &str, fallback_fn: F) -> PathBuf
where
    F: Fn(&str) -> PathBuf,
{
    log_debug!("[Edit] Resolving {} file path", file_type);

    // Check for SDB_CONFIG_PATH environment variable first
    if let Ok(config_dir) = env::var("SDB_CONFIG_PATH") {
        log_debug!(
            "[Edit] Using SDB_CONFIG_PATH environment variable: {}",
            config_dir
        );
        let mut path = PathBuf::from(config_dir);
        if file_type == "config" {
            path.push("configs");
            path.push("config.yaml");
        } else {
            path.push(format!("{file_type}.json"));
        }
        return path;
    }

    // Fall back to default location using the provided function
    let home_dir = env::var("HOME").unwrap_or_else(|_| {
        log_error!("[Edit] HOME environment variable not found");
        eprintln!("{}", "Error: HOME environment variable not found".red());
        std::process::exit(1);
    });

    log_debug!(
        "[Edit] Using default location in HOME directory: {}",
        home_dir
    );
    let path = fallback_fn(&home_dir);

    log_debug!("[Edit] Final {} file path: {:?}", file_type, path);
    path
}

/// Gets the path to the main config.yaml file
///
/// # Returns
/// PathBuf pointing to the main configuration file
fn get_main_config_path() -> PathBuf {
    get_config_file_path_with_fallback("config", |home_dir| {
        let mut path = PathBuf::from(home_dir);
        path.push(".setup-devbox");
        path.push("configs");
        path.push("config.yaml");
        path
    })
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

/// Opens a file in the user's preferred editor and waits for it to complete
///
/// # Arguments
/// * `file_path` - Path to the file to open
///
/// # Returns
/// Result indicating success or failure
fn open_file_in_editor(file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    log_debug!("[Edit] Opening file in editor: {:?}", file_path);

    // Get the editor from environment variable or use defaults
    let editor_env = env::var("EDITOR").unwrap_or_else(|_| {
        log_debug!("[Edit] EDITOR environment variable not set, checking for common editors");

        // Try to find a common editor
        let common_editors = ["nano", "vim", "vi", "code", "zed"];
        for editor in &common_editors {
            if Command::new("which").arg(editor).output().is_ok() {
                log_debug!("[Edit] Found editor: {}", editor);
                return editor.to_string();
            }
        }

        log_warn!("[Edit] No common editor found, defaulting to vi");
        "vi".to_string()
    });

    // Configure editor command with appropriate wait flags
    let (editor_cmd, editor_args) = get_editor_command_with_wait_flag(&editor_env);

    log_debug!(
        "[Edit] Using editor command: {} with args: {:?}",
        editor_cmd,
        editor_args
    );
    log_info!(
        "Opening File: {} with editor: {}",
        file_path.display(),
        editor_cmd.cyan()
    );

    // For GUI editors that may not block properly, show additional instructions
    if is_gui_editor(&editor_cmd) {
        println!();
        println!(
            "{}",
            "Please close the editor window/tab when you're done editing to continue..."
                .yellow()
                .dimmed()
        );
        println!();
    }

    // Launch the editor and wait for it to complete
    let mut command = Command::new(&editor_cmd);
    for arg in &editor_args {
        command.arg(arg);
    }
    command.arg(file_path);

    let start_time = std::time::Instant::now();

    match command.status() {
        Ok(status) => {
            let duration = start_time.elapsed();
            log_debug!("[Edit] Editor process took: {:?}", duration);

            // If the editor returned very quickly (less than 1 second), it might not have waited properly
            if duration.as_secs() < 1 && is_gui_editor(&editor_cmd) {
                log_warn!(
                    "[Edit] Editor returned very quickly ({:?}), it may not support waiting properly",
                    duration
                );
                println!(
                    "{}",
                    "⚠️  Warning: The editor returned very quickly.".yellow()
                );
                println!(
                    "{}",
                    "This might mean it doesn't support waiting for the file to be closed."
                        .yellow()
                );

                // Ask user to confirm they're done editing
                println!();
                print!("Have you finished editing the file and saved your changes? [y/N]: ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();

                if !input.trim().to_lowercase().starts_with('y') {
                    log_info!("[Edit] User indicated they haven't finished editing");
                    return Err("User indicated they haven't finished editing".into());
                }
            }

            if status.success() {
                log_info!("[Edit] Editor exited successfully");
                Ok(())
            } else {
                log_warn!("[Edit] Editor exited with status: {:?}", status);
                Err(format!("Editor exited with status: {status:?}").into())
            }
        }
        Err(e) => {
            log_error!("[Edit] Failed to launch editor: {:?}", e);
            Err(format!("Error launching editor '{editor_cmd}': {e}").into())
        }
    }
}

/// Determines the appropriate command and arguments for an editor to make it wait
///
/// # Arguments
/// * `editor` - The editor command from environment or detection
///
/// # Returns
/// Tuple of (command, arguments) where arguments include wait flags if needed
fn get_editor_command_with_wait_flag(editor: &str) -> (String, Vec<String>) {
    match editor {
        "code" => ("code".to_string(), vec!["--wait".to_string()]),
        "zed" => ("zed".to_string(), vec!["--wait".to_string()]),
        "subl" | "sublime_text" => ("subl".to_string(), vec!["--wait".to_string()]),
        "atom" => ("atom".to_string(), vec!["--wait".to_string()]),
        "gedit" => ("gedit".to_string(), vec!["--wait".to_string()]),
        "kate" => ("kate".to_string(), vec!["--block".to_string()]),
        // Terminal editors typically block by default
        "nano" | "vim" | "vi" | "emacs" | "micro" | "joe" => (editor.to_string(), vec![]),
        // For any other editor, try as-is first
        _ => {
            // Check if it looks like a GUI editor by name
            if editor.contains("code")
                || editor.contains("zed")
                || editor.contains("subl")
                || editor.contains("atom")
                || editor.contains("gedit")
            {
                log_debug!(
                    "[Edit] Unknown GUI editor '{}', trying with --wait flag",
                    editor
                );
                (editor.to_string(), vec!["--wait".to_string()])
            } else {
                (editor.to_string(), vec![])
            }
        }
    }
}

/// Checks if an editor is likely a GUI editor that might not block properly
///
/// # Arguments
/// * `editor` - The editor command
///
/// # Returns
/// True if it's likely a GUI editor
fn is_gui_editor(editor: &str) -> bool {
    matches!(
        editor,
        "code" | "zed" | "subl" | "sublime_text" | "atom" | "gedit" | "kate"
    ) || editor.contains("code")
        || editor.contains("zed")
        || editor.contains("subl")
        || editor.contains("atom")
        || editor.contains("gedit")
}

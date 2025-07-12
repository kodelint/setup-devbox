// Imports necessary schema definitions for tools.
use crate::schema::{ToolEntry, ToolState};
// Imports custom logging macros.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to terminal output.
use colored::Colorize;
// For executing external commands and capturing their output.
use std::process::{Command, Output};
// For working with file paths, specifically to construct installation paths.
use std::path::PathBuf;

/// Installs a Rust crate using `cargo install`.
///
/// This function constructs and executes the `cargo install` command.
/// It handles versioning via `--version` and passes additional options
/// like `--features` (if provided in `tool_entry.options`).
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing details for the Rust crate.
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if the installation was successful, containing the installed state.
/// * `None` if the installation failed or encountered an error.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!("[Cargo Installer] Attempting to install Cargo tool: {}", tool_entry.name.bold());

    // Basic validation: Ensure 'cargo' command is available in the system's PATH.
    if Command::new("cargo").arg("--version").output().is_err() {
        log_error!(
            "[Cargo Installer] 'cargo' command not found. Please ensure Rust/Cargo is installed and in your PATH."
        );
        return None;
    }

    // Initialize command arguments for `cargo install`. The tool name is always required.
    let mut command_args = vec!["install", &tool_entry.name];

    // Handle versioning if provided in the `ToolEntry`. Appends `--version <VERSION>` to the command.
    if let Some(version) = &tool_entry.version {
        command_args.push("--version");
        command_args.push(version);
    }

    // Add any additional options specified in the `ToolEntry`, such as `--features`.
    if let Some(options) = &tool_entry.options {
        for opt in options {
            command_args.push(opt);
        }
    }

    log_info!("[Cargo Installer] Executing: {} {}", "cargo".cyan().bold(), command_args.join(" ").cyan());

    // Execute the `cargo install` command and capture its output.
    let output: Output = match Command::new("cargo")
        .args(&command_args)
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            log_error!("[Cargo Installer] Failed to execute 'cargo install' command for '{}': {}", tool_entry.name.bold().red(), e);
            return None;
        }
    };

    // Check if the command executed successfully (exit code 0).
    if output.status.success() {
        log_info!(
            "[Cargo Installer] Successfully installed Cargo tool: {}",
            tool_entry.name.bold().green()
        );
        // Log standard output and error, as they might contain useful information or warnings.
        if !output.stdout.is_empty() {
            log_debug!("[Cargo Installer] Stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            log_warn!("[Cargo Installer] Stderr (might contain warnings): {}", String::from_utf8_lossy(&output.stderr));
        }

        // Determine the installation path. Cargo typically installs binaries to `~/.cargo/bin/`.
        let install_path = if let Ok(mut home) = std::env::var("HOME") {
            home.push_str("/.cargo/bin/");
            PathBuf::from(home).join(&tool_entry.name).to_string_lossy().into_owned()
        } else {
            // Fallback path if HOME environment variable is not set.
            "/usr/local/bin/".to_string()
        };

        // Return a `ToolState` struct to record the successful installation.
        Some(ToolState {
            // Record actual version or "latest".
            version: tool_entry.version.clone().unwrap_or_else(|| "latest".to_string()),
            // The determined installation path.
            install_path,
            // Mark as installed by this application.
            installed_by_devbox: true,
            // Specify the installation method.
            install_method: "cargo-install".to_string(),
            // Record if the tool was renamed.
            renamed_to: tool_entry.rename_to.clone(),
            // Define the package type.
            package_type: "rust-crate".to_string(),
            // Not directly applicable for cargo installs from tool_entry.
            repo: None,
            // Not directly applicable for cargo installs from tool_entry.
            tag: None,
            // Pass the options from ToolEntry to ToolState.
            options: tool_entry.options.clone(),
        })
    } else {
        // Handle failed installation.
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error!(
            "[Cargo Installer] Failed to install Cargo tool '{}'. Exit code: {}. Error: {}",
            tool_entry.name.bold().red(),
            output.status.code().unwrap_or(-1),
            stderr.red()
        );
        // Log stdout on failure for debugging.
        if !output.stdout.is_empty() {
            log_debug!("[Cargo Installer] Stdout (on failure): {}", String::from_utf8_lossy(&output.stdout));
        }
        None // Return None to indicate failure.
    }
}
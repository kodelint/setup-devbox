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

/// Installs a Go binary using `go install`.
///
/// This function constructs and executes the `go install` command.
/// It intelligently handles versioning (if specified) using Go module syntax (`@version`)
/// and passes additional options (e.g., build flags like `-ldflags`).
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing details for the Go binary.
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if the installation was successful, containing the installed state.
/// * `None` if the installation failed or encountered an error.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!("[Go Installer] Attempting to install Go tool: {}", tool_entry.name.bold());

    // Basic validation: Ensure 'go' command is available in the system's PATH.
    if Command::new("go").arg("version").output().is_err() {
        log_error!(
            "[Go Installer] 'go' command not found. Please ensure Go is installed and in your PATH."
        );
        return None;
    }

    // Initialize command arguments for `go install`.
    let mut command_args = vec!["install"];

    // Handle versioning if provided. Go modules typically use `@version` syntax.
    let package_path = if let Some(version) = &tool_entry.version {
        format!("{}@{}", tool_entry.name, version)
    } else {
        tool_entry.name.clone() // Install latest if no version specified.
    };
    command_args.push(&package_path);

    // Add any additional options specified in the `ToolEntry`, e.g., build flags.
    if let Some(options) = &tool_entry.options {
        for opt in options {
            command_args.push(opt);
        }
    }

    log_info!("[Go Installer] Executing: {} {}", "go".cyan().bold(), command_args.join(" ").cyan());

    // Execute the `go install` command and capture its output.
    let output: Output = match Command::new("go")
        .args(&command_args)
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            log_error!("[Go Installer] Failed to execute 'go install' command for '{}': {}", tool_entry.name.bold().red(), e);
            return None;
        }
    };

    // Check if the command executed successfully (exit code 0).
    if output.status.success() {
        log_info!(
            "[Go Installer] Successfully installed Go tool: {}",
            tool_entry.name.bold().green()
        );
        // Log standard output and error, as they might contain useful information or warnings.
        if !output.stdout.is_empty() {
            log_debug!("[Go Installer] Stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            log_warn!("[Go Installer] Stderr (might contain warnings): {}", String::from_utf8_lossy(&output.stderr));
        }

        // Determine the installation path. Go typically installs binaries to GOPATH/bin or GOBIN.
        // This is a common default for `go install`. A more robust solution might parse `go env GOBIN`.
        let install_path = if let Ok(mut home) = std::env::var("HOME") {
            home.push_str("/go/bin/"); // Common default for go install.
            PathBuf::from(home).join(&tool_entry.name).to_string_lossy().into_owned()
        } else {
            "/usr/local/go/bin/".to_string() // Fallback path if HOME is not set.
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
            install_method: "go-install".to_string(),
            // Record if the tool was renamed.
            renamed_to: tool_entry.rename_to.clone(),
            // Define the package type.
            package_type: "go-module".to_string(),
            // Not directly applicable for Go module installs from tool_entry.
            repo: None,
            // Not directly applicable for Go module installs from tool_entry.
            tag: None,
            // Pass the options from ToolEntry to ToolState.
            options: tool_entry.options.clone(),
        })
    } else {
        // Handle failed installation.
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error!(
            "[Go Installer] Failed to install Go tool '{}'. Exit code: {}. Error: {}",
            tool_entry.name.bold().red(),
            output.status.code().unwrap_or(-1),
            stderr.red()
        );
        // Log stdout on failure for debugging.
        if !output.stdout.is_empty() {
            log_debug!("[Go Installer] Stdout (on failure): {}", String::from_utf8_lossy(&output.stdout));
        }
        None // Return None to indicate failure.
    }
}
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

/// Installs a Python package using `pip`.
///
/// This function constructs and executes the `pip install` command.
/// It handles package names, optional versions (e.g., `package==1.2.3`),
/// and additional pip options (e.g., `--user`, `--index-url`) passed via `tool_entry.options`.
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing details for the Python package.
///                 `tool_entry.name` is the package name (e.g., "requests", "black").
///                 `tool_entry.version` is the optional version specifier (e.g., "1.2.3").
///                 `tool_entry.options` can contain a list of additional pip arguments.
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if the installation was successful.
/// * `None` if the installation failed.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!("[Pip Installer] Attempting to install Python package: {}", tool_entry.name.bold());

    // 1. Basic validation: Ensure 'pip' command is available.
    // We try to find 'pip3' first, as it's often the preferred modern Python 3 pip.
    // If not found, fall back to 'pip'.
    let pip_command = if Command::new("pip3").arg("--version").output().is_ok() {
        "pip3"
    } else if Command::new("pip").arg("--version").output().is_ok() {
        "pip"
    } else {
        log_error!(
            "[Pip Installer] 'pip' or 'pip3' command not found. Please ensure Python and Pip are installed and in your PATH."
        );
        return None;
    };

    let mut command_args = vec!["install"];

    // Construct the package name with an optional version specifier.
    let package_specifier = if let Some(version) = &tool_entry.version {
        format!("{}=={}", tool_entry.name, version)
    } else {
        tool_entry.name.clone()
    };
    command_args.push(&package_specifier);

    // Add any additional options specified in the `ToolEntry` (e.g., --user, --upgrade).
    if let Some(options) = &tool_entry.options {
        for opt in options {
            command_args.push(opt);
        }
    }

    log_info!("[Pip Installer] Executing: {} {}", pip_command.cyan().bold(), command_args.join(" ").cyan());

    // Execute the `pip install` command.
    let output: Output = match Command::new(pip_command)
        .args(&command_args)
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            log_error!("[Pip Installer] Failed to execute '{} install' command for '{}': {}", pip_command.bold().red(), tool_entry.name.bold().red(), e);
            return None;
        }
    };

    if output.status.success() {
        log_info!(
            "[Pip Installer] Successfully installed Python package: {}",
            tool_entry.name.bold().green()
        );
        if !output.stdout.is_empty() {
            log_debug!("[Pip Installer] Stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            log_warn!("[Pip Installer] Stderr (might contain warnings): {}", String::from_utf8_lossy(&output.stderr));
        }

        // Determining the exact install_path for pip packages can be complex (site-packages, user-base, venvs).
        // For simplicity, we'll indicate a common user-level path or a generic Python bin path.
        // A more robust solution might parse `pip show <package>` output for 'Location' or 'Installed-Location'.
        let install_path = if let Ok(mut home) = std::env::var("HOME") {
            // This is a common location for user-installed packages, but not always the executable path.
            // For executables installed by pip (e.g., `black`), they might be in ~/.local/bin
            home.push_str("/.local/bin/"); // Common location for scripts installed by pip --user
            PathBuf::from(home).join(&tool_entry.name).to_string_lossy().into_owned()
        } else {
            // Generic fallback, actual binary might be elsewhere
            "/usr/local/bin/".to_string()
        };


        Some(ToolState {
            version: tool_entry.version.clone().unwrap_or_else(|| "latest".to_string()),
            install_path,
            installed_by_devbox: true,
            install_method: "pip".to_string(),
            renamed_to: tool_entry.rename_to.clone(),
            package_type: "python-package".to_string(),
            repo: None, // Pip packages don't typically map directly to GitHub repos in ToolEntry.
            tag: None,  // Pip packages don't typically map directly to a tag in ToolEntry.
            options: tool_entry.options.clone(), // Store the additional options used.
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error!(
            "[Pip Installer] Failed to install Python package '{}'. Exit code: {}. Error: {}",
            tool_entry.name.bold().red(),
            output.status.code().unwrap_or(-1),
            stderr.red()
        );
        if !output.stdout.is_empty() {
            log_debug!("[Pip Installer] Stdout (on failure): {}", String::from_utf8_lossy(&output.stdout));
        }
        None
    }
}
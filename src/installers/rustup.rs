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

/// Installs a Rust toolchain and optionally components using `rustup`.
///
/// This function first ensures `rustup` is available. It then constructs and
/// executes `rustup toolchain install` using the `tool_entry.version` as the
/// toolchain name (e.g., "stable", "nightly", "1.70.0").
/// If `tool_entry.options` are provided, they are treated as `rustup component add`
/// arguments for the newly installed toolchain.
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing details for the Rust toolchain.
///                 `tool_entry.name` is expected to be "rust" or "rustup".
///                 `tool_entry.version` is the toolchain name (e.g., "stable", "nightly").
///                 `tool_entry.options` can contain a list of components to add (e.g., "rust-src", "rust-docs").
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if the installation (toolchain + components) was successful.
/// * `None` if any part of the installation failed.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!("[Rustup Installer] Attempting to install Rust toolchain: {}", tool_entry.name.bold());

    // 1. Basic validation: Ensure 'rustup' command is available.
    if Command::new("rustup").arg("--version").output().is_err() {
        log_error!(
            "[Rustup Installer] 'rustup' command not found. Please ensure Rustup is installed and in your PATH."
        );
        return None;
    }

    // Determine the toolchain to install.
    // We expect the 'version' field in ToolEntry to specify the toolchain (e.g., "stable", "nightly").
    let toolchain_name = match &tool_entry.version {
        Some(v) => v.clone(),
        None => {
            log_error!(
                "[Rustup Installer] Toolchain version (e.g., 'stable', 'nightly', '1.70.0') is required for rustup tool '{}'.",
                tool_entry.name.bold().red()
            );
            return None;
        }
    };

    // 2. Install the Rust toolchain.
    let toolchain_command_args = vec!["toolchain", "install", &toolchain_name];

    log_info!("[Rustup Installer] Executing: {} {}", "rustup".cyan().bold(), toolchain_command_args.join(" ").cyan());

    let output_toolchain: Output = match Command::new("rustup")
        .args(&toolchain_command_args)
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            log_error!("[Rustup Installer] Failed to execute 'rustup toolchain install' for '{}': {}", toolchain_name.bold().red(), e);
            return None;
        }
    };

    if !output_toolchain.status.success() {
        let stderr = String::from_utf8_lossy(&output_toolchain.stderr);
        log_error!(
            "[Rustup Installer] Failed to install toolchain '{}'. Exit code: {}. Error: {}",
            toolchain_name.bold().red(),
            output_toolchain.status.code().unwrap_or(-1),
            stderr.red()
        );
        if !output_toolchain.stdout.is_empty() {
            log_debug!("[Rustup Installer] Stdout (on failure): {}", String::from_utf8_lossy(&output_toolchain.stdout));
        }
        return None;
    } else {
        log_info!(
            "[Rustup Installer] Successfully installed toolchain: {}",
            toolchain_name.bold().green()
        );
        if !output_toolchain.stdout.is_empty() {
            log_debug!("[Rustup Installer] Stdout: {}", String::from_utf8_lossy(&output_toolchain.stdout));
        }
        if !output_toolchain.stderr.is_empty() {
            log_warn!("[Rustup Installer] Stderr (might contain warnings): {}", String::from_utf8_lossy(&output_toolchain.stderr));
        }
    }

    // 3. Install components if specified in options.
    if let Some(components) = &tool_entry.options {
        for component in components {
            let component_command_args = vec!["component", "add", component, "--toolchain", &toolchain_name];

            log_info!("[Rustup Installer] Executing: {} {}", "rustup".cyan().bold(), component_command_args.join(" ").cyan());

            let output_component: Output = match Command::new("rustup")
                .args(&component_command_args)
                .output()
            {
                Ok(out) => out,
                Err(e) => {
                    log_error!(
                        "[Rustup Installer] Failed to execute 'rustup component add' for '{}' on toolchain '{}': {}",
                        component.bold().red(),
                        toolchain_name.bold().red(),
                        e
                    );
                    return None; // Fail if any component fails
                }
            };

            if !output_component.status.success() {
                let stderr = String::from_utf8_lossy(&output_component.stderr);
                log_error!(
                    "[Rustup Installer] Failed to add component '{}' to toolchain '{}'. Exit code: {}. Error: {}",
                    component.bold().red(),
                    toolchain_name.bold().red(),
                    output_component.status.code().unwrap_or(-1),
                    stderr.red()
                );
                if !output_component.stdout.is_empty() {
                    log_debug!("[Rustup Installer] Stdout (on failure): {}", String::from_utf8_lossy(&output_component.stdout));
                }
                return None; // Fail if any component fails
            } else {
                log_info!(
                    "[Rustup Installer] Successfully added component '{}' to toolchain '{}'.",
                    component.bold().green(),
                    toolchain_name.bold().green()
                );
                if !output_component.stdout.is_empty() {
                    log_debug!("[Rustup Installer] Stdout: {}", String::from_utf8_lossy(&output_component.stdout));
                }
                if !output_component.stderr.is_empty() {
                    log_warn!("[Rustup Installer] Stderr (might contain warnings): {}", String::from_utf8_lossy(&output_component.stderr));
                }
            }
        }
    }

    // Determine the installation path. Rustup manages its own paths, often in ~/.rustup/toolchains/<toolchain_name>
    let install_path = if let Ok(mut home) = std::env::var("HOME") {
        home.push_str(&format!("/.rustup/toolchains/{}/bin", toolchain_name));
        PathBuf::from(home).to_string_lossy().into_owned()
    } else {
        // A generic fallback, though less accurate for rustup
        "/usr/local/bin/".to_string()
    };

    // Return a ToolState indicating successful installation.
    Some(ToolState {
        version: toolchain_name,
        install_path,
        installed_by_devbox: true,
        install_method: "rustup".to_string(),
        renamed_to: tool_entry.rename_to.clone(), // Renaming not typical for rustup, but kept for schema consistency.
        package_type: "rust-toolchain".to_string(),
        repo: None, // Not directly from a single repo.
        tag: None,  // Not directly from a single tag.
        options: tool_entry.options.clone(), // Store the components that were added.
    })
}
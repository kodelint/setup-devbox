// For working with file paths, particularly for the state file.
use std::path::PathBuf;
// For adding color to terminal output, enhancing readability.
use colored::Colorize;
// Imports custom logging macros.
use crate::{log_debug, log_error, log_info, log_warn};
use crate::installers::{brew, cargo, github, go, pip, rustup};
// Imports schema definitions for application state and tool configurations.
use crate::schema::{DevBoxState, ToolConfig};
// Imports the function for saving the application state.
use crate::libs::state_management::save_devbox_state;

/// Installs tools based on the provided configuration and updates the application state.
///
/// This function iterates through each tool defined in `tools_cfg`,
/// checks if it's already installed according to `state`,
/// and delegates to the appropriate installer (brew, GitHub, go, cargo) for new or updated tools.
/// It also handles state persistence by saving changes to `state.json`.
///
/// # Arguments
/// * `tools_cfg`: A `ToolConfig` struct containing the list of tools to install.
/// * `state`: A mutable reference to the `DevBoxState` to update installed tools.
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file for saving.
pub fn install_tools(tools_cfg: ToolConfig, state: &mut DevBoxState, state_path_resolved: &PathBuf) {
    // Add a newline for better visual separation in the terminal.
    eprintln!("\n");
    log_info!("[Tools] Processing Tools Installations..."); // Informative log message.
    log_debug!("Entering install_tools() function."); // Debug log for function entry.

    // Flag to track if any tools were installed or updated.
    let mut tools_updated = false;
    // List to store names of skipped tools.
    let mut skipped_tools: Vec<String> = Vec::new();

    // Iterate through each tool entry defined in the `tools.yaml` configuration.
    for tool in &tools_cfg.tools {
        log_debug!("[Tools] Considering tool: {:?}", tool.name.bold());
        // Check if the tool is already recorded in the current `DevBoxState`.
        if !state.tools.contains_key(&tool.name) {
            print!("\n"); // Add newline for visual clarity before installing a new tool.
            eprintln!("{}", "==============================================================================================".bright_blue()); // Visual separator.
            log_info!("[Tools] Installing new tool from {}: {}", tool.source.to_string().bright_yellow(), tool.name.to_string().bright_blue().bold()); // Informative log for new installation.
            log_debug!("[Tools] Full configuration details for tool '{}': {:?}", tool.name, tool); // Debug log of the tool's full config.

            // Dispatch the installation to the appropriate installer function based on the tool's source.
            let installation_result = match tool.source.as_str() {
                "github" => github::install(tool), // Call GitHub installer for "GitHub" source.
                "brew" => brew::install(tool),     // Call Homebrew installer for "brew" source.
                "go" => go::install(tool),         // Call Go installer for "go" source.
                "cargo" => cargo::install(tool),   // Call Cargo installer for "cargo" source.
                "rustup" => rustup::install(tool), // Call Rustup installer for "rustup" source.
                "pip" => pip::install(tool),       // Call PIP installer for "python" source.
                other => {
                    // Log a warning if the source is not supported and skip the tool.
                    log_warn!(
                        "[Tools] Unsupported source '{}' for tool '{}'. Skipping this tool's installation.",
                        other.yellow(),
                        tool.name.bold()
                    );
                    None
                }
            };

            // If installation was successful, update the `DevBoxState` and set the `tools_updated` flag.
            if let Some(tool_state) = installation_result {
                state.tools.insert(tool.name.clone(), tool_state); // Add the newly installed tool's state to the map.
                tools_updated = true; // Mark that changes occurred.
                log_info!("[Tools] {}: {}", "Successfully installed tool".yellow() ,tool.name.bold().bright_green()); // Success message.
                eprintln!("{}", "==============================================================================================".blue()); // Visual separator.
                print!("\n"); // Add newline.
            } else {
                // Log an error if installation failed.
                log_error!(
                    "[Tools] Failed to install tool: {}. Please review previous logs for specific errors during installation.",
                    tool.name.bold().red()
                );
            }
        } else {
            // If the tool is already installed, add it to the skipped list.
            skipped_tools.push(tool.name.clone());
            log_debug!("[Tools] Tool '{}' is already recorded as installed. Added to skipped list.", tool.name.blue());
        }
    }

    // Report on any tools that were skipped because they were already installed.
    if !skipped_tools.is_empty() {
        let skipped_tools_str = skipped_tools.join(", ");
        log_info!(
            "[Tools] The following tools were already recorded as installed and were skipped: {}",
            skipped_tools_str.blue()
        );
    } else {
        log_debug!("[Tools] No tools were skipped as they were not found in the state.");
    }

    // If any tools were installed or updated, save the `DevBoxState` to `state.json`.
    if tools_updated {
        log_info!("[Tools] New tools installed or state updated. Saving current DevBox state...");
        if !save_devbox_state(state, state_path_resolved) { // Call utility to save state.
            log_error!("[StateSave] Failed to save state after tool installations. Data loss risk!");
        }
        log_info!("[StateSave] State saved successfully after tool updates.");
    } else {
        log_info!("[Tools] No new tools installed or state changes detected for tools.");
    }
    eprintln!(); // Final newline for consistent output spacing.
    log_debug!("Exiting install_tools() function."); // Debug log for function exit.
}
use std::path::PathBuf;
use colored::Colorize;
use crate::{log_debug, log_error, log_info, log_warn};
use crate::installers::{brew, github};
use crate::schema::{DevBoxState, ToolConfig};
use crate::libs::state_management::save_devbox_state;

/// Installs tools based on the provided configuration and updates the application state.
///
/// This function iterates through each tool defined in `tools_cfg`, checks if it's
/// already installed according to `state`, and delegates to the appropriate installer
/// (brew, github) for new or updated tools. It also handles state persistence.
///
/// # Arguments
/// * `tools_cfg`: A `ToolConfig` struct containing the list of tools to install.
/// * `state`: A mutable reference to the `DevBoxState` to update installed tools.
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file for saving.
pub fn install_tools(tools_cfg: ToolConfig, state: &mut DevBoxState, state_path_resolved: &PathBuf) {
    eprintln!("\n");
    log_info!("[Tools] Processing Tools Installations...");
    log_debug!("Entering install_tools() function.");

    let mut tools_updated = false;
    let mut skipped_tools: Vec<String> = Vec::new();

    for tool in &tools_cfg.tools {
        log_debug!("[Tools] Considering tool: {:?}", tool.name.bold());
        if !state.tools.contains_key(&tool.name) {
            print!("\n");
            eprintln!("{}", "==============================================================================================".bright_blue());
            log_info!("[Tools] Installing new tool from {}: {}", tool.source.to_string().bright_yellow(), tool.name.to_string().bright_blue().bold());
            log_debug!("[Tools] Full configuration details for tool '{}': {:?}", tool.name, tool);

            let installation_result = match tool.source.as_str() {
                "github" => github::install(tool),
                "brew" => brew::install(tool),
                other => {
                    log_warn!(
                        "[Tools] Unsupported source '{}' for tool '{}'. Skipping this tool's installation.",
                        other.yellow(),
                        tool.name.bold()
                    );
                    None
                }
            };

            if let Some(tool_state) = installation_result {
                state.tools.insert(tool.name.clone(), tool_state);
                tools_updated = true;
                log_info!("[Tools] {}: {}", "Successfully installed tool".yellow() ,tool.name.bold().bright_green());
                eprintln!("{}", "==============================================================================================".blue());
                print!("\n");
            } else {
                log_error!(
                    "[Tools] Failed to install tool: {}. Please review previous logs for specific errors during installation.",
                    tool.name.bold().red()
                );
            }
        } else {
            skipped_tools.push(tool.name.clone());
            log_debug!("[Tools] Tool '{}' is already recorded as installed. Added to skipped list.", tool.name.blue());
        }
    }

    if !skipped_tools.is_empty() {
        let skipped_tools_str = skipped_tools.join(", ");
        log_info!(
            "[Tools] The following tools were already recorded as installed and were skipped: {}",
            skipped_tools_str.blue()
        );
    } else {
        log_debug!("[Tools] No tools were skipped as they were not found in the state.");
    }

    if tools_updated {
        log_info!("[Tools] New tools installed or state updated. Saving current DevBox state...");
        if !save_devbox_state(state, state_path_resolved) {
            log_error!("[StateSave] Failed to save state after tool installations. Data loss risk!");
        }
        log_info!("[StateSave] State saved successfully after tool updates.");
    } else {
        log_info!("[Tools] No new tools installed or state changes detected for tools.");
    }
    eprintln!();
    log_debug!("Exiting install_tools() function.");
}
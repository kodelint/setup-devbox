// For working with file paths, particularly for the state file.
use chrono::Duration;
use std::path::PathBuf;
// For adding color to terminal output, enhancing readability.
// The `Colorize` trait provides methods like `.bright_green()` or `.bold()`
// to style strings for a more informative command-line interface.
use colored::Colorize;
// Imports custom logging macros from the crate root. These macros provide
// consistent logging behavior (e.g., `log_info!`, `log_error!`).
use crate::{log_debug, log_error, log_info, log_warn};
// Imports individual installer modules. Each module is expected to provide
// functions for installing a specific type of tool (e.g., Homebrew packages, Cargo crates).
use crate::installers::{brew, cargo, github, go, pip, rustup, url, uv};
// Imports schema definitions for application state (`DevBoxState`) and tool configurations (`ToolConfig`).
// `DevBoxState` holds the persistent record of installed items, and `ToolConfig` defines
// the structure of the `tools.yaml` file.
use crate::schemas::sdb_schema::{DevBoxState, ToolConfig, ToolEntry, ToolEntryError};
// Imports the function for saving the application state, usually defined in `state_management.rs`.
// This is crucial for persisting changes made during the tool installation process.
use crate::libs::state_management::save_devbox_state;
use crate::libs::utilities::assets::{is_timestamp_older_than, parse_duration, time_since};
use crate::libs::utilities::misc_utils::format_duration;
use crate::libs::utilities::platform::{
    check_installer_command_available, execute_additional_commands,
};

/// Installs tools based on the provided configuration and updates the application state.
///
/// This function iterates through each tool defined in `tools_cfg`,
/// checks if it's already installed and at the correct version according to `state`,
/// and delegates to the appropriate installer for new or updated tools. It also handles
/// state persistence by saving changes to `state.json` after successful installations.
///
/// # Arguments
/// * `tools_cfg`: A `ToolConfig` struct containing the list of tools to install, as parsed from `tools.yaml`.
/// * `state`: A mutable reference to the `DevBoxState` to update the record of installed tools and their versions.
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file.
pub fn install_tools(
    tools_cfg: ToolConfig,
    state: &mut DevBoxState,
    state_path_resolved: &PathBuf,
    update_latest: bool,
) {
    // Add a newline for better visual separation in the terminal output, improving readability of logs.
    eprintln!("\n");
    // Informative log message indicating the start of the tool processing phase.
    log_info!("[Tools] Processing Tools Installations...");
    // Debug log for tracing function entry, useful for detailed debugging.
    log_debug!("Entering install_tools() function.");

    // `tools_updated` flag: Set to `true` if any tool is actually installed or updated during this session.
    // This determines if the `DevBoxState` needs to be saved at the end.
    let mut tools_updated = false;
    // `skipped_tools` vector: Stores the names of tools that were found to be up-to-date and were skipped.
    // This list is used to provide a summary to the user at the end of the process.
    let mut skipped_tools: Vec<String> = Vec::new();
    // Parse the update_latest_only_after duration if provided
    let update_duration = if update_latest {
        // If --update-latest flag is set, use zero duration to force updates
        Duration::seconds(0)
    } else {
        // Otherwise, use the configured duration
        tools_cfg
            .update_latest_only_after
            .as_ref()
            .and_then(|d| parse_duration(d))
            .unwrap_or_else(|| Duration::days(0))
    };

    log_debug!(
        "[Tools] Update policy: {}",
        if update_latest {
            "forced update of all 'latest' version tools (--update-latest flag)".to_string()
        } else {
            format!(
                "only update 'latest' versions if older than {:?}",
                update_duration
            )
        }
    );

    // Iterate over each `tool` entry provided in the `tools_cfg`.
    // The `&tools_cfg.tools` takes a reference to the vector of tools to avoid moving it,
    // allowing `tools_cfg` to be used later if needed.
    // Each `tool` here is a `ToolEntry` struct containing details like name, type, version, etc.,
    // representing a single tool definition from the configuration.
    for tool in &tools_cfg.tools {
        // Log the current tool being considered for installation.
        log_debug!("[Tools] Considering tool: {}", tool.name.bright_green());

        // Schema Validation Step
        // Before attempting any installation, validate the `ToolEntry` against its defined schema.
        // This ensures that the tool's configuration (e.g., 'source' field) is valid and well-formed.
        match tool.validate() {
            Ok(_) => {
                // If validation passes, log a debug message. This indicates the tool's configuration
                // adheres to the expected structure.
                log_debug!(
                    "[Tools] Tool {} passed schema validation.",
                    tool.name.bright_green()
                );
            }
            Err(e) => {
                // If validation fails, handle the specific error.
                match e {
                    ToolEntryError::InvalidSource(_) => {
                        // This specific error indicates that the 'source' field in the tool configuration
                        // (e.g., in `tools.yaml`) is not one of the recognized types (e.g., "GitHub", "Brew").
                        // Log an error message, highlighting the invalid tool and specifying the supported sources.
                        log_error!(
                            "[Tools] Skipping tool '{}' due to configuration error: {}",
                            tool.name.bold().red(),
                            e.to_string().red()
                        );
                        log_error!(
                            "{}: {}",
                            "[Tools] Supported sources are".bright_white(),
                            "GitHub, Brew, Cargo, Rustup, Pip, Go, URL.".bright_green()
                        );
                    }
                    _ => {
                        // For any other type of validation error (not an `InvalidSource`), use a general
                        // error message. This covers cases like missing required fields or other malformed data.
                        log_error!(
                            "[Tools] Skipping tool '{}' due to configuration error: {}",
                            tool.name.bold().red(),
                            e.to_string().red()
                        );
                    }
                }
                // Continue to the next tool in the loop, skipping installation for the invalid entry.
                continue;
            }
        }

        // Installer Availability Check
        // Convert the tool's source enum variant into a lowercase string.
        // This string (e.g., "brew", "go", "cargo") will be used to check for the presence of the corresponding
        // installer command on the system.
        let installer_name_for_check = tool.source.to_string().to_lowercase();

        // Determine if the current tool's source requires a system-level installer command check.
        // Sources like "GitHub" are handled internally (e.g., via `git` or `curl`), so they don't
        // require a separate pre-check for an installer executable in the PATH.
        let needs_installer_check = matches!(
            installer_name_for_check.as_str(),
            "brew" | "go" | "cargo" | "rustup" | "pip" | "uv"
        );

        if needs_installer_check {
            // Check if the required installer command is available in the system's PATH.
            // This prevents the program from trying to run a command that doesn't exist.
            if let Err(installer_err) = check_installer_command_available(&installer_name_for_check)
            {
                // If the installer is not found, log an error and skip the tool.
                log_error!(
                    "[Tools] Skipping tool '{}' (source: {}) because {}. \
                    Please ensure the necessary installer is installed and in your system's PATH.",
                    tool.name.bold().red(),
                    installer_name_for_check.yellow(),
                    installer_err.to_string().red()
                );
                continue;
            }
        }

        // Version Comparison Logic
        // Flag to determine if a new installation or update is needed. It is initially set to true.
        let mut should_install_tool = true;
        // Flag to identify why to skip
        // 1. Specified version is already installed
        // 2. Latest version is already installed and within the threshold
        //    - Threshold is defined in the `tools.yaml` as `update_latest_only_after`
        //    - Compare with `last_updated` from `state.json` file
        let mut skip_reason = None;

        // Use `if let Some(...)` to safely check if a tool with the same name already exists in the state file.
        // If it does, `existing_tool_state` will be a reference to the `ToolState` struct for that tool.
        if let Some(existing_tool_state) = state.tools.get(&tool.name) {
            // Log that the tool was found in the state file, indicating a potential update rather than a new install.
            log_debug!(
                "[Tools] Tool '{}' found in state file. Comparing versions.",
                tool.name.bright_green()
            );

            // Check if tool version is "latest" and should respect update policy
            let is_latest_version = tool.version.as_ref().map_or(true, |v| v == "latest")
                || existing_tool_state.version == "latest";

            if is_latest_version && !update_latest {
                // Check if we should skip update based on last_updated timestamp
                // The threshold is defined in `tools.yaml` as `update_latest_only_after`
                // Only apply time-based update policy if --update-latest flag is NOT set
                if let Some(last_updated) = &existing_tool_state.last_updated {
                    if !is_timestamp_older_than(last_updated, &update_duration) {
                        should_install_tool = false;
                        skip_reason = Some(format!(
                            "version 'latest' was updated {} (within {} threshold)",
                            time_since(last_updated).unwrap_or_else(|| "recently".to_string()),
                            format_duration(&update_duration)
                        ));
                    }
                }
                // If no last_updated timestamp exists, we should update
            }

            if should_install_tool {
                if let Some(config_version) = &tool.version {
                    if existing_tool_state.version == *config_version && config_version != "latest"
                    {
                        should_install_tool = false;
                        skip_reason = Some("specified version already installed".to_string());
                    }
                } else {
                    // No version was specified in the configuration (`tools.yaml`).
                    // In this case, we assume the user wants the latest version. For simplicity,
                    // we proceed with installation to ensure the tool is at its latest available version.
                    if !existing_tool_state.version.is_empty()
                        && existing_tool_state.version != "latest"
                    {
                        should_install_tool = false;
                        skip_reason =
                            Some("version not specified but tool is installed".to_string());
                    }
                }
            }

            // If the `should_install_tool` flag is `false` (meaning versions matched),
            // add the tool name to the skipped list and continue to the next tool in the loop.
            if !should_install_tool {
                if let Some(reason) = skip_reason {
                    log_debug!(
                        "[Tools] {} '{}': {}",
                        "Skipping tool".italic().dimmed(),
                        tool.name.bright_green().bold().italic(),
                        reason.italic().dimmed()
                    );
                }
                skipped_tools.push(tool.name.clone());
                log_debug!(
                    "[Tools] {}",
                    format!(
                        "Tool '{}' added to skipped list.",
                        tool.name.bright_green().bold().italic()
                    )
                    .italic()
                    .dimmed()
                );
                continue;
            }
        } else {
            // The tool was not found in the state file, which means it's a new installation.
            // Log an info message and proceed with the installation process.
            log_info!(
                "[Tools] Tool '{}' not found in state file. It will be installed.",
                tool.name.bright_green()
            );
        }

        // Installation Process
        // This part of the code is only reached if the tool is new or needs an update.
        print!("\n");
        eprintln!(
            "{}",
            "==================================================================\
        ============================"
                .bright_blue()
        );

        // Determine installation reason for logging
        let install_reason = if state.tools.contains_key(&tool.name) {
            "Updating"
        } else {
            "Installing"
        };

        log_info!(
            "[Tools] {} tool from {}: {}",
            install_reason.bright_yellow(),
            tool.source.to_string().bright_yellow(),
            tool.name.to_string().bright_blue().bold()
        );

        log_debug!(
            "[Tools] Full configuration details for tool '{}': name='{}', version='{}', source='{}', \
            repo='{}', tag='{}', rename_to='{}' and reason='{}'",
            tool.name.green(),
            tool.name,
            tool.version.as_deref().unwrap_or("N/A"),
            tool.source,
            tool.repo.as_deref().unwrap_or("N/A"),
            tool.tag.as_deref().unwrap_or("N/A"),
            tool.rename_to.as_deref().unwrap_or("N/A"),
            install_reason,
        );

        // Use a `match` statement to call the correct installer function based on the tool's source.
        // Each installer function (e.g., `github::install`, `brew::install`) returns an `Option<ToolState>`.
        // `Some(tool_state)` on success, `None` on failure.
        let installation_result = match tool.source.to_string().to_lowercase().as_str() {
            "github" => github::install(tool),
            "brew" => brew::install(tool),
            "go" => go::install(tool),
            "cargo" => cargo::install(tool),
            "rustup" => rustup::install(tool),
            "pip" => pip::install(tool),
            "uv" => uv::install(tool),
            "url" => url::install(tool),
            other => {
                // Handle unsupported or unrecognized source types.
                log_warn!(
                    "[Tools] Installer for source type '{}' not yet implemented (or \
                    unrecognized for this context) for tool '{}'. Skipping this tool's installation.",
                    other.yellow(),
                    tool.name.bold()
                );
                // Return `None` to indicate installation was not attempted.
                None
            }
        };

        // Handle the result of the installation attempt.
        if let Some(tool_state) = installation_result {
            // If installation was successful, insert the new `ToolState` into the `DevBoxState` HashMap.
            // This overwrites any old state for the same tool, effectively performing an update.
            state.tools.insert(tool.name.clone(), tool_state);
            // Set the `tools_updated` flag to `true` to signal that the state needs to be saved.
            tools_updated = true;
            // Log a success message for the installed tool.
            log_info!(
                "[Tools] {}: {}",
                "Successfully installed tool".yellow(),
                tool.name.bold().bright_green()
            );
            eprintln!(
                "{}",
                "=================================================================\
            ============================="
                    .blue()
            );
            print!("\n");
        } else {
            // If `installation_result` is `None`, it means the installation failed.
            // Log an error message to inform the user.
            log_error!(
                "[Tools] Failed to install tool: {}. Please review previous logs for specific errors during installation.",
                tool.name.bold().red()
            );
        }
    }

    // Post-Installation Summary
    // After the loop, check if any tools were skipped.
    if !skipped_tools.is_empty() {
        println!();
        // If there were skipped tools, format and print a summary.
        log_info!("[Tools] Below tools were already up to date and were skipped:");
        eprintln!(
            "{}",
            "====================================================================".blue()
        );
        // Print tools in chunks of 5 per line
        for chunk in skipped_tools.chunks(5) {
            let tools_line = chunk
                .iter()
                .map(|tool| tool.bright_yellow().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!("[Skipped Tools] {}", tools_line);
        }
        eprintln!(
            "{}",
            "====================================================================".blue()
        );
        println!();
    } else {
        // If no tools were skipped, log a debug message to provide more context for a user looking at detailed logs.
        log_debug!(
            "[Tools] No tools were skipped as they were either not found in the state or needed an update."
        );
    }

    // State Management
    // Check if the `tools_updated` flag was set to `true` during the process.
    if tools_updated {
        // If so, it means changes were made, and the state file must be saved.
        log_info!("[Tools] New tools installed or state updated. Saving current DevBox state...");
        if !save_devbox_state(state, state_path_resolved) {
            // If saving fails (e.g., due to file permissions), log a critical error.
            log_error!(
                "[StateSave] Failed to save state after tool installations. Data loss risk!"
            );
        } else {
            // On success, log an informative message.
            log_info!("[StateSave] State saved successfully after tool updates.");
        }
    } else {
        // If no tools were installed or updated, no need to save the state.
        log_info!("[Tools] No new tools installed or state changes detected for tools.");
    }

    eprintln!();
    log_debug!("Exiting install_tools() function.");
}

/// Executes additional commands after the main installation is complete.
/// These commands are often used for post-installation setup, such as copying
/// configuration files, creating directories, or setting up symbolic links.
///
/// # Arguments
/// * `installer_prefix` - The prefix for log messages (e.g., "[GitHub]", "[Cargo]")
/// * `tool_entry` - Reference to the ToolEntry containing the tool configuration
/// * `working_dir` - The working directory where commands should be executed
///
/// # Returns
/// * `Option<Vec<String>>` - Some with executed commands if successful, None if no commands to execute or if commands failed
/// * The function continues execution even if commands fail, only logging warnings instead of errors
pub(crate) fn execute_post_installation_commands(
    installer_prefix: &str,
    tool_entry: &ToolEntry,
    working_dir: &std::path::Path,
) -> Option<Vec<String>> {
    if let Some(ref additional_commands) = tool_entry.additional_cmd {
        if !additional_commands.is_empty() {
            log_info!(
                "{} Tool {} has {} additional command(s) to execute",
                installer_prefix,
                tool_entry.name.bold(),
                additional_commands.len().to_string().yellow()
            );

            log_debug!(
                "{} Additional commands will execute from working directory: {}",
                installer_prefix,
                working_dir.display().to_string().cyan()
            );

            match execute_additional_commands(
                installer_prefix,
                additional_commands,
                working_dir,
                &tool_entry.name,
            ) {
                Ok(executed_cmds) => {
                    log_info!(
                        "{} Successfully completed all additional commands for {}",
                        installer_prefix,
                        tool_entry.name.to_string().green()
                    );
                    Some(executed_cmds)
                }
                Err(err) => {
                    log_warn!(
                        "{} Failed to execute additional commands for {}: {}. Continuing with installation.",
                        installer_prefix,
                        tool_entry.name.to_string().yellow(),
                        err.yellow()
                    );
                    None
                }
            }
        } else {
            log_debug!(
                "{} Tool {} has additional_cmd field but it's empty, skipping",
                installer_prefix,
                tool_entry.name.dimmed()
            );
            None
        }
    } else {
        log_debug!(
            "{} No additional commands specified for {}",
            installer_prefix,
            tool_entry.name.dimmed()
        );
        None
    }
}

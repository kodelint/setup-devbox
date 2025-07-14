// For working with file paths, particularly for the state file.
use std::path::PathBuf;
// For adding color to terminal output, enhancing readability.
use colored::Colorize;
// Imports custom logging macros from the crate root. These macros provide
// consistent logging behavior (e.g., `log_info!`, `log_error!`).
use crate::{log_debug, log_error, log_info, log_warn};
// Imports individual installer modules. Each module is expected to provide
// functions for installing a specific type of tool (e.g., Homebrew packages, Cargo crates).
use crate::installers::{brew, cargo, github, go, pip, rustup};
// Imports schema definitions for application state (`DevBoxState`) and tool configurations (`ToolConfig`).
// `DevBoxState` holds the persistent record of installed items, and `ToolConfig` defines
// the structure of the `tools.yaml` file.
use crate::schema::{DevBoxState, ToolConfig, ToolEntryError}; // <--- ADD ToolEntryError here
// Imports the function for saving the application state, usually defined in `state_management.rs`.
// This is crucial for persisting changes made during the tool installation process.
use crate::libs::state_management::save_devbox_state;
use crate::libs::utilities::platform::check_installer_command_available;

/// Installs tools based on the provided configuration and updates the application state.
///
/// This function iterates through each tool defined in `tools_cfg`,
/// checks if it's already installed according to `state` (to prevent re-installation or unnecessary work),
/// and delegates to the appropriate installer (e.g., `brew`, `GitHub`, `go`, `cargo`, `pip`, `rustup`)
/// for new or updated tools. It also handles state persistence by saving changes to `state.json`
/// after successful installations.
///
/// # Arguments
/// * `tools_cfg`: A `ToolConfig` struct containing the list of tools to install, as parsed from `tools.yaml`.
/// * `state`: A mutable reference to the `DevBoxState` to update the record of installed tools and their versions.
///            This allows tracking what's already on the system.
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file. This path is used when saving
///                          the updated application state to disk.
pub fn install_tools(tools_cfg: ToolConfig, state: &mut DevBoxState, state_path_resolved: &PathBuf) {
    // Add a newline for better visual separation in the terminal output, improving readability of logs.
    eprintln!("\n");
    // Informative log message indicating the start of the tool processing phase.
    log_info!("[Tools] Processing Tools Installations...");
    // Debug log for tracing function entry, useful for detailed debugging.
    log_debug!("Entering install_tools() function.");

    // `tools_updated` flag: Set to `true` if any tool is actually installed or updated during this session.
    // This determines if the `DevBoxState` needs to be saved at the end.
    let mut tools_updated = false;
    // `skipped_tools` vector: Stores the names of tools that were found to be already installed
    // and up-to-date, so their installation was skipped.
    let mut skipped_tools: Vec<String> = Vec::new();

    // Iterate over each `tool` entry provided in the `tools_cfg`.
    // The `&tools_cfg.tools` takes a reference to the vector of tools to avoid moving it,
    // allowing `tools_cfg` to be used later if needed.
    // Each `tool` here is a `ToolEntry` struct containing details like name, type, version, etc.,
    // representing a single tool definition from the configuration.
    for tool in &tools_cfg.tools {
        // Construct a unique key for the tool based on its name.
        // This key is used to store and retrieve the tool's state in the `DevBoxState` HashMap.
        // In this specific snippet, `tool.name` is used directly as the key.
        log_debug!("[Tools] Considering tool: {:?}", tool.name.bold());

        // Schema Validation Step
        // Before attempting any installation, validate the `ToolEntry` against its defined schema.
        // This ensures that the tool's configuration (e.g., 'source' field) is valid and well-formed.
        match tool.validate() {
            Ok(_) => {
                // If validation passes, log a debug message. This indicates the tool's configuration
                // adheres to the expected structure.
                log_debug!("[Tools] Tool '{}' passed schema validation.", tool.name.bold());
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
                        log_error!("{}: {}","[Tools] Supported sources are".bright_white(), "GitHub, Brew, Cargo, Rustup, Pip, Go, URL.".bright_green())
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

        // Convert the tool's source enum variant into a lowercase string.
        // This string (e.g., "brew", "go", "cargo") will be used to check for the presence of the corresponding
        // installer command on the system.
        let installer_name_for_check = tool.source.to_string().to_lowercase();

        // Determine if the current tool's source requires a system-level installer command check.
        // Sources like "github" are handled internally (e.g., via `git` or `curl`), so they don't
        // require a separate pre-check for an installer executable in the PATH.
        let needs_installer_check = matches!(
            installer_name_for_check.as_str(),
            "brew" | "go" | "cargo" | "rustup" | "pip"
        );

        // Installer Availability Check
        // If the tool's source is one that relies on a system-level installer (e.g., Homebrew, Go, Cargo),
        // perform a check to ensure that the installer's command is actually available in the system's PATH.
        if needs_installer_check {
            // Call `check_installer_command_available` to verify the presence of the installer.
            // If it returns an `Err`, it means the installer command was not found.
            if let Err(installer_err) = check_installer_command_available(&installer_name_for_check) {
                // Log a detailed error message indicating why the tool is being skipped.
                // This message provides actionable advice, telling the user to install the missing installer.
                log_error!(
                    "[Tools] Skipping tool '{}' (source: {}) because {}. \
                    Please ensure the necessary installer is installed and in your system's PATH.",
                    tool.name.bold().red(),        // Highlight the skipped tool's name
                    installer_name_for_check.yellow(), // Show the missing installer's name
                    installer_err.to_string().red()    // Display the specific error message from `InstallerError`
                );
                // Skip to the next tool in the configuration if the required installer is not available.
                continue;
            }
        }
        // Check if the tool is already recorded in the current `DevBoxState`.
        // `!state.tools.contains_key(&tool.name)`: This condition is true if the tool's name
        // is NOT found as a key in the `state.tools` HashMap, meaning it's a new tool
        // that needs to be installed.
        if !state.tools.contains_key(&tool.name) {
            // Add a newline character for visual clarity in the terminal output,
            // creating a separation before displaying installation logs for a new tool.
            print!("\n");
            // Print a visual separator line to make the start of a new tool installation
            // visually distinct in the terminal, using a bright blue color for emphasis.
            eprintln!("{}", "==================================================================\
            ============================".bright_blue());
            // Log an informative message indicating that a new tool is being installed,
            // displaying its source and name with distinct colors for easy identification.
            log_info!("[Tools] Installing new tool from {}: {}", tool.source.to_string().bright_yellow(), tool.name.to_string().bright_blue().bold());
            // Log the full configuration details of the tool at debug level. This is helpful
            // for understanding exactly what parameters are being used for the installation.
            log_debug!("[Tools] Full configuration details for tool '{}': {:?}", tool.name, tool);

            // Dispatch the installation to the appropriate installer function based on the tool's `source`.
            // The `match` statement checks the string value of `tool.source` (e.g., "GitHub", "brew").
            // Each branch calls a specific installer function (e.g., `github::install(tool)`),
            // which is responsible for the actual installation logic for that source type.
            // Each installer is expected to return an `Option<ToolState>`, which will be `Some` on success
            // containing the installed tool's state, or `None` on failure.
            // Convert enum to string for matching
            let installation_result = match tool.source.to_string().to_lowercase().as_str() {
                "github" => github::install(tool), // Call the GitHub installer for tools with "GitHub" source.
                "brew" => brew::install(tool),     // Call the Homebrew installer for tools with "brew" source.
                "go" => go::install(tool),         // Call the Go installer for tools with "go" source.
                "cargo" => cargo::install(tool),   // Call the Cargo installer for tools with "cargo" source.
                "rustup" => rustup::install(tool), // Call the Rustup installer for tools with "rustup" source.
                "pip" => pip::install(tool),       // Call the PIP installer for tools with "pip" source.
                // "url" => direct_url_installer::install(tool), // ToDo: need to create this module for direct URL installations
                other => {
                    // If the tool's source is not recognized or supported, log a warning
                    // message and skip the installation of this particular tool.
                    log_warn!(
                "[Tools] Installer for source type '{}' not yet implemented (or \
                unrecognized for this context) for tool '{}'. Skipping this tool's installation.",
                other.yellow(),
                tool.name.bold()
            );
                    None // Return `None` to indicate that no tool state was generated due to skipping.
                }
            };

            // Check the `installation_result`. If it's `Some(tool_state)`, it means the installation
            // was successful and `tool_state` contains the details of the installed tool.
            if let Some(tool_state) = installation_result {
                // If installation was successful, insert the newly installed tool's state
                // into the `state.tools` HashMap. The `tool.name.clone()` is used as the key,
                // and `tool_state` (which typically includes version, source, etc., specific to the installed instance)
                // is the value. This updates the application's persistent state.
                state.tools.insert(tool.name.clone(), tool_state);
                // Set the `tools_updated` flag to `true`. This flag is used at the end of the `install_tools`
                // function to determine if the `DevBoxState` needs to be saved to disk.
                tools_updated = true;
                // Log a success message for the installed tool, indicating that it was successfully handled.
                log_info!("[Tools] {}: {}", "Successfully installed tool".yellow() ,tool.name.bold().bright_green());
                // Print another visual separator line, this time in blue, to mark the end of
                // a successful tool installation block in the terminal output.
                eprintln!("{}", "=================================================================\
                =============================".blue());
                // Add a final newline for further visual separation.
                print!("\n");
            } else {
                // If `installation_result` was `None`, it means the installation failed for some reason.
                // Log an error message, prompting the user to review earlier logs for more specific details
                // about the failure.
                log_error!(
            "[Tools] Failed to install tool: {}. Please review previous logs for specific errors during installation.",
            tool.name.bold().red()
        );
            }
        } else {
            // This block is executed if the `!state.tools.contains_key(&tool.name)` condition is false,
            // meaning the tool is already recorded in the `DevBoxState` HashMap (i.e., it was previously installed).
            // Add the name of the already installed tool to the `skipped_tools` list.
            skipped_tools.push(tool.name.clone());
            // Log a debug message indicating that the tool was skipped because it's already recorded
            // as installed, providing the tool's name.
            log_debug!("[Tools] Tool '{}' is already recorded as installed. Added to skipped list.", tool.name.blue());
        }
    }

    // Report on any tools that were skipped because they were already installed.
    // This block provides a summary to the user about tools that did not require installation
    // or updates based on the current `DevBoxState`.
    if !skipped_tools.is_empty() {
        // If the `skipped_tools` vector is not empty, it means there were tools that were skipped.
        // Join the vector of skipped tool names into a single comma-separated string for display.
        let skipped_tools_str = skipped_tools.join(", ");
        // Log an informative message listing all the tools that were skipped,
        // indicating that they were already recorded as installed and up-to-date.
        // Display the list of skipped tools in blue for emphasis.
        log_info!(
        "[Tools] Below tools were already recorded as installed and were skipped:"
        );
        log_info!(
        "[Tools] Installed Tools: [ {} ]",
        skipped_tools_str.blue()
        );
    } else {
        // If the `skipped_tools` vector is empty, it means no tools were skipped.
        // Log a debug message confirming that all tools found in the configuration were processed
        // (either installed/updated or an attempt was made).
        log_debug!("[Tools] No tools were skipped as they were not found in the state.");
    }

    // If any tools were installed or updated, save the `DevBoxState` to `state.json`.
    // This crucial step persists the changes made to the application's state,
    // ensuring that `setup-devbox` remembers which tools are installed and their details
    // for future runs.
    if tools_updated {
        // If the `tools_updated` flag is `true`, it indicates that at least one tool
        // was newly installed or updated during the current execution.
        log_info!("[Tools] New tools installed or state updated. Saving current DevBox state...");
        // Call the `save_devbox_state` function. This function attempts to serialize
        // the current `state` (which now reflects the changes) to the specified
        // `state_path_resolved` file. It returns `true` on success, `false` on failure.
        if !save_devbox_state(state, state_path_resolved) {
            // If `save_devbox_state` returns `false` (meaning save failed), log a critical error.
            // This alerts the user to potential data loss, as the application's memory
            // of installed tools might not be correctly preserved.
            log_error!("[StateSave] Failed to save state after tool installations. Data loss risk!");
        } else {
            // If `save_devbox_state` returns `true` (meaning save succeeded), log a success message.
            log_info!("[StateSave] State saved successfully after tool updates.");
        }
    } else {
        // If the `tools_updated` flag is `false`, it means no new tools were installed,
        // and no existing tools were updated in this session.
        // Log an informative message indicating that the state file does not need to be written
        // because no relevant changes occurred in this phase.
        log_info!("[Tools] No new tools installed or state changes detected for tools.");
    }
    // Print an empty line to the standard error stream. This is typically used for
    // consistent visual spacing in the terminal, separating the output of this function
    // from subsequent logs or commands.
    eprintln!();
    // Log a debug message indicating the function is about to exit. This is useful
    // for tracing the execution flow of the application during debugging.
    log_debug!("Exiting install_tools() function.");
}
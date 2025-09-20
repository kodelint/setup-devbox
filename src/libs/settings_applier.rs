// This module is responsible for applying system-level settings, primarily targeting macOS
// using the `defaults` command. It reads desired settings from the `SettingsConfig` and
// compares them against the current `DevBoxState` to determine if an update is needed.
// The module handles various data types for settings (boolean, integer, float, string, array, dictionary)
// and persists the changes back to the `DevBoxState` if settings are successfully applied.
// It also includes platform-specific conditional compilation to ensure macOS-specific code
// only runs on macOS systems.

// Internal module imports:
use crate::libs::state_management::save_devbox_state; // Imports the function to save the `DevBoxState` to disk.
// Imports schema definitions. `DevBoxState` for application's runtime state,
// `SettingState` for representing the state of an individual setting, and `SettingsConfig`
// for the configuration parsed from `settings.yaml`.
use crate::schemas::os_settings::SettingsConfig;
use crate::schemas::state_file::{DevBoxState, SettingState};
use crate::{log_debug, log_error, log_info, log_warn}; // Custom logging macros for various log levels.

// External crate imports:
use colored::Colorize; // Imports the `Colorize` trait for adding color to console output.
use std::path::PathBuf; // Provides `PathBuf` for working with file paths.
use std::process::Command;

// Provides `Command` for spawning and managing child processes (e.g., `defaults` command).

/// Applies system settings based on the provided configuration and updates the application state.
///
/// This function is the core logic for applying macOS system settings. It iterates through
/// each setting defined in `settings_cfg.settings.macos`, compares its current value (if recorded
/// in `state`) with the desired value from the configuration, and executes the `defaults write`
/// command if an update is necessary. It handles different data types for `defaults` commands
/// and updates the `DevBoxState` upon successful application, ensuring state persistence.
/// On non-macOS systems, it logs a warning if macOS-specific settings are present but skips their application.
///
/// # Arguments
/// * `settings_cfg`: A `SettingsConfig` struct containing the desired system settings.
/// * `state`: A mutable reference to the `DevBoxState` to read current setting states and
///            to update with newly applied settings.
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file, used for saving the
///                          updated application state.
pub fn apply_system_settings(
    settings_cfg: SettingsConfig,
    state: &mut DevBoxState,
    state_path_resolved: &PathBuf,
) {
    eprintln!("\n");
    eprintln!("{}:", "OS Settings".bright_yellow().bold());
    println!("{}\n", "=".repeat(12).bright_yellow());
    log_info!("[OS Settings] Applying System Settings..."); // Informative log that the settings application process has started.
    log_debug!("Entering apply_system_settings() function."); // Debug log for function entry.

    let mut settings_updated_in_session = false; // Flag to track if any settings were applied or updated in the current session.

    // Conditional compilation: This block is only compiled and executed when the target OS is macOS.
    #[cfg(target_os = "macos")]
    {
        // Iterate over each setting entry defined under `settings.macos` in the configuration.
        for entry in settings_cfg.settings.macos {
            // Construct the full key string (e.g., "com.apple.finder.ShowStatusBar").
            let full_key = format!("{}.{}", entry.domain, entry.key);
            let desired_value = entry.value; // The value to be set.
            let setting_type = entry.value_type; // The data type of the setting (e.g., "bool", "string").

            log_debug!(
                "[OS Settings] Considering setting: {} = {} (type: {})",
                full_key.bold(),
                desired_value.blue(),
                setting_type.cyan()
            );

            // Check if the setting needs to be applied:
            // 1. If the setting is not in the `state` yet (first time application).
            // 2. If the setting is in the `state` but its recorded value or type differs from the desired one.
            if state.settings.get(&full_key).map_or(true, |s_state| {
                s_state.value != desired_value || s_state.value_type != setting_type
            }) {
                log_info!(
                    "[OS Settings] Attempting to apply setting: {} = {} (type: {})",
                    full_key.bold().cyan(),
                    desired_value.yellow(),
                    setting_type.magenta()
                );

                // Determine the correct `defaults` command flag based on the `setting_type`.
                let type_flag = match setting_type.as_str() {
                    "bool" => "-bool",
                    "int" => "-int",
                    "float" => "-float",
                    "string" => "-string",
                    "array" => "-array",
                    "dict" => "-dict",
                    _ => {
                        // Log a warning if an unknown type is encountered and default to "-string".
                        log_warn!(
                            "[OS Settings] Unknown setting type '{}' for key '{}'. Defaulting to '-string'.",
                            setting_type.yellow(),
                            full_key.yellow()
                        );
                        "-string"
                    },
                };

                // Create a new `Command` to execute `defaults write`.
                let mut command = Command::new("defaults");
                command
                    .arg("write") // Subcommand for writing a default.
                    .arg(&entry.domain) // The domain (e.g., "com.apple.finder").
                    .arg(&entry.key) // The key within the domain.
                    .arg(type_flag); // The type flag (e.g., "-bool", "-string").

                // Add the value argument(s) based on the setting type.
                match type_flag {
                    "-array" => {
                        // For arrays, parse the desired_value string (e.g., "(item1, item2)") into individual strings.
                        let parsed_values: Vec<String> = desired_value
                            .trim_matches(|c| c == '(' || c == ')') // Remove leading/trailing parentheses.
                            .split(',') // Split by comma.
                            .map(|s| s.trim().to_string()) // Trim whitespace and convert to String.
                            .collect();
                        command.args(&parsed_values); // Add each parsed item as a separate argument.
                    },
                    "-dict" => {
                        // For dictionaries, parse the desired_value string (e.g., "{key1=val1, key2=val2}") into key-value pairs.
                        let parsed_items: Vec<String> = desired_value
                            .trim_matches(|c| c == '{' || c == '}') // Remove leading/trailing braces.
                            .split(',') // Split by comma.
                            .flat_map(|pair| {
                                let mut split = pair.splitn(2, '='); // Split each pair by the first '='.
                                match (split.next(), split.next()) {
                                    (Some(k), Some(v)) => vec![k.trim().to_string(), v.trim().to_string()], // Collect key and value.
                                    _ => {
                                        log_warn!("[OS Settings] Malformed dictionary entry: '{}'. Skipping.", pair.yellow());
                                        vec![] // Return empty vector if malformed, effectively skipping this pair.
                                    }
                                }
                            })
                            .collect();
                        command.args(&parsed_items); // Add all parsed keys and values as arguments.
                    },
                    _ => {
                        // For other types (bool, int, float, string), the value is a single argument.
                        command.arg(&desired_value);
                    },
                }

                // Execute the `defaults` command and capture its output.
                match command.output() {
                    Ok(output) => {
                        if output.status.success() {
                            log_info!(
                                "[OS Settings] Successfully applied '{}' to domain '{}'.",
                                entry.key,
                                entry.domain
                            );
                            // Store the new `SettingState` in `DevBoxState`, including the value and its type.
                            state.settings.insert(
                                full_key.clone(),
                                SettingState {
                                    domain: entry.domain.clone(),
                                    key: entry.key.clone(),
                                    value: desired_value.clone(),
                                    value_type: setting_type.clone(), // Store the type here!
                                },
                            );
                            settings_updated_in_session = true; // Mark that a setting was updated.
                        } else {
                            // Log a detailed error if the command failed.
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            log_error!(
                                "[OS Settings] Failed to apply setting '{}' (Domain: '{}', Key: '{}'). Exit code: {}. Error: {}",
                                full_key.bold().red(),
                                entry.domain.red(),
                                entry.key.red(),
                                output.status.code().unwrap_or(-1), // Get exit code, default to -1 if not available.
                                stderr.red()
                            );
                            if !output.stdout.is_empty() {
                                log_debug!(
                                    "[OS Settings] Stdout (on failure): {}",
                                    String::from_utf8_lossy(&output.stdout)
                                );
                            }
                        }
                    },
                    Err(e) => {
                        // Log an error if the `defaults` command itself could not be executed (e.g., command not found).
                        log_error!(
                            "[OS Settings] Could not execute 'defaults' command for setting '{}'. Error: {}",
                            full_key.bold().red(),
                            e.to_string().red()
                        );
                    },
                }
            } else {
                log_debug!(
                    "[OS Settings] Setting '{}' already matches desired value. Skipping application.",
                    full_key.blue()
                );
            }
        }
    }

    // Conditional compilation: This block is only compiled and executed when the target OS is NOT macOS.
    #[cfg(not(target_os = "macos"))]
    {
        // Check if macOS-specific settings were defined in the config on a non-macOS system.
        if !settings_cfg.settings.macos.is_empty() {
            log_warn!(
                "[OS Settings] macOS specific settings were found in config but this is not a macOS system. Skipping macOS settings application. Support for other operating systems is planned."
            );
        } else {
            log_debug!(
                "[OS Settings] No macOS settings found in config for non-macOS system. Skipping settings application phase."
            );
        }
    }

    // If any settings were updated during the session, save the `DevBoxState`.
    if settings_updated_in_session {
        log_info!(
            "[OS Settings] One or more settings were applied or updated. Saving current DevBox state..."
        );
        if !save_devbox_state(state, state_path_resolved) {
            log_error!(
                "[StateSave] Failed to save state after settings application. Data loss risk!"
            ); // Error if state saving fails.
        } else {
            log_info!("[StateSave] State saved successfully after settings updates."); // Success if state saved.
        }
    } else {
        log_info!(
            "[OS Settings] No new settings applied or state changes detected for settings in this run."
        ); // Informative log if no changes.
    }

    eprintln!(); // Print a newline for final console formatting.
    log_debug!("Exiting apply_system_settings() function."); // Debug log for function exit.
}

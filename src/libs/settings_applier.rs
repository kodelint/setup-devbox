// This module is responsible for applying system-level settings, primarily targeting macOS
// using the `defaults` command. It reads desired settings from the `SettingsConfig` and
// compares them against the current `DevBoxState` to determine if an update is needed.
// The module handles various data types for settings (boolean, integer, float, string, array, dictionary)
// and persists the changes back to the `DevBoxState` if settings are successfully applied.
// It also includes platform-specific conditional compilation to ensure macOS-specific code
// only runs on macOS systems.

// =========================================================================== //
//                          STANDARD LIBRARY DEPENDENCIES                      //
// =========================================================================== //

use std::path::PathBuf; // Provides `PathBuf` for working with file paths.
use std::process::Command;

// =========================================================================== //
//                             EXTERNAL DEPENDENCIES                           //
// =========================================================================== //

use colored::Colorize;

// =========================================================================== //
//                              INTERNAL IMPORTS                               //
// ===========================================================================

use crate::libs::state_management::save_devbox_state;
use crate::schemas::os_settings::SettingsConfig;
use crate::schemas::state_file::{DevBoxState, SettingState};
use crate::{log_debug, log_error, log_info, log_warn};

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
///   to update with newly applied settings.
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file, used for saving the
///   updated application state.
pub fn apply_system_settings(
    settings_cfg: SettingsConfig,
    state: &mut DevBoxState,
    state_path_resolved: &PathBuf,
) {
    eprintln!("\n");
    eprintln!("{}:", "OS Settings".bright_yellow().bold());
    println!("{}\n", "=".repeat(12).bright_yellow());
    log_info!("[OS Settings] Applying System Settings...");
    log_debug!("Entering apply_system_settings() function.");

    let mut settings_updated_in_session = false;

    // macOS settings using defaults command
    #[cfg(target_os = "macos")]
    {
        for entry in settings_cfg.settings.macos {
            let full_key = format!("{}.{}", entry.domain, entry.key);
            let desired_value = entry.value;
            let setting_type = entry.value_type;

            log_debug!(
                "[OS Settings] Considering setting: {} = {} (type: {})",
                full_key.bold(),
                desired_value.blue(),
                setting_type.cyan()
            );

            if !state.settings.get(&full_key).is_some_and(|s_state| {
                s_state.value == desired_value && s_state.value_type == setting_type
            }) {
                log_info!(
                    "[OS Settings] Attempting to apply setting: {} = {} (type: {})",
                    full_key.bold().cyan(),
                    desired_value.yellow(),
                    setting_type.magenta()
                );

                let type_flag = match setting_type.as_str() {
                    "bool" => "-bool",
                    "int" => "-int",
                    "float" => "-float",
                    "string" => "-string",
                    "array" => "-array",
                    "dict" => "-dict",
                    _ => {
                        log_warn!(
                            "[OS Settings] Unknown setting type '{}' for key '{}'. Defaulting to '-string'.",
                            setting_type.yellow(),
                            full_key.yellow()
                        );
                        "-string"
                    }
                };

                let mut command = Command::new("defaults");
                command
                    .arg("write")
                    .arg(&entry.domain)
                    .arg(&entry.key)
                    .arg(type_flag);

                match type_flag {
                    "-array" => {
                        let parsed_values: Vec<String> = desired_value
                            .trim_matches(|c| c == '(' || c == ')')
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect();
                        command.args(&parsed_values);
                    }
                    "-dict" => {
                        let parsed_items: Vec<String> = desired_value
                            .trim_matches(|c| c == '{' || c == '}')
                            .split(',')
                            .flat_map(|pair| {
                                let mut split = pair.splitn(2, '=');
                                match (split.next(), split.next()) {
                                    (Some(k), Some(v)) => vec![k.trim().to_string(), v.trim().to_string()],
                                    _ => {
                                        log_warn!("[OS Settings] Malformed dictionary entry: '{}'. Skipping.", pair.yellow());
                                        vec![]
                                    }
                                }
                            })
                            .collect();
                        command.args(&parsed_items);
                    }
                    _ => {
                        command.arg(&desired_value);
                    }
                }

                let output = match command.output() {
                    Ok(out) => out,
                    Err(e) => {
                        log_error!(
                            "[OS Settings] Could not execute 'defaults' command for setting '{}'. Error: {}",
                            full_key.bold().red(),
                            e.to_string().red()
                        );
                        return;
                    }
                };

                if output.status.success() {
                    log_info!(
                        "[OS Settings] Successfully applied '{}' to domain '{}'.",
                        entry.key,
                        entry.domain
                    );
                    state.settings.insert(
                        full_key.clone(),
                        SettingState {
                            domain: entry.domain.clone(),
                            key: entry.key.clone(),
                            value: desired_value.clone(),
                            value_type: setting_type.clone(),
                        },
                    );
                    settings_updated_in_session = true;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log_error!(
                        "[OS Settings] Failed to apply setting '{}' (Domain: '{}', Key: '{}'). Exit code: {}. Error: {}",
                        full_key.bold().red(),
                        entry.domain.red(),
                        entry.key.red(),
                        output.status.code().unwrap_or(-1),
                        stderr.red()
                    );
                    if !output.stdout.is_empty() {
                        log_debug!(
                            "[OS Settings] Stdout (on failure): {}",
                            String::from_utf8_lossy(&output.stdout)
                        );
                    }
                }
            } else {
                log_debug!(
                    "[OS Settings] Setting '{}' already matches desired value. Skipping application.",
                    full_key.blue()
                );
            }
        }
    }

    // Linux settings using gsettings (GNOME) and dconf
    #[cfg(target_os = "linux")]
    {
        for entry in settings_cfg.settings.linux {
            // Linux settings typically use domain.key format for gsettings
            // Example: org.gnome.desktop.interface.clock-format
            let full_key = format!("{}.{}", entry.domain, entry.key);
            let desired_value = entry.value;
            let setting_type = entry.value_type;

            log_debug!(
                "[OS Settings] Considering setting: {} = {} (type: {})",
                full_key.bold(),
                desired_value.blue(),
                setting_type.cyan()
            );

            if !state.settings.get(&full_key).is_some_and(|s_state| {
                s_state.value == desired_value && s_state.value_type == setting_type
            }) {
                log_info!(
                    "[OS Settings] Attempting to apply setting: {} = {} (type: {})",
                    full_key.bold().cyan(),
                    desired_value.yellow(),
                    setting_type.magenta()
                );

                // Determine the correct gsettings type based on the setting_type
                let gsettings_value = match setting_type.as_str() {
                    "bool" => desired_value.clone(),
                    "int" => desired_value.clone(),
                    "double" | "float" => desired_value.clone(),
                    "string" => format!("'{}'", desired_value), // gsettings requires quoted strings
                    "array" => {
                        // Convert "(item1, item2)" to "['item1', 'item2']"
                        let items: Vec<String> = desired_value
                            .trim_matches(|c| c == '(' || c == ')')
                            .split(',')
                            .map(|s| format!("'{}'", s.trim()))
                            .collect();
                        format!("[{}]", items.join(", "))
                    }
                    _ => {
                        log_warn!(
                            "[OS Settings] Unknown setting type '{}' for key '{}'. Defaulting to string.",
                            setting_type.yellow(),
                            full_key.yellow()
                        );
                        format!("'{}'", desired_value)
                    }
                };

                // Use gsettings to set the value
                let output = match Command::new("gsettings")
                    .arg("set")
                    .arg(&entry.domain)
                    .arg(&entry.key)
                    .arg(&gsettings_value)
                    .output()
                {
                    Ok(out) => out,
                    Err(e) => {
                        log_error!(
                            "[OS Settings] Could not execute 'gsettings' command for setting '{}'. Error: {}",
                            full_key.bold().red(),
                            e.to_string().red()
                        );
                        log_warn!(
                            "[OS Settings] Make sure gsettings is installed (usually part of GNOME desktop)."
                        );
                        continue; // Continue to next setting instead of returning
                    }
                };

                if output.status.success() {
                    log_info!(
                        "[OS Settings] Successfully applied '{}' to domain '{}'.",
                        entry.key,
                        entry.domain
                    );
                    state.settings.insert(
                        full_key.clone(),
                        SettingState {
                            domain: entry.domain.clone(),
                            key: entry.key.clone(),
                            value: desired_value.clone(),
                            value_type: setting_type.clone(),
                        },
                    );
                    settings_updated_in_session = true;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log_error!(
                        "[OS Settings] Failed to apply setting '{}' (Domain: '{}', Key: '{}'). Exit code: {}. Error: {}",
                        full_key.bold().red(),
                        entry.domain.red(),
                        entry.key.red(),
                        output.status.code().unwrap_or(-1),
                        stderr.red()
                    );
                    if !output.stdout.is_empty() {
                        log_debug!(
                            "[OS Settings] Stdout (on failure): {}",
                            String::from_utf8_lossy(&output.stdout)
                        );
                    }
                }
            } else {
                log_debug!(
                    "[OS Settings] Setting '{}' already matches desired value. Skipping application.",
                    full_key.blue()
                );
            }
        }
    }

    // Fallback for unsupported operating systems
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        log_warn!(
            "[OS Settings] Settings application is not supported on this operating system. Only macOS and Linux are currently supported."
        );
    }

    // Cross-platform warning if wrong settings are defined
    #[cfg(target_os = "macos")]
    {
        if !settings_cfg.settings.linux.is_empty() {
            log_warn!(
                "[OS Settings] Linux specific settings were found in config but this is a macOS system. Skipping Linux settings."
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        if !settings_cfg.settings.macos.is_empty() {
            log_warn!(
                "[OS Settings] macOS specific settings were found in config but this is a Linux system. Skipping macOS settings."
            );
        }
    }

    // Save state if any settings were updated
    if settings_updated_in_session {
        log_info!(
            "[OS Settings] One or more settings were applied or updated. Saving current DevBox state..."
        );
        if !save_devbox_state(state, state_path_resolved) {
            log_error!(
                "[StateSave] Failed to save state after settings application. Data loss risk!"
            );
        } else {
            log_info!("[StateSave] State saved successfully after settings updates.");
        }
    } else {
        log_info!(
            "[OS Settings] No new settings applied or state changes detected for settings in this run."
        );
    }

    eprintln!();
    log_debug!("Exiting apply_system_settings() function.");
}

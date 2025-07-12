// src/libs/settings_applier.rs

use std::process::Command;
use std::path::PathBuf;
use colored::Colorize;
use crate::{log_debug, log_error, log_info, log_warn};
// Make sure to import SettingEntry here (since it's used as the config entry type)
use crate::schema::{DevBoxState, SettingsConfig, SettingEntry, SettingState};
use crate::libs::state_management::save_devbox_state;

/// Applies system settings based on the provided configuration and updates the application state.
// ... (documentation remains the same) ...
pub fn apply_system_settings(settings_cfg: SettingsConfig, state: &mut DevBoxState, state_path_resolved: &PathBuf) {
    eprintln!("\n");
    log_info!("[OS Settings] Applying System Settings...");
    log_debug!("Entering apply_system_settings() function.");

    let mut settings_updated_in_session = false;

    #[cfg(target_os = "macos")]
    {
        for entry in settings_cfg.settings.macos { // `entry` is now `SettingEntry`
            let full_key = format!("{}.{}", entry.domain, entry.key);
            let desired_value = entry.value;
            let setting_type = entry.value_type; // Correctly using entry.value_type from the config

            log_debug!("[OS Settings] Considering setting: {} = {} (type: {})", full_key.bold(), desired_value.blue(), setting_type.cyan());

            if state.settings.get(&full_key).map_or(true, |s_state| s_state.value != desired_value || s_state.value_type != setting_type) {
                log_info!("[OS Settings] Attempting to apply setting: {} = {} (type: {})", full_key.bold().cyan(), desired_value.yellow(), setting_type.magenta());

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

                match command.output() {
                    Ok(output) => {
                        if output.status.success() {
                            log_info!("[OS Settings] Successfully applied '{}' to domain '{}'.", entry.key, entry.domain);
                            // Store a new SettingState instance in the DevBoxState, including value_type
                            state.settings.insert(
                                full_key.clone(),
                                SettingState {
                                    domain: entry.domain.clone(),
                                    key: entry.key.clone(),
                                    value: desired_value.clone(),
                                    value_type: setting_type.clone(), // NEW: Store the type here!
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
                                log_debug!("[OS Settings] Stdout (on failure): {}", String::from_utf8_lossy(&output.stdout));
                            }
                        }
                    }
                    Err(e) => {
                        log_error!(
                            "[OS Settings] Could not execute 'defaults' command for setting '{}'. Error: {}",
                            full_key.bold().red(),
                            e.to_string().red()
                        );
                    }
                }
            } else {
                log_debug!("[OS Settings] Setting '{}' already matches desired value. Skipping application.", full_key.blue());
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        if !settings_cfg.settings.macos.is_empty() {
            log_warn!(
                "[OS Settings] macOS specific settings were found in config but this is not a macOS system. Skipping macOS settings application. Support for other operating systems is planned."
            );
        } else {
            log_debug!("[OS Settings] No macOS settings found in config for non-macOS system. Skipping settings application phase.");
        }
    }

    if settings_updated_in_session {
        log_info!("[OS Settings] One or more settings were applied or updated. Saving current DevBox state...");
        if !save_devbox_state(state, state_path_resolved) {
            log_error!("[StateSave] Failed to save state after settings application. Data loss risk!");
        } else {
            log_info!("[StateSave] State saved successfully after settings updates.");
        }
    } else {
        log_info!("[OS Settings] No new settings applied or state changes detected for settings in this run.");
    }

    eprintln!();
    log_debug!("Exiting apply_system_settings() function.");
}
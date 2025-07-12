use colored::Colorize;
use std::path::PathBuf;
use crate::{log_debug, log_error, log_info};
use crate::utils::expand_tilde;

/// Determines and resolves the absolute paths for the main configuration file
/// and the application state file.
///
/// This function encapsulates the logic for handling default paths and tilde expansion,
/// ensuring that `setup-devbox` always knows exactly where to find its essential files.
///
/// # Arguments
/// * `config_path`: An `Option<String>` allowing the user to specify a custom config path.
/// * `state_path`: An `Option<String>` allowing the user to specify a custom state file path.
///
/// # Returns
/// A tuple containing `(config_path_resolved, config_filename, state_path_resolved)`.
/// Returns `None` if essential paths cannot be resolved, indicating a critical error.
pub fn resolve_paths(
        config_path: Option<String>,
        state_path: Option<String>,
    ) -> Option<(PathBuf, String, PathBuf)> {
        log_debug!("Entering resolve_paths() function.");
        log_debug!("Initial config_path parameter: {:?}", config_path);
        log_debug!("Initial state_path parameter: {:?}", state_path);
    
        // Define the default location for our main configuration file (`config.yaml`).
        let default_main_config = "~/.setup-devbox/configs/config.yaml";
        // Resolve the actual, absolute path to the main configuration file.
        let config_path_resolved: PathBuf = expand_tilde(config_path.as_deref().unwrap_or(default_main_config));
        // Extract just the filename (e.g., "config.yaml", "tools.yaml") from the resolved config path.
        let config_filename = config_path_resolved.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        // Resolve the actual, absolute path to the application's internal state file (`state.json`).
        let state_path_resolved: PathBuf = expand_tilde(state_path.as_deref().unwrap_or("~/.setup-devbox/state.json"));
    
        // Log the final, resolved paths. Vital for confirming correct file system locations.
        log_info!("Using configuration file: {}", config_path_resolved.display().to_string().cyan());
        log_debug!("Managing application state in: {}", state_path_resolved.display().to_string().yellow());
        log_debug!("Resolved config_path: {:?}", config_path_resolved);
        log_debug!("Resolved state_path: {:?}", state_path_resolved);
        log_debug!("Detected config filename: '{}'", config_filename.blue());
    
        // Basic check to ensure paths are not empty or invalid.
        if config_path_resolved.as_os_str().is_empty() || state_path_resolved.as_os_str().is_empty() {
            log_error!("Resolved config or state path is empty. This is an internal error.");
            return None;
        }
    
        log_debug!("Exiting resolve_paths() function.");
        Some((config_path_resolved, config_filename, state_path_resolved))
}
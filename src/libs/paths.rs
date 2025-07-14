// Imports the `Colorize` trait for adding color to console output.
use colored::Colorize; 
// Provides `PathBuf` for working with file paths.
use std::path::PathBuf; 
// Custom logging macros for various log levels.
use crate::{log_debug, log_error, log_info}; 
// Imports a utility function to expand the `~` character in paths.
use crate::libs::utilities::path_helpers::expand_tilde; 

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
    config_path: Option<String>, // User-provided path for the main configuration file.
    state_path: Option<String>,   // User-provided path for the application state file.
) -> Option<(PathBuf, String, PathBuf)> {
    log_debug!("Entering resolve_paths() function."); // Debug log for function entry.
    log_debug!("Initial config_path parameter: {:?}", config_path); // Log the initial `config_path` argument.
    log_debug!("Initial state_path parameter: {:?}", state_path);   // Log the initial `state_path` argument.

    // Define the default location for our main configuration file (`config.yaml`).
    let default_main_config = "~/.setup-devbox/configs/config.yaml";
    // Resolve the actual, absolute path to the main configuration file.
    // If `config_path` is `Some`, use its value; otherwise, use `default_main_config`.
    // `as_deref()` converts `Option<String>` to `Option<&str>`, then `unwrap_or` gets the value.
    let config_path_resolved: PathBuf = expand_tilde(config_path.as_deref().unwrap_or(default_main_config));
    // Extract just the filename (e.g., "config.yaml", "tools.yaml") from the resolved config path.
    // `file_name()` returns an `Option<&OsStr>`, `and_then` unwraps it and converts to `&str`.
    // `unwrap_or("")` provides an empty string fallback, then `to_string()` converts to `String`.
    let config_filename = config_path_resolved.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
    // Resolve the actual, absolute path to the application's internal state file (`state.json`).
    // Similar logic as `config_path_resolved`, using a default state file path.
    let state_path_resolved: PathBuf = expand_tilde(state_path.as_deref().unwrap_or("~/.setup-devbox/state.json"));

    // Log the final, resolved paths. Vital for confirming correct file system locations.
    log_info!("Using configuration file: {}", config_path_resolved.display().to_string().cyan()); // Informative log for the config file path.
    log_debug!("Managing application state in: {}", state_path_resolved.display().to_string().yellow()); // Debug log for the state file path.
    log_debug!("Resolved config_path: {:?}", config_path_resolved); // Debug log of the resolved config `PathBuf`.
    log_debug!("Resolved state_path: {:?}", state_path_resolved);     // Debug log of the resolved state `PathBuf`.
    log_debug!("Detected config filename: '{}'", config_filename.blue()); // Debug log for the extracted filename.

    // Basic check to ensure paths are not empty or invalid. `as_os_str().is_empty()` checks if the underlying OS string is empty.
    if config_path_resolved.as_os_str().is_empty() || state_path_resolved.as_os_str().is_empty() {
        log_error!("Resolved config or state path is empty. This is an internal error."); // Error log if paths are empty.
        return None; // Return `None` to indicate a critical error.
    }

    log_debug!("Exiting resolve_paths() function."); // Debug log for function exit.
    // Return `Some` tuple containing the resolved config path, filename, and state path.
    Some((config_path_resolved, config_filename, state_path_resolved))
}
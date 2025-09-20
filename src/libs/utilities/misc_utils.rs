// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_info, log_warn};
// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use chrono::Duration;
// The 'colored' crate helps us make our console output look pretty and readab
use colored::Colorize;
// For getting environment variables, like HOME.
// `std::env` is used to find the user's home directory to determine rustup's installation path.
use std::env;
use std::path::PathBuf;

/// A super useful function to resolve paths that start with a tilde `~`.
/// On Unix-like systems, `~` is a shortcut for the user's home directory.
/// This function expands that `~` into the full, absolute path, like `/Users/yourusername/`.
/// This is crucial for user-friendly path inputs.
///
/// # Arguments
/// * `path`: A string slice (`&str`) representing the path, which might start with `~`.
///
/// # Returns
/// * `PathBuf`: The fully resolved path if `~` was present and the home directory
///              could be determined. Otherwise, it returns the original path unchanged.
pub fn expand_tilde(path: &str) -> PathBuf {
    // Check if the input path string actually begins with a tilde character.
    if path.starts_with("~") {
        // Attempt to retrieve the current user's home directory.
        // `dirs::home_dir()` is a cross-platform way to get this path.
        if let Some(home) = dirs::home_dir() {
            // If the home directory was successfully found:
            // 1. Convert the home directory `PathBuf` into a string slice (`to_string_lossy()`)
            //    which safely handles non-UTF8 characters by replacing them.
            // 2. Use `replacen` to replace only the *first* occurrence of `~` with the home path.
            //    This ensures paths like `~/Documents/~/file.txt` are handled correctly.
            return PathBuf::from(path.replacen("~", &home.to_string_lossy(), 1));
        }
    }
    // If the path does not start with `~`, or if `dirs::home_dir()` failed to find
    // the home directory, simply convert the original input path string into a `PathBuf`
    // and return it as is.
    PathBuf::from(path)
}

/// Converts a Chrono `Duration` object into a human-readable string representation.
///
/// This function formats time durations for display purposes, selecting the most
/// appropriate time unit (days, hours, or minutes) based on the duration's magnitude.
/// It's particularly useful for user-facing messages, logs, and configuration displays
/// where raw duration values would be less intuitive.
///
/// # Arguments
/// * `duration` - A reference to a Chrono `Duration` object to be formatted
///
/// # Returns
/// A `String` containing the formatted duration in the most appropriate time unit:
/// - Days for durations ≥ 1 day
/// - Hours for durations ≥ 1 hour but less than 1 day
/// - Minutes for durations ≥ 1 minute but less than 1 hour
/// - "0 minutes" for durations less than 1 minute
///
/// # Unit Selection Logic
/// The function uses a hierarchical approach to determine the best unit:
/// 1. **Days**: If the duration contains any complete days (≥ 86400 seconds)
/// 2. **Hours**: If no days but contains complete hours (≥ 3600 seconds)
/// 3. **Minutes**: If no hours but contains complete minutes (≥ 60 seconds)
/// 4. **Fallback**: "0 minutes" for sub-minute durations
pub fn format_duration(duration: &Duration) -> String {
    // Check if the duration contains any complete days
    // Using num_days() which returns the total number of whole days in the duration
    if duration.num_days() > 0 {
        format!("{} days", duration.num_days())
    }
    // If no days, check for complete hours
    // num_hours() returns total whole hours, including those that might be part of days
    else if duration.num_hours() > 0 {
        format!("{} hours", duration.num_hours())
    }
    // If no hours, check for complete minutes
    // num_minutes() returns total whole minutes in the duration
    else if duration.num_minutes() > 0 {
        format!("{} minutes", duration.num_minutes())
    } else {
        // Fallback for durations less than 1 minute
        // This ensures we always return a meaningful string, even for very short durations
        "0 minutes".to_string()
    }
}

/// Returns the canonical path to the DevBox directory, typically `~/.setup-devbox`.
/// This is the base directory where `state.json` and the `config` folder reside.
/// If the home directory cannot be determined, it will log an error and
/// fall back to the current directory, which might lead to unexpected behavior.
///
/// # Returns
/// * `PathBuf`: The path to the `.setup-devbox` directory.
pub fn get_devbox_dir() -> PathBuf {
    // Attempt to get the user's home directory.
    if let Some(home_dir) = dirs::home_dir() {
        // Corrected: Use ".setup-devbox" instead of ".devbox"
        let setup_devbox_dir = home_dir.join(".setup-devbox");
        log_debug!(
            "[Utils] DevBox directory resolved to: {}",
            setup_devbox_dir.display().to_string().cyan()
        );
        setup_devbox_dir
    } else {
        // If the home directory cannot be determined, log an error and use the current directory as a fallback.
        log_error!(
            "[Utils] Could not determine home directory. Falling back to current directory for .setup-devbox path."
        );
        // Get the current working directory.
        let current_dir = env::current_dir().unwrap_or_else(|e| {
            // If even current directory can't be found, panic as it's a critical error.
            panic!("Failed to get current directory and home directory: {}", e);
        });
        // Corrected: Use ".setup-devbox" for the fallback path
        let fallback_dir = current_dir.join(".setup-devbox");
        log_warn!(
            "[Utils] Fallback DevBox directory: {}",
            fallback_dir.display().to_string().yellow()
        );
        fallback_dir
    }
}

/// Extracts just the version number part from input string
pub fn extract_version_number(input: &str) -> String {
    // Find the first digit in the string and take everything from there
    if let Some(pos) = input.find(|c: char| c.is_ascii_digit()) {
        input[pos..].to_string()
    } else {
        input.to_string() // Fallback: return original input if no digits found
    }
}

/// Helper function to get the default Python installation path
pub fn default_python_path(package_name: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        format!("{}/.local/share/uv/python/{}", home, package_name)
    } else {
        format!("/usr/local/share/uv/python/{}", package_name)
    }
}

/// Determines and resolves the absolute paths for the main configuration file
/// and the application state file based on environment variables and fallbacks.
///
/// Priority order:
/// 1. Main config: SDB_CONFIG_PATH env var -> default path
/// 2. State file: SDB_STATE_FILE_PATH env var -> SDB_CONFIG_PATH env var -> default path
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
    state_path: Option<String>,  // User-provided path for the application state file.
) -> Option<(PathBuf, String, PathBuf)> {
    log_debug!("Entering resolve_paths() function.");
    log_debug!(
        "Initial config_path parameter: {}",
        config_path.as_deref().unwrap_or("None")
    );
    log_debug!(
        "Initial state_path parameter: {}",
        state_path.as_deref().unwrap_or("None")
    );

    // Resolve main configuration path
    let config_path_resolved = if let Some(user_config_path) = config_path {
        // User-provided config path takes highest priority
        expand_tilde(&user_config_path)
    } else if let Ok(env_config_path) = std::env::var("SDB_CONFIG_PATH") {
        // Environment variable SDB_CONFIG_PATH
        expand_tilde(&format!("{}/configs/config.yaml", env_config_path))
    } else {
        // Default fallback
        expand_tilde("~/.setup-devbox/configs/config.yaml")
    };

    // Resolve state file path
    let state_path_resolved = if let Some(user_state_path) = state_path {
        // User-provided state path takes highest priority
        expand_tilde(&user_state_path)
    } else if let Ok(env_state_path) = std::env::var("SDB_STATE_FILE_PATH") {
        // Environment variable SDB_STATE_FILE_PATH
        expand_tilde(&format!("{}/state.json", env_state_path))
    } else if let Ok(env_config_path) = std::env::var("SDB_CONFIG_PATH") {
        // Fallback to SDB_CONFIG_PATH
        expand_tilde(&format!("{}/state.json", env_config_path))
    } else {
        // Default fallback
        expand_tilde("~/.setup-devbox/state.json")
    };

    // Extract config filename
    let config_filename = config_path_resolved
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    // Log the final, resolved paths
    log_info!(
        "Using configuration file: {}",
        config_path_resolved.display().to_string().cyan()
    );
    log_debug!(
        "Managing application state in: {}",
        state_path_resolved.display().to_string().yellow()
    );
    log_debug!(
        "Resolved config_path: {}",
        config_path_resolved.to_string_lossy()
    );
    log_debug!(
        "Resolved state_path: {}",
        state_path_resolved.to_string_lossy()
    );
    log_debug!("Detected config filename: '{}'", config_filename.blue());

    // Basic check to ensure paths are not empty or invalid
    if config_path_resolved.as_os_str().is_empty() || state_path_resolved.as_os_str().is_empty() {
        log_error!("Resolved config or state path is empty. This is an internal error.");
        return None;
    }

    log_debug!("Exiting resolve_paths() function.");
    Some((config_path_resolved, config_filename, state_path_resolved))
}

// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_warn};
// The 'colored' crate helps us make our console output look pretty and readab
use colored::Colorize;
// For getting environment variables, like HOME.
// `std::env` is used to find the user's home directory to determine rustup's installation path.
use std::env;
// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
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
        log_debug!("[Utils] DevBox directory resolved to: {}", setup_devbox_dir.display().to_string().cyan());
        setup_devbox_dir
    } else {
        // If the home directory cannot be determined, log an error and use the current directory as a fallback.
        log_error!("[Utils] Could not determine home directory. Falling back to current directory for .setup-devbox path.");
        // Get the current working directory.
        let current_dir = env::current_dir().unwrap_or_else(|e| {
            // If even current directory can't be found, panic as it's a critical error.
            panic!("Failed to get current directory and home directory: {}", e);
        });
        // Corrected: Use ".setup-devbox" for the fallback path
        let fallback_dir = current_dir.join(".setup-devbox");
        log_warn!("[Utils] Fallback DevBox directory: {}", fallback_dir.display().to_string().yellow());
        fallback_dir
    }
}
// This module is responsible for managing the application's persistent state.
// It handles loading the `DevBoxState` from a JSON file (typically `state.json`),
// initializing a new state if no existing file is found, and saving the updated
// state back to the file system. The state includes information about installed
// tools, fonts, and applied settings, allowing `devbox` to remember its
// configuration across runs and avoid redundant operations.
//
// Key functionalities include:
// - Deserializing `DevBoxState` from JSON.
// - Serializing `DevBoxState` to pretty-printed JSON.
// - Error handling for file I/O and JSON parsing.
// - Ensuring parent directories exist before writing.

use crate::libs::utilities::timestamps::current_timestamp;
use crate::schemas::configuration_management::ConfigurationManagerState;
use crate::schemas::state_file::{DevBoxState, ToolState};
use crate::schemas::tools::ToolEntry;
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Loads the application's state from `state.json` or initializes a new one.
///
/// This function centralizes the logic for reading the existing state file,
/// handling potential parsing errors, and creating a fresh state if the file
/// doesn't exist. It also ensures the parent directory for the state file exists.
/// This is a critical startup function, exiting the application if unrecoverable
/// errors occur (e.g., unreadable file, malformed JSON that cannot be parsed).
///
/// # Arguments
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file where the state
///   is expected to be loaded from or saved to initially.
///
/// # Returns
/// * `DevBoxState`: A `DevBoxState` struct representing the loaded or newly initialized state.
///   The function will `std::process::exit(1)` if a critical error
///   (like an unreadable or unparsable state file) occurs.
pub fn load_or_initialize_state(state_path_resolved: &PathBuf) -> DevBoxState {
    log_debug!("Entering load_or_initialize_state() function."); // Debug log for function entry.

    let state: DevBoxState = if state_path_resolved.exists() {
        // If the state file exists, attempt to load it.
        log_debug!(
            "State file found at {:?}. Attempting to load...",
            state_path_resolved
        );
        match fs::read_to_string(state_path_resolved) {
            Ok(contents) => {
                // If file content is read successfully, attempt to deserialize the JSON.
                match serde_json::from_str(&contents) {
                    Ok(parsed_state) => {
                        log_info!(
                            "[SDB] Using state file: {}",
                            state_path_resolved.display().to_string().cyan()
                        ); // Informative log about using existing state file.
                        log_debug!(
                            "Existing state file: {}",
                            state_path_resolved.display().to_string().yellow()
                        ); // Debug log for path.
                        // Attempt to pretty-print the loaded state for detailed debugging.
                        match serde_json::to_string_pretty(&parsed_state) {
                            Ok(pretty_json) => {
                                log_debug!("Loaded DevBoxState:\n{}", pretty_json);
                            }
                            Err(e) => {
                                log_warn!(
                                    "Failed to re-serialize loaded DevBoxState for pretty printing: {}",
                                    e
                                );
                                log_debug!(
                                    "Loaded DevBoxState (raw debug format): {:?}",
                                    parsed_state
                                );
                            }
                        }
                        parsed_state // Return the successfully parsed state.
                    }
                    Err(err) => {
                        // If JSON deserialization fails (e.g., corrupted file, schema mismatch).
                        log_error!(
                            "Invalid state.json format at {}: {}. Please check the file's content or delete it to start fresh.",
                            state_path_resolved.display().to_string().red(),
                            err
                        );
                        std::process::exit(1); // Exit due to critical error.
                    }
                }
            }
            Err(err) => {
                // If the file cannot be read (e.g., permissions error).
                log_error!(
                    "Failed to read state file {}: {}. Please verify file permissions.",
                    state_path_resolved.display().to_string().red(),
                    err
                );
                std::process::exit(1); // Exit due to critical error.
            }
        }
    } else {
        // If the state file does not exist, initialize a new, empty state.
        log_info!(
            "State file not found at {:?}. Creating a brand new state file.",
            state_path_resolved.display().to_string().yellow()
        );
        let initial_state = DevBoxState {
            tools: HashMap::new(),    // Initialize with empty HashMap for tools.
            fonts: HashMap::new(),    // Initialize with empty HashMap for fonts.
            settings: HashMap::new(), // Initialize with empty HashMap for settings.
        };

        // Ensure the parent directory for the state file exists before attempting to write.
        if let Some(parent_dir) = state_path_resolved.parent() {
            log_debug!(
                "Checking/creating parent directory for state file: {:?}",
                parent_dir
            );
            if let Err(e) = fs::create_dir_all(parent_dir) {
                // If directory creation fails, log error and exit.
                log_error!(
                    "Failed to create directory for state file at {:?}: {}. Cannot save state.",
                    parent_dir.display().to_string().red(),
                    e
                );
                std::process::exit(1); // Exit due to critical error.
            }
        }

        // Attempt to serialize and write the initial empty state to the file.
        match serde_json::to_string_pretty(&initial_state) {
            Ok(serialized_state) => {
                if let Err(err) = fs::write(state_path_resolved, serialized_state) {
                    // If writing fails, log a non-critical error (app can still run but won't save state).
                    log_error!(
                        "Failed to write initial state file to {:?}: {}. This might prevent future state tracking.",
                        state_path_resolved.display().to_string().red(),
                        err
                    );
                } else {
                    log_info!(
                        "Initial state file successfully created at {:?}",
                        state_path_resolved.display().to_string().green()
                    ); // Success log for initial state creation.
                }
            }
            Err(err) => {
                // If serialization fails, it's an internal application error.
                log_error!(
                    "Failed to serialize initial state: {}. This is an internal application error.",
                    err
                );
                std::process::exit(1); // Exit due to critical error.
            }
        }
        initial_state // Return the newly initialized state.
    };
    log_debug!("Exiting load_or_initialize_state() function."); // Debug log for function exit.
    state // Return the final loaded or initialized state.
}

/// Saves the current `DevBoxState` to the specified `state.json` file.
///
/// This function serializes the `DevBoxState` struct into a human-readable, pretty-printed JSON format
/// and writes it to the disk. It also handles creating any necessary parent directories for the
/// state file if they do not exist, ensuring the save operation can proceed smoothly.
/// This is essential for `setup-devbox` to persist its "memory" of installed tools and settings.
///
/// # Arguments
/// * `state`: A reference to the `DevBoxState` struct that needs to be saved.
/// * `state_path`: A reference to a `PathBuf` indicating the full path where the state file
///   (`state.json`) should be saved.
///
/// # Returns
/// * `bool`:
///   - `true` if the state was successfully serialized and written to the file.
///   - `false` otherwise (e.g., failed to create directories, failed to serialize, failed to write).
pub fn save_devbox_state(state: &DevBoxState, state_path: &PathBuf) -> bool {
    log_debug!(
        "[StateSave] Attempting to save DevBoxState to: {:?}",
        state_path.display()
    ); // Debug log for save attempt.

    // Ensure the parent directory for the state file exists.
    // `state_path.parent()` returns `Some(Path)` if the path has a parent directory.
    if let Some(parent_dir) = state_path.parent() {
        // Check if the parent directory already exists.
        if !parent_dir.exists() {
            log_info!(
                "[StateSave] Parent directory {:?} does not exist. Creating it now.",
                parent_dir.display()
            ); // Informative log about directory creation.
            // Attempt to create all necessary parent directories.
            // If `fs::create_dir_all` fails, log an error and return `false` because saving cannot proceed.
            if let Err(e) = fs::create_dir_all(parent_dir) {
                log_error!(
                    "[StateSave] Failed to create directory for state file at {:?}: {}. Cannot save state.",
                    parent_dir.display().to_string().red(),
                    e
                );
                return false; // Critical failure, cannot save state.
            }
        }
    }

    // Try to serialize the `DevBoxState` struct into a pretty-printed JSON string.
    // `serde_json::to_string_pretty` makes the JSON output readable for debugging and inspection.
    match serde_json::to_string_pretty(state) {
        Ok(serialized_state) => {
            // If serialization was successful, attempt to write the JSON string to the state file.
            // `fs::write` is a convenience function that creates the file (or truncates it) and writes all data.
            match fs::write(state_path, serialized_state) {
                Ok(_) => {
                    // Print an empty line to ensure clean terminal output, separating logs from other output.
                    eprintln!("\n");
                    log_info!(
                        "[StateSave] DevBox state saved successfully to {}",
                        state_path.display().to_string().cyan()
                    ); // Success log for state saving.
                    log_debug!("[StateSave] State content written to disk."); // Debug log confirmation.
                    true // Indicate successful saving.
                }
                Err(err) => {
                    // If writing to the file fails (e.g., disk full, permission denied).
                    log_error!(
                        "[StateSave] Failed to write updated state file to {:?}: {}. Your `setup-devbox` memory might not be saved correctly.",
                        state_path.display().to_string().red(),
                        err
                    );
                    false // Indicate failure to write.
                }
            }
        }
        Err(err) => {
            // If serialization itself fails, it indicates an internal application error
            // (e.g., `DevBoxState` struct cannot be serialized, or data is invalid).
            log_error!(
                "[StateSave] Failed to serialize DevBox state for saving: {}. This is an internal application error.",
                err
            );
            false // Indicate failure to serialize.
        }
    }
}

/// Saves the current state to file if changes were made
pub fn save_state_to_file(state: &DevBoxState, state_file_path: &PathBuf) {
    log_info!("[Tools] Saving updated state...");

    if save_devbox_state(state, state_file_path) {
        log_info!("[StateSave] State saved successfully.");
    } else {
        log_error!("[StateSave] Failed to save state - data loss risk!");
    }
}

impl ToolState {
    /// Helper method to update configuration manager state.
    ///
    /// Used when configuration files are synchronized or updated to store
    /// the new state information including file hashes and timestamps.
    ///
    /// ## Parameters
    /// - `config_state`: The new configuration management state to store
    pub fn set_configuration_manager(&mut self, config_state: ConfigurationManagerState) {
        self.configuration_manager = Some(config_state);
    }

    /// Helper method to get configuration manager state.
    ///
    /// Returns a reference to the current configuration management state
    /// if it exists, or `None` if no configuration management is active.
    ///
    /// ## Returns
    /// - `Some(&ConfigurationManagerState)`: Configuration state exists
    /// - `None`: No configuration management for this tool
    pub fn get_configuration_manager(&self) -> Option<&ConfigurationManagerState> {
        self.configuration_manager.as_ref()
    }

    /// Creates a comprehensive ToolState object for tracking the installation.
    ///
    /// This method serves as a constructor for creating a new ToolState instance
    /// with all the necessary information for tracking tool installations.
    ///
    /// ## Parameters
    /// - `tool_entry`: The tool configuration from the devbox configuration
    /// - `install_path`: Path where the tool is installed
    /// - `install_method`: Method used for installation (e.g., "cargo", "brew")
    /// - `package_type`: Type of package (e.g., "binary", "go-module")
    /// - `actual_version`: The actual version that was installed
    /// - `executed_post_installation_hooks`: Any post-installation commands executed
    ///
    /// ## Returns
    /// A fully populated `ToolState` instance ready for storage in the state file
    pub fn new(
        tool_entry: &ToolEntry,
        install_path: &PathBuf,
        install_method: String,
        package_type: String,
        actual_version: String,
        url: Option<String>,
        executable_after_extract: Option<String>,
        executed_post_installation_hooks: Option<Vec<String>>,
    ) -> Self {
        // 1. Process the URL to ensure empty strings are treated as None.
        let processed_url = url.and_then(|u| {
            // Trim whitespace and check if the remaining string is empty.
            let trimmed = u.trim();
            if trimmed.is_empty() {
                None // Return None if the URL is empty or just whitespace
            } else {
                Some(trimmed.to_string()) // Keep it if it has content
            }
        });
        Self {
            // The version recorded for the tool. Uses the specified version or "latest" as a fallback.
            version: actual_version,
            // The canonical path where the tool's executable was installed. This is the path
            // that will be recorded in the `state.json` file.
            install_path: install_path.to_string_lossy().into_owned(),
            // Flag indicating that this tool was installed by `setup-devbox`. This helps distinguish
            // between tools managed by our system and those installed manually.
            installed_by_devbox: true,
            // The method of installation, useful for future diagnostics or differing update logic.
            install_method,
            // Records if the binary was renamed during installation. For `cargo install`, this is
            // usually `None` unless `--bin` or `--example` flags are used in `options`.
            renamed_to: tool_entry.rename_to.clone(),
            // The actual package type detected by the `file` command or inferred. This is for diagnostic
            // purposes, providing the most accurate type even if the installation logic
            // used a filename-based guess (e.g., "binary", "macos-pkg-installer").
            package_type,
            // Repository for the tool
            repo: tool_entry.repo.clone(),
            // Version Tag for the tool
            tag: tool_entry.tag.clone(),
            // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
            options: tool_entry.options.clone(),
            // For direct URL installations: The original URL from which the tool was downloaded.
            // This is important for re-downloading or verifying in the future.
            url: processed_url,
            // Record the timestamp when the tool was installed or updated
            last_updated: Some(current_timestamp()),
            // This field is currently `None` but could be used to store the path to an executable
            // *within* an extracted archive if `install_path` points to the archive's root.
            executable_path_after_extract: executable_after_extract,
            // Record any additional commands that were executed during installation.
            // This is useful for tracking what was done and potentially for cleanup during uninstall.
            executed_post_installation_hooks,
            // Configuration Manager for the tool, if SDB is managing the configuration for the tool.
            configuration_manager: None,
        }
    }

    /// Normalizes installation method names to standard source types
    ///
    /// State files use verbose, descriptive names for installation methods,
    /// while configuration files use shorter, standardized identifiers.
    ///
    /// # Mapping Table
    ///
    /// | State install_method | Config source |
    /// |---------------------|---------------|
    /// | uv-python           | uv            |
    /// | uv-tool             | uv            |
    /// | cargo-install       | cargo         |
    /// | go-install          | go            |
    /// | direct-url          | url           |
    /// | brew                | brew          |
    /// | github              | github        |
    ///
    /// # Arguments
    ///
    /// * `install_method` - The verbose installation method from state
    ///
    /// # Returns
    ///
    /// The standardized source type for configuration
    pub fn normalize_source_type(install_method: &str) -> String {
        match install_method {
            "uv-python" => "uv",
            "uv-tool" => "uv",
            "cargo-install" => "cargo",
            "go-install" => "go",
            "direct-url" => "url",
            other => other,
        }
        .to_string()
    }
}

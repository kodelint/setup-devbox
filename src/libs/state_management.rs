use colored::Colorize;
use std::collections::HashMap;
use std::{fs, io};
use std::path::{Path, PathBuf};
use crate::{log_debug, log_error, log_info, log_warn};
use crate::schema::DevBoxState;

/// Loads the application's state from `state.json` or initializes a new one.
///
/// This function centralizes the logic for reading the existing state file,
/// handling potential parsing errors, and creating a fresh state if the file
/// doesn't exist. It also ensures the parent directory for the state file exists.
///
/// # Arguments
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file.
///
/// # Returns
/// A `DevBoxState` struct representing the loaded or newly initialized state.
/// Exits the application if a critical error (like unreadable or malformed state file) occurs.
pub fn load_or_initialize_state(state_path_resolved: &PathBuf) -> DevBoxState {
    log_debug!("Entering load_or_initialize_state() function.");

    let state: DevBoxState = if state_path_resolved.exists() {
        log_debug!("State file found at {:?}. Attempting to load...", state_path_resolved);
        match fs::read_to_string(state_path_resolved) {
            Ok(contents) => {
                match serde_json::from_str(&contents) {
                    Ok(parsed_state) => {
                        log_info!("Using state file: {}", state_path_resolved.display().to_string().cyan());
                        log_debug!("Existing state file: {}", state_path_resolved.display().to_string().yellow());
                        match serde_json::to_string_pretty(&parsed_state) {
                            Ok(pretty_json) => {
                                log_debug!("Loaded DevBoxState:\n{}", pretty_json);
                            },
                            Err(e) => {
                                log_warn!("Failed to re-serialize loaded DevBoxState for pretty printing: {}", e);
                                log_debug!("Loaded DevBoxState (raw debug format): {:?}", parsed_state);
                            }
                        }
                        parsed_state
                    },
                    Err(err) => {
                        log_error!(
                            "Invalid state.json format at {:?}: {}. Please check the file's content or delete it to start fresh.",
                            state_path_resolved.display().to_string().red(),
                            err
                        );
                        std::process::exit(1); // Exit due to critical error
                    }
                }
            },
            Err(err) => {
                log_error!(
                    "Failed to read state file {:?}: {}. Please verify file permissions.",
                    state_path_resolved.display().to_string().red(),
                    err
                );
                std::process::exit(1); // Exit due to critical error
            }
        }
    } else {
        log_info!("State file not found at {:?}. Creating a brand new state file.", state_path_resolved.display().to_string().yellow());
        let initial_state = DevBoxState {
            tools: HashMap::new(),
            fonts: HashMap::new(),
            settings: HashMap::new(),
        };

        if let Some(parent_dir) = state_path_resolved.parent() {
            log_debug!("Checking/creating parent directory for state file: {:?}", parent_dir);
            if let Err(e) = fs::create_dir_all(parent_dir) {
                log_error!(
                    "Failed to create directory for state file at {:?}: {}. Cannot save state.",
                    parent_dir.display().to_string().red(),
                    e
                );
                std::process::exit(1); // Exit due to critical error
            }
        }

        match serde_json::to_string_pretty(&initial_state) {
            Ok(serialized_state) => {
                if let Err(err) = fs::write(state_path_resolved, serialized_state) {
                    log_error!(
                        "Failed to write initial state file to {:?}: {}. This might prevent future state tracking.",
                        state_path_resolved.display().to_string().red(),
                        err
                    );
                } else {
                    log_info!("Initial state file successfully created at {:?}", state_path_resolved.display().to_string().green());
                }
            },
            Err(err) => {
                log_error!("Failed to serialize initial state: {}. This is an internal application error.", err);
                std::process::exit(1); // Exit due to critical error
            }
        }
        initial_state
    };
    log_debug!("Exiting load_or_initialize_state() function.");
    state
}

/// Saves the current `DevBoxState` to the specified `state.json` file.
///
/// This function serializes the `DevBoxState` struct into a human-readable, pretty-printed JSON format
/// and writes it to the disk. It also handles creating any necessary parent directories for the
/// state file if they do not exist, ensuring the save operation can proceed smoothly.
/// This is essential for `setup-devbox` to persist its "memory" of installed tools.
///
/// # Arguments
/// * `state`: A reference to the `DevBoxState` struct that needs to be saved.
/// * `state_path`: A reference to a `PathBuf` indicating the full path where the state file
///                 (`state.json`) should be saved.
///
/// # Returns
/// * `bool`:
///   - `true` if the state was successfully serialized and written to the file.
///   - `false` otherwise (e.g., failed to create directories, failed to serialize, failed to write).
pub fn save_devbox_state(state: &DevBoxState, state_path: &PathBuf) -> bool {
    log_debug!("[StateSave] Attempting to save DevBoxState to: {:?}", state_path.display());

    // Ensure the parent directory for the state file exists.
    // `state_path.parent()` returns `Some(Path)` if the path has a parent directory.
    if let Some(parent_dir) = state_path.parent() {
        // Check if the parent directory already exists.
        if !parent_dir.exists() {
            log_info!("[StateSave] Parent directory {:?} does not exist. Creating it now.", parent_dir.display());
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
                    eprint!("\n");
                    log_info!("[StateSave] DevBox state saved successfully to {}", state_path.display().to_string().green());
                    log_debug!("[StateSave] State content written to disk.");
                    true // Indicate successful saving.
                },
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
        },
        Err(err) => {
            // If serialization itself fails, it indicates an internal application error
            // (e.g., `DevBoxState` struct cannot be serialized, or data is invalid).
            log_error!("[StateSave] Failed to serialize DevBox state for saving: {}. This is an internal application error.", err);
            false // Indicate failure to serialize.
        }
    }
}

/// Loads the `DevBoxState` from a specified `state.json` file.
///
/// This function attempts to read the JSON content from the given path,
/// then deserialize it into a `DevBoxState` struct. If the file does not exist,
/// an empty (default) `DevBoxState` is returned, signifying a fresh start.
///
/// # Arguments
/// * `state_path`: A reference to a `Path` indicating the full path to the `state.json` file.
///
/// # Returns
/// * `io::Result<DevBoxState>`:
///   - `Ok(DevBoxState)` containing the loaded state, or a default empty state if the file
///     doesn't exist.
///   - `Err(io::Error)` if the file exists but cannot be read (e.g., permissions),
///     or if the JSON content is invalid and cannot be deserialized.
pub fn read_devbox_state(state_path: &Path) -> io::Result<DevBoxState> {
    log_debug!("[StateLoad] Attempting to load DevBoxState from: {:?}", state_path.display().to_string().blue());

    // Check if the state file exists.
    if !state_path.exists() {
        log_warn!("[StateLoad] DevBox state file does not exist at {:?}. Initializing with default (empty) state.", state_path.display().to_string().yellow());
        // If the file doesn't exist, return a default (empty) state.
        return Ok(DevBoxState::default());
    }

    // Read the file content into a string.
    let file_content = fs::read_to_string(state_path)
        .map_err(|e| {
            // If reading fails, log an error and return an `io::Error`.
            log_error!("[StateLoad] Failed to read state file {:?}: {}", state_path.display().to_string().red(), e);
            e
        })?; // Propagate the error if `read_to_string` fails.

    // Try to deserialize the JSON string content into a `DevBoxState` struct.
    serde_json::from_str(&file_content)
        .map_err(|e| {
            // If deserialization fails (e.g., malformed JSON, schema mismatch),
            // log an error and return an `io::Error`.
            log_error!("[StateLoad] Failed to parse state file {:?}: {}. The file might be corrupted.", state_path.display().to_string().red(), e);
            io::Error::new(io::ErrorKind::InvalidData, format!("Failed to parse state file: {}", e))
        })
}
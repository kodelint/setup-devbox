// This module is dedicated to the robust parsing and loading of all configuration files
// used by the `devbox` application. It acts as the central point for reading various
// YAML-based configuration definitions, including `tools.yaml`, `settings.yaml`,
// `shellrc.yaml` (or `shellac.yaml`), and `fonts.yaml`.
//
// The core functionality encompasses reading file contents, expanding user-friendly
// paths (like `~/`), and deserializing the YAML data into strongly-typed Rust structs.
// This abstraction ensures that the rest of the application can consume configuration
// data in a structured and reliable manner, driving `devbox`'s behavior for
// tool installations, environment setup, and font management.

// External crate imports:
use colored::Colorize; // Imports the `Colorize` trait for adding color to console output.

// Standard library module for interacting with the file system (e.g., reading files).
// Provides `PathBuf` for working with file paths.
use std::path::PathBuf;
// Provides file system operations like `read_to_string`.
// Standard library module for constructing and manipulating file paths in an OS-agnostic way.
use std::fs;

// Internal module imports:
// Custom logging macros for consistent, level-based output (debug, error, info, warn).
use crate::{log_debug, log_error, log_info, log_warn};

// Importing schema definitions. These structs (e.g., `ToolConfig`, `FontConfig`) define
// the expected data structure for each type of YAML configuration file, enabling `serde`
// to correctly parse them. `MainConfig` specifically defines the structure of the primary
// `config.yaml` file that links to other configuration files.
use crate::schemas::sdb_schema::{FontConfig, MainConfig, SettingsConfig, ShellConfig, ToolConfig};

// Imports a utility function to expand the `~` character in paths.
use crate::libs::utilities::misc_utils::expand_tilde;

/// A composite struct designed to hold all the parsed configuration data.
///
/// This structure serves as a single, convenient container for all `devbox`'s
/// configurations once they have been loaded from their respective YAML files.
/// Using an `Option<T>` for each field allows `devbox` to gracefully handle
/// scenarios where certain configuration files might be absent or fail to parse
/// without halting the entire application.
pub struct ParsedConfigs {
    /// Stores the parsed `ToolConfig` if `tools.yaml` is found and successfully deserialized.
    pub(crate) tools: Option<ToolConfig>,
    /// Stores the parsed `SettingsConfig` if `settings.yaml` is found and successfully deserialized.
    pub(crate) settings: Option<SettingsConfig>,
    /// Stores the parsed `ShellConfig` if `shellrc.yaml` or `shellac.yaml` is found and successfully deserialized.
    pub(crate) shell: Option<ShellConfig>,
    /// Stores the parsed `FontConfig` if `fonts.yaml` is found and successfully deserialized.
    pub(crate) fonts: Option<FontConfig>,
}

/// A generic helper function to load and deserialize an individual configuration file.
///
/// This function encapsulates the common logic for reading a YAML file, handling path expansion
/// (specifically the `~` for the home directory), and deserializing its contents into a
/// specified Rust struct. It provides detailed logging for various outcomes: successful loading,
/// file not found, or YAML parsing errors, making it a robust and reusable component.
///
/// # Type Parameters
/// * `T`: The target type for deserialization. This must be a struct that implements
///        `serde::de::DeserializeOwned` (allowing `serde_yaml` to create an instance from YAML)
///        and `std::fmt::Debug` (for logging purposes).
///
/// # Arguments
/// * `path_option`: An `Option<&String>` representing the path to the configuration file.
///                  This path is typically read from the `MainConfig` struct.
/// * `config_name`: A human-readable name for the configuration type (e.g., "tools", "settings"),
///                  used in log messages to identify which config is being processed.
/// * `bold_name`: A string slice used for consistent, bolded prefixes in log messages (e.g., "[Tools]", "[Settings]").
///
/// # Returns
/// * `Option<T>`:
///   - `Some(T)` if the file is successfully read, parsed, and deserialized into the type `T`.
///   - `None` if `path_option` is `None`, the file is not found, or any I/O or parsing error occurs.
pub fn load_individual_config<T>(
    path_option: Option<&String>,
    config_name: &str,
    bold_name: &str,
) -> Option<T>
where
    T: serde::de::DeserializeOwned + std::fmt::Debug, // Trait bounds for deserialization and debugging.
{
    // Check if a path string was provided. If not, log and return None, as there's no file to load.
    if let Some(path_str) = path_option {
        // Expand the `~` character in the path to the actual home directory path.
        let path = expand_tilde(path_str);
        // Log the attempt to load the configuration, including the resolved path.
        log_debug!(
            "Attempting to load {} config from: {}",
            config_name,
            path.display()
        );

        // Attempt to read the file content into a string.
        match fs::read_to_string(&path) {
            Ok(contents) => {
                // If file reading is successful, attempt to deserialize the YAML content.
                match serde_yaml::from_str::<T>(&contents) {
                    Ok(cfg) => {
                        // Log success message with bolded prefix and colored path.
                        log_debug!(
                            "{} Successfully loaded {} configuration from {}",
                            bold_name.bold(),
                            config_name,
                            path.display().to_string().green()
                        );
                        Some(cfg) // Return the successfully parsed configuration.
                    }
                    Err(e) => {
                        // Log a detailed error message if YAML parsing fails (e.g., syntax errors).
                        log_error!(
                            "Failed to parse {}.yaml at {}: {}. Please check its YAML syntax.",
                            config_name,
                            path.display().to_string().red(),
                            e
                        );
                        None // Return None on parsing failure.
                    }
                }
            }
            Err(_) => {
                // Log a warning if the file is not found or is unreadable. This is not critical,
                // as not all configurations are mandatory.
                log_warn!(
                    "{} configuration file not found or unreadable at {}. Skipping {} setup.",
                    config_name.yellow(),
                    path.display().to_string().yellow(),
                    config_name
                );
                None // Return None if the file cannot be read.
            }
        }
    } else {
        log_debug!(
            "No path provided for {} config. Skipping load.",
            config_name
        ); // Log if no path was provided.
        None // No path was provided, so nothing to load.
    }
}

/// Loads all configurations from a master `config.yaml` file and its linked sub-files.
///
/// This is a primary orchestrator function. It first reads and parses the main
/// `config.yaml` (whose path is provided by `config_path_resolved`). This `MainConfig`
/// then provides paths to other specific configuration files (`tools.yaml`, `settings.yaml`, etc.).
/// This function then uses the `load_individual_config` helper to load each of those sub-configurations.
/// It includes critical error handling, exiting the application if the main `config.yaml`
/// itself is missing or malformed, as `devbox` cannot function without its core configuration.
///
/// # Arguments
/// * `config_path_resolved`: A `PathBuf` representing the absolute and resolved path
///                           to the main `config.yaml` file.
///
/// # Returns
/// * `ParsedConfigs`: A struct containing `Option`s for each type of configuration.
///                    Each `Option` will be `Some(T)` if the corresponding config
///                    file was found and successfully parsed, or `None` otherwise.
///                    The function will `std::process::exit(1)` if the master config
///                    cannot be loaded.
pub fn load_master_configs(config_path_resolved: &PathBuf) -> ParsedConfigs {
    log_debug!("Entering load_master_configs() function.");
    // Inform the user about which master config file is being loaded.
    log_info!(
        "Loading configurations as per master config file: {}",
        config_path_resolved.display().to_string().blue()
    );

    // Attempt to read the contents of the main `config.yaml` file.
    let main_cfg_content = match fs::read_to_string(config_path_resolved) {
        Ok(c) => c, // Successfully read the file.
        Err(e) => {
            // If the main config file cannot be read (e.g., not found, permissions),
            // this is a critical error, so log and exit the application.
            log_error!(
                "Failed to read main config.yaml at {}: {}. Please ensure the file exists and is readable.",
                config_path_resolved.display().to_string().red(),
                e
            );
            std::process::exit(1); // Exit with a non-zero status code to indicate failure.
        }
    };

    // Attempt to deserialize the content into the `MainConfig` struct.
    let main_cfg: MainConfig = match serde_yaml::from_str(&main_cfg_content) {
        Ok(cfg) => cfg, // Successfully parsed the YAML into MainConfig.
        Err(e) => {
            // If parsing fails (e.g., invalid YAML syntax in main config),
            // this is also a critical error, so log and exit.
            log_error!(
                "Failed to parse main config.yaml at {}: {}. Please check your YAML syntax for errors.",
                config_path_resolved.display().to_string().red(),
                e
            );
            std::process::exit(1); // Exit with a non-zero status code.
        }
    };
    // log_debug!("MainConfig loaded: Tools: {}, Fonts: {}, Shell: {} and Settings: {}", main_cfg.fonts.as_deref()); // Log the loaded MainConfig for debugging.
    log_debug!(
        "MainConfig loaded:
        Tools config: {},
        Settings config: {},
        Shell config: {},
        Fonts config: {}",
        main_cfg.tools.as_deref().unwrap_or("N/A"),
        main_cfg.settings.as_deref().unwrap_or("N/A"),
        main_cfg.shellrc.as_deref().unwrap_or("N/A"),
        main_cfg.fonts.as_deref().unwrap_or("N/A")
    );

    // Use the `load_individual_config` helper function for each linked configuration file.
    // The `as_ref()` is used to convert `Option<String>` into `Option<&String>`,
    // which is required by `load_individual_config`. This avoids consuming the `String` within the `Option`.
    let tools_config = load_individual_config(main_cfg.tools.as_ref(), "tools", "[Tools]");
    let settings_config =
        load_individual_config(main_cfg.settings.as_ref(), "settings", "[Settings]");
    // Note: `shellrc` is the field name in `MainConfig`, but "shell config" is used for clarity in logs.
    let shell_config =
        load_individual_config(main_cfg.shellrc.as_ref(), "shell config", "[Shell Config]");
    let fonts_config = load_individual_config(main_cfg.fonts.as_ref(), "fonts", "[Fonts]");

    log_debug!("Exiting load_master_configs() function.");
    // Return the `ParsedConfigs` struct containing all loaded sub-configurations.
    ParsedConfigs {
        tools: tools_config,
        settings: settings_config,
        shell: shell_config,
        fonts: fonts_config,
    }
}

/// Loads a single configuration file directly, bypassing the master `config.yaml`.
///
/// This function is designed for scenarios where `devbox` is invoked with a direct path
/// to a specific configuration file (e.g., `devbox --config path/to/tools.yaml`).
/// It determines the type of configuration file based on its filename and attempts
/// to parse it accordingly. If the file cannot be read or its type is not recognized,
/// the application will exit.
///
/// # Arguments
/// * `config_path_resolved`: A `PathBuf` representing the absolute and resolved path
///                           to the single configuration file provided by the user.
/// * `config_filename`: A string slice containing just the filename (e.g., "tools.yaml")
///                      from `config_path_resolved`, used to infer the configuration type.
///
/// # Returns
/// * `ParsedConfigs`: A struct containing `Option`s for each type of configuration.
///                    Only the `Option` corresponding to the loaded single config file
///                    will be `Some(T)`; others will remain `None`.
///                    The function will `std::process::exit(1)` if the single config
///                    cannot be loaded or is of an unsupported type.
pub fn load_single_config(config_path_resolved: &PathBuf, config_filename: &str) -> ParsedConfigs {
    log_debug!("Entering load_single_config() function.");
    // Inform the user that a single config file is being loaded.
    log_info!(
        "Loading configuration from single file: {}",
        config_path_resolved.display().to_string().blue()
    );
    log_debug!(
        "Attempting to load configuration directly from: {}",
        config_path_resolved.display()
    );

    // Attempt to read the content of the single configuration file.
    let contents = match fs::read_to_string(config_path_resolved) {
        Ok(c) => c, // Successfully read the file.
        Err(e) => {
            // If the file cannot be read, it's a critical error for single config mode.
            log_error!(
                "Failed to read single config file {}: {}. Please check its existence and permissions.",
                config_path_resolved.display().to_string().red(),
                e
            );
            std::process::exit(1); // Exit the application.
        }
    };

    // Initialize `ParsedConfigs` with all fields set to `None`. Only one will be populated
    // based on the `config_filename`.
    let mut parsed_configs = ParsedConfigs {
        tools: None,
        settings: None,
        shell: None,
        fonts: None,
    };

    // Match the `config_filename` to determine which type of configuration to parse it as.
    match config_filename {
        "tools.yaml" => {
            log_debug!("Identified as tools.yaml. Attempting to parse...");
            // Attempt to deserialize as `ToolConfig`.
            parsed_configs.tools = match serde_yaml::from_str(&contents) {
                Ok(cfg) => {
                    log_info!("[Tools] Successfully parsed tools.yaml.");
                    Some(cfg)
                }
                Err(e) => {
                    log_error!("Failed to parse tools.yaml: {}", e);
                    None
                }
            }
        }
        "settings.yaml" => {
            log_debug!("Identified as settings.yaml. Attempting to parse...");
            // Attempt to deserialize as `SettingsConfig`.
            parsed_configs.settings = match serde_yaml::from_str(&contents) {
                Ok(cfg) => {
                    log_info!("[Settings] Successfully parsed settings.yaml.");
                    Some(cfg)
                }
                Err(e) => {
                    log_error!("Failed to parse settings.yaml: {}", e);
                    None
                }
            }
        }
        "shellrc.yaml" | "shellac.yaml" => {
            // Support for both common shell config filenames.
            log_debug!("Identified as shell config file. Attempting to parse...");
            // Attempt to deserialize as `ShellConfig`.
            parsed_configs.shell = match serde_yaml::from_str(&contents) {
                Ok(cfg) => {
                    log_info!("[ShellRC] Successfully parsed shell config.");
                    Some(cfg)
                }
                Err(e) => {
                    log_error!("Failed to parse shell config: {}", e);
                    None
                }
            }
        }
        "fonts.yaml" => {
            log_debug!("Identified as fonts.yaml. Attempting to parse...");
            // Attempt to deserialize as `FontConfig`.
            parsed_configs.fonts = match serde_yaml::from_str(&contents) {
                Ok(cfg) => {
                    log_info!("[Fonts] Successfully parsed fonts.yaml.");
                    Some(cfg)
                }
                Err(e) => {
                    log_error!("Failed to parse fonts.yaml: {}", e);
                    None
                }
            }
        }
        other => {
            // If the filename does not match any recognized single config type, it's an error.
            log_error!(
                "Unsupported single config file type: '{}'. Expected 'tools.yaml', 'settings.yaml', 'shellrc.yaml', or 'fonts.yaml'.",
                other.red()
            );
            std::process::exit(1); // Exit the application due to an unhandled config type.
        }
    }
    log_debug!("Exiting load_single_config() function.");
    parsed_configs // Return the populated `ParsedConfigs`.
}

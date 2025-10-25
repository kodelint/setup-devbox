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
use std::collections::{HashMap, HashSet, VecDeque};
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
use crate::schemas::common::MainConfig;
use crate::schemas::fonts::FontConfig;
use crate::schemas::os_settings::SettingsConfig;
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::shell_configuration::ShellConfig;
use crate::schemas::tools::{ToolConfig, ToolEntry};

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
///   `serde::de::DeserializeOwned` (allowing `serde_yaml` to create an instance from YAML)
///   and `std::fmt::Debug` (for logging purposes).
///
/// # Arguments
/// * `path_option`: An `Option<&String>` representing the path to the configuration file.
///   This path is typically read from the `MainConfig` struct.
/// * `config_name`: A human-readable name for the configuration type (e.g., "tools", "settings"),
///   used in log messages to identify which config is being processed.
/// * `bold_name`: A string slice used for consistent, bolded prefixes in log messages (e.g., `"[Tools]"`, `"[Settings]"`).
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
        let path = PathResolver::expand_tilde(path_str);
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
///   to the main `config.yaml` file.
///
/// # Returns
/// * `ParsedConfigs`: A struct containing `Option`s for each type of configuration.
///   Each `Option` will be `Some(T)` if the corresponding config
///   file was found and successfully parsed, or `None` otherwise.
///   The function will `std::process::exit(1)` if the master config
///   cannot be loaded.
pub fn load_master_configs(config_path_resolved: &PathBuf) -> ParsedConfigs {
    log_debug!("[SDB::ConfigLoader] Entering load_master_configs() function.");
    // Attempt to read the contents of the main `config.yaml` file.
    let main_cfg_content = match fs::read_to_string(config_path_resolved) {
        Ok(c) => c, // Successfully read the file.
        Err(e) => {
            // If the main config file cannot be read (e.g., not found, permissions),
            // this is a critical error, so log and exit the application.
            log_error!(
                "[SDB::ConfigLoader] Failed to read main config.yaml at {}: {}. Please ensure the file exists and is readable.",
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
                "[SDB::ConfigLoader] Failed to parse main config.yaml at {}: {}. Please check your YAML syntax for errors.",
                config_path_resolved.display().to_string().red(),
                e
            );
            std::process::exit(1); // Exit with a non-zero status code.
        }
    };
    // log_debug!("MainConfig loaded: Tools: {}, Fonts: {}, Shell: {} and Settings: {}", main_cfg.fonts.as_deref()); // Log the loaded MainConfig for debugging.
    log_debug!(
        "[SDB::ConfigLoader] MainConfig loaded:
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

    log_debug!("[SDB::ConfigLoader] Exiting load_master_configs() function.");
    // Return the `ParsedConfigs` struct containing all loaded sub-configurations.
    let parsed_configs = ParsedConfigs {
        tools: tools_config,
        settings: settings_config,
        shell: shell_config,
        fonts: fonts_config,
    };

    // Reorder tools based on dependencies before returning
    reorder_tools_by_dependency(parsed_configs)
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
///   to the single configuration file provided by the user.
/// * `config_filename`: A string slice containing just the filename (e.g., "tools.yaml")
///   from `config_path_resolved`, used to infer the configuration type.
///
/// # Returns
/// * `ParsedConfigs`: A struct containing `Option`s for each type of configuration.
///   Only the `Option` corresponding to the loaded single config file
///   will be `Some(T)`; others will remain `None`.
///   The function will `std::process::exit(1)` if the single config
///   cannot be loaded or is of an unsupported type.
pub fn load_single_config(config_path_resolved: &PathBuf, config_filename: &str) -> ParsedConfigs {
    log_debug!("[SDB::ConfigLoader] Entering load_single_config() function.");
    // Inform the user that a single config file is being loaded.
    log_info!(
        "[SDB] Loading configuration from single file: {}",
        config_path_resolved.display().to_string().blue()
    );
    log_debug!(
        "[SDB::ConfigLoader] Attempting to load configuration directly from: {}",
        config_path_resolved.display()
    );

    // Attempt to read the content of the single configuration file.
    let contents = match fs::read_to_string(config_path_resolved) {
        Ok(c) => c, // Successfully read the file.
        Err(e) => {
            // If the file cannot be read, it's a critical error for single config mode.
            log_error!(
                "[SDB::ConfigLoader] Failed to read single config file {}: {}. Please check its existence and permissions.",
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
            log_debug!("[SDB::ConfigLoader] Identified as tools.yaml. Attempting to parse...");
            // Attempt to deserialize as `ToolConfig`.
            parsed_configs.tools = match serde_yaml::from_str(&contents) {
                Ok(cfg) => {
                    log_info!("[SDB::Tools] Successfully parsed tools.yaml.");
                    Some(cfg)
                }
                Err(e) => {
                    log_error!("[SDB::ConfigLoader] Failed to parse tools.yaml: {}", e);
                    None
                }
            }
        }
        "settings.yaml" => {
            log_debug!("[SDB::ConfigLoader] Identified as settings.yaml. Attempting to parse...");
            // Attempt to deserialize as `SettingsConfig`.
            parsed_configs.settings = match serde_yaml::from_str(&contents) {
                Ok(cfg) => {
                    log_info!("[SDB::Settings] Successfully parsed settings.yaml.");
                    Some(cfg)
                }
                Err(e) => {
                    log_error!("[SDB::ConfigLoader] Failed to parse settings.yaml: {}", e);
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
                    log_info!("[[SDB::ShellCofig] Successfully parsed shell config.");
                    Some(cfg)
                }
                Err(e) => {
                    log_error!("[SDB::ConfigLoader] Failed to parse shell config: {}", e);
                    None
                }
            }
        }
        "fonts.yaml" => {
            log_debug!("Identified as fonts.yaml. Attempting to parse...");
            // Attempt to deserialize as `FontConfig`.
            parsed_configs.fonts = match serde_yaml::from_str(&contents) {
                Ok(cfg) => {
                    log_info!("[SDB::Fonts] Successfully parsed fonts.yaml.");
                    Some(cfg)
                }
                Err(e) => {
                    log_error!("[SDB::ConfigLoader] Failed to parse fonts.yaml: {}", e);
                    None
                }
            }
        }
        other => {
            // If the filename does not match any recognized single config type, it's an error.
            log_error!(
                "[SDB::ConfigLoader] Unsupported single config file type: '{}'. Expected 'tools.yaml', 'settings.yaml', 'shellrc.yaml', or 'fonts.yaml'.",
                other.red()
            );
            std::process::exit(1); // Exit the application due to an unhandled config type.
        }
    }
    log_debug!("[SDB::ConfigLoader] Exiting single config loader function.");
    // Reorder tools based on dependencies and return.
    reorder_tools_by_dependency(parsed_configs)
}

/// Reorders tool entries so that source installers appear before the tools that depend on them.
/// This ensures correct installation sequencing, especially when tools rely on other tools
/// (e.g., `cargo` depends on `rust`, which may depend on `rustup`).
///
/// # Example
/// If `cargo` is listed as a source for other tools, and `cargo` itself is a tool entry,
/// it will be placed before any tools that use it as their source.
///
/// # Parameters
/// - `parsed_configs`: Parsed configuration object containing tool definitions.
///
/// # Returns
/// - A reordered `ParsedConfigs` object with tools sorted by dependency order.
pub fn reorder_tools_by_dependency(mut parsed_configs: ParsedConfigs) -> ParsedConfigs {
    log_debug!("[SDB::ConfigLoader] Reordering tools based on source dependencies.");

    // Only proceed if the tools section exists in the config
    if let Some(ref mut tools_cfg) = parsed_configs.tools {
        // Perform topological sort to determine correct tool order
        let sorted_tools = topological_sort_tools(&tools_cfg.tools);
        tools_cfg.tools = sorted_tools;
    }

    parsed_configs
}

/// Performs a topological sort on tool entries based on their declared or implicit source dependencies.
/// This ensures that installers (tools used as sources) are processed before the tools that depend on them.
///
/// # Special Handling
/// - Rust toolchain is treated specially: `cargo` → `rust` → `rustup`
/// - Implicit dependencies are inferred even if not explicitly declared.
///
/// # Parameters
/// - `tools`: Slice of `ToolEntry` structs representing all configured tools.
///
/// # Returns
/// - A vector of `ToolEntry` sorted in dependency-respecting order.
fn topological_sort_tools(tools: &[ToolEntry]) -> Vec<ToolEntry> {
    // Map tool names to their full entries for fast lookup
    let mut tool_map: HashMap<String, ToolEntry> = HashMap::new();
    let mut tool_names: HashSet<String> = HashSet::new();

    for tool in tools {
        tool_map.insert(tool.name.clone(), tool.clone());
        tool_names.insert(tool.name.clone());
    }

    // Detect presence of Rust toolchain components
    let has_rustup = tool_names.contains("rustup");
    let has_rust = tool_names.contains("rust");
    let has_cargo_users = tools.iter().any(|t| t.source == "cargo");

    // Dependency graph: maps a tool name to the list of tools that depend on it
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    // In-degree map: tracks how many dependencies each tool has
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Set of tools that act as installers (i.e., used as sources by other tools)
    let mut is_installer: HashSet<String> = HashSet::new();

    // Initialize in-degree for all tools
    for tool_name in &tool_names {
        in_degree.insert(tool_name.clone(), 0);
    }

    // Build dependency graph and in-degree map
    for tool in tools {
        let tool_name = &tool.name;
        let source = &tool.source;

        // Case 1: Explicit dependency on another tool
        if tool_names.contains(source) {
            // Direct dependency: this tool depends on 'source'
            graph
                .entry(source.clone())
                .or_default()
                .push(tool_name.clone());

            *in_degree.get_mut(tool_name).unwrap() += 1;
            is_installer.insert(source.clone());

            log_debug!(
                "[SDB::ConfigLoader] Dependency detected: '{}' depends on '{}'",
                tool_name,
                source
            );
        }
        // Case 2: Implicit dependency on Rust via cargo
        else if source == "cargo" && has_rust {
            // Tools using cargo depend on rust
            graph
                .entry("rust".to_string())
                .or_default()
                .push(tool_name.clone());

            *in_degree.get_mut(tool_name).unwrap() += 1;
            is_installer.insert("rust".to_string());

            log_debug!(
                "[SDB::ConfigLoader] Dependency detected: '{}' depends on 'rust' (via cargo source)",
                tool_name
            );
        }
    }

    // Case 3: Enforce rustup → rust dependency if both exist
    if has_rustup && has_rust {
        // Ensure rust depends on rustup
        let rust_already_depends_on_rustup = graph
            .get("rustup")
            .map(|deps| deps.contains(&"rust".to_string()))
            .unwrap_or(false);

        if !rust_already_depends_on_rustup {
            graph
                .entry("rustup".to_string())
                .or_default()
                .push("rust".to_string());

            *in_degree.get_mut("rust").unwrap() += 1;
            is_installer.insert("rustup".to_string());

            log_debug!("[SDB::ConfigLoader] Enforced dependency: 'rust' depends on 'rustup'");
        }
    }

    // Log all tools identified as installers
    if !is_installer.is_empty() {
        let installer_list: Vec<&String> = is_installer.iter().collect();
        log_debug!(
            "[SDB::ConfigLoader] Identified installers (will be prioritized): {}",
            installer_list
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Warn about missing Rust toolchain components
    if has_cargo_users && !has_rust {
        log_warn!(
            "[SDB::ConfigLoader] Some tools use 'cargo' as source, but 'rust' is not in the configuration. \
             Ensure cargo is available on the system or add 'rust' to your config."
        );
    }

    if has_rust && !has_rustup {
        log_warn!(
            "[SDB::ConfigLoader] Tool 'rust' is configured but 'rustup' is not. \
             Ensure rust is installed or add 'rustup' to your config."
        );
    }

    // Begin topological sort using modified Kahn's algorithm
    let mut installer_queue: VecDeque<String> = VecDeque::new();
    let mut regular_queue: VecDeque<String> = VecDeque::new();

    // Start with tools that have no dependencies (in-degree = 0)
    // Separate them into installers and regular tools
    for (tool_name, &degree) in &in_degree {
        if degree == 0 {
            if is_installer.contains(tool_name) {
                installer_queue.push_back(tool_name.clone());
                log_debug!(
                    "[SDB::ConfigLoader] Prioritizing installer '{}' (used by other tools)",
                    tool_name
                );
            } else {
                regular_queue.push_back(tool_name.clone());
                let tool_source = tool_map
                    .get(tool_name)
                    .map(|t| t.source.as_str())
                    .unwrap_or("unknown");
                log_debug!(
                    "[SDB::ConfigLoader] Queuing tool '{}' with external source '{}' (no internal dependencies)",
                    tool_name,
                    tool_source
                );
            }
        }
    }

    log_debug!(
        "[SDB::ConfigLoader] Starting topological sort: {} installers in priority queue, {} tools with external sources queued",
        installer_queue.len(),
        regular_queue.len()
    );

    let mut sorted_names: Vec<String> = Vec::new();
    let mut processing_installers = true;

    // Main sorting loop: process installers first, then regular tools
    while !installer_queue.is_empty() || !regular_queue.is_empty() {
        // Prefer installer queue
        let tool_name = if let Some(name) = installer_queue.pop_front() {
            if processing_installers {
                log_debug!("[SDB::ConfigLoader] Processing installer: '{}'", name);
            } else {
                log_debug!(
                    "[SDB::ConfigLoader] Switching back to installer: '{}'",
                    name
                );
                processing_installers = true;
            }
            name
        } else if let Some(name) = regular_queue.pop_front() {
            if processing_installers {
                log_debug!(
                    "[SDB::ConfigLoader] Installers complete. Processing regular tools starting with: '{}'",
                    name
                );
                processing_installers = false;
            }
            name
        } else {
            break;
        };

        sorted_names.push(tool_name.clone());

        // Decrease in-degree of dependent tools and enqueue if ready
        if let Some(dependents) = graph.get(&tool_name) {
            for dependent in dependents {
                let degree = in_degree.get_mut(dependent).unwrap();
                *degree -= 1;

                if *degree == 0 {
                    // Add to appropriate queue based on whether it's an installer
                    if is_installer.contains(dependent) {
                        installer_queue.push_back(dependent.clone());
                        log_debug!(
                            "[SDB::ConfigLoader] Unlocked installer '{}' for priority processing (all dependencies satisfied)",
                            dependent
                        );
                    } else {
                        regular_queue.push_back(dependent.clone());
                        log_debug!(
                            "[SDB::ConfigLoader] Unlocked tool '{}' (all dependencies satisfied)",
                            dependent
                        );
                    }
                }
            }
        }
    }

    // Detect and handle circular dependencies
    if sorted_names.len() != tools.len() {
        log_warn!(
            "[SDB::ConfigLoader] Circular dependency detected in tool sources. \
             Some tools may be processed in suboptimal order."
        );

        // Add remaining tools (those involved in circular dependencies)
        for tool in tools {
            if !sorted_names.contains(&tool.name) {
                sorted_names.push(tool.name.clone());
            }
        }
    }

    // Final debug log of tool processing order
    log_debug!(
        "[SDB::ConfigLoader] Tools will be processed in the following order: {}",
        sorted_names.join(" -> ")
    );

    // Rebuild the tools vector in sorted order
    sorted_names
        .iter()
        .filter_map(|name| tool_map.get(name).cloned())
        .collect()
}

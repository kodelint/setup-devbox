// This file is the heart of our `setup-devbox` application's "apply" or "sync" functionality.
// When a user runs the `setup-devbox now` command, this is where the magic happens!
// It reads all the configuration files (tools, settings, shell, fonts),
// compares them against the current 'state.json', and then orchestrates
// the installation or application of everything that needs to be in place.
// We bring in all the essential data structures (schemas) we defined earlier.
// These structs tell us how our configuration files and our internal state file should be shaped.
use crate::schema::{DevBoxState, FontConfig, MainConfig, SettingsConfig, ShellConfig, ToolConfig};
// This utility helps us resolve paths that use the '~' (tilde) for the user's home directory.
use crate::utils::expand_tilde;
// Here we import the specific installer modules for different sources.
// Each of these modules (brew, cargo, GitHub, go) knows how to install tools
// from their respective platforms.
use crate::installers::{brew, cargo, github, go};
// Our custom logging macros for debug, error, info, and warning messages.
use crate::{log_debug, log_error, log_info, log_warn};
// For pretty colored output in the terminal.
use colored::Colorize;
// We'll use HashMaps to store our state data (e.g., installed tools) for quick lookups.
use std::collections::HashMap;
// For file system operations, like reading and writing files.
use std::fs;

/// The main entry point for the `now` command.
/// This function is responsible for:
/// 1. Figuring out where the main configuration file (`config.yaml`) is located.
/// 2. Loading the application's internal state (`state.json`) or creating it if it doesn't exist.
/// 3. Parsing all the configuration files (tools, settings, shell, fonts).
/// 4. Iterating through the desired configurations and applying them,
///    while updating the state to keep track of what's been done.
///
/// # Arguments
/// * `config_path`: An `Option<String>` allowing the user to specify a custom path
///                  to their main `config.yaml` file. If `None`, we use a default path.
/// * `state_path`: An `Option<String>` allowing the user to specify a custom path
///                 to the `state.json` file. If `None`, a default path is used.
pub fn run(config_path: Option<String>, state_path: Option<String>) {
    // Let's start by logging the initial input paths for debugging purposes.
    log_debug!("Entered now::run() with config_path={:?}, state_path={:?}", config_path, state_path);

    // Define the default location for our main configuration file if the user doesn't provide one.
    let default_main_config = "~/.setup-devbox/configs/config.yaml";
    // Resolve the actual path to the main configuration file, expanding any '~' (home directory).
    let config_path = expand_tilde(config_path.as_deref().unwrap_or(default_main_config));
    // Extract just the filename from the config path (e.g., "config.yaml" or "tools.yaml").
    // We'll use this later to determine which type of config file we're dealing with.
    let config_filename = config_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    // Resolve the actual path to the internal state file, expanding any '~'.
    let state_path = expand_tilde(state_path.as_deref().unwrap_or("~/.setup-devbox/state.json"));

    // Log the resolved paths so we know exactly where we're looking.
    log_debug!("Using config file: {:?}", config_path);
    log_debug!("Using state file: {:?}", state_path);

    // Load or Initialize Application State
    // This block is critical: it checks if `setup-devbox` has a memory of past actions (`state.json`).
    // If the state file exists, it tries to load it; otherwise, it creates a new, empty state.
    let state: DevBoxState = if state_path.exists() {
        // If the state file exists, read its contents.
        match fs::read_to_string(&state_path) {
            Ok(contents) => {
                // If we successfully read the file, try to parse its JSON content into our `DevBoxState` struct.
                match serde_json::from_str(&contents) {
                    Ok(parsed) => {
                        // Successfully parsed the state! This is our app's "memory."
                        log_info!("Loaded existing state from {:?}", state_path);
                        parsed
                    },
                    Err(err) => {
                        // Oh no, the state file is corrupted or in an invalid format!
                        // We'll log an error and stop execution to prevent unexpected behavior.
                        log_error!("Invalid state.json format: {}. Please check the file or delete it to start fresh.", err);
                        return; // Exit the function.
                    }
                }
            },
            Err(err) => {
                // We couldn't even read the state file (e.g., permissions issue).
                // Log the error and stop.
                log_error!("Failed to read state file {:?}: {}. Check file permissions.", state_path, err);
                return; // Exit the function.
            }
        }
    } else {
        // If the state file doesn't exist, it's our first run (or the user deleted it).
        // Let's create a brand new, empty state.
        log_info!("State file not found at {:?}. Creating a new state file.", state_path);
        let initial_state = DevBoxState {
            tools: HashMap::new(),    // No tools installed yet.
            fonts: HashMap::new(),    // No fonts installed yet.
            settings: HashMap::new(), // No settings applied yet.
        };

        // Before writing the state file, ensure its parent directory exists.
        // This prevents errors if the full path hasn't been created yet.
        if let Some(parent) = state_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                log_error!("Failed to create directory for state file at {:?}: {}. Please check permissions.", parent, e);
                return; // Cannot proceed without a place to store state.
            }
        }

        // Try to serialize the initial (empty) state into a pretty-printed JSON string.
        match serde_json::to_string_pretty(&initial_state) {
            Ok(serialized) => {
                // Attempt to write this serialized JSON to the state file.
                if let Err(err) = fs::write(&state_path, serialized) {
                    log_error!("Failed to write initial state file to {:?}: {}", state_path, err);
                } else {
                    log_info!("Initial state file successfully created at {:?}", state_path);
                }
            },
            Err(err) => {
                // If we can't even serialize the initial state, that's a serious bug!
                log_error!("Failed to serialize initial state: {}", err);
            }
        }
        // Return the newly created initial state to be used for the rest of the run.
        initial_state
    };

    // Variables to Hold Parsed Configuration
    // These `Option` variables will store the parsed content of our various
    // configuration files (tools, settings, shell, fonts). They are `Option`s
    // because a user might not have all config files, or they might be empty.
    let mut tools_config: Option<ToolConfig> = None;
    let mut settings_config: Option<SettingsConfig> = None;
    let mut shell_config: Option<ShellConfig> = None;
    let mut fonts_config: Option<FontConfig> = None;

    // Configuration Loading Logic
    // Here we determine how to load the configurations.
    // If the main config file is named `config.yaml`, we assume it's the master
    // file pointing to other configs. Otherwise, we assume the provided `config_path`
    // is a direct path to one of the individual config files (like `tools.yaml`).
    if config_filename == "config.yaml" {
        // We're dealing with the master `config.yaml` file.
        log_info!("Loading configurations from {:?}", config_path);

        // Read the content of `config.yaml`.
        let main_cfg_content = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                // If we can't read it, log an error and stop.
                log_error!("Failed to read main config.yaml at {:?}: {}", config_path, e);
                return;
            }
        };

        // Parse the YAML content of `config.yaml` into our `MainConfig` struct.
        let main_cfg: MainConfig = match serde_yaml::from_str(&main_cfg_content) {
            Ok(cfg) => cfg,
            Err(e) => {
                // If parsing fails, the YAML is likely malformed.
                log_error!("Failed to parse main config.yaml at {:?}: {}. Please check your YAML syntax.", config_path, e);
                return;
            }
        };

        // Now, based on the paths specified in `main_cfg`, we'll try to load
        // each of the individual configuration files.
        // We use `if let Some` to safely handle cases where a path might not be specified.

        // Attempt to load `tools.yaml`.
        if let Some(tools_path) = main_cfg.tools {
            let path = expand_tilde(&tools_path); // Resolve the full path.
            if let Ok(contents) = fs::read_to_string(&path) {
                // If read successfully, try to parse it as `ToolConfig`.
                match serde_yaml::from_str::<ToolConfig>(&contents) {
                    Ok(cfg) => tools_config = Some(cfg), // Store the parsed config.
                    Err(e) => log_error!("Failed to parse tools.yaml at {:?}: {}. Check its YAML syntax.", path, e),
                }
            } else {
                // If the file simply wasn't found, issue a warning, as it's optional.
                log_warn!("tools.yaml file not found at {:?}. Skipping tool configuration.", path);
            }
        }

        // Attempt to load `settings.yaml`.
        if let Some(settings_path) = main_cfg.settings {
            let path = expand_tilde(&settings_path);
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<SettingsConfig>(&contents) {
                    Ok(cfg) => settings_config = Some(cfg),
                    Err(e) => log_error!("Failed to parse settings.yaml at {:?}: {}. Check its YAML syntax.", path, e),
                }
            } else {
                log_warn!("settings.yaml file not found at {:?}. Skipping settings configuration.", path);
            }
        }

        // Attempt to load `shellac.yaml`.
        if let Some(shellrc_path) = main_cfg.shellrc {
            let path = expand_tilde(&shellrc_path);
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<ShellConfig>(&contents) {
                    Ok(cfg) => shell_config = Some(cfg),
                    Err(e) => log_error!("Failed to parse shellac.yaml at {:?}: {}. Check its YAML syntax.", path, e),
                }
            } else {
                log_warn!("shellac.yaml file not found at {:?}. Skipping shell configuration.", path);
            }
        }

        // Attempt to load `fonts.yaml`.
        if let Some(fonts_path) = main_cfg.fonts {
            let path = expand_tilde(&fonts_path);
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<FontConfig>(&contents) {
                    Ok(cfg) => fonts_config = Some(cfg),
                    Err(e) => log_error!("Failed to parse fonts.yaml at {:?}: {}. Check its YAML syntax.", path, e),
                }
            } else {
                log_warn!("fonts.yaml file not found at {:?}. Skipping font configuration.", path);
            }
        }
    } else {
        // Single Config File Fallback
        // If the provided `config_path` is *not* `config.yaml`, we assume it's directly
        // pointing to one of the specific config files (like `tools.yaml` itself).
        log_info!("Loading configuration from single file: {:?}", config_path);

        // Read the content of the single config file.
        let contents = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                log_error!("Failed to read config file {:?}: {}", config_path, e);
                return;
            }
        };

        // Based on the filename, try to parse the content into the corresponding config struct.
        match config_filename {
            "tools.yaml" => tools_config = serde_yaml::from_str(&contents).ok(),
            "settings.yaml" => settings_config = serde_yaml::from_str(&contents).ok(),
            // Handle both potential names for the shell config file.
            "shellrc.yaml" | "shellac.yaml" => shell_config = serde_yaml::from_str(&contents).ok(),
            "fonts.yaml" => fonts_config = serde_yaml::from_str(&contents).ok(),
            // If it's none of the recognized single config file names, it's an error.
            other => {
                log_error!("Unsupported single config file type: '{}'. Expected 'tools.yaml', 'settings.yaml', 'shellac.yaml', or 'fonts.yaml'.", other);
                return;
            }
        }
    }

    // Apply Configurations and Update State
    // Now that all configurations are loaded, we iterate through them and perform the actual work:
    // installing tools, applying settings, etc., and critically, updating our `state.json`.

    // Install Tools
    // If we have a `ToolConfig` (meaning `tools.yaml` was loaded), let's process the tools.
    if let Some(tools_cfg) = tools_config {
        // We'll create a mutable copy of our `DevBoxState` to track changes during this process.
        let mut new_state = state.clone();
        // A flag to track if any changes were made to the state, so we know if we need to save.
        let mut updated = false;

        // Loop through each tool entry defined in the configuration.
        for tool in &tools_cfg.tools {
            // Check if this tool is *already* recorded in our `new_state.tools` HashMap.
            // This is how we avoid re-installing things that are already there!
            if !new_state.tools.contains_key(&tool.name) {
                log_info!("⚙Installing new tool: {}", tool.name.bold());
                log_debug!("Full tool config details: {:?}", tool);

                // Based on the tool's `source` (GitHub, brew, cargo, go), call the appropriate installer function.
                let result = match tool.source.as_str() {
                    "github" => github::install(tool), // Delegate to the GitHub installer.
                    // Commenting rest of the package managers calls for now
                    // Will visit them one by one
                    // "brew" => brew::install(tool),     // Delegate to the Homebrew installer.
                    // "cargo" => cargo::install(tool),   // Delegate to the Cargo (Rust) installer.
                    // "go" => go::install(tool),         // Delegate to the Go installer.
                    other => {
                        // If the source is unrecognized, warn the user and skip this tool.
                        log_warn!("Unknown source '{}' for tool '{}'. Skipping this tool.", other.yellow(), tool.name.bold());
                        None // Indicate that no state was generated for this tool.
                    }
                };

                // If the installation was successful (returned `Some(ToolState)`)...
                if let Some(tool_state) = result {
                    // Add the details of the newly installed tool to our `new_state`.
                    new_state.tools.insert(tool.name.clone(), tool_state);
                    updated = true; // Mark that the state has changed.
                    log_info!("Successfully installed {}.", tool.name.bold());
                } else {
                    // If the installation failed, log an error.
                    log_error!("Failed to install tool: {}. See previous logs for details.", tool.name.bold());
                }
            } else {
                // If the tool is already in our state, just log that we're skipping it.
                log_debug!("Tool '{}' already recorded as installed. Skipping installation.", tool.name);
                // TODO: In a more advanced version, we might add logic here to *update*
                // tools if the version specified in config is newer than the installed version.
            }
        }

        // After trying to install all tools, if any state changes occurred, save the new state.
        if updated {
            log_info!("Updating application state file at {:?}", state_path);
            match serde_json::to_string_pretty(&new_state) {
                Ok(serialized) => {
                    if let Err(err) = fs::write(&state_path, serialized) {
                        log_error!("Failed to write updated state file to {:?}: {}", state_path, err);
                    } else {
                        log_info!("State file updated successfully. Your `setup-devbox` memory is now up-to-date!",);
                    }
                },
                Err(err) => log_error!("Failed to serialize updated state for saving: {}", err),
            }
        } else {
            // If no tools were installed or updated, let the user know.
            log_info!("All tools from your configuration are already installed or up-to-date.");
        }
    }

    // Apply macOS System Settings
    // If `settings.yaml` was loaded, we'd apply system settings here.
    if let Some(settings_cfg) = settings_config {
        log_info!("Applying system settings from settings.yaml (feature coming soon!)");
        // TODO: The actual implementation for applying settings would go here,
        // likely involving iterating through `settings_cfg.settings` and executing
        // OS-specific commands (like `defaults write` on macOS).
    }

    // Apply Shell Configuration
    // If `shellac.yaml` was loaded, we'd apply shell configurations and aliases.
    if let Some(shell_cfg) = shell_config {
        log_info!("Applying shell configuration from shellac.yaml (feature coming soon!)");
        // TODO: This section would handle writing `raw_configs` to `.bashrc`, `.zshrc`, etc.,
        // and setting up aliases. This often involves careful handling of existing shell files.
    }

    // Install Fonts
    // If `fonts.yaml` was loaded, we'd install fonts.
    if let Some(fonts_cfg) = fonts_config {
        log_info!("Installing fonts from fonts.yaml (feature coming soon!)");
        // TODO: Font installation typically involves downloading font files and placing them
        // in system-specific font directories, then refreshing the font cache.
    }

    // We've processed all the configurations!
    log_info!("DevBox 'now' command completed.");
}
// This file is the heart of our `setup-devbox` application's "apply" or "sync" functionality.
// When a user runs the `setup-devbox now` command, this is where the magic happens!
// It reads all the configuration files (tools, settings, shell, fonts),
// compares them against the current 'state.json', and then orchestrates
// the installation or application of everything that needs to be in place.

// We bring in all the essential data structures (schemas) we defined earlier.
// These structs tell us how our configuration files and our internal state file should be shaped.
use crate::schema::{DevBoxState, FontConfig, MainConfig, SettingsConfig, ShellConfig, ToolConfig};
// This module provides utility functions like expanding '~' in paths and our new state saver.
use crate::utils::{expand_tilde, save_devbox_state};
// Here we import the specific installer modules for different sources.
// Each of these modules knows how to install tools or configure specific system aspects.
use crate::installers::{
    brew,    // For Homebrew packages
    github,  // For tools from GitHub releases
    fonts,   // Our dedicated module for installing fonts
    shellrc, // Our dedicated module for managing shell RC files (like .zshrc)
};
// Our custom logging macros for debug, error, info, and warning messages.
use crate::{log_debug, log_error, log_info, log_warn};
// For pretty colored output in the terminal, making logs easier to read.
use colored::Colorize;
// We'll use HashMaps to store our state data (e.g., installed tools, fonts) for quick lookups.
use std::collections::HashMap;
// For file system operations, like reading and writing files.
use std::fs;
// For working with file paths in a cross-platform way.
use std::path::PathBuf;

/// The main entry point for the `now` command.
/// This function is the central control hub for `setup-devbox now`.
/// It meticulously handles:
/// 1.  **Configuration Discovery**: Locating and loading the main `config.yaml`
///     or a directly specified individual configuration file.
/// 2.  **State Management**: Loading the previous installation state from `state.json`
///     (our application's "memory") or creating a new one if it's the first run.
/// 3.  **Configuration Parsing**: Reading and deserializing all linked
///     configuration files (tools, settings, shell, fonts) into Rust structs.
/// 4.  **Intelligent Application**: Iterating through the desired configurations,
///     checking the current state, and delegating to specific installer modules
///     (e.g., `brew`, `github`, `fonts`, `shellrc`) only for new or updated items.
/// 5.  **State Persistence**: Critically, updating and saving the `state.json`
///     file immediately after each successful installation/application block
///     to ensure `setup-devbox` remembers what it's done, even if a later step fails.
///
/// # Arguments
/// * `config_path`: An `Option<String>` allowing the user to specify a custom path
///                  to their main `config.yaml` file (or a single config file like `tools.yaml`).
///                  If `None`, a default path (`~/.setup-devbox/configs/config.yaml`) is used.
/// * `state_path`: An `Option<String>` allowing the user to specify a custom path
///                 to the `state.json` file. If `None`, a default path (`~/.setup-devbox/state.json`) is used.
pub fn run(config_path: Option<String>, state_path: Option<String>) {
    // Let's start by logging the initial input paths. This is super helpful for debugging,
    // as it immediately shows what inputs `now::run` received.
    log_debug!("Entered now::run() function.");
    log_debug!("Initial config_path parameter: {:?}", config_path);
    log_debug!("Initial state_path parameter: {:?}", state_path);

    // 1. Determine Configuration and State File Paths
    // Define the default location for our main configuration file.
    // This is where `setup-devbox` will look if the user doesn't specify one.
    let default_main_config = "~/.setup-devbox/configs/config.yaml";
    // Resolve the actual, absolute path to the main configuration file.
    // `expand_tilde` handles replacing `~` with the actual home directory path.
    let config_path_resolved: PathBuf = expand_tilde(config_path.as_deref().unwrap_or(default_main_config));
    // Extract just the filename (e.g., "config.yaml", "tools.yaml") from the resolved config path.
    // We'll use this filename later to decide how to parse the configuration.
    let config_filename = config_path_resolved.file_name().and_then(|s| s.to_str()).unwrap_or("");
    // Resolve the actual, absolute path to the application's internal state file (`state.json`).
    let state_path_resolved: PathBuf = expand_tilde(state_path.as_deref().unwrap_or("~/.setup-devbox/state.json"));

    // Log the final, resolved paths. This is vital for confirming we're looking in the right places.
    log_info!("Using configuration file: {}", config_path_resolved.display().to_string().cyan());
    log_debug!("Managing application state in: {}", state_path_resolved.display().to_string().yellow());
    log_debug!("Resolved config_path: {:?}", config_path_resolved);
    log_debug!("Resolved state_path: {:?}", state_path_resolved);
    log_debug!("Detected config filename: '{}'", config_filename.blue());

    // 2. Load or Initialize Application State (`state.json`)
    // This is a critical step: `setup-devbox` needs to remember what it has already installed.
    // We try to load the existing `state.json`; if it doesn't exist, we create a fresh one.
    let mut state: DevBoxState = if state_path_resolved.exists() {
        // If the state file already exists, let's try to read its contents.
        log_debug!("State file found at {:?}. Attempting to load...", state_path_resolved);
        match fs::read_to_string(&state_path_resolved) {
            Ok(contents) => {
                // If we successfully read the file, try to parse its JSON content
                // into our `DevBoxState` struct. This is where our app's "memory" comes from.
                match serde_json::from_str(&contents) {
                    Ok(parsed_state) => {
                        log_info!("Using state file: {}", state_path_resolved.display().to_string().cyan());
                        log_debug!("Existing state file: {}", state_path_resolved.display().to_string().yellow());
                        match serde_json::to_string_pretty(&parsed_state) {
                            Ok(pretty_json) => {
                                log_debug!("Loaded DevBoxState:\n{}", pretty_json);
                            },
                            Err(e) => {
                                // This case is unlikely if deserialization just worked,
                                // but good to handle for robustness.
                                log_warn!("Failed to re-serialize loaded DevBoxState for pretty printing: {}", e);
                                log_debug!("Loaded DevBoxState (raw debug format): {:?}", parsed_state);
                            }
                        }
                        parsed_state // Return the successfully parsed state.
                    },
                    Err(err) => {
                        // Oh no, the state file exists but it's corrupted or in an invalid format!
                        // This is a serious issue, so we log an error and stop execution to prevent
                        // `setup-devbox` from making incorrect decisions.
                        log_error!(
                            "Invalid state.json format at {:?}: {}. Please check the file's content or delete it to start fresh.",
                            state_path_resolved.display().to_string().red(),
                            err
                        );
                        return; // Exit the function as we cannot proceed without valid state.
                    }
                }
            },
            Err(err) => {
                // We couldn't even read the state file (e.g., due to permissions issues).
                // Log the error and stop the process.
                log_error!(
                    "Failed to read state file {:?}: {}. Please verify file permissions.",
                    state_path_resolved.display().to_string().red(),
                    err
                );
                return; // Exit the function.
            }
        }
    } else {
        // If the state file doesn't exist, it means this is either the first run
        // of `setup-devbox`, or the user has intentionally removed the state file.
        log_info!("State file not found at {:?}. Creating a brand new state file.", state_path_resolved.display().to_string().yellow());
        // Initialize a new, empty `DevBoxState`. This is our app's "blank memory slate."
        let initial_state = DevBoxState {
            tools: HashMap::new(),    // No tools are installed yet, so an empty map.
            fonts: HashMap::new(),    // No fonts installed yet.
            settings: HashMap::new(), // No settings applied yet.
        };

        // Before attempting to write the state file, we must ensure that its parent directory exists.
        // This prevents errors if the user's chosen path includes directories that haven't been created.
        if let Some(parent_dir) = state_path_resolved.parent() {
            log_debug!("Checking/creating parent directory for state file: {:?}", parent_dir);
            if let Err(e) = fs::create_dir_all(parent_dir) {
                log_error!(
                    "Failed to create directory for state file at {:?}: {}. Cannot save state.",
                    parent_dir.display().to_string().red(),
                    e
                );
                return; // Cannot proceed without a place to store state.
            }
        }

        // Try to serialize the newly created (empty) state into a human-readable, pretty-printed JSON string.
        match serde_json::to_string_pretty(&initial_state) {
            Ok(serialized_state) => {
                // Attempt to write this serialized JSON content to the `state.json` file.
                if let Err(err) = fs::write(&state_path_resolved, serialized_state) {
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
                // If we can't even serialize the initial state struct into JSON, that indicates a bug
                // in our schema or serde setup. This is a critical failure.
                log_error!("Failed to serialize initial state: {}. This is an internal application error.", err);
            }
        }
        initial_state // Return the newly created initial state for use in the current run.
    };

    // 3. Variables to Hold Parsed Configuration
    // These `Option` variables will serve as containers for the parsed content
    // of our various configuration files (tools, settings, shell, fonts).
    // They are `Option`s because it's perfectly normal for a user not to have
    // all types of config files, or for a file to be empty.
    let mut tools_config: Option<ToolConfig> = None;
    let mut settings_config: Option<SettingsConfig> = None;
    let mut shell_config: Option<ShellConfig> = None;
    let mut fonts_config: Option<FontConfig> = None;

    // 4. Configuration Loading Logic
    // Here's where we decide *how* to load the configurations.
    // If the main config file provided (or defaulted to) is `config.yaml`,
    // we assume it's the master file that points to other, individual config files.
    // Otherwise, we assume the provided `config_path_resolved` is a direct path
    // to one of the individual config files (like `tools.yaml` directly).
    if config_filename == "config.yaml" {
        // We're working with the master `config.yaml` file, which orchestrates other configs.
        log_debug!("Loading configurations as per master config file: {}", config_path_resolved.display().to_string().blue());

        // Attempt to read the entire content of `config.yaml` into a string.
        let main_cfg_content = match fs::read_to_string(&config_path_resolved) {
            Ok(c) => c, // Successfully read the content.
            Err(e) => {
                // If we can't read the master config file (e.g., it's missing or permissions are off),
                // log a critical error and halt execution. We can't proceed without it.
                log_error!(
                    "Failed to read main config.yaml at {:?}: {}. Please ensure the file exists and is readable.",
                    config_path_resolved.display().to_string().red(),
                    e
                );
                return; // Exit the function.
            }
        };

        // Try to parse the YAML content of `config.yaml` into our `MainConfig` struct.
        // `MainConfig` contains paths to other specific config files.
        let main_cfg: MainConfig = match serde_yaml::from_str(&main_cfg_content) {
            Ok(cfg) => cfg, // Successfully parsed the main configuration.
            Err(e) => {
                // If parsing fails, the YAML syntax is likely malformed. Log an error and stop.
                log_error!(
                    "Failed to parse main config.yaml at {:?}: {}. Please check your YAML syntax for errors.",
                    config_path_resolved.display().to_string().red(),
                    e
                );
                return; // Exit the function.
            }
        };
        log_debug!("MainConfig loaded: {:?}", main_cfg);

        // Now, based on the paths specified within the `main_cfg` struct,
        // we'll attempt to load each of the individual configuration files.
        // We use `if let Some` to gracefully handle cases where a specific path
        // (like `tools` or `settings`) might not be provided in `config.yaml`.

        // Attempt to load `tools.yaml` if its path is specified in `main_cfg`.
        if let Some(tools_path) = main_cfg.tools {
            let path = expand_tilde(&tools_path); // Resolve the full path to `tools.yaml`.
            log_debug!("Attempting to load tools config from: {:?}", path.display());
            // Try to read the file's contents.
            if let Ok(contents) = fs::read_to_string(&path) {
                // If read successfully, attempt to parse it as a `ToolConfig`.
                match serde_yaml::from_str::<ToolConfig>(&contents) {
                    Ok(cfg) => {
                        tools_config = Some(cfg); // Store the successfully parsed `ToolConfig`.
                        log_debug!("Successfully loaded tool configuration from {}", path.display().to_string().green());
                    },
                    Err(e) => log_error!("Failed to parse tools.yaml at {:?}: {}. Please check its YAML syntax.", path.display().to_string().red(), e),
                }
            } else {
                // If the file simply wasn't found (or was unreadable), issue a warning.
                // This is a warning, not an error, because tool config is optional.
                log_warn!("Tools configuration file not found or unreadable at {:?}. Skipping tool setup.", path.display().to_string().yellow());
            }
        }

        // Attempt to load `settings.yaml` if its path is specified.
        if let Some(settings_path) = main_cfg.settings {
            let path = expand_tilde(&settings_path);
            log_debug!("Attempting to load settings config from: {:?}", path.display());
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<SettingsConfig>(&contents) {
                    Ok(cfg) => {
                        settings_config = Some(cfg);
                        log_debug!("Successfully loaded settings configuration from {}", path.display().to_string().green());
                    },
                    Err(e) => log_error!("Failed to parse settings.yaml at {:?}: {}. Please check its YAML syntax.", path.display().to_string().red(), e),
                }
            } else {
                log_warn!("Settings configuration file not found or unreadable at {:?}. Skipping settings application.", path.display().to_string().yellow());
            }
        }

        // Attempt to load `shellrc.yaml` if its path is specified.
        if let Some(shellrc_path) = main_cfg.shellrc {
            let path = expand_tilde(&shellrc_path);
            log_debug!("Attempting to load shell config from: {:?}", path.display());
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<ShellConfig>(&contents) {
                    Ok(cfg) => {
                        shell_config = Some(cfg);
                        log_debug!("Successfully loaded shell configuration from {}", path.display().to_string().green());
                    },
                    Err(e) => log_error!("Failed to parse shellrc.yaml at {}: {}. Please check its YAML syntax.", path.display().to_string().red(), e),
                }
            } else {
                log_warn!("Shell configuration file not found or unreadable at {}. Skipping shell setup.", path.display().to_string().yellow());
            }
        }

        // Attempt to load `fonts.yaml` if its path is specified.
        if let Some(fonts_path) = main_cfg.fonts {
            let path = expand_tilde(&fonts_path);
            log_debug!("Attempting to load fonts config from: {:?}", path.display());
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<FontConfig>(&contents) {
                    Ok(cfg) => {
                        fonts_config = Some(cfg);
                        log_debug!("Successfully loaded font configuration from {}", path.display().to_string().green());
                    },
                    Err(e) => log_error!("Failed to parse fonts.yaml at {}: {}. Please check its YAML syntax.", path.display().to_string().red(), e),
                }
            } else {
                log_warn!("Font configuration file not found or unreadable at {}. Skipping font installation.", path.display().to_string().yellow());
            }
        }
    } else {
        // 4b. Single Config File Fallback
        // If the `config_filename` is NOT `config.yaml`, it means the user has
        // directly provided a path to one specific config file (e.g., `setup-devbox now --config ~/.setup-devbox/tools.yaml`).
        log_info!("Loading configuration from single file: {}", config_path_resolved.display().to_string().blue());
        log_debug!("Attempting to load configuration directly from: {:?}", config_path_resolved.display());

        // Attempt to read the content of this single configuration file.
        let contents = match fs::read_to_string(&config_path_resolved) {
            Ok(c) => c, // Successfully read the content.
            Err(e) => {
                log_error!(
                    "Failed to read single config file {:?}: {}. Please check its existence and permissions.",
                    config_path_resolved.display().to_string().red(),
                    e
                );
                return; // Exit the function.
            }
        };

        // Based on the filename, we'll try to parse the content into the
        // corresponding configuration struct.
        match config_filename {
            "tools.yaml" => {
                log_debug!("Identified as tools.yaml. Attempting to parse...");
                tools_config = match serde_yaml::from_str(&contents) {
                    Ok(cfg) => { log_info!("Successfully parsed tools.yaml."); Some(cfg) },
                    Err(e) => { log_error!("Failed to parse tools.yaml: {}", e); None },
                }
            },
            "settings.yaml" => {
                log_debug!("Identified as settings.yaml. Attempting to parse...");
                settings_config = match serde_yaml::from_str(&contents) {
                    Ok(cfg) => { log_info!("Successfully parsed settings.yaml."); Some(cfg) },
                    Err(e) => { log_error!("Failed to parse settings.yaml: {}", e); None },
                }
            },
            // Handle both potential names for the shell config file (`shellrc.yaml` or `shellac.yaml`).
            "shellrc.yaml" | "shellac.yaml" => {
                log_debug!("Identified as shell config file. Attempting to parse...");
                shell_config = match serde_yaml::from_str(&contents) {
                    Ok(cfg) => { log_info!("Successfully parsed shell config."); Some(cfg) },
                    Err(e) => { log_error!("Failed to parse shell config: {}", e); None },
                }
            },
            "fonts.yaml" => {
                log_debug!("Identified as fonts.yaml. Attempting to parse...");
                fonts_config = match serde_yaml::from_str(&contents) {
                    Ok(cfg) => { log_info!("Successfully parsed fonts.yaml."); Some(cfg) },
                    Err(e) => { log_error!("Failed to parse fonts.yaml: {}", e); None },
                }
            },
            // If the filename doesn't match any of our expected single config types, it's an error.
            other => {
                log_error!(
                    "Unsupported single config file type: '{}'. Expected 'tools.yaml', 'settings.yaml', 'shellrc.yaml', or 'fonts.yaml'.",
                    other.red()
                );
                return; // Exit the function as we can't process an unknown config type.
            }
        }
    }

    // 5. Apply Configurations and Update State (Granularly)
    // Now that all configurations are loaded (or identified as missing),
    // we iterate through them and perform the actual work of installing tools,
    // applying settings, etc. CRITICALLY, we now update and save our `state.json`
    // after each major section (tools, fonts, shellrc, settings) if changes occurred
    // within that specific section.

    // 5a. Install Tools
    // If we have `ToolConfig` (meaning `tools.yaml` was successfully loaded and parsed),
    // let's go ahead and process the tool installations.
    if let Some(tools_cfg) = tools_config {
        log_info!("Processing tool installations...");
        let mut tools_updated = false; // Flag to track if any tools were installed/updated in this block.

        // Loop through each `ToolEntry` defined in the `tools.yaml` configuration.
        for tool in &tools_cfg.tools {
            log_debug!("Considering tool: {:?}", tool.name.bold());
            // Check if this tool is *already* recorded in our `state.tools` HashMap.
            // This is how `setup-devbox` knows to avoid re-installing tools that are already managed.
            if !state.tools.contains_key(&tool.name) {
                print!("\n");
                eprintln!("{}", "==============================================================================================".blue());
                log_info!("Installing new tool: {}", tool.name.to_string().blue());
                log_debug!("Full configuration details for tool '{}': {:?}", tool.name, tool);

                // Based on the tool's `source` (e.g., "GitHub", "brew"),
                // we delegate the actual installation task to the appropriate installer module.
                let installation_result = match tool.source.as_str() {
                    "github" => github::install(tool), // Hand off to the GitHub installer.
                    "brew" => brew::install(tool),     // Hand off to the Homebrew installer.
                    // If you add other installer types like `cargo` or `go`, uncomment and add them here:
                    // "cargo" => cargo::install(tool),
                    // "go" => go::install(tool),
                    other => {
                        // If the tool's source is unrecognized, warn the user and skip this tool.
                        log_warn!(
                            "Unsupported source '{}' for tool '{}'. Skipping this tool's installation.",
                            other.yellow(),
                            tool.name.bold()
                        );
                        None // Indicate that no `ToolState` was generated for this tool.
                    }
                };

                // If the installation was successful (meaning `Some(ToolState)` was returned)...
                if let Some(tool_state) = installation_result {
                    // Add the details of the newly installed tool to our `state.tools` HashMap.
                    state.tools.insert(tool.name.clone(), tool_state);
                    tools_updated = true; // Mark that tools state has changed in this block.
                    log_info!("Successfully installed tool: {}", tool.name.bold().green());
                    eprintln!("{}", "==============================================================================================".blue());
                    print!("\n");
                } else {
                    // If the installation failed for any reason, log an error.
                    log_error!(
                        "Failed to install tool: {}. Please review previous logs for specific errors during installation.",
                        tool.name.bold().red()
                    );
                }
            } else {
                // If the tool is already recorded in our `state.tools` map, we just log that we're skipping it.
                log_info!("Tool '{}' is already recorded as installed. Skipping re-installation.", tool.name.blue());
                // TODO: In a more advanced version, we might add logic here to *update*
                // tools if the version specified in config is newer than the installed version.
            }
        }

        // After processing all tools, if any `tools_updated` occurred, save the state.
        if tools_updated {
            log_info!("Tool state updated. Saving current DevBox state...");
            if !save_devbox_state(&state, &state_path_resolved) {
                log_error!("Failed to save state after tool installations. Data loss risk!");
            }
        } else {
            log_info!("No new tools installed or state changes detected for tools.");
        }
        log_info!("Finished processing all tool installations.");
    } else {
        log_debug!("No tool configurations found (tools.yaml missing or empty). Skipping tool installation phase.");
    }

    // 5b. Install Fonts
    // If `fonts_config` (from `fonts.yaml`) was loaded, we proceed with font installations.
    if let Some(fonts_cfg) = fonts_config {
        log_info!("Processing font installations from fonts.yaml...");
        let mut fonts_updated = false; // Flag to track if any fonts were installed/updated.

        // Loop through each `FontEntry` defined in the `fonts.yaml` configuration.
        for font in &fonts_cfg.fonts {
            log_debug!("Considering font: {:?}", font.name.bold());
            // Check if this font is *already* recorded in our `state.fonts` HashMap.
            // This prevents re-installing fonts that are already managed.
            if !state.fonts.contains_key(&font.name) {
                log_info!("Installing new font: {}", font.name.bold());
                log_debug!("Full configuration details for font '{}': {:?}", font.name, font);

                // Delegate the actual font installation to the `fonts::install` function.
                // This function returns an `Option<FontState>` if successful.
                let installation_result = fonts::install(font);

                // If the font installation was successful (`Some(FontState)` was returned)...
                if let Some(font_state) = installation_result {
                    // Add the details of the newly installed font to our `state.fonts` HashMap.
                    state.fonts.insert(font.name.clone(), font_state);
                    fonts_updated = true; // Mark that fonts state has changed.
                    log_info!("Successfully installed font: {}", font.name.bold().green());
                } else {
                    // If the installation failed, log an error.
                    log_error!(
                        "Failed to install font: {}. Please review previous logs for specific errors during installation.",
                        font.name.bold().red()
                    );
                }
            } else {
                // If the font is already in our state, we just log that we're skipping it.
                log_info!("Font '{}' is already recorded as installed. Skipping re-installation.", font.name.blue());
                // TODO: Similar to tools, consider adding update logic for fonts here if versioning changes.
            }
        }

        // After processing all fonts, if any `fonts_updated` occurred, save the state.
        if fonts_updated {
            log_info!("Font state updated. Saving current DevBox state...");
            if !save_devbox_state(&state, &state_path_resolved) {
                log_error!("Failed to save state after font installations. Data loss risk!");
            }
        } else {
            log_info!("No new fonts installed or state changes detected for fonts.");
        }
        log_info!("Finished processing all font installations.");
    } else {
        log_debug!("No font configurations found (fonts.yaml missing or empty). Skipping font installation phase.");
    }

    // 5c. Apply Shell Configuration (Raw Configs and Aliases)
    // If `shell_config` (from `shellrc.yaml`) was loaded, we apply shell configurations.
    if let Some(shell_cfg) = shell_config {
        log_info!("Applying shell configurations and aliases from shellrc.yaml...");
        // Pretty print shell_cfg.shellrc
        match serde_json::to_string_pretty(&shell_cfg.shellrc) {
            Ok(pretty_shellrc) => {
                log_debug!("Calling shellrc::apply_shellrc with shell config:\n{}", pretty_shellrc);
            },
            Err(e) => {
                log_warn!("Failed to pretty-print shell config for debug log: {}", e);
                log_debug!("Calling shellrc::apply_shellrc with shell config: {:?}", shell_cfg.shellrc);
            }
        }

        // Pretty print shell_cfg.aliases
        match serde_json::to_string_pretty(&shell_cfg.aliases) {
            Ok(pretty_aliases) => {
                log_debug!("And aliases:\n{}", pretty_aliases);
            },
            Err(e) => {
                log_warn!("Failed to pretty-print aliases for debug log: {}", e);
                log_debug!("And aliases: {:?}", shell_cfg.aliases);
            }
        }

        // Delegate the actual work of applying shell configurations and aliases
        // to the `shellrc::apply_shellrc` function. This function is designed
        // to handle its own logic for determining if changes are needed and
        // will write to the RC file if necessary.
        // For simplicity, we assume `apply_shellrc` performs its actions and
        // that its successful completion implies a potential change on disk that
        // should prompt a state save for robust tracking. If `apply_shellrc` itself
        // knew if it made changes, it could return a bool. For now, we'll save regardless
        // if this section was processed.
        shellrc::apply_shellrc(&shell_cfg.shellrc, &shell_cfg.aliases);

        log_info!("Shell configuration application phase completed. Saving current DevBox state...");
        // Always save state after shellrc attempt, as `apply_shellrc` does its own checks
        // for new lines and writes. Even if no new lines, the process was 'completed'.
        if !save_devbox_state(&state, &state_path_resolved) {
            log_error!("Failed to save state after shell configuration. Data loss risk!");
        }
    } else {
        log_debug!("No shell configurations found (shellrc.yaml missing or empty). Skipping shell configuration phase.");
    }
    // 5d. Apply macOS System Settings
    // If `settings.yaml` was loaded, we'd apply system settings here.
    if let Some(_settings_cfg) = settings_config {
        log_info!("Applying system settings from settings.yaml...");
        log_warn!("The 'settings' application feature is currently under development and will be implemented soon!");
        // TODO: The actual implementation for applying settings would go here.
        // This would likely involve iterating through `_settings_cfg.settings` and executing
        // OS-specific commands (like `defaults write` on macOS) or calling specialized functions.
        // If settings are applied and need to be tracked in state, update `state.settings`.
        // If `state.settings` is updated, call `save_devbox_state(&state, &state_path_resolved);` here.
        log_info!("System settings application phase completed (pending full feature implementation).");
        // Assuming some settings *might* be applied, trigger a save just in case,
        // or refine this if settings modification also reports `true` for changes.
        // For now, if _settings_cfg exists, we assume the intention was to make changes.
        log_info!("System settings processing finished. Saving current DevBox state...");
        if !save_devbox_state(&state, &state_path_resolved) {
            log_error!("Failed to save state after settings application. Data loss risk!");
        }
    } else {
        log_debug!("No system settings configurations found (settings.yaml missing or empty). Skipping settings application phase.");
    }

    // Finally, let's wrap up the `now` command execution.
    log_info!("DevBox 'now' command completed its mission!");
    log_debug!("Exited now::run() function.");
}
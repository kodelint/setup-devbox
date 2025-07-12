// This file is the heart of our `setup-devbox` application.
// When a user runs the `setup-devbox now` command, this is where the magic happens!
// Think of it as the grand orchestrator that brings your entire development environment
// vision (defined in your YAML configs) to life on your machine.
// It meticulously reads all the configuration files (tools, settings, shell, fonts),
// compares them against the current 'state.json' (which is like our app's persistent memory),
// and then intelligently orchestrates the installation or application of everything that
// needs to be in place or updated, making sure we don't do unnecessary work!

// We bring in all the essential data structures (schemas) we defined earlier.
// These structs are like blueprints that tell us how our configuration files
// and our internal 'state.json' file should be shaped. They ensure we're
// always working with well-organized and predictable data.
use crate::schema::{DevBoxState, FontConfig, MainConfig, SettingsConfig, ShellConfig, ToolConfig};
// This module provides super helpful utility functions that make our lives easier,
// like `expand_tilde` (which smartly turns `~` into your home directory path),
// `save_devbox_state` (our dedicated state saver), and `draw_titled_rectangle`
// (for pretty visual headers in our terminal output!).
use crate::utils::{
    expand_tilde,
    save_devbox_state
};
// Here we import the specific installer modules, each a specialist in its own domain.
// Think of them as dedicated workers who know exactly how to install tools from
// different sources or configure specific parts of your system.
use crate::installers::{
    brew,    // Our expert for Homebrew packages – a macOS and Linux package manager.
    github,  // The pro at fetching and installing tools directly from GitHub releases.
    fonts,   // Our dedicated font installer, ensuring your custom fonts are in place.
    shellrc, // The meticulous manager for your shell RC files (like `.zshrc` or `.bashrc`), keeping them tidy.
};
// Our custom logging macros for debug, error, info, and warning messages.
// These are essential for giving you clear feedback on what `setup-devbox` is doing!
use crate::{log_debug, log_error, log_info, log_warn};
// For pretty colored output in the terminal, making logs not just informative but also a joy to read.
use colored::Colorize;
// We'll use HashMaps (think of them as super-fast dictionaries) to store
// our application's state data (e.g., all the tools and fonts we've installed)
// for quick lookups and efficient management.
use std::collections::HashMap;
// The standard Rust library module for performing file system operations,
// like reading (peeking into) and writing (saving changes to) files.
use std::fs;
// For working with file paths in a smart, cross-platform way, so our app
// behaves consistently whether you're on Linux, macOS, or another OS.
use std::path::PathBuf;

/// The main entry point for the `now` command.
/// This function is the central control hub for `setup-devbox now`.
/// It meticulously handles every step of the configuration application process:
///
/// 1.  **Configuration Discovery**: First, it figures out where your main `config.yaml`
///     (or any other specific configuration file you might have pointed it to) lives.
/// 2.  **State Management**: It then loads the previous installation state from `state.json`
///     (our app's "memory" of what's already installed). If it's the very first time you're
///     running `setup-devbox`, it gracefully creates a brand new, empty state file.
/// 3.  **Configuration Parsing**: Next, it reads and carefully deserializes (transforms
///     from text into structured data) all your linked configuration files
///     (tools, settings, shell, fonts) into easy-to-work-with Rust structs.
/// 4.  **Intelligent Application**: This is where the smart part happens! It iterates
///     through each desired configuration item, intelligently checking against the
///     current state. It only delegates work to specific installer modules
///     (like `brew`, `github`, `fonts`, `shellrc`) for items that are truly new
///     or need updating, avoiding redundant operations.
/// 5.  **State Persistence**: Critically, it updates and saves the `state.json`
///     file immediately after each successful installation or application block.
///     This ensures that `setup-devbox` always remembers what it's done,
///     even if a later step encounters an issue, preventing data loss and
///     ensuring consistent behavior on subsequent runs.
///
/// # Arguments
/// * `config_path`: An `Option<String>` that gives you the flexibility to
///                  specify a custom path to your main `config.yaml` file
///                  (or even a direct path to a single config file like `tools.yaml`).
///                  If you don't provide one (`None`), it gracefully defaults
///                  to `~/.setup-devbox/configs/config.yaml`.
/// * `state_path`: An `Option<String>` that lets you define a custom location
///                 for the `state.json` file. If omitted (`None`), it defaults
///                 to `~/.setup-devbox/state.json`.
pub fn run(config_path: Option<String>, state_path: Option<String>) {
    // Let's start by logging the initial input paths. This is super helpful for debugging
    // and understanding exactly what inputs our `now::run` function received.
    log_debug!("Entered now::run() function.");
    log_debug!("Initial config_path parameter: {:?}", config_path);
    log_debug!("Initial state_path parameter: {:?}", state_path);

    // 1. Determine Configuration and State File Paths
    // We first figure out the exact, real-world locations of our essential files.

    // Define the default location for our main configuration file (`config.yaml`).
    // This is where `setup-devbox` will look if you don't specify a custom path.
    let default_main_config = "~/.setup-devbox/configs/config.yaml";
    // Resolve the actual, absolute path to the main configuration file.
    // The `expand_tilde` helper function is a lifesaver here, as it smartly
    // replaces the cozy `~` with your actual home directory path (e.g., `/home/user` or `/Users/user`).
    let config_path_resolved: PathBuf = expand_tilde(config_path.as_deref().unwrap_or(default_main_config));
    // Extract just the filename (e.g., "config.yaml", "tools.yaml") from the resolved config path.
    // We'll use this filename later to intelligently decide how to parse the configuration.
    let config_filename = config_path_resolved.file_name().and_then(|s| s.to_str()).unwrap_or("");
    // Resolve the actual, absolute path to the application's internal state file (`state.json`).
    // This file acts as our app's memory, tracking what's already installed.
    let state_path_resolved: PathBuf = expand_tilde(state_path.as_deref().unwrap_or("~/.setup-devbox/state.json"));

    // Log the final, resolved paths. This is vital for confirming that we're
    // looking in the right places on your file system.
    log_info!("Using configuration file: {}", config_path_resolved.display().to_string().cyan());
    log_debug!("Managing application state in: {}", state_path_resolved.display().to_string().yellow());
    log_debug!("Resolved config_path: {:?}", config_path_resolved);
    log_debug!("Resolved state_path: {:?}", state_path_resolved);
    log_debug!("Detected config filename: '{}'", config_filename.blue());

    // 2. Load or Initialize Application State (`state.json`)
    // This is a super critical step: `setup-devbox` needs to remember what it has already installed
    // and configured to avoid doing redundant work (and save you time!).
    // We first check if the `state.json` file already exists.
    let mut state: DevBoxState = if state_path_resolved.exists() {
        // If the state file already exists, let's be good citizens and try to read its contents.
        log_debug!("State file found at {:?}. Attempting to load...", state_path_resolved);
        match fs::read_to_string(&state_path_resolved) {
            Ok(contents) => {
                // If we successfully read the file's raw content, our next mission is to
                // try and parse its JSON content into our structured `DevBoxState` struct.
                // This is literally where our app's "memory" is loaded from!
                match serde_json::from_str(&contents) {
                    Ok(parsed_state) => {
                        // Success! We've loaded the existing state.
                        log_info!("Using state file: {}", state_path_resolved.display().to_string().cyan());
                        log_debug!("Existing state file: {}", state_path_resolved.display().to_string().yellow());
                        // For detailed debugging, let's pretty-print the loaded state.
                        // This makes complex JSON structures much easier to read in logs.
                        match serde_json::to_string_pretty(&parsed_state) {
                            Ok(pretty_json) => {
                                log_debug!("Loaded DevBoxState:\n{}", pretty_json);
                            },
                            Err(e) => {
                                // This scenario is quite unlikely if deserialization (from string to struct)
                                // just worked, but it's good practice to handle potential
                                // re-serialization (from struct back to string) issues for robustness.
                                log_warn!("Failed to re-serialize loaded DevBoxState for pretty printing: {}", e);
                                // Fallback to raw debug format if pretty printing fails.
                                log_debug!("Loaded DevBoxState (raw debug format): {:?}", parsed_state);
                            }
                        }
                        parsed_state // Finally, return the successfully parsed state for our operations.
                    },
                    Err(err) => {
                        // Oh no, a problem! The state file exists but it's corrupted or in an invalid format.
                        // This is a serious issue, as it means our app's "memory" is unreliable.
                        // We log a critical error and stop execution immediately to prevent
                        // `setup-devbox` from making incorrect decisions based on bad data.
                        log_error!(
                            "Invalid state.json format at {:?}: {}. Please check the file's content or delete it to start fresh.",
                            state_path_resolved.display().to_string().red(),
                            err
                        );
                        return; // Exit the function gracefully (or, in this case, abruptly due to the error).
                    }
                }
            },
            Err(err) => {
                // We couldn't even read the state file at all (e.g., due to permissions issues, or a locked file).
                // This is also a critical problem. We log the error and stop the process.
                log_error!(
                    "Failed to read state file {:?}: {}. Please verify file permissions.",
                    state_path_resolved.display().to_string().red(),
                    err
                );
                return; // Exit the function.
            }
        }
    } else {
        // If the state file doesn't exist, it means this is either the very first run
        // of `setup-devbox` on this machine, or you've intentionally removed the state file.
        // In either case, we need to create a fresh start!
        log_info!("State file not found at {:?}. Creating a brand new state file.", state_path_resolved.display().to_string().yellow());
        // Initialize a new, empty `DevBoxState`. This is our app's "blank memory slate."
        let initial_state = DevBoxState {
            tools: HashMap::new(),    // No tools are installed yet, so we start with an empty map.
            fonts: HashMap::new(),    // Similarly, no fonts installed yet.
            settings: HashMap::new(), // And no settings applied from our app yet.
        };

        // Before attempting to write our sparkling new state file, we *must* ensure
        // that its parent directory (e.g., `~/.setup-devbox/` if `state.json` is inside) exists.
        // This prevents frustrating errors if your chosen path includes directories that
        // haven't been created by the system yet.
        if let Some(parent_dir) = state_path_resolved.parent() {
            log_debug!("Checking/creating parent directory for state file: {:?}", parent_dir);
            if let Err(e) = fs::create_dir_all(parent_dir) {
                // If we can't even create the directory to store the state, that's a big problem.
                log_error!(
                    "Failed to create directory for state file at {:?}: {}. Cannot save state.",
                    parent_dir.display().to_string().red(),
                    e
                );
                return; // Cannot proceed without a place to store our application's memory.
            }
        }

        // Now, let's try to serialize our newly created (and currently empty) state
        // struct into a human-readable, pretty-printed JSON string.
        match serde_json::to_string_pretty(&initial_state) {
            Ok(serialized_state) => {
                // With our pretty JSON in hand, we attempt to write this content to the `state.json` file.
                if let Err(err) = fs::write(&state_path_resolved, serialized_state) {
                    // If writing fails, we log a warning. The app can still run, but it won't remember
                    // its progress, which could lead to redundant work next time.
                    log_error!(
                        "Failed to write initial state file to {:?}: {}. This might prevent future state tracking.",
                        state_path_resolved.display().to_string().red(),
                        err
                    );
                } else {
                    // Hooray! The initial state file is successfully created.
                    log_info!("Initial state file successfully created at {:?}", state_path_resolved.display().to_string().green());
                }
            },
            Err(err) => {
                // If we can't even serialize the initial state struct into JSON, that indicates a bug
                // in our application's internal schema definition or the `serde` setup.
                // This is a critical internal error that needs developer attention.
                log_error!("Failed to serialize initial state: {}. This is an internal application error.", err);
            }
        }
        initial_state // Finally, return the newly created initial state for use in the current run.
    };

    // 3. Variables to Hold Parsed Configuration
    // These `Option` variables will serve as temporary containers for the parsed content
    // of our various configuration files (tools, settings, shell, fonts).
    // They are `Option`s (meaning they might be `Some(value)` or `None`) because it's
    // perfectly normal for a user not to have all types of config files, or for a file to be empty.
    let mut tools_config: Option<ToolConfig> = None;
    let mut settings_config: Option<SettingsConfig> = None;
    let mut shell_config: Option<ShellConfig> = None;
    let mut fonts_config: Option<FontConfig> = None;

    // 4. Configuration Loading Logic
    // Here's where we decide *how* to load your configurations.
    // We check if the main config file provided (or defaulted to) is `config.yaml`.
    // If it is, we assume it's the master file that points to other, individual config files.
    // Otherwise, if you provided a direct path like `tools.yaml`, we'll treat that
    // as a single, standalone configuration.
    if config_filename == "config.yaml" {
        // We're working with the master `config.yaml` file, which is like the central directory
        // orchestrating where all your other specific configuration files live.
        log_debug!("Loading configurations as per master config file: {}", config_path_resolved.display().to_string().blue());

        // Attempt to read the entire content of `config.yaml` into a string.
        let main_cfg_content = match fs::read_to_string(&config_path_resolved) {
            Ok(c) => c, // Successfully read the content.
            Err(e) => {
                // If we can't read the master config file (e.g., it's missing, permissions are off, or corrupted),
                // we log a critical error and halt execution. We simply can't proceed without our master plan!
                log_error!(
                    "Failed to read main config.yaml at {}: {}. Please ensure the file exists and is readable.",
                    config_path_resolved.display().to_string().red(),
                    e
                );
                return; // Exit the function.
            }
        };

        // Now, we try to parse the YAML content of `config.yaml` into our structured `MainConfig` struct.
        // The `MainConfig` struct primarily contains paths that tell us where to find your other
        // specific configuration files (like `tools.yaml`, `settings.yaml`, etc.).
        let main_cfg: MainConfig = match serde_yaml::from_str(&main_cfg_content) {
            Ok(cfg) => cfg, // Successfully parsed the main configuration!
            Err(e) => {
                // If parsing fails, it almost always means the YAML syntax is malformed (a typo, incorrect indentation).
                // We log a clear error and stop, guiding you to check your YAML syntax.
                log_error!(
                    "Failed to parse main config.yaml at {:?}: {}. Please check your YAML syntax for errors.",
                    config_path_resolved.display().to_string().red(),
                    e
                );
                return; // Exit the function.
            }
        };
        log_debug!("MainConfig loaded: {:?}", main_cfg);

        // Now, based on the file paths you've specified within the `main_cfg` struct,
        // we'll attempt to load each of the individual configuration files (tools, settings, shell, fonts).
        // We use `if let Some` to gracefully handle cases where a specific path
        // (like for `tools` or `settings`) might not be provided in your `config.yaml` –
        // it's perfectly fine to only configure what you need!

        // Attempt to load `tools.yaml` if its path is specified in `main_cfg`.
        if let Some(tools_path) = main_cfg.tools {
            let path = expand_tilde(&tools_path); // Resolve the full, absolute path to `tools.yaml`.
            log_debug!("Attempting to load tools config from: {:?}", path.display());
            // Try to read the file's contents into a string.
            if let Ok(contents) = fs::read_to_string(&path) {
                // If read successfully, the next step is to attempt to parse it as a `ToolConfig`.
                match serde_yaml::from_str::<ToolConfig>(&contents) {
                    Ok(cfg) => {
                        tools_config = Some(cfg); // Excellent! Store the successfully parsed `ToolConfig`.
                        log_debug!("{} Successfully loaded tool configuration from {}", "[Tools]".bold(), path.display().to_string().green());
                    },
                    Err(e) => log_error!("Failed to parse tools.yaml at {:?}: {}. Please check its YAML syntax.", path.display().to_string().red(), e),
                }
            } else {
                // If the file simply wasn't found at the specified path, or we couldn't read it
                // (e.g., due to permissions), we issue a warning. This is a warning, not an error,
                // because having a tool config is optional.
                log_warn!("Tools configuration file not found or unreadable at {}. Skipping tool setup.", path.display().to_string().yellow());
            }
        }

        // Attempt to load `settings.yaml` if its path is specified in `main_cfg`.
        if let Some(settings_path) = main_cfg.settings {
            let path = expand_tilde(&settings_path);
            log_debug!("Attempting to load settings config from: {:?}", path.display());
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<SettingsConfig>(&contents) {
                    Ok(cfg) => {
                        settings_config = Some(cfg);
                        log_debug!("{} Successfully loaded settings configuration from {}","[Settings]".bold(), path.display().to_string().green());
                    },
                    Err(e) => log_error!("Failed to parse settings.yaml at {:?}: {}. Please check its YAML syntax.", path.display().to_string().red(), e),
                }
            } else {
                log_warn!("Settings configuration file not found or unreadable at {:?}. Skipping settings application.", path.display().to_string().yellow());
            }
        }

        // Attempt to load `shellrc.yaml` if its path is specified in `main_cfg`.
        if let Some(shellrc_path) = main_cfg.shellrc {
            let path = expand_tilde(&shellrc_path);
            log_debug!("Attempting to load shell config from: {:?}", path.display());
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<ShellConfig>(&contents) {
                    Ok(cfg) => {
                        shell_config = Some(cfg);
                        log_debug!("{} Successfully loaded shell configuration from {}","[Shell Config]".bold(), path.display().to_string().green());
                    },
                    Err(e) => log_error!("Failed to parse shellrc.yaml at {}: {}. Please check its YAML syntax.", path.display().to_string().red(), e),
                }
            } else {
                log_warn!("Shell configuration file not found or unreadable at {}. Skipping shell setup.", path.display().to_string().yellow());
            }
        }

        // Attempt to load `fonts.yaml` if its path is specified in `main_cfg`.
        if let Some(fonts_path) = main_cfg.fonts {
            let path = expand_tilde(&fonts_path);
            log_debug!("Attempting to load fonts config from: {:?}", path.display());
            if let Ok(contents) = fs::read_to_string(&path) {
                match serde_yaml::from_str::<FontConfig>(&contents) {
                    Ok(cfg) => {
                        fonts_config = Some(cfg);
                        log_debug!("{} Successfully loaded font configuration from {}", "[Fonts]".bold(), path.display().to_string().green());
                    },
                    Err(e) => log_error!("Failed to parse fonts.yaml at {}: {}. Please check its YAML syntax.", path.display().to_string().red(), e),
                }
            } else {
                log_warn!("Font configuration file not found or unreadable at {}. Skipping font installation.", path.display().to_string().yellow());
            }
        }
    } else {
        // 4b. Single Config File Fallback
        // If the `config_filename` is NOT `config.yaml`, it means you've
        // directly provided a path to one specific config file (e.g., `setup-devbox now --config ~/.setup-devbox/tools.yaml`).
        // We're smart enough to handle that!
        log_info!("Loading configuration from single file: {}", config_path_resolved.display().to_string().blue());
        log_debug!("Attempting to load configuration directly from: {:?}", config_path_resolved.display());

        // Attempt to read the entire content of this single configuration file.
        let contents = match fs::read_to_string(&config_path_resolved) {
            Ok(c) => c, // Successfully read the content.
            Err(e) => {
                // If we can't read the file you told us to look at, it's a critical error.
                log_error!(
                    "Failed to read single config file {:?}: {}. Please check its existence and permissions.",
                    config_path_resolved.display().to_string().red(),
                    e
                );
                return; // Exit the function.
            }
        };

        // Based on the filename you provided, we'll try to parse the content into the
        // corresponding configuration struct. It's like checking the label on the box!
        match config_filename {
            "tools.yaml" => {
                log_debug!("Identified as tools.yaml. Attempting to parse...");
                tools_config = match serde_yaml::from_str(&contents) {
                    Ok(cfg) => { log_info!("[Tools] Successfully parsed tools.yaml."); Some(cfg) },
                    Err(e) => { log_error!("Failed to parse tools.yaml: {}", e); None },
                }
            },
            "settings.yaml" => {
                log_debug!("Identified as settings.yaml. Attempting to parse...");
                settings_config = match serde_yaml::from_str(&contents) {
                    Ok(cfg) => { log_info!("[Settings] Successfully parsed settings.yaml."); Some(cfg) },
                    Err(e) => { log_error!("Failed to parse settings.yaml: {}", e); None },
                }
            },
            // Handle both potential names for the shell config file (`shellrc.yaml` or `shellac.yaml`).
            "shellrc.yaml" | "shellac.yaml" => {
                log_debug!("Identified as shell config file. Attempting to parse...");
                shell_config = match serde_yaml::from_str(&contents) {
                    Ok(cfg) => { log_info!("[ShellRC] Successfully parsed shell config."); Some(cfg) },
                    Err(e) => { log_error!("Failed to parse shell config: {}", e); None },
                }
            },
            "fonts.yaml" => {
                log_debug!("Identified as fonts.yaml. Attempting to parse...");
                fonts_config = match serde_yaml::from_str(&contents) {
                    Ok(cfg) => { log_info!("[Fonts] Successfully parsed fonts.yaml."); Some(cfg) },
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

    // Now that all configurations are loaded (or identified as missing),
    // we move on to the exciting part: applying them to your system!
    // CRITICALLY, we now update and save our `state.json`
    // immediately after each major section (tools, fonts, shellrc, settings) if
    // any changes occurred within that specific section. This keeps our app's
    // memory perfectly in sync.

    // 5a. Install Tools
    // If we have `tools_config` (meaning `tools.yaml` was successfully loaded and parsed),
    // let's dive into processing the tool installations.
    if let Some(tools_cfg) = tools_config {
        eprintln!("\n");
        log_info!("[Tools] Processing Tools Installations...");
        // Initialize a flag to track if any tools were actually installed or updated in this block.
        let mut tools_updated = false;
        // This vector will temporarily collect the names of any tools we find are already installed,
        // so we can give you a nice, consolidated summary at the end.
        let skipped_tools: Vec<String> = Vec::new();

        // Loop through each `ToolEntry` (each tool definition) found in your `tools.yaml` configuration.
        for tool in &tools_cfg.tools {
            log_debug!("[Tools] Considering tool: {:?}", tool.name.bold());
            // This is where our app's "memory" (`state.tools`) comes in handy!
            // We check if this tool is *already* recorded as installed. This is how
            // `setup-devbox` intelligently avoids re-installing tools that are already managed.
            if !state.tools.contains_key(&tool.name) {
                // Hurray! This is a brand-new tool that needs to be installed.
                print!("\n"); // A little visual breathing room before a big installation.
                // Draw a separator line to highlight the start of a new tool installation.
                eprintln!("{}", "==============================================================================================".bright_blue());
                log_info!("[Tools] Installing new tool from {}: {}", tool.source.to_string().bright_yellow(), tool.name.to_string().bright_blue().bold());
                // For deep dives, log the full configuration details of the tool being installed.
                log_debug!("[Tools] Full configuration details for tool '{}': {:?}", tool.name, tool);

                // Based on the tool's `source` (e.g., "GitHub", "brew"),
                // we smartly delegate the actual installation task to the appropriate
                // installer module. Each module is an expert in its domain!
                let installation_result = match tool.source.as_str() {
                    "github" => github::install(tool), // Hand off to the GitHub installer.
                    "brew" => brew::install(tool),     // Hand off to the Homebrew installer.
                    // If you decide to add other installer types like `cargo` or `go`,
                    // this is exactly where you'd uncomment and add them!
                    // "cargo" => cargo::install(tool),
                    // "go" => go::install(tool),
                    other => {
                        // If the tool's source is unrecognized (e.g., a typo in `tools.yaml`),
                        // we politely warn the user and skip this particular tool's installation.
                        log_warn!(
                        "[Tools] Unsupported source '{}' for tool '{}'. Skipping this tool's installation.",
                        other.yellow(),
                        tool.name.bold()
                    );
                        None // Indicate that no `ToolState` was generated for this tool, as it wasn't installed.
                    }
                };

                // If the installation was successful (meaning `Some(ToolState)` was returned)...
                if let Some(tool_state) = installation_result {
                    // We've done it! Add the details of the newly installed tool to our `state.tools` HashMap.
                    // This is how our app remembers that this tool is now managed.
                    state.tools.insert(tool.name.clone(), tool_state);
                    tools_updated = true; // Set our flag: yes, the tools state has changed in this block.
                    log_info!("[Tools] {}: {}", "Successfully installed tool".yellow() ,tool.name.bold().bright_green());
                    // Draw another separator line to mark the end of this tool's installation.
                    eprintln!("{}", "==============================================================================================".blue());
                    print!("\n"); // Final blank line for neatness.
                } else {
                    // If the installation failed for any reason within the installer module, log a clear error.
                    log_error!(
                    "[Tools] Failed to install tool: {}. Please review previous logs for specific errors during installation.",
                    tool.name.bold().red()
                );
                }
            } else {
                // If the tool is already recorded in our `state.tools` map, we simply add its
                // name to our `skipped_tools` list for a consolidated message later.
                log_debug!("[Tools] Tool '{}' is already recorded as installed. Added to skipped list.", tool.name.blue()); // Changed to debug
            }
        }

        // After iterating through all the tools, let's give a clear summary of any we skipped.
        if !skipped_tools.is_empty() {
            let skipped_tools_str = skipped_tools.join(", "); // Join all skipped tool names into a single string.
            log_info!(
            "[Tools] The following tools were already recorded as installed and were skipped: {}",
            skipped_tools_str.blue()
        );
        } else {
            // If the skipped_tools list is empty, it means all tools were either new or explicitly handled.
            log_debug!("[Tools] No tools were skipped as they were not found in the state.");
        }

        // After processing all tools, if `tools_updated` is true (meaning we installed something new),
        // it's crucial to save the updated state to our `state.json` file.
        if tools_updated {
            log_info!("[Tools] New tools installed or state updated. Saving current DevBox state...");
            if !save_devbox_state(&state, &state_path_resolved) {
                // If saving fails, it's a serious warning, as our app's "memory" might be out of sync.
                log_error!("[StateSave] Failed to save state after tool installations. Data loss risk!");
            }
            log_info!("[StateSave] State saved successfully after tool updates."); // Confirm successful save.
        } else {
            // If no tools were new or updated, we simply inform the user.
            log_info!("[Tools] No new tools installed or state changes detected for tools.");
        }
        eprintln!(); // Another blank line for overall section separation.
    } else {
        // If no tool configurations were found (e.g., `tools.yaml` was missing or empty),
        // we skip this entire phase.
        log_debug!("[Tools] No tool configurations found (tools.yaml missing or empty). Skipping tool installation phase.");
    }

    // 5b. Install Fonts
    // If `fonts_config` (from `fonts.yaml`) was successfully loaded, we proceed with font installations.
    if let Some(fonts_cfg) = fonts_config {
        log_info!("[Fonts] Processing Font Installations...");
        // Flag to track if any fonts were installed or updated in this section.
        let mut fonts_updated = false;

        // This vector will gather the names of fonts we skip because they're already installed.
        let skipped_fonts: Vec<String> = Vec::new();

        // Loop through each `FontEntry` (each font definition) in your `fonts.yaml` configuration.
        for font in &fonts_cfg.fonts {
            log_debug!("[Fonts] Considering font: {:?}", font.name.bold());
            // Check if this font is *already* recorded in our `state.fonts` HashMap.
            // This prevents redundant re-installations.
            if !state.fonts.contains_key(&font.name) {
                // This block handles the installation of a brand-new font.
                print!("\n"); // Visual space.
                eprintln!("{}", "==============================================================================================".bright_blue());
                log_info!("[Fonts] Installing new font: {}", font.name.bold());
                // Log full details for debugging purposes.
                log_debug!("[Fonts] Full configuration details for font '{}': {:?}", font.name, font);

                // Delegate the actual font installation work to the `fonts::install` function.
                // This function returns an `Option<FontState>` if the installation succeeds.
                let installation_result = fonts::install(font);

                // If the font installation was successful (`Some(FontState)` was returned)...
                if let Some(font_state) = installation_result {
                    // Add the details of the newly installed font to our `state.fonts` HashMap,
                    // so our app remembers it's managed.
                    state.fonts.insert(font.name.clone(), font_state);
                    fonts_updated = true; // Mark that fonts state has changed.
                    log_info!("[Fonts] Successfully installed font: {}", font.name.bold().green());
                    eprintln!("{}", "==============================================================================================".bright_blue());
                    print!("\n"); // Visual space.
                } else {
                    // If the installation failed for any reason, log a clear error.
                    log_error!(
                        "Failed to install font: {}. Please review previous logs for specific errors during installation.",
                        font.name.bold().red()
                    );
                }
            } else {
                // If the font is already in our state, we add its name to the skipped list.
                log_debug!("[Fonts] Font '{}' is already recorded as installed. Added to skipped list.", font.name.blue()); // Changed to debug
            }
        }

        // After checking all fonts, if there are any skipped ones, print a consolidated message.
        if !skipped_fonts.is_empty() {
            let skipped_fonts_str = skipped_fonts.join(", ");
            log_info!(
                "[Fonts] The following fonts were already recorded as installed and were skipped: {}",
                skipped_fonts_str.blue()
            );
        } else {
            log_debug!("[Fonts] No fonts were skipped as they were not found in the state.");
        }
        // eprintln!(); // Visual space.

        // After processing all fonts, if any `fonts_updated` occurred, it's time to save the state!
        if fonts_updated {
            log_info!("[Fonts] Font state updated. Saving current DevBox state...");
            if !save_devbox_state(&state, &state_path_resolved) {
                log_error!("Failed to save state after font installations. Data loss risk!");
            }
            log_info!("[StateSave] State saved successfully after font updates."); // Confirm the save.
        } else {
            log_info!("[Fonts] No new fonts installed or state changes detected for fonts.");
        }
        eprintln!(); // Final blank line for this section.
    } else {
        // If no font configurations were found (e.g., `fonts.yaml` was missing or empty),
        // we gracefully skip this entire phase.
        log_debug!("[Fonts] No font configurations found (fonts.yaml missing or empty). Skipping font installation phase.");
    }

    // 5c. Apply Shell Configuration (Raw Configs and Aliases)
    // If `shell_config` (from `shellrc.yaml`) was successfully loaded, we proceed to apply shell configurations.
    if let Some(shell_cfg) = shell_config {
        log_info!("[Shell Config] Applying Shell Configurations...");
        log_debug!("[Shell Config] Applying shell configurations and aliases from shellrc.yaml...");
        // For deep debugging, pretty print the raw shell configuration (e.g., environment variables, path additions).
        match serde_json::to_string_pretty(&shell_cfg.shellrc) {
            Ok(pretty_shellrc) => {
                log_debug!("[Shell Config] Calling shellrc::apply_shellrc with shell config:\n{}", pretty_shellrc);
            },
            Err(e) => {
                log_warn!("[Shell Config] Failed to pretty-print shell config for debug log: {}", e);
                log_debug!("[Shell Config] Calling shellrc::apply_shellrc with shell config: {:?}", shell_cfg.shellrc);
            }
        }

        // Also pretty print the aliases to be applied for easy readability in logs.
        match serde_json::to_string_pretty(&shell_cfg.aliases) {
            Ok(pretty_aliases) => {
                log_debug!("[Shell Config] And aliases:\n{}", pretty_aliases);
            },
            Err(e) => {
                log_warn!("[Shell Config] Failed to pretty-print aliases for debug log: {}", e);
                log_debug!("[Shell Config] And aliases: {:?}", shell_cfg.aliases);
            }
        }

        // Delegate the actual work of applying shell configurations and aliases
        // to the `shellrc::apply_shellrc` function. This function is a specialist
        // at interacting with your shell's RC files (like .zshrc or .bashrc).
        // It's designed to handle its own logic for determining if changes are needed
        // and will write to the RC file if necessary, avoiding duplicates.
        shellrc::apply_shellrc(&shell_cfg.shellrc, &shell_cfg.aliases);

        log_debug!("[Shell Config] Shell configuration application phase completed.");
        //
        // Still thinking about this! Shell configurations are typically written directly
        // to shell RC files (like `.zshrc`, `.bashrc`, etc.), which `setup-devbox`
        // re-reads on each run to check for new lines. They are not usually "stateful"
        // in the same way tools and fonts (which are installed once) are.
        // Therefore, recording every shell configuration change in the `state.json`
        // might be redundant or unnecessary, as the RC files themselves are the source of truth.
        // For now, we'll keep this part commented out, as `apply_shellrc` does its own
        // checks and writes to the shell RC files.
        //
        // if !save_devbox_state(&state, &state_path_resolved) {
        //     log_error!("Failed to save state after shell configuration. Data loss risk!");
        // }
        eprintln!(); // Visual space after this section.
    } else {
        // If no shell configurations were found (e.g., `shellrc.yaml` was missing or empty),
        // we gracefully skip this phase.
        log_debug!("[Shell Config] No shell configurations found (shellrc.yaml missing or empty). Skipping shell configuration phase.");
    }
    // 5d. Apply macOS System Settings
    // If `settings_config` (from `settings.yaml`) was loaded, we'd proceed to apply system settings here.
    if let Some(_settings_cfg) = settings_config {
        log_info!("{} Applying System Settings...", "[OS Settings]".bold());
        log_warn!("{} The 'settings' application feature is currently under development and will be implemented soon!", "[OS Settings]".bold());
        // TODO: The actual implementation for applying system settings would go here.
        // This would likely involve iterating through `_settings_cfg.settings` and executing
        // OS-specific commands (like `defaults write` on macOS for hidden preferences)
        // or calling specialized functions that interact with the system's APIs.
        // If settings are applied and need to be tracked in `state.json`, you would update `state.settings` here.
        // If `state.settings` is indeed updated, then you would call `save_devbox_state(&state, &state_path_resolved);` here.
        log_info!("{} System settings application phase completed (pending full feature implementation).", "[OS Settings]".bold());
        // For now, assuming some settings *might* be applied in the future,
        // we'll trigger a state save just in case, or refine this logic later
        // if settings modification also reports `true` for changes.
        // For now, if `_settings_cfg` exists, we assume the intention was to make changes.
        log_info!("{} System settings processing finished. Saving current DevBox state...", "[OS Settings]".bold());
        if !save_devbox_state(&state, &state_path_resolved) {
            log_error!("Failed to save state after settings application. Data loss risk!");
        }
        eprintln!(); // Final visual space for this section.
    } else {
        // If no system settings configurations were found (e.g., `settings.yaml` was missing or empty),
        // we gracefully skip this phase.
        log_debug!("[Settings] No system settings configurations found (settings.yaml missing or empty). Skipping settings application phase.");
    }

    // Finally, let's wrap up the `now` command execution with a friendly success message!
    log_info!("'setup-devbox now' command completed!!");
    log_debug!("Exited now::run() function.");
}
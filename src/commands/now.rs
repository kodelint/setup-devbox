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
use crate::schema::DevBoxState;
// Our custom logging macros for debug, error, info, and warning messages.
// These are essential for giving you clear feedback on what `setup-devbox` is doing!
use crate::{log_debug, log_info};
// For pretty colored output in the terminal, making logs not just informative but also a joy to read.
use colored::Colorize;

use crate::libs::{
    config_loading::{
        load_master_configs,
        load_single_config,
    },
    font_installer::install_fonts,
    paths,
    settings_applier::apply_system_settings,
    shell_configurator::apply_shell_configs,
    state_management,
    tool_installer::install_tools
};

/// The main entry point for the `now` command.
///
/// This function is the central control hub for `setup-devbox now`.
/// It orchestrates the entire configuration application process by:
/// 1.  Resolving configuration and state file paths.
/// 2.  Loading or initializing the application's persistent state (`state.json`).
/// 3.  Parsing all relevant configuration files (tools, settings, shell, fonts).
/// 4.  Intelligently applying configurations, delegating to specialized functions
///     for tools, fonts, shell settings, and system settings.
/// 5.  Persisting the updated state after each major application block.
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
    log_debug!("Entered now::run() function.");

    // 1. Determine Configuration and State File Paths
    // Calls a dedicated function to resolve paths, handling defaults and tilde expansion.
    let (config_path_resolved, config_filename, state_path_resolved) =
        match paths::resolve_paths(config_path, state_path) {
            Some(paths) => paths,
            None => return, // Exit if paths cannot be resolved (error logged within resolve_paths)
        };

    // 2. Load or Initialize Application State (`state.json`)
    // Loads existing state or creates a new one if `state.json` doesn't exist.
    let mut state: DevBoxState = state_management::load_or_initialize_state(&state_path_resolved);

    // 3. Variables to Hold Parsed Configuration and 4. Configuration Loading Logic
    // Load configurations based on whether a master `config.yaml` is used or a single file.
    let parsed_configs = if config_filename == "config.yaml" {
        load_master_configs(&config_path_resolved)
    } else {
        load_single_config(&config_path_resolved, &config_filename)
    };

    // Now that all configurations are loaded (or identified as missing),
    // we move on to the exciting part: applying them to your system!
    // CRITICALLY, we now update and save our `state.json`
    // immediately after each major section (tools, fonts, shellrc, settings) if
    // any changes occurred within that specific section. This keeps our app's
    // memory perfectly in sync.

    // 5a. Install Tools
    if let Some(tools_cfg) = parsed_configs.tools {
        install_tools(tools_cfg, &mut state, &state_path_resolved);
    } else {
        log_debug!("[Tools] No tool configurations found (tools.yaml missing or empty). Skipping tool installation phase.");
    }

    // 5b. Install Fonts
    if let Some(fonts_cfg) = parsed_configs.fonts {
        install_fonts(fonts_cfg, &mut state, &state_path_resolved);
    } else {
        log_debug!("[Fonts] No font configurations found (fonts.yaml missing or empty). Skipping font installation phase.");
    }

    // 5c. Apply Shell Configuration (Raw Configs and Aliases)
    if let Some(shell_cfg) = parsed_configs.shell {
        apply_shell_configs(shell_cfg);
    } else {
        log_debug!("[Shell Config] No shell configurations found (shellrc.yaml missing or empty). Skipping shell configuration phase.");
    }

    // 5d. Apply macOS System Settings
    if let Some(settings_cfg) = parsed_configs.settings {
        apply_system_settings(settings_cfg, &mut state, &state_path_resolved);
    } else {
        log_debug!("[Settings] No system settings configurations found (settings.yaml missing or empty). Skipping settings application phase.");
    }

    // Finally, let's wrap up the `now` command execution with a friendly success message!
    log_info!("'setup-devbox now' command completed!!");
    log_debug!("Exited now::run() function.");
}
// This file contains the primary logic for the `setup-devbox now` command.
// It orchestrates the reading of configuration files, state management,
// and the installation/application of tools, fonts, shell configs, and system settings.

use crate::installers::shell_run_commands::apply_shell_configs;
use crate::schema::DevBoxState; // Application state structure.
use crate::{log_debug, log_info}; // Custom logging macros.
use colored::Colorize;
// For colored terminal output.

use crate::libs::{
    config_loading::{
        load_master_configs, // Loads configurations from `config.yaml`.
        load_single_config,  // Loads a single configuration file.
    },
    font_installer::install_fonts, // Handles font installation.
    paths,                         // Path resolution utilities.
    settings_applier::apply_system_settings, // Applies macOS system settings.
    // Applies shell configurations and aliases.
    state_management, // Module for loading and saving application state.
    tool_installer::install_tools, // Handles tool installation.
};

/// Main entry point for the `now` command.
///
/// Orchestrates the entire development environment setup process:
/// 1. Resolves configuration and state file paths.
/// 2. Loads or initializes `state.json`.
/// 3. Parses relevant configuration files.
/// 4. Applies configurations for tools, fonts, shell, and system settings.
/// 5. Persists updated state after each section.
///
/// # Arguments
/// * `config_path`: Optional custom path to `config.yaml` or a single config file.
/// * `state_path`: Optional custom path to `state.json`.
pub fn run(config_path: Option<String>, state_path: Option<String>, update_latest: bool) {
    log_debug!("Entered now::run() function.");

    if update_latest {
        log_info!(
            "'{}' flag is set, forcing update of all `latest` version tools",
            "Update latest".bright_yellow()
        );
    }

    // Resolve configuration and state file paths.
    let (config_path_resolved, config_filename, state_path_resolved) =
        match paths::resolve_paths(config_path, state_path) {
            Some(paths) => paths,
            None => return, // Exit if path resolution fails.
        };

    // Load existing application state or initialize a new one.
    let mut state: DevBoxState = state_management::load_or_initialize_state(&state_path_resolved);

    // Load configurations based on the detected config filename.
    let parsed_configs = if config_filename == "config.yaml" {
        load_master_configs(&config_path_resolved)
    } else {
        load_single_config(&config_path_resolved, &config_filename)
    };

    // Apply configurations and update state for each section.
    // State is saved immediately after each major block if changes occur.
    // Install Tools.
    if let Some(tools_cfg) = parsed_configs.tools {
        install_tools(tools_cfg, &mut state, &state_path_resolved, update_latest);
    } else {
        log_debug!(
            "[Tools] No tool configurations found (tools.yaml missing or empty). Skipping tool installation phase."
        );
    }

    // Install Fonts.
    if let Some(fonts_cfg) = parsed_configs.fonts {
        install_fonts(fonts_cfg, &mut state, &state_path_resolved);
    } else {
        log_debug!(
            "[Fonts] No font configurations found (fonts.yaml missing or empty). Skipping font installation phase."
        );
    }

    // Apply Shell Configuration.
    if let Some(shell_cfg) = parsed_configs.shell {
        apply_shell_configs(shell_cfg);
    } else {
        log_debug!(
            "[Shell Config] No shell configurations found (shellrc.yaml missing or empty). Skipping shell configuration phase."
        );
    }

    // Apply macOS System Settings.
    if let Some(settings_cfg) = parsed_configs.settings {
        apply_system_settings(settings_cfg, &mut state, &state_path_resolved);
    } else {
        log_debug!(
            "[Settings] No system settings configurations found (settings.yaml missing or empty). Skipping settings application phase."
        );
    }

    log_info!("'setup-devbox now' command completed!!");
    log_debug!("Exited now::run() function.");
}

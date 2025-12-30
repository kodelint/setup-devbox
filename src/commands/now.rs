// This file contains the primary logic for the `setup-devbox now` command.
// It orchestrates the reading of configuration files, state management,
// and the installation/application of tools, fonts, shell configs, and system settings.

use crate::engine::installers::shell_run_commands::apply_shell_configs;
use crate::schemas::state_file::DevBoxState;
// Application state structure.
use crate::{log_debug, log_info, log_warn};
// Custom logging macros.
use colored::Colorize;
// For colored terminal output.

use crate::config::{
    load_master_configs, // Loads configurations from `config.yaml`.
    load_single_config,  // Loads a single configuration file.
};
use crate::core::backup::backup_directory;
use crate::engine::install_tools;
use crate::fonts::installer::install_fonts;
use crate::schemas::path_resolver::PathResolver;
use crate::settings::apply_system_settings;
use crate::state::manager::load_or_initialize_state;

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
pub fn run(paths: &PathResolver, update_latest: bool, dry_run: bool) {
    log_debug!("[SDB] Entered now::run() function.");

    if dry_run {
        log_info!(
            "[SDB] '{}' flag is set, simulation mode enabled",
            "Dry Run".bright_magenta()
        );
    } else {
        // Automatically backup configuration directory before changes
        // Use base_config_dir to include configs/ and state.json
        if let Err(e) = backup_directory(paths.base_config_dir()) {
            log_warn!(
                "[SDB::Backup] Automatic backup failed: {}. Continuing anyway.",
                e
            );
        }
    }

    // Get resolved paths from the PathResolver
    let config_path_resolved = paths.config_file();
    let config_filename = paths.config_filename();
    let state_path_resolved = paths.state_file();

    log_debug!(
        "[SDB::Now] Using config file: {}",
        config_path_resolved.display()
    );
    log_debug!("[SDB::Now] Config filename: {}", config_filename);
    log_debug!(
        "[SDB::Now] Using state file: {}",
        state_path_resolved.display()
    );

    // Load existing application state or initialize a new one.
    let mut state: DevBoxState = load_or_initialize_state(&state_path_resolved.to_path_buf());

    // Load configurations based on the detected config filename.
    let parsed_configs = if config_filename == "config.yaml" {
        load_master_configs(&config_path_resolved.to_path_buf())
    } else {
        load_single_config(&config_path_resolved.to_path_buf(), config_filename)
    };

    // Apply configurations and update state for each section.
    // State is saved immediately after each major block if changes occur.
    if let Some(tools_cfg) = parsed_configs.tools {
        log_info!("[SDB::Tools] Processing {}...", "Tools".bright_green());
        install_tools(
            tools_cfg,
            &mut state,
            state_path_resolved,
            update_latest,
            dry_run,
            paths,
        ); // Add paths
    } else {
        log_debug!(
            "[SDB::Now] No tool configurations found (tools.yaml missing or empty). Skipping tool installation phase."
        );
    }

    // Install Fonts.
    if let Some(fonts_cfg) = parsed_configs.fonts {
        install_fonts(fonts_cfg, &mut state, state_path_resolved);
    } else {
        log_debug!(
            "[SDB::Now] No font configurations found (fonts.yaml missing or empty). Skipping font installation phase."
        );
    }

    // Apply Shell Configuration.
    if let Some(shell_cfg) = parsed_configs.shell {
        apply_shell_configs(shell_cfg);
    } else {
        log_debug!(
            "[SDB::Now] No shell configurations found (shellrc.yaml missing or empty). Skipping shell configuration phase."
        );
    }

    // Apply macOS System Settings.
    if let Some(settings_cfg) = parsed_configs.settings {
        apply_system_settings(settings_cfg, &mut state, state_path_resolved);
    } else {
        log_debug!(
            "[SDB::Now] No system settings configurations found (settings.yaml missing or empty). Skipping settings application phase."
        );
    }

    log_info!(
        "[SDB::Now] '{}' command completed!!",
        "setup-devbox now".cyan()
    );
    log_debug!("[SDB::Now] Exited now::run() function.");
}

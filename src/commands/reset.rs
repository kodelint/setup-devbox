//! # Reset Command Implementation
//!
//! This module provides the logic for resetting the application state.
//! It allows users to wipe the entire state or specific tool entries,
//! forcing the next 'now' run to treat them as new installations.

use crate::core::backup::backup_directory;
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::state_file::DevBoxState;
use crate::state::manager::{load_or_initialize_state, save_state_to_file};
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use std::path::Path;

/// Entry point for the 'reset' subcommand
pub fn run(tool: Option<String>, all: bool, state_path: Option<String>) {
    log_debug!("[SDB::Reset] Entering reset::run()");

    // 1. Resolve state file path
    let paths = match PathResolver::new(None, state_path) {
        Ok(p) => p,
        Err(e) => {
            log_error!("Failed to resolve paths: {}", e);
            return;
        }
    };

    // Automatically backup before reset
    if let Err(e) = backup_directory(paths.base_config_dir()) {
        log_warn!(
            "[SDB::Backup] Automatic backup failed: {}. Continuing anyway.",
            e
        );
    }

    let state_file = paths.state_file().to_path_buf();

    if !state_file.exists() {
        log_warn!(
            "State file not found at: {}. Nothing to reset.",
            state_file.display()
        );
        return;
    }

    // 2. Load current state
    let mut state: DevBoxState = load_or_initialize_state(&state_file);

    // 3. Perform reset based on arguments
    if all {
        reset_all(&mut state, &state_file);
    } else if let Some(tool_name) = tool {
        reset_tool(&mut state, &tool_name, &state_file);
    } else {
        log_error!("Please specify either --all or --tool <name> to reset.");
    }
}

/// Wipes all entries from the state file
fn reset_all(state: &mut DevBoxState, path: &Path) {
    log_info!("[SDB::Reset] Resetting entire state...");

    state.tools.clear();
    state.fonts.clear();
    state.settings.clear();
    // We might want to keep some metadata if any, but currently these are the main collections

    save_state_to_file(state, path);
    log_info!(
        "[SDB::Reset] {} State file successfully wiped.",
        "Success:".green().bold()
    );
}

/// Removes a specific tool from the state file
fn reset_tool(state: &mut DevBoxState, tool_name: &str, path: &Path) {
    log_info!(
        "[SDB::Reset] Resetting tool '{}' in state...",
        tool_name.cyan()
    );

    if state.tools.remove(tool_name).is_some() {
        save_state_to_file(state, path);
        log_info!(
            "[SDB::Reset] {} Tool '{}' removed from state.",
            "Success:".green().bold(),
            tool_name.cyan()
        );
    } else {
        log_warn!(
            "[SDB::Reset] Tool '{}' not found in state.",
            tool_name.yellow()
        );
    }
}

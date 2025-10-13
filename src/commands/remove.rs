//! # Remove Command Implementation
//!
//! This module handles the removal of tools, fonts, aliases, and settings from the system.
//! It provides a clean, orchestrated approach to uninstalling resources and cleaning up
//! their associated configuration files and state entries.
//!
//! ## Architecture
//!
//! The module is organized around three main components:
//!
//! 1. **Tool Uninstallers**: Strategy pattern implementations for different installation methods
//!    (cargo, brew, pip, etc.). Each uninstaller knows how to remove tools installed via its method.
//!
//! 2. **Configuration Cleaner**: Handles all YAML configuration file manipulation, including
//!    removing entries from tools.yaml, fonts.yaml, shellrc.yaml, and settings.yaml.
//!
//! 3. **Removal Orchestrator**: Coordinates the entire removal process, calling uninstallers,
//!    cleaning configurations, and updating state files.
//!
//! ## Key Features
//!
//! - Supports multiple installation methods (cargo, pip, brew, go, rustup, uv, github, url)
//! - Handles tool aliases (tools can be referenced by their original name or renamed alias)
//! - Cleans up configuration files managed by the configuration manager
//! - Provides detailed logging and user-friendly summary output
//! - Gracefully handles missing files and partial failures
//!
//! ## Usage
//!
//! ```rust
//! // Remove a tool
//! remove_tool("git".to_string());
//!
//! // Remove a font
//! remove_font("JetBrainsMono".to_string());
//!
//! // Remove an alias
//! remove_alias("ll".to_string());
//!
//! // Remove a setting
//! remove_setting("com.apple.dock".to_string(), "autohide".to_string());
//! ```

use crate::libs::removal_orchestrator::RemovalOrchestrator;
use crate::libs::state_management::{load_or_initialize_state, save_state_to_file};
use crate::libs::uninstallers::{ConfigurationCleaner, RemovalResult, RemovalSummary};
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::state_file::DevBoxState;
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use std::path::PathBuf;

// ============================================================================
//                          INITIALIZATION HELPERS
// ============================================================================

/// Initializes core components needed for removal operations.
///
/// This function handles the common initialization pattern used by all
/// removal operations:
/// 1. Initialize PathResolver to locate system directories
/// 2. Determine the state file path
/// 3. Load or initialize the state file
///
/// # Returns
///
/// * `Ok((PathResolver, PathBuf, DevBoxState))` - Initialized components
/// * `Err(String)` - If initialization failed
///
/// # Error Handling
///
/// This function converts all errors to formatted strings with color coding
/// for consistent error reporting throughout the removal system.
fn initialize_removal_components() -> Result<(PathResolver, PathBuf, DevBoxState), String> {
    log_debug!("[SDB::Remove::Init] Initializing removal components");

    // Initialize path resolver to locate system directories
    let paths = PathResolver::new(None, None)
        .map_err(|e| format!("Failed to initialize path resolver: {e}"))?;

    // Get the state file path and convert to owned PathBuf
    let state_file_path: PathBuf = paths.state_file().to_path_buf();

    // Load existing state or create new state if file doesn't exist
    let state = load_or_initialize_state(&state_file_path);

    log_debug!("[SDB::Remove::Init] Initialized successfully");
    Ok((paths, state_file_path, state))
}

/// Handles the complete lifecycle of state-based removal operations.
///
/// This function encapsulates the common pattern for removing tools and fonts:
/// 1. Initialize components
/// 2. Create orchestrator
/// 3. Execute removal action
/// 4. Update summary based on result
/// 5. Save state if changes were made
/// 6. Display summary to user
///
/// # Type Parameters
///
/// * `F` - Closure that performs the actual removal using the orchestrator
///
/// # Arguments
///
/// * `item_name` - Name of the item to remove
/// * `item_type` - Type description for logging ("tool" or "font")
/// * `remove_action` - Closure that executes the removal
fn handle_state_based_removal<F>(item_name: String, item_type: &str, remove_action: F)
where
    F: FnOnce(&mut RemovalOrchestrator, &str) -> RemovalResult,
{
    log_info!(
        "[SDB::Remove] Starting {} removal: {}",
        item_type,
        item_name.cyan()
    );

    // Initialize core components
    let (paths, state_file_path, mut state) = match initialize_removal_components() {
        Ok(components) => components,
        Err(e) => {
            log_error!("[SDB::Remove] Initialization failed: {}", e.red());
            std::process::exit(1);
        }
    };

    // Create orchestrator
    let mut orchestrator = match RemovalOrchestrator::new(&mut state, &paths) {
        Ok(orch) => orch,
        Err(e) => {
            log_error!("[SDB::Remove] Failed to create orchestrator: {}", e.red());
            std::process::exit(1);
        }
    };

    // Execute removal and build summary
    let mut summary = RemovalSummary::default();

    match remove_action(&mut orchestrator, &item_name) {
        RemovalResult::Removed => {
            log_info!(
                "[SDB::Removed] Successfully removed {}: {}",
                item_type,
                item_name.green()
            );
            // Add to appropriate summary list based on type
            if item_type == "tool" {
                summary.removed_tools.push(item_name);
            } else {
                summary.removed_fonts.push(item_name);
            }
        }
        RemovalResult::NotFound => {
            log_warn!(
                "[SDB::Remove] {} not found: {}",
                item_type,
                item_name.yellow()
            );
            summary.not_found_items.push(item_name);
        }
        RemovalResult::Failed(reason) => {
            log_error!(
                "[SDB::Remove] Failed to remove {}: {}",
                item_type,
                reason.red()
            );
            summary.failed_removals.push((item_name, reason));
        }
    }

    // Save state if any items were successfully removed
    if !summary.removed_tools.is_empty() || !summary.removed_fonts.is_empty() {
        log_debug!(
            "[SDB::Remove] Saving state to: {}",
            state_file_path.display()
        );
        save_state_to_file(&state, &state_file_path);
    }

    // Display summary to user
    summary.display();
}

/// Handles the complete lifecycle of configuration-only removal operations.
///
/// This function encapsulates the common pattern for removing aliases and settings:
/// 1. Initialize PathResolver and ConfigurationCleaner
/// 2. Execute removal action
/// 3. Log appropriate success/warning/error message
///
/// # Type Parameters
///
/// * `F` - Closure that performs the actual removal using the cleaner
///
/// # Arguments
///
/// * `item_name` - Name of the item to remove
/// * `item_type` - Type description for logging ("alias" or "setting")
/// * `removal_action` - Closure that executes the removal
fn handle_config_removal<F>(item_name: String, item_type: &str, removal_action: F)
where
    F: FnOnce(&ConfigurationCleaner) -> Result<bool, String>,
{
    log_info!(
        "[SDB::Remove::Config] Starting {} removal: {}",
        item_type,
        item_name.cyan()
    );

    // Initialize path resolver
    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Remove::Config] Failed to initialize paths: {}",
            e.red()
        );
        std::process::exit(1);
    });

    // Create configuration cleaner
    let cleaner = ConfigurationCleaner::new(&paths).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Remove::Config] Failed to initialize cleaner: {}",
            e.red()
        );
        std::process::exit(1);
    });

    // Execute removal and handle result
    match removal_action(&cleaner) {
        Ok(true) => {
            log_info!(
                "[SDB::Removed] Successfully removed {}: {}",
                item_type,
                item_name.green()
            );
            println!(
                "\n{} {} removed successfully",
                "✓".green(),
                item_name.green()
            );
        }
        Ok(false) => {
            log_warn!(
                "[SDB::Remove::Config] {} not found: {}",
                item_type,
                item_name.yellow()
            );
            println!("\n{} {} not found", "⚠".yellow(), item_name.yellow());
        }
        Err(e) => {
            log_error!(
                "[SDB::Remove::Config] Failed to remove {}: {}",
                item_type,
                e.red()
            );
            println!("\n{} Failed: {}", "✗".red(), e.red());
            std::process::exit(1);
        }
    }
}

// ============================================================================
//                                 PUBLIC API
// ============================================================================

/// Removes a tool from the system.
///
/// This is the main entry point for tool removal. It handles:
/// - Binary/package uninstallation via the appropriate installer
/// - Configuration file cleanup
/// - State file updates
/// - YAML configuration updates
///
/// # Arguments
///
/// * `tool_name` - Name or alias of the tool to remove
///
/// # Examples
///
/// ```rust
/// // Remove by original name
/// remove_tool("ripgrep".to_string());
///
/// // Remove by alias (if tool was renamed)
/// remove_tool("rg".to_string());
/// ```
///
/// # Exit Codes
///
/// This function may call `std::process::exit(1)` if critical initialization
/// fails. Otherwise, it completes gracefully and displays a summary.
pub fn remove_tool(tool_name: String) {
    handle_state_based_removal(tool_name, "tool", |orch, name| orch.remove_tool(name));
}

/// Removes a font from the system.
///
/// This is the main entry point for font removal. It handles:
/// - Font file deletion from the fonts directory
/// - State file updates
/// - YAML configuration updates
///
/// # Arguments
///
/// * `font_name` - Name of the font to remove
///
/// # Examples
///
/// ```rust
/// remove_font("JetBrainsMono".to_string());
/// ```
///
/// # Font File Matching
///
/// All .ttf files in the fonts directory containing the font name will be removed.
/// For example, removing "JetBrainsMono" would delete:
/// - JetBrainsMono-Regular.ttf
/// - JetBrainsMono-Bold.ttf
/// - JetBrainsMono-Italic.ttf
/// - etc.
pub fn remove_font(font_name: String) {
    handle_state_based_removal(font_name, "font", |orch, name| orch.remove_font(name));
}

/// Removes an alias definition from shellrc.yaml.
///
/// This only removes the alias definition from the configuration file.
/// It does not affect the shell environment until the shell is restarted
/// or the configuration is reloaded.
///
/// # Arguments
///
/// * `alias_name` - Name of the alias to remove
///
/// # Examples
///
/// ```rust
/// remove_alias("ll".to_string());  // Remove 'll' alias
/// ```
pub fn remove_alias(alias_name: String) {
    handle_config_removal(alias_name.clone(), "alias", |cleaner| {
        cleaner.remove_list_item("shellrc.yaml", "aliases:", "name:", &alias_name)
    });
}

/// Removes a macOS system setting from settings.yaml.
///
/// This only removes the setting definition from the configuration file.
/// It does not revert the actual macOS system preference. The user would
/// need to manually change the setting back using System Preferences.
///
/// # Arguments
///
/// * `domain` - The macOS defaults domain (e.g., "com.apple.dock")
/// * `key` - The setting key within the domain (e.g., "autohide")
///
/// # Examples
///
/// ```rust
/// // Remove dock autohide setting
/// remove_setting("com.apple.dock".to_string(), "autohide".to_string());
/// ```
pub fn remove_setting(domain: String, key: String) {
    let setting_name = format!("{domain}.{key}");
    handle_config_removal(setting_name, "setting", |cleaner| {
        cleaner.remove_setting(&domain, &key)
    });
}

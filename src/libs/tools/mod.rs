use crate::libs::state::manager::save_state_to_file;
use crate::libs::utilities::platform::execute_hooks;
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::state_file::DevBoxState;
use crate::schemas::tools::{
    InstallationConfiguration, InstallationSummary, ToolConfig, ToolEntry,
    ToolInstallationOrchestrator,
};
use crate::{log_debug, log_info, log_warn};
use colored::Colorize;
use std::path::PathBuf;

pub(crate) mod configuration;
pub(crate) mod installation;
pub(crate) mod uninstaller;

// ============================================================================
// PUBLIC FUNCTIONS
// ============================================================================

/// ### Public Functions
/// The main public entry point for the tool installation process.
///
/// This function sets up the installation configuration, initializes the orchestrator,
/// and processes all tools. It then displays a summary and saves the final state
/// if any changes were made.
///
/// ## Parameters
/// - `tools_configuration`: Tool configuration containing all tool definitions
/// - `state`: Mutable reference to the application state
/// - `state_file_path`: Path to the state file for persistence
/// - `force_update_latest`: Whether to force updates of "latest" version tools
///
pub fn install_tools(
    tools_configuration: ToolConfig,
    state: &mut DevBoxState,
    state_file_path: &PathBuf,
    force_update_latest: bool,
    paths: &PathResolver, // Add PathResolver parameter
) {
    eprintln!("\n");
    eprintln!("{}:", "TOOLS".bright_yellow().bold());
    eprintln!("{}", "=".repeat(7).bright_yellow());

    // Create the installation configuration based on the provided parameters.
    let installation_config =
        InstallationConfiguration::new(&tools_configuration, force_update_latest);
    // Initialize the main orchestrator with the shared state, configuration, and paths.
    let mut orchestrator = ToolInstallationOrchestrator::new(state, &installation_config, paths);

    log_debug!(
        "[SDB::Tools] Update policy: {}",
        if installation_config.force_update_enabled {
            "forced update of all 'latest' version tools (--update-latest flag)".to_string()
        } else {
            format!(
                "only update 'latest' versions if older than {:?}",
                installation_config.update_threshold_duration
            )
        }
    );

    // Process all tools and collect the results.
    let processing_results = orchestrator.process_all_tools(&tools_configuration.tools);
    // Create a summary object from the results for reporting.
    let summary = InstallationSummary::from_processing_results(processing_results);

    // Display the final summary to the user.
    summary.display_summary();

    // Only save the state file if there were changes to prevent unnecessary writes.
    if summary.has_state_changes() {
        save_state_to_file(state, state_file_path);
    } else {
        log_info!("[SDB::Tools] No new tools installed or state changes detected.");
    }

    eprintln!();
}

/// Executes a set of additional shell commands for a tool after its installation.
///
/// This function provides a way to run custom commands, such as `pip install -r requirements.txt`,
/// as part of the tool installation process. It handles logging and error reporting.
///
/// ## Parameters
/// - `installer_prefix`: Prefix for log messages identifying the installer
/// - `tool_entry`: Tool entry containing additional commands
/// - `working_directory`: Directory where commands should be executed
///
/// ## Returns
/// `Some(Vec<String>)` with executed commands if successful, `None` if no commands or failure
pub fn execute_post_installation_hooks(
    installer_prefix: &str,
    tool_entry: &ToolEntry,
    working_directory: &std::path::Path,
) -> Option<Vec<String>> {
    // Return `None` if there are no additional commands.
    let post_install_hooks = tool_entry.post_installation_hooks.as_ref()?;

    if post_install_hooks.is_empty() {
        log_debug!(
            "[SDB::Tools] {} No additional commands for {}",
            installer_prefix,
            tool_entry.name.dimmed()
        );
        return None;
    }

    log_info!(
        "[SDB::Tools] {} Executing {} post hook(s) for {}",
        installer_prefix,
        post_install_hooks.len().to_string().yellow(),
        tool_entry.name.bold()
    );

    log_debug!(
        "[SDB::Tools::PostHooks] {} Working directory: {}",
        installer_prefix,
        working_directory.display().to_string().cyan()
    );

    // Delegate the actual command execution to a separate utility function.
    match execute_hooks(
        installer_prefix,
        post_install_hooks,
        working_directory,
        &tool_entry.name,
    ) {
        Ok(executed_commands) => {
            log_info!(
                "[SDB::Tools] {} Successfully completed additional hooks/commands for {}",
                installer_prefix,
                tool_entry.name.green()
            );
            Some(executed_commands)
        }
        Err(execution_error) => {
            log_warn!(
                "[SDB::Tools] {} Additional hooks/commands failed for {}: {}. Continuing.",
                installer_prefix,
                tool_entry.name.yellow(),
                execution_error.yellow()
            );
            None
        }
    }
}

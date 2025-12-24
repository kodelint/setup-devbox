pub mod configuration;
pub mod installation;
pub mod installers;
pub mod uninstaller;

use crate::core::platform::execute_hooks;
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::state_file::DevBoxState;
use crate::schemas::tools_types::{
    InstallationConfiguration, InstallationSummary, ToolConfig, ToolEntry,
    ToolInstallationOrchestrator,
};
use crate::state::manager::save_state_to_file;
use crate::{log_debug, log_info, log_warn};
use colored::Colorize;
use std::path::Path;

// ============================================================================
// PUBLIC FUNCTIONS
// ============================================================================

pub fn install_tools(
    tools_configuration: ToolConfig,
    state: &mut DevBoxState,
    state_file_path: &Path,
    force_update_latest: bool,
    paths: &PathResolver,
) {
    eprintln!("\n");
    eprintln!("{}:", "TOOLS".bright_yellow().bold());
    eprintln!("{}", "=".repeat(7).bright_yellow());

    let installation_config =
        InstallationConfiguration::new(&tools_configuration, force_update_latest);
    let mut orchestrator = ToolInstallationOrchestrator::new(state, &installation_config, paths);

    log_debug!(
        "[SDB::Engine] Update policy: {}",
        if installation_config.force_update_enabled {
            "forced update of all 'latest' version tools (--update-latest flag)".to_string()
        } else {
            format!(
                "only update 'latest' versions if older than {:?}",
                installation_config.update_threshold_duration
            )
        }
    );

    let processing_results = orchestrator.process_all_tools(&tools_configuration.tools);
    let summary = InstallationSummary::from_processing_results(processing_results);

    summary.display_summary();

    if summary.has_state_changes() {
        save_state_to_file(state, state_file_path);
    } else {
        log_info!("[SDB::Engine] No new tools installed or state changes detected.");
    }

    eprintln!();
}

pub fn execute_post_installation_hooks(
    installer_prefix: &str,
    tool_entry: &ToolEntry,
    working_directory: &std::path::Path,
) -> Option<Vec<String>> {
    let post_install_hooks = tool_entry.post_installation_hooks.as_ref()?;

    if post_install_hooks.is_empty() {
        log_debug!(
            "[SDB::Engine] {} No additional commands for {}",
            installer_prefix,
            tool_entry.name.dimmed()
        );
        return None;
    }

    log_info!(
        "[SDB::Engine] {} Executing {} post hook(s) for {}",
        installer_prefix,
        post_install_hooks.len().to_string().yellow(),
        tool_entry.name.bold()
    );

    match execute_hooks(
        installer_prefix,
        post_install_hooks,
        working_directory,
        &tool_entry.name,
    ) {
        Ok(executed_commands) => {
            log_info!(
                "[SDB::Engine] {} Successfully completed additional hooks/commands for {}",
                installer_prefix,
                tool_entry.name.green()
            );
            Some(executed_commands)
        }
        Err(execution_error) => {
            log_warn!(
                "[SDB::Engine] {} Additional hooks/commands failed for {}: {}. Continuing.",
                installer_prefix,
                tool_entry.name.yellow(),
                execution_error.yellow()
            );
            None
        }
    }
}

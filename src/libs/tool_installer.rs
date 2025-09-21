//! tool_installer.rs
//!
//! This module serves as the central orchestrator for the tool installation pipeline.
//! It is responsible for determining the correct action for each tool (install, update,
//! or skip), invoking the appropriate installer, and managing the tool's state and
//! configuration. It integrates with various other modules to perform tasks such as
//! state management, path resolution, and command execution.

use chrono::Duration;
use colored::Colorize;
use std::path::PathBuf;
// Import all available installer modules
use crate::installers::{brew, cargo, github, go, pip, rustup, url, uv};
// Import utility functions for state and time management
use crate::libs::state_management::save_state_to_file;
use crate::libs::utilities::assets::{is_timestamp_older_than, parse_duration, time_since};
use crate::libs::utilities::misc_utils::format_duration;
use crate::libs::utilities::platform::{
    check_installer_command_available, execute_additional_commands,
};
use crate::schemas::configuration_management::ConfigurationManagerProcessor;
// Import data schemas and the configuration processor
use crate::schemas::state_file::{DevBoxState, ToolState};
use crate::schemas::tools::{
    ConfigurationAction, InstallationConfiguration, InstallationSummary, ToolAction, ToolConfig,
    ToolEntry, ToolInstallationOrchestrator, ToolProcessingResult, VersionAction,
};
// Import logging macros
use crate::{log_debug, log_error, log_info, log_warn};

impl InstallationConfiguration {
    /// Creates a new `InstallationConfiguration` instance.
    ///
    /// The `force_update` flag takes precedence and, if true, sets the update threshold
    /// to zero, effectively forcing an update on every run for tools with `latest` versions.
    pub(crate) fn new(tools_config: &ToolConfig, force_update: bool) -> Self {
        let update_threshold_duration = if force_update {
            // If forced, the threshold is 0, so any tool update is always considered 'older'
            Duration::seconds(0)
        } else {
            // Parse the duration from the config file, or default to 0 days
            tools_config
                .update_latest_only_after
                .as_ref()
                .and_then(|duration_str| parse_duration(duration_str))
                .unwrap_or_else(|| Duration::days(0))
        };

        Self {
            update_threshold_duration,
            force_update_enabled: force_update,
        }
    }
}

impl<'a> ToolInstallationOrchestrator<'a> {
    /// Creates a new `ToolInstallationOrchestrator`.
    ///
    /// It initializes the configuration manager processor, which handles the path
    /// resolution for tool configuration files.
    fn new(state: &'a mut DevBoxState, configuration: &'a InstallationConfiguration) -> Self {
        // The `ConfigurationManagerProcessor` is created with a `None` base path,
        // relying on its internal fallback logic to resolve the correct path.
        let config_processor = ConfigurationManagerProcessor::new(None);

        Self {
            state,
            configuration,
            config_processor,
        }
    }

    /// Determines the high-level action to be taken for a specific tool.
    ///
    /// This is the primary decision-making method that combines the results of
    /// version and configuration analysis.
    fn determine_required_action(
        &self,
        tool: &ToolEntry,
        current_state: Option<&ToolState>,
    ) -> ToolAction {
        // If the tool is not in the current state, it must be installed.
        match current_state {
            None => ToolAction::Install,
            Some(state) => {
                // Analyze version and configuration requirements independently.
                let version_action = self.analyze_version_requirements(tool, state);
                let config_action = self.analyze_configuration_requirements(tool, state);

                // Combine the individual actions into a final `ToolAction`.
                self.combine_actions(version_action, config_action)
            }
        }
    }

    /// Analyzes the version requirements for a tool to determine if an update is needed.
    ///
    /// This function handles several scenarios:
    /// - **`latest` version with a threshold**: Checks if the last update time is within the threshold.
    /// - **Forced update**: Skips the threshold check and always returns `Update`.
    /// - **Specific version**: Checks if the required version is already installed.
    fn analyze_version_requirements(
        &self,
        tool: &ToolEntry,
        current_state: &ToolState,
    ) -> VersionAction {
        let requested_version = tool.version.as_deref().unwrap_or("latest");
        let is_latest_version_scenario =
            requested_version == "latest" || current_state.version == "latest";

        // Handle the "latest" version logic with an update threshold.
        if is_latest_version_scenario && !self.configuration.force_update_enabled {
            if let Some(last_updated_timestamp) = &current_state.last_updated {
                if !is_timestamp_older_than(
                    last_updated_timestamp,
                    &self.configuration.update_threshold_duration,
                ) {
                    let time_since_update = time_since(last_updated_timestamp)
                        .unwrap_or_else(|| "recently".to_string());
                    let threshold_description =
                        format_duration(&self.configuration.update_threshold_duration);

                    // Skip because the 'latest' version was recently updated.
                    return VersionAction::Skip(format!(
                        "version 'latest' updated {} (within {} threshold)",
                        time_since_update, threshold_description
                    ));
                }
            }
            // The tool is older than the threshold, so it needs an update.
            return VersionAction::Update;
        }

        // Handle specific version logic.
        if let Some(required_version) = &tool.version {
            if required_version != "latest" && current_state.version == *required_version {
                // Skip because the specified version is already installed.
                return VersionAction::Skip("specified version already installed".to_string());
            }
        } else if !current_state.version.is_empty() && current_state.version != "latest" {
            // A tool with no specified version is already installed with a specific version.
            return VersionAction::Skip("version not specified but tool is installed".to_string());
        }

        // Default case: a specific version is required and not installed, or it's a forced update.
        VersionAction::Update
    }

    /// Analyzes the configuration requirements for a tool.
    ///
    /// This method leverages the `ConfigurationManagerProcessor` to determine if the
    /// configuration file needs to be updated based on SHA hashes and file existence.
    fn analyze_configuration_requirements(
        &self,
        tool: &ToolEntry,
        current_state: &ToolState,
    ) -> ConfigurationAction {
        // If the configuration is not enabled in the tool entry, we can skip this check.
        if !tool.configuration_manager.enabled {
            return ConfigurationAction::Skip("configuration disabled".to_string());
        }

        // Delegate the core logic to the `ConfigurationManagerProcessor`.
        match self.config_processor.evaluate_configuration_change_needed(
            &tool.name,
            &tool.configuration_manager,
            current_state.get_configuration_manager(),
        ) {
            Ok(true) => ConfigurationAction::Update,
            Ok(false) => ConfigurationAction::Skip("configuration up-to-date".to_string()),
            Err(e) => {
                // Log a warning if evaluation fails but assume an update is needed to be safe.
                log_warn!(
                    "[Tools] Error evaluating configuration for {}: {}. Assuming update needed.",
                    tool.name,
                    e
                );
                ConfigurationAction::Update
            }
        }
    }

    /// Combines the individual `VersionAction` and `ConfigurationAction` into a single
    /// `ToolAction`.
    ///
    /// The logic here prioritizes a full tool update over a configuration-only update.
    fn combine_actions(
        &self,
        version_action: VersionAction,
        config_action: ConfigurationAction,
    ) -> ToolAction {
        match (version_action, config_action) {
            // If a version update is needed, perform a full tool update.
            (VersionAction::Update, _) => ToolAction::Update,
            // If the version is up-to-date but the configuration needs an update,
            // perform a configuration-only update.
            (VersionAction::Skip(_), ConfigurationAction::Update) => {
                ToolAction::UpdateConfigurationOnly
            }
            // If both version and configuration are up-to-date, determine the appropriate skip reason.
            (VersionAction::Skip(version_reason), ConfigurationAction::Skip(config_reason)) => {
                // Check if the configuration was actually evaluated and found up-to-date.
                if config_reason == "configuration up-to-date" {
                    ToolAction::SkipConfigurationOnly(format!(
                        "{}, {}",
                        version_reason, config_reason
                    ))
                } else {
                    // This is a regular skip, typically for tools with disabled configuration.
                    ToolAction::Skip(version_reason)
                }
            }
        }
    }

    /// Iterates through all tools in the configuration and processes each one.
    fn process_all_tools(&mut self, tools: &[ToolEntry]) -> Vec<(String, ToolProcessingResult)> {
        tools
            .iter()
            .map(|tool| {
                let result = self.process_individual_tool(tool);
                (tool.name.clone(), result)
            })
            .collect()
    }

    /// Handles the complete processing pipeline for a single tool.
    /// This includes validation, action determination, and execution.
    fn process_individual_tool(&mut self, tool: &ToolEntry) -> ToolProcessingResult {
        log_debug!("[Tools] Processing tool: {}", tool.name.bright_green());

        // Step 1: Validate the tool's configuration.
        if let Err(validation_error) = tool.validate() {
            return ToolProcessingResult::Failed(format!(
                "Configuration validation failed: {}",
                validation_error
            ));
        }

        // Step 2: Validate that the required installer command is available.
        if let Err(installer_error) = self.validate_installer_availability(tool) {
            return ToolProcessingResult::Failed(installer_error);
        }

        // Step 3: Determine and execute the required action.
        let current_state = self.state.tools.get(&tool.name);
        let required_action = self.determine_required_action(tool, current_state);

        self.execute_action(tool, required_action)
    }

    /// Validates that the command-line tool for the installer exists on the system.
    /// This prevents failed installations due to missing prerequisites like `brew` or `go`.
    fn validate_installer_availability(&self, tool: &ToolEntry) -> Result<(), String> {
        let installer_name = tool.source.to_string().to_lowercase();

        // Only validate installers that require a system command to be present.
        if matches!(
            installer_name.as_str(),
            "brew" | "go" | "cargo" | "rustup" | "pip" | "uv"
        ) {
            check_installer_command_available(&installer_name)
                .map_err(|error| format!("Installer '{}' not available: {}", installer_name, error))
        } else {
            // Installers like `github` or `url` don't require a pre-existing command.
            Ok(())
        }
    }

    /// Executes the determined `ToolAction`.
    fn execute_action(&mut self, tool: &ToolEntry, action: ToolAction) -> ToolProcessingResult {
        match action {
            ToolAction::Skip(reason) => ToolProcessingResult::Skipped(reason),
            ToolAction::SkipConfigurationOnly(reason) => {
                ToolProcessingResult::ConfigurationSkipped(reason)
            }
            ToolAction::Install => self.execute_installation(tool, "Installing"),
            ToolAction::Update => self.execute_installation(tool, "Updating"),
            ToolAction::UpdateConfigurationOnly => self.execute_configuration_update(tool),
        }
    }

    /// Handles the `UpdateConfigurationOnly` action.
    /// This is a specialized path that only processes the configuration manager without
    /// invoking the tool installer.
    fn execute_configuration_update(&mut self, tool: &ToolEntry) -> ToolProcessingResult {
        log_info!("[Tools] Configuration Management...");
        log_info!(
            "[Tools] Updating configuration for: {}",
            tool.name.bright_green()
        );

        // Get the existing state for the tool.
        if let Some(mut existing_state) = self.state.tools.get(&tool.name).cloned() {
            // Process the configuration and update the state if successful.
            match self.process_configuration_management(tool, &mut existing_state) {
                Ok(()) => {
                    self.state.tools.insert(tool.name.clone(), existing_state);
                    ToolProcessingResult::ConfigurationUpdated
                }
                Err(error) => {
                    ToolProcessingResult::Failed(format!("Configuration update failed: {}", error))
                }
            }
        } else {
            // This should not happen if the logic is correct, but it's a safe guard.
            ToolProcessingResult::Failed("Tool not found in state".to_string())
        }
    }

    /// Encapsulates the common installation logic for both `Install` and `Update` actions.
    /// It invokes the installer, handles post-installation commands, and updates the state.
    fn execute_installation(
        &mut self,
        tool: &ToolEntry,
        operation_type: &str,
    ) -> ToolProcessingResult {
        log_info!("[Tools] Installing {}...", "Tools".bright_green());
        self.display_installation_header(tool, operation_type);

        // Invoke the correct installer based on the tool's `source`.
        match self.invoke_appropriate_installer(tool) {
            Some(mut tool_state) => {
                // Process configuration management as a non-fatal step.
                // An error here will be logged as a warning but won't fail the overall installation.
                if let Err(error) = self.process_configuration_management(tool, &mut tool_state) {
                    log_warn!(
                        "[Tools] Configuration management warning for {}: {}. Continuing.",
                        tool.name.yellow(),
                        error
                    );
                }

                // Update the state with the new tool information.
                self.state.tools.insert(tool.name.clone(), tool_state);
                self.display_installation_success(tool, operation_type);

                // Return the appropriate success result.
                match operation_type {
                    "Installing" => ToolProcessingResult::Installed,
                    _ => ToolProcessingResult::Updated,
                }
            }
            None => {
                // If the installer returns `None`, it signifies a failure.
                self.display_installation_failure(tool, operation_type);
                ToolProcessingResult::Failed(format!("{} failed", operation_type))
            }
        }
    }

    // A simple helper function to display a formatted header for installation.
    fn display_installation_header(&self, tool: &ToolEntry, operation_type: &str) {
        println!("\n{}", "=".repeat(80).bright_blue());
        log_info!(
            "[Tools] {} tool from {}: {}",
            operation_type.bright_yellow(),
            tool.source.to_string().bright_yellow(),
            tool.name.bright_blue().bold()
        );
    }

    // A helper function to display a formatted success message.
    fn display_installation_success(&self, tool: &ToolEntry, operation_type: &str) {
        log_info!(
            "[Tools] Successfully {} tool: {}",
            operation_type.to_lowercase(),
            tool.name.bold().bright_green()
        );
        println!("{}\n", "=".repeat(80).blue());
    }

    // A helper function to display a formatted failure message.
    fn display_installation_failure(&self, tool: &ToolEntry, operation_type: &str) {
        log_error!(
            "[Tools] Failed to {} tool: {}",
            operation_type.to_lowercase(),
            tool.name.bold().red()
        );
    }

    /// Invokes the correct installer function based on the tool's `source`.
    ///
    /// The `match` statement dispatches to a specific module (e.g., `github::install`).
    /// This design keeps the orchestration logic clean and separates it from the
    /// implementation details of each installer.
    fn invoke_appropriate_installer(&self, tool: &ToolEntry) -> Option<ToolState> {
        match tool.source.to_string().to_lowercase().as_str() {
            "github" => github::install(tool),
            "brew" => brew::install(tool),
            "go" => go::install(tool),
            "cargo" => cargo::install(tool),
            "rustup" => rustup::install(tool),
            "pip" => pip::install(tool),
            "uv" => uv::install(tool),
            "url" => url::install(tool),
            unsupported_installer => {
                log_warn!(
                    "[Tools] Unsupported installer: {} for tool: {}",
                    unsupported_installer.yellow(),
                    tool.name.bold()
                );
                None
            }
        }
    }

    /// Calls the `ConfigurationManagerProcessor` to process the configuration for a tool.
    ///
    /// This method is called after both successful installations and for
    /// `UpdateConfigurationOnly` actions.
    fn process_configuration_management(
        &self,
        tool: &ToolEntry,
        tool_state: &mut ToolState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get the existing configuration state from the tool's current state.
        let existing_config_state = tool_state.get_configuration_manager();

        // Process the configuration and handle the result.
        match self.config_processor.process_tool_configuration(
            &tool.name,
            &tool.configuration_manager,
            existing_config_state,
        )? {
            Some(new_config_state) => {
                // If a new state is returned, update the tool's state.
                tool_state.set_configuration_manager(new_config_state);
                Ok(())
            }
            None => {
                // If `None` is returned, it means configuration was disabled or no update was needed.
                Ok(())
            }
        }
    }
}

impl InstallationSummary {
    /// Creates a new `InstallationSummary` from the raw `ToolProcessingResult`s.
    fn from_processing_results(results: Vec<(String, ToolProcessingResult)>) -> Self {
        let mut summary = Self {
            installed_tools: Vec::new(),
            updated_tools: Vec::new(),
            configuration_updated_tools: Vec::new(),
            skipped_tools: Vec::new(),
            configuration_skipped_tools: Vec::new(),
            failed_tools: Vec::new(),
        };

        // Categorize each result into the appropriate vector.
        for (tool_name, result) in results {
            match result {
                ToolProcessingResult::Installed => summary.installed_tools.push(tool_name),
                ToolProcessingResult::Updated => summary.updated_tools.push(tool_name),
                ToolProcessingResult::ConfigurationUpdated => {
                    summary.configuration_updated_tools.push(tool_name)
                }
                ToolProcessingResult::Skipped(reason) => {
                    summary.skipped_tools.push((tool_name, reason))
                }
                ToolProcessingResult::ConfigurationSkipped(reason) => summary
                    .configuration_skipped_tools
                    .push((tool_name, reason)),
                ToolProcessingResult::Failed(reason) => {
                    summary.failed_tools.push((tool_name, reason))
                }
            }
        }

        summary
    }

    /// Checks if any state-changing operations (install, update, config update) occurred.
    /// This is used to decide whether the state file needs to be saved.
    fn has_state_changes(&self) -> bool {
        !self.installed_tools.is_empty()
            || !self.updated_tools.is_empty()
            || !self.configuration_updated_tools.is_empty()
    }

    /// Prints the complete summary to the console.
    fn display_summary(&self) {
        self.display_skipped_tools();
        self.display_configuration_skipped_tools();
        self.display_failed_tools();
        self.display_success_summary();
    }

    /// Prints a formatted list of skipped tools.
    fn display_skipped_tools(&self) {
        if self.skipped_tools.is_empty() {
            return;
        }

        println!();
        println!(
            "{} Skipped tools (already up to date) {}",
            "============".blue(),
            "=============".blue()
        );

        for tools_chunk in self.skipped_tools.chunks(3) {
            let tools_line = tools_chunk
                .iter()
                .map(|(name, _)| name.bright_yellow().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("[Skipped Install] {}", tools_line);
        }
        println!("{}", "=".repeat(61).blue());
    }

    /// Prints a formatted list of tools where configuration syncing was skipped.
    fn display_configuration_skipped_tools(&self) {
        if self.configuration_skipped_tools.is_empty() {
            return;
        }

        println!();
        println!(
            "{} Skipped Configuration Sync (already up to date) {}",
            "======".blue(),
            "======".blue()
        );

        for tools_chunk in self.configuration_skipped_tools.chunks(3) {
            let tools_line = tools_chunk
                .iter()
                .map(|(name, _)| name.bright_yellow().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("[Skipped Configuration] {}", tools_line);
        }
        println!("{}\n", "=".repeat(61).blue());
    }

    /// Prints a formatted list of failed tool installations.
    fn display_failed_tools(&self) {
        if self.failed_tools.is_empty() {
            return;
        }

        println!();
        log_error!("[Tools] Failed installations:");
        for (tool_name, failure_reason) in &self.failed_tools {
            log_error!("  {} - {}", tool_name.red().bold(), failure_reason.red());
        }
    }

    /// Prints a final summary of successfully processed tools.
    fn display_success_summary(&self) {
        let total_processed = self.installed_tools.len()
            + self.updated_tools.len()
            + self.configuration_updated_tools.len();

        if total_processed > 0 {
            log_info!("[Tools] Successfully processed {} tool(s)", total_processed);
        }
    }
}

/// ### Public Functions
/// The main public entry point for the tool installation process.
///
/// This function sets up the installation configuration, initializes the orchestrator,
/// and processes all tools. It then displays a summary and saves the final state
/// if any changes were made.
pub fn install_tools(
    tools_configuration: ToolConfig,
    state: &mut DevBoxState,
    state_file_path: &PathBuf,
    force_update_latest: bool,
) {
    eprintln!("\n");
    eprintln!("{}:", "TOOLS".bright_yellow().bold());
    eprintln!("{}", "=".repeat(7).bright_yellow());

    // Create the installation configuration based on the provided parameters.
    let installation_config =
        InstallationConfiguration::new(&tools_configuration, force_update_latest);
    // Initialize the main orchestrator with the shared state and configuration.
    let mut orchestrator = ToolInstallationOrchestrator::new(state, &installation_config);

    log_debug!(
        "[Tools] Update policy: {}",
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
        log_info!("[Tools] No new tools installed or state changes detected.");
    }

    eprintln!();
}

/// Executes a set of additional shell commands for a tool after its installation.
///
/// This function provides a way to run custom commands, such as `pip install -r requirements.txt`,
/// as part of the tool installation process. It handles logging and error reporting.
pub fn execute_post_installation_commands(
    installer_prefix: &str,
    tool_entry: &ToolEntry,
    working_directory: &std::path::Path,
) -> Option<Vec<String>> {
    // Return `None` if there are no additional commands.
    let additional_commands = tool_entry.additional_cmd.as_ref()?;

    if additional_commands.is_empty() {
        log_debug!(
            "[Tools] {} No additional commands for {}",
            installer_prefix,
            tool_entry.name.dimmed()
        );
        return None;
    }

    log_info!(
        "[Tools] {} Executing {} additional command(s) for {}",
        installer_prefix,
        additional_commands.len().to_string().yellow(),
        tool_entry.name.bold()
    );

    log_debug!(
        "[Tools] {} Working directory: {}",
        installer_prefix,
        working_directory.display().to_string().cyan()
    );

    // Delegate the actual command execution to a separate utility function.
    match execute_additional_commands(
        installer_prefix,
        additional_commands,
        working_directory,
        &tool_entry.name,
    ) {
        Ok(executed_commands) => {
            log_info!(
                "[Tools] {} Successfully completed additional commands for {}",
                installer_prefix,
                tool_entry.name.green()
            );
            Some(executed_commands)
        }
        Err(execution_error) => {
            log_warn!(
                "[Tools] {} Additional commands failed for {}: {}. Continuing.",
                installer_prefix,
                tool_entry.name.yellow(),
                execution_error.yellow()
            );
            None
        }
    }
}

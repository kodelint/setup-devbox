//! # Tool Installation Orchestrator
//!
//! This module serves as the central orchestrator for the tool installation pipeline.
//! It is responsible for determining the correct action for each tool (install, update,
//! or skip), invoking the appropriate installer, and managing the tool's state and
//! configuration. It integrates with various other modules to perform tasks such as
//! state management, path resolution, and command execution.
//!
//! ## Core Responsibilities
//!
//! - **Action Determination**: Analyzes tool state and configuration to decide what action to take
//! - **Installer Dispatch**: Routes tools to the appropriate installer based on source type
//! - **State Management**: Maintains and updates the installation state for all tools
//! - **Configuration Synchronization**: Manages tool configuration file updates
//! - **Result Processing**: Collects and categorizes installation results for reporting
//!
//! ## Installation Pipeline
//!
//! The orchestrator follows a structured pipeline for each tool:
//! 1. **Validation**: Check tool configuration and installer availability
//! 2. **Analysis**: Determine required action based on current state and configuration
//! 3. **Execution**: Perform installation, update, or configuration synchronization
//! 4. **State Update**: Record results and update persistent state
//! 5. **Reporting**: Categorize and display results to the user

use chrono::Duration;
use colored::Colorize;
use std::path::PathBuf;
// Import all available installer modules
use crate::installers::{brew, cargo, github, go, pip, rustup, url, uv};
// Import utility functions for state and time management
use crate::libs::configuration_manager::ConfigurationEvaluationResult;
use crate::libs::state_management::save_state_to_file;
use crate::libs::utilities::assets::{is_timestamp_older_than, parse_duration, time_since};
use crate::libs::utilities::misc_utils::format_duration;
use crate::libs::utilities::platform::{check_installer_command_available, execute_hooks};
use crate::schemas::configuration_management::ConfigurationManagerProcessor;
// Import data schemas and the configuration processor
use crate::schemas::state_file::{DevBoxState, ToolState};
use crate::schemas::tools::{
    ConfigurationAction, InstallationConfiguration, InstallationSummary, ToolAction, ToolConfig,
    ToolEntry, ToolInstallationOrchestrator, ToolProcessingResult, VersionAction,
};
// Import logging macros
use crate::{log_debug, log_error, log_info, log_warn};

// ============================================================================
// INSTALLATION CONFIGURATION IMPLEMENTATION
// ============================================================================

impl InstallationConfiguration {
    /// Creates a new `InstallationConfiguration` instance.
    ///
    /// The `force_update` flag takes precedence and, if true, sets the update threshold
    /// to zero, effectively forcing an update on every run for tools with `latest` versions.
    ///
    /// ## Parameters
    /// - `tools_config`: Tool configuration containing update threshold settings
    /// - `force_update`: Whether to force updates regardless of threshold
    ///
    /// ## Returns
    /// New `InstallationConfiguration` instance with resolved update settings
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

// ============================================================================
// TOOL INSTALLATION ORCHESTRATOR IMPLEMENTATION
// ============================================================================

impl<'a> ToolInstallationOrchestrator<'a> {
    /// Creates a new `ToolInstallationOrchestrator`.
    ///
    /// It initializes the configuration manager processor, which handles the path
    /// resolution for tool configuration files.
    ///
    /// ## Parameters
    /// - `state`: Mutable reference to the application state
    /// - `configuration`: Reference to installation configuration settings
    ///
    /// ## Returns
    /// New `ToolInstallationOrchestrator` instance
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

    /// Helper function to normalize a version string by removing a leading 'v'.
    ///
    /// "v1.2.3" -> "1.2.3"
    /// "1.2.3" -> "1.2.3"
    /// "latest" -> "latest"
    fn normalize_version(version: &str) -> &str {
        version.strip_prefix('v').unwrap_or(version)
    }

    /// Determines the high-level action to be taken for a specific tool.
    /// This now performs a single comprehensive evaluation to avoid duplicate SHA calculations.
    ///
    /// This is the primary decision-making method that combines the results of
    /// version and configuration analysis.
    ///
    /// ## Parameters
    /// - `tool`: Tool entry to analyze
    /// - `current_state`: Current state of the tool (if it exists)
    ///
    /// ## Returns
    /// Tuple containing the required action and optional cached configuration evaluation
    fn determine_required_action(
        &self,
        tool: &ToolEntry,
        current_state: Option<&ToolState>,
    ) -> (ToolAction, Option<ConfigurationEvaluationResult>) {
        // If the tool is not in the current state, it must be installed.
        match current_state {
            None => (ToolAction::Install, None),
            Some(state) => {
                // Analyze version requirements first
                let version_action = self.analyze_version_requirements(tool, state);

                // Perform comprehensive configuration evaluation once
                let config_evaluation = match self
                    .config_processor
                    .evaluate_configuration_requirements(
                        &tool.name,
                        &tool.configuration_manager,
                        state.get_configuration_manager(),
                    ) {
                    Ok(evaluation) => Some(evaluation),
                    Err(e) => {
                        log_warn!(
                            "[Tools] Error evaluating configuration for {}: {}. Assuming update needed.",
                            tool.name,
                            e
                        );
                        // Create a default evaluation that assumes update is needed
                        Some(ConfigurationEvaluationResult {
                            needs_update: true,
                            current_source_sha: String::new(),
                            current_destination_sha: None,
                            reason: Some(format!("evaluation error: {e}")),
                        })
                    }
                };

                // Convert evaluation result to ConfigurationAction
                let config_action = match &config_evaluation {
                    Some(eval) if eval.needs_update => ConfigurationAction::Update,
                    Some(eval) => ConfigurationAction::Skip(
                        eval.reason
                            .clone()
                            .unwrap_or_else(|| "configuration up-to-date".to_string()),
                    ),
                    None => ConfigurationAction::Skip("configuration disabled".to_string()),
                };

                // Combine the individual actions into a final `ToolAction`.
                let final_action = self.combine_actions(version_action, config_action);
                (final_action, config_evaluation)
            }
        }
    }

    /// Analyzes the version requirements for a tool to determine if an update is needed.
    ///
    /// This function handles several scenarios:
    /// - **`latest` version with a threshold**: Checks if the last update time is within the threshold.
    /// - **Forced update**: Skips the threshold check and always returns `Update`.
    /// - **Specific version**: Checks if the required version is already installed.
    ///
    /// ## Parameters
    /// - `tool`: Tool entry with version requirements
    /// - `current_state`: Current installed state of the tool
    ///
    /// ## Returns
    /// `VersionAction` indicating whether version update is needed
    fn analyze_version_requirements(
        &self,
        tool: &ToolEntry,
        current_state: &ToolState,
    ) -> VersionAction {
        let requested_version = tool.version.as_deref().unwrap_or("latest");
        let is_latest_version_scenario = requested_version == "latest"
            || current_state.version == "latest"
            // For Rustup Toolchain:
            //  1. Treat `stable` and `nightly` as `latest`
            //  2. Follow the `update_latest_only_after` configuration
            || current_state.version.contains("stable")
            || current_state.version.contains("nightly");

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
                        "version 'latest' updated {time_since_update} (within {threshold_description} threshold)"
                    ));
                }
            }
            // The tool is older than the threshold, so it needs an update.
            return VersionAction::Update;
        }

        // Handle specific version logic.
        if let Some(required_version) = &tool.version {
            let normalized_required = Self::normalize_version(required_version);
            let normalized_current = Self::normalize_version(&current_state.version);

            if required_version != "latest" && normalized_current == normalized_required {
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

    /// Combines the individual `VersionAction` and `ConfigurationAction` into a single
    /// `ToolAction`.
    ///
    /// The logic here prioritizes a full tool update over a configuration-only update.
    ///
    /// ## Parameters
    /// - `version_action`: Version-related action decision
    /// - `config_action`: Configuration-related action decision
    ///
    /// ## Returns
    /// Combined `ToolAction` representing the overall required action
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
                    ToolAction::SkipConfigurationOnly(format!("{version_reason}, {config_reason}"))
                } else {
                    // This is a regular skip, typically for tools with disabled configuration.
                    ToolAction::Skip(version_reason)
                }
            }
        }
    }

    /// Iterates through all tools in the configuration and processes each one.
    ///
    /// ## Parameters
    /// - `tools`: Slice of tool entries to process
    ///
    /// ## Returns
    /// Vector of tuples containing tool names and their processing results
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
    /// Now optimized to avoid duplicate SHA calculations by using cached evaluation results.
    ///
    /// ## Parameters
    /// - `tool`: Tool entry to process
    ///
    /// ## Returns
    /// `ToolProcessingResult` indicating the outcome of the processing
    fn process_individual_tool(&mut self, tool: &ToolEntry) -> ToolProcessingResult {
        log_debug!("[Tools] Processing tool: {}", tool.name.bright_green());

        // Step 1: Validate the tool's configuration.
        if let Err(validation_error) = tool.validate() {
            return ToolProcessingResult::Failed(format!(
                "Configuration validation failed: {validation_error}",
            ));
        }

        // Step 2: Validate that the required installer command is available.
        if let Err(installer_error) = self.validate_installer_availability(tool) {
            return ToolProcessingResult::Failed(installer_error);
        }

        // Step 3: Determine and execute the required action.
        let current_state = self.state.tools.get(&tool.name);
        log_debug!(
            "[Tools] Determining if the tool: {} is already installed",
            &tool.name.cyan()
        );
        let (required_action, cached_config_evaluation) =
            self.determine_required_action(tool, current_state);

        self.execute_action(tool, required_action, cached_config_evaluation)
    }

    /// Validates that the command-line tool for the installer exists on the system.
    /// This prevents failed installations due to missing prerequisites like `brew` or `go`.
    ///
    /// ## Parameters
    /// - `tool`: Tool entry to validate installer for
    ///
    /// ## Returns
    /// `Ok(())` if installer is available, `Err(String)` with error message if not
    fn validate_installer_availability(&self, tool: &ToolEntry) -> Result<(), String> {
        let installer_name = tool.source.to_string().to_lowercase();

        // Only validate installers that require a system command to be present.
        if matches!(
            installer_name.as_str(),
            "brew" | "go" | "cargo" | "rustup" | "pip3" | "uv"
        ) {
            check_installer_command_available(&installer_name)
                .map_err(|error| format!("Installer '{installer_name}' not available: {error}"))
        } else {
            // Installers like `github` or `url` don't require a pre-existing command.
            Ok(())
        }
    }

    /// Executes the determined `ToolAction` with optional cached configuration evaluation.
    ///
    /// ## Parameters
    /// - `tool`: Tool entry to process
    /// - `action`: Action to execute
    /// - `cached_config_evaluation`: Optional pre-computed configuration evaluation
    ///
    /// ## Returns
    /// `ToolProcessingResult` indicating the outcome of the action execution
    fn execute_action(
        &mut self,
        tool: &ToolEntry,
        action: ToolAction,
        cached_config_evaluation: Option<ConfigurationEvaluationResult>,
    ) -> ToolProcessingResult {
        match action {
            ToolAction::Skip(reason) => ToolProcessingResult::Skipped(reason),
            ToolAction::SkipConfigurationOnly(reason) => {
                ToolProcessingResult::ConfigurationSkipped(reason)
            }
            ToolAction::Install => {
                self.execute_installation(tool, "Installing", cached_config_evaluation)
            }
            ToolAction::Update => {
                self.execute_installation(tool, "Updating", cached_config_evaluation)
            }
            ToolAction::UpdateConfigurationOnly => {
                self.execute_configuration_update(tool, cached_config_evaluation)
            }
        }
    }

    /// Handles the `UpdateConfigurationOnly` action with cached evaluation.
    /// This is a specialized path that only processes the configuration manager without
    /// invoking the tool installer.
    ///
    /// ## Parameters
    /// - `tool`: Tool entry to update configuration for
    /// - `cached_config_evaluation`: Optional pre-computed configuration evaluation
    ///
    /// ## Returns
    /// `ToolProcessingResult` indicating the outcome of configuration update
    fn execute_configuration_update(
        &mut self,
        tool: &ToolEntry,
        cached_config_evaluation: Option<ConfigurationEvaluationResult>,
    ) -> ToolProcessingResult {
        log_info!("[Tools] Configuration Management...");
        log_info!(
            "[Tools] Updating configuration for: {}",
            tool.name.bright_green()
        );

        // Get the existing state for the tool.
        if let Some(mut existing_state) = self.state.tools.get(&tool.name).cloned() {
            // Process the configuration with cached evaluation to avoid duplicate SHA calculations
            match self.process_configuration_management(
                tool,
                &mut existing_state,
                cached_config_evaluation,
            ) {
                Ok(()) => {
                    self.state.tools.insert(tool.name.clone(), existing_state);
                    ToolProcessingResult::ConfigurationUpdated
                }
                Err(error) => {
                    ToolProcessingResult::Failed(format!("Configuration update failed: {error}"))
                }
            }
        } else {
            // This should not happen if the logic is correct, but it's a safeguard.
            ToolProcessingResult::Failed("Tool not found in state".to_string())
        }
    }

    /// Encapsulates the common installation logic for both `Install` and `Update` actions.
    /// It invokes the installer, handles post-installation commands, and updates the state.
    /// Now uses cached configuration evaluation to avoid duplicate SHA calculations.
    ///
    /// ## Parameters
    /// - `tool`: Tool entry to install or update
    /// - `operation_type`: String describing the operation ("Installing" or "Updating")
    /// - `cached_config_evaluation`: Optional pre-computed configuration evaluation
    ///
    /// ## Returns
    /// `ToolProcessingResult` indicating the outcome of the installation
    fn execute_installation(
        &mut self,
        tool: &ToolEntry,
        operation_type: &str,
        cached_config_evaluation: Option<ConfigurationEvaluationResult>,
    ) -> ToolProcessingResult {
        log_info!("[Tools] Installing {}...", "Tools".bright_green());
        self.display_installation_header(tool, operation_type);

        // Invoke the correct installer based on the tool's `source`.
        match self.invoke_appropriate_installer(tool) {
            Some(mut tool_state) => {
                // Process configuration management as a non-fatal step with cached evaluation.
                // An error here will be logged as a warning but won't fail the overall installation.
                if let Err(error) = self.process_configuration_management(
                    tool,
                    &mut tool_state,
                    cached_config_evaluation,
                ) {
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
                ToolProcessingResult::Failed(format!("{operation_type} failed"))
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
    ///
    /// ## Parameters
    /// - `tool`: Tool entry to install
    ///
    /// ## Returns
    /// `Some(ToolState)` if installation succeeded, `None` if it failed
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
    /// Now accepts cached evaluation results to avoid duplicate SHA calculations.
    ///
    /// This method is called after both successful installations and for
    /// `UpdateConfigurationOnly` actions.
    ///
    /// ## Parameters
    /// - `tool`: Tool entry to process configuration for
    /// - `tool_state`: Mutable reference to the tool's state
    /// - `cached_config_evaluation`: Optional pre-computed configuration evaluation
    ///
    /// ## Returns
    /// `Ok(())` if configuration processing succeeded, `Err` if it failed
    fn process_configuration_management(
        &self,
        tool: &ToolEntry,
        tool_state: &mut ToolState,
        cached_config_evaluation: Option<ConfigurationEvaluationResult>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get the existing configuration state from the tool's current state.
        let existing_config_state = tool_state.get_configuration_manager();

        // Process the configuration using cached evaluation if available
        match self.config_processor.process_tool_configuration(
            &tool.name,
            &tool.configuration_manager,
            existing_config_state,
            cached_config_evaluation,
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

// ============================================================================
// INSTALLATION SUMMARY IMPLEMENTATION
// ============================================================================

impl InstallationSummary {
    /// Creates a new `InstallationSummary` from the raw `ToolProcessingResult`s.
    ///
    /// ## Parameters
    /// - `results`: Vector of tool names and their processing results
    ///
    /// ## Returns
    /// `InstallationSummary` with categorized results
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
    ///
    /// ## Returns
    /// `true` if any state changes occurred, `false` otherwise
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
            println!("[Skipped Install] {tools_line}");
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
            println!("[Skipped Configuration] {tools_line}");
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
            "[Tools] {} No additional commands for {}",
            installer_prefix,
            tool_entry.name.dimmed()
        );
        return None;
    }

    log_info!(
        "[Tools] {} Executing {} additional command(s) for {}",
        installer_prefix,
        post_install_hooks.len().to_string().yellow(),
        tool_entry.name.bold()
    );

    log_debug!(
        "[Tools] {} Working directory: {}",
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

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
//! - **Result Processing**: Collects and categorizes installation results for reporting
//!
//! ## Installation Pipeline
//!
//! The orchestrator follows a structured pipeline for each tool:
//! 1. **Validation**: Check tool configuration and installer availability
//! 2. **Analysis**: Determine required action based on current state and configuration
//! 3. **Execution**: Perform installation, update, or configuration synchronization
//! 4. **State Update**: Record results and update persistent state

// Import all available installer modules
use crate::engine::installation::planner::InstallationPlanner;
use crate::engine::installers::errors::InstallerError;
use crate::engine::installers::factory::InstallerFactory;
// Import utility functions for state and time management
use crate::core::platform::check_installer_command_available;
// Import logging macros
use crate::schemas::config_manager::{
    ConfigurationEvaluationResult, ConfigurationManagerProcessor,
};
// Import data schemas and the configuration processor
use crate::schemas::state_file::{DevBoxState, ToolState};
use crate::schemas::tools_enums::{SourceType, ToolAction, ToolProcessingResult};
use crate::schemas::tools_types::{
    InstallationConfiguration, ToolEntry, ToolInstallationOrchestrator,
};
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;

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
    pub(crate) fn new(
        state: &'a mut DevBoxState,
        configuration: &'a InstallationConfiguration,
        config_processor: ConfigurationManagerProcessor,
        installer_factory: InstallerFactory,
    ) -> Self {
        Self {
            state,
            configuration,
            config_processor,
            installer_factory,
        }
    }

    /// Iterates through all tools in the configuration and processes each one.
    ///
    /// ## Parameters
    /// - `tools`: Slice of tool entries to process
    ///
    /// ## Returns
    /// Vector of tuples containing tool names and their processing results
    pub(crate) fn process_all_tools(
        &mut self,
        tools: &[ToolEntry],
    ) -> Vec<(String, ToolProcessingResult)> {
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
        log_debug!("[SDB::Tools] Processing tool: {}", tool.name.bright_green());

        // Step 1: Validate the tool's configuration.
        if let Err(validation_error) = tool.validate() {
            return ToolProcessingResult::Failed(format!(
                "[SDB::Tools::Configuration] Configuration validation failed: {validation_error}",
            ));
        }

        // Step 2: Validate that the required installer command is available.
        if let Err(installer_error) = self.validate_installer_availability(tool) {
            return ToolProcessingResult::Failed(installer_error);
        }

        // Step 3: Determine and execute the required action.
        let current_state = self.state.tools.get(&tool.name);
        log_debug!(
            "[SDB::Tools] Determining if the tool: {} is already installed",
            &tool.name.cyan()
        );

        // Use the Planner to determine action
        let planner = InstallationPlanner::new(self.configuration, self.config_processor.clone());
        let (required_action, cached_config_evaluation) =
            planner.determine_required_action(tool, current_state);

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
        let cmd_to_check = match tool.source {
            SourceType::Brew => Some("brew"),
            SourceType::Go => Some("go"),
            SourceType::Cargo => Some("cargo"),
            SourceType::Rustup => Some("rustup"),
            SourceType::Pip => Some("pip3"), // Explicitly check for pip3 as usually preferred
            SourceType::Uv => Some("uv"),
            _ => None,
        };

        if let Some(cmd) = cmd_to_check {
            check_installer_command_available(cmd)
                .map_err(|error| format!("[SDB::Tools] Installer '{cmd}' not available: {error}"))
        } else {
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
        if self.configuration.dry_run {
            let dry_run_msg = match action {
                ToolAction::Skip(reason) => format!("Would skip: {reason}"),
                ToolAction::SkipConfigurationOnly(reason) => {
                    format!("Would skip configuration: {reason}")
                }
                ToolAction::Install => format!("Would install {} using {}", tool.name, tool.source),
                ToolAction::Update => format!("Would update {} using {}", tool.name, tool.source),
                ToolAction::UpdateConfigurationOnly => {
                    format!("Would update configuration for {}", tool.name)
                }
            };
            log_info!("[SDB::DryRun] {}", dry_run_msg.bright_magenta());
            return ToolProcessingResult::DryRun(dry_run_msg);
        }

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
        // log_info!("[SDB::Tools] Installing {}...", "Tools".bright_green());
        self.display_installation_header(tool, operation_type);

        // Invoke the correct installer based on the tool's `source`.
        match self.invoke_appropriate_installer(tool) {
            Ok(mut tool_state) => {
                // Process configuration management as a non-fatal step with cached evaluation.
                // An error here will be logged as a warning but won't fail the overall installation.
                if let Err(error) = self.process_configuration_management(
                    tool,
                    &mut tool_state,
                    cached_config_evaluation,
                ) {
                    log_warn!(
                        "[SDB::Tools] Configuration management warning for {}: {}. Continuing.",
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
            Err(e) => {
                // If the installer returns `Err`, it signifies a failure.
                self.display_installation_failure(tool, operation_type);
                log_error!("[SDB::Tools] Failure reason: {}", e);
                ToolProcessingResult::Failed(format!("[SDB::Tools] {operation_type} failed: {e}"))
            }
        }
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
    /// `Result<ToolState, InstallerError>` if installation succeeded or failed
    fn invoke_appropriate_installer(&self, tool: &ToolEntry) -> Result<ToolState, InstallerError> {
        let installer = self
            .installer_factory
            .get_installer(&tool.source)
            .ok_or_else(|| {
                InstallerError::ConfigurationError(format!(
                    "No installer registered for source type: {:?}",
                    tool.source
                ))
            })?;

        installer.install(tool)
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
        log_info!("[SDB::Tools] Configuration Management...");
        log_info!(
            "[SDB::Tools::Configurations] Updating configuration for: {}",
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
                Err(error) => ToolProcessingResult::Failed(format!(
                    "[SDB::Tools::Configuration] Configuration update failed: {error}"
                )),
            }
        } else {
            // This should not happen if the logic is correct, but it's a safeguard.
            ToolProcessingResult::Failed("[SDB::Tools] Tool not found in state".to_string())
        }
    }

    // A simple helper function to display a formatted header for installation.
    fn display_installation_header(&self, tool: &ToolEntry, operation_type: &str) {
        println!("\nTool Name: {}", tool.name.bright_green().bold());
        println!("{}", "=".repeat(80).blue());
        log_info!(
            "[SDB::Tools] {} {} tool using {}",
            operation_type.bright_blue().bold(),
            tool.name.bright_green().bold(),
            tool.source.to_string().bright_cyan(),
        );
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

    // A helper function to display a formatted success message.
    fn display_installation_success(&self, tool: &ToolEntry, operation_type: &str) {
        log_info!(
            "[SDB::Tools] Successfully completed {} tool: {}",
            operation_type.to_lowercase(),
            tool.name.bold().bright_green()
        );
        println!("{}\n", "=".repeat(80).blue());
    }

    // A helper function to display a formatted failure message.
    fn display_installation_failure(&self, tool: &ToolEntry, operation_type: &str) {
        log_error!(
            "[SDB::Tools] Failed to {} tool: {}",
            operation_type.to_lowercase(),
            tool.name.bold().red()
        );
    }
}

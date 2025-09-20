use chrono::Duration;
use colored::Colorize;
use std::path::PathBuf;

use crate::installers::{brew, cargo, github, go, pip, rustup, url, uv};
use crate::libs::state_management::save_state_to_file;
use crate::libs::utilities::assets::{is_timestamp_older_than, parse_duration, time_since};
use crate::libs::utilities::misc_utils::format_duration;
use crate::libs::utilities::platform::{
    check_installer_command_available, execute_additional_commands,
};
use crate::schemas::sdb_schema::{
    ConfigurationManagerProcessor, DevBoxState, ToolConfig, ToolEntry, ToolState,
};
use crate::{log_debug, log_error, log_info, log_warn};

/// Results from processing a single tool
#[derive(Debug)]
pub enum ToolProcessingResult {
    Installed,
    Updated,
    ConfigurationUpdated,
    Skipped(String),
    ConfigurationSkipped(String),
    Failed(String),
}

/// Actions that can be taken for a tool
#[derive(Debug, PartialEq)]
enum ToolAction {
    Install,
    Update,
    UpdateConfigurationOnly,
    Skip(String),
    SkipConfigurationOnly(String),
}

/// Version-related actions
#[derive(Debug, PartialEq)]
enum VersionAction {
    Update,
    Skip(String),
}

/// Configuration-related actions
#[derive(Debug, PartialEq)]
enum ConfigurationAction {
    Update,
    Skip(String),
}

/// Installation configuration parameters
#[derive(Debug)]
struct InstallationConfiguration {
    update_threshold_duration: Duration,
    force_update_enabled: bool,
}

impl InstallationConfiguration {
    fn new(tools_config: &ToolConfig, force_update: bool) -> Self {
        let update_threshold_duration = if force_update {
            Duration::seconds(0)
        } else {
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

/// Main orchestrator for tool installation
pub struct ToolInstallationOrchestrator<'a> {
    state: &'a mut DevBoxState,
    configuration: &'a InstallationConfiguration,
    config_processor: ConfigurationManagerProcessor,
}

impl<'a> ToolInstallationOrchestrator<'a> {
    fn new(state: &'a mut DevBoxState, configuration: &'a InstallationConfiguration) -> Self {
        let config_processor = ConfigurationManagerProcessor::new(None);

        Self {
            state,
            configuration,
            config_processor,
        }
    }

    /// Determines what action should be taken for a tool
    fn determine_required_action(
        &self,
        tool: &ToolEntry,
        current_state: Option<&ToolState>,
    ) -> ToolAction {
        match current_state {
            None => ToolAction::Install,
            Some(state) => {
                let version_action = self.analyze_version_requirements(tool, state);
                let config_action = self.analyze_configuration_requirements(tool, state);

                self.combine_actions(version_action, config_action)
            }
        }
    }

    /// Analyzes whether a tool's version needs to be updated
    fn analyze_version_requirements(
        &self,
        tool: &ToolEntry,
        current_state: &ToolState,
    ) -> VersionAction {
        let requested_version = tool.version.as_deref().unwrap_or("latest");
        let is_latest_version_scenario =
            requested_version == "latest" || current_state.version == "latest";

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

                    return VersionAction::Skip(format!(
                        "version 'latest' updated {} (within {} threshold)",
                        time_since_update, threshold_description
                    ));
                }
            }
            return VersionAction::Update;
        }

        if let Some(required_version) = &tool.version {
            if required_version != "latest" && current_state.version == *required_version {
                return VersionAction::Skip("specified version already installed".to_string());
            }
        } else if !current_state.version.is_empty() && current_state.version != "latest" {
            return VersionAction::Skip("version not specified but tool is installed".to_string());
        }

        VersionAction::Update
    }

    /// Analyzes whether a tool's configuration needs to be updated
    fn analyze_configuration_requirements(
        &self,
        tool: &ToolEntry,
        current_state: &ToolState,
    ) -> ConfigurationAction {
        // If configuration is not enabled, always skip
        if !tool.configuration_manager.enabled {
            return ConfigurationAction::Skip("configuration disabled".to_string());
        }

        match self.config_processor.evaluate_configuration_change_needed(
            &tool.name,
            &tool.configuration_manager,
            current_state.get_configuration_manager(),
        ) {
            Ok(true) => ConfigurationAction::Update,
            Ok(false) => ConfigurationAction::Skip("configuration up-to-date".to_string()),
            Err(e) => {
                log_warn!(
                    "[Tools] Error evaluating configuration for {}: {}. Assuming update needed.",
                    tool.name,
                    e
                );
                ConfigurationAction::Update
            }
        }
    }

    /// Combines version and configuration actions into a final tool action
    fn combine_actions(
        &self,
        version_action: VersionAction,
        config_action: ConfigurationAction,
    ) -> ToolAction {
        match (version_action, config_action) {
            (VersionAction::Update, _) => ToolAction::Update,
            (VersionAction::Skip(_), ConfigurationAction::Update) => {
                ToolAction::UpdateConfigurationOnly
            }
            (VersionAction::Skip(version_reason), ConfigurationAction::Skip(config_reason)) => {
                // Only show as configuration skip if the tool actually has configuration management enabled
                // and the config reason indicates it was evaluated (not just disabled)
                if config_reason == "configuration up-to-date" {
                    ToolAction::SkipConfigurationOnly(format!(
                        "{}, {}",
                        version_reason, config_reason
                    ))
                } else {
                    // Regular skip for tools without configuration or with disabled configuration
                    ToolAction::Skip(version_reason)
                }
            }
        }
    }

    /// Processes all tools and returns results
    fn process_all_tools(&mut self, tools: &[ToolEntry]) -> Vec<(String, ToolProcessingResult)> {
        tools
            .iter()
            .map(|tool| {
                let result = self.process_individual_tool(tool);
                (tool.name.clone(), result)
            })
            .collect()
    }

    /// Processes a single tool through the complete installation pipeline
    fn process_individual_tool(&mut self, tool: &ToolEntry) -> ToolProcessingResult {
        log_debug!("[Tools] Processing tool: {}", tool.name.bright_green());

        // Validate tool configuration
        if let Err(validation_error) = tool.validate() {
            return ToolProcessingResult::Failed(format!(
                "Configuration validation failed: {}",
                validation_error
            ));
        }

        // Validate installer availability
        if let Err(installer_error) = self.validate_installer_availability(tool) {
            return ToolProcessingResult::Failed(installer_error);
        }

        // Determine and execute required action
        let current_state = self.state.tools.get(&tool.name);
        let required_action = self.determine_required_action(tool, current_state);

        self.execute_action(tool, required_action)
    }

    /// Validates that the required installer is available on the system
    fn validate_installer_availability(&self, tool: &ToolEntry) -> Result<(), String> {
        let installer_name = tool.source.to_string().to_lowercase();

        if matches!(
            installer_name.as_str(),
            "brew" | "go" | "cargo" | "rustup" | "pip" | "uv"
        ) {
            check_installer_command_available(&installer_name)
                .map_err(|error| format!("Installer '{}' not available: {}", installer_name, error))
        } else {
            Ok(())
        }
    }

    /// Executes the determined action for a tool
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

    /// Executes configuration-only update
    fn execute_configuration_update(&mut self, tool: &ToolEntry) -> ToolProcessingResult {
        log_info!("[Tools] Configuration Management...");
        log_info!(
            "[Tools] Updating configuration for: {}",
            tool.name.bright_green()
        );

        if let Some(mut existing_state) = self.state.tools.get(&tool.name).cloned() {
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
            ToolProcessingResult::Failed("Tool not found in state".to_string())
        }
    }

    /// Common installation process for both install and update operations
    fn execute_installation(
        &mut self,
        tool: &ToolEntry,
        operation_type: &str,
    ) -> ToolProcessingResult {
        log_info!("[Tools] Installing tools...");
        self.display_installation_header(tool, operation_type);

        match self.invoke_appropriate_installer(tool) {
            Some(mut tool_state) => {
                // Process configuration management with non-fatal error handling
                if let Err(error) = self.process_configuration_management(tool, &mut tool_state) {
                    log_warn!(
                        "[Tools] Configuration management warning for {}: {}. Continuing.",
                        tool.name.yellow(),
                        error
                    );
                }

                self.state.tools.insert(tool.name.clone(), tool_state);
                self.display_installation_success(tool, operation_type);

                match operation_type {
                    "Installing" => ToolProcessingResult::Installed,
                    _ => ToolProcessingResult::Updated,
                }
            }
            None => {
                self.display_installation_failure(tool, operation_type);
                ToolProcessingResult::Failed(format!("{} failed", operation_type))
            }
        }
    }

    /// Displays header for installation operations
    fn display_installation_header(&self, tool: &ToolEntry, operation_type: &str) {
        println!("\n{}", "=".repeat(80).bright_blue());
        log_info!(
            "[Tools] {} tool from {}: {}",
            operation_type.bright_yellow(),
            tool.source.to_string().bright_yellow(),
            tool.name.bright_blue().bold()
        );
    }

    /// Displays success message for installation operations
    fn display_installation_success(&self, tool: &ToolEntry, operation_type: &str) {
        log_info!(
            "[Tools] Successfully {} tool: {}",
            operation_type.to_lowercase(),
            tool.name.bold().bright_green()
        );
        println!("{}\n", "=".repeat(80).blue());
    }

    /// Displays failure message for installation operations
    fn display_installation_failure(&self, tool: &ToolEntry, operation_type: &str) {
        log_error!(
            "[Tools] Failed to {} tool: {}",
            operation_type.to_lowercase(),
            tool.name.bold().red()
        );
    }

    /// Invokes the appropriate installer based on tool source
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

    /// Processes configuration management using the dedicated processor
    fn process_configuration_management(
        &self,
        tool: &ToolEntry,
        tool_state: &mut ToolState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let existing_config_state = tool_state.get_configuration_manager();

        match self.config_processor.process_tool_configuration(
            &tool.name,
            &tool.configuration_manager,
            existing_config_state,
        )? {
            Some(new_config_state) => {
                tool_state.set_configuration_manager(new_config_state);
                Ok(())
            }
            None => Ok(()),
        }
    }
}

/// Summary of installation results
struct InstallationSummary {
    installed_tools: Vec<String>,
    updated_tools: Vec<String>,
    configuration_updated_tools: Vec<String>,
    skipped_tools: Vec<(String, String)>,
    configuration_skipped_tools: Vec<(String, String)>,
    failed_tools: Vec<(String, String)>,
}

impl InstallationSummary {
    fn from_processing_results(results: Vec<(String, ToolProcessingResult)>) -> Self {
        let mut summary = Self {
            installed_tools: Vec::new(),
            updated_tools: Vec::new(),
            configuration_updated_tools: Vec::new(),
            skipped_tools: Vec::new(),
            configuration_skipped_tools: Vec::new(),
            failed_tools: Vec::new(),
        };

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

    fn has_state_changes(&self) -> bool {
        !self.installed_tools.is_empty()
            || !self.updated_tools.is_empty()
            || !self.configuration_updated_tools.is_empty()
    }

    fn display_summary(&self) {
        self.display_skipped_tools();
        self.display_configuration_skipped_tools();
        self.display_failed_tools();
        self.display_success_summary();
    }

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

    fn display_configuration_skipped_tools(&self) {
        if self.configuration_skipped_tools.is_empty() {
            return;
        }

        println!();
        // log_info!("[Tools] Configuration Management...\n");
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

    fn display_success_summary(&self) {
        let total_processed = self.installed_tools.len()
            + self.updated_tools.len()
            + self.configuration_updated_tools.len();

        if total_processed > 0 {
            log_info!("[Tools] Successfully processed {} tool(s)", total_processed);
        }
    }
}

/// Main entry point for tool installation
pub fn install_tools(
    tools_configuration: ToolConfig,
    state: &mut DevBoxState,
    state_file_path: &PathBuf,
    force_update_latest: bool,
) {
    eprintln!("\n");
    eprintln!("{}:", "TOOLS".bright_yellow().bold());
    println!("{}\n", "=".repeat(7).bright_yellow());
    // log_info!("[Tools] Installing tools...");

    let installation_config =
        InstallationConfiguration::new(&tools_configuration, force_update_latest);
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

    let processing_results = orchestrator.process_all_tools(&tools_configuration.tools);
    let summary = InstallationSummary::from_processing_results(processing_results);

    summary.display_summary();

    if summary.has_state_changes() {
        save_state_to_file(state, state_file_path);
    } else {
        log_info!("[Tools] No new tools installed or state changes detected.");
    }

    eprintln!();
}

/// Executes additional commands after installation
pub fn execute_post_installation_commands(
    installer_prefix: &str,
    tool_entry: &ToolEntry,
    working_directory: &std::path::Path,
) -> Option<Vec<String>> {
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

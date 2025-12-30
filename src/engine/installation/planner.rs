use crate::core::timestamps::{format_duration, is_timestamp_older_than, time_since};
use crate::log_warn;
use crate::schemas::config_manager::{
    ConfigurationEvaluationResult, ConfigurationManagerProcessor,
};
use crate::schemas::state_file::ToolState;
use crate::schemas::tools_enums::{ConfigurationAction, ToolAction, VersionAction};
use crate::schemas::tools_types::{InstallationConfiguration, ToolEntry};

/// The InstallationPlanner is responsible for determining the actions required for a tool.
/// It encapsulates the business logic for version checks, update thresholds, and configuration evaluations.
pub struct InstallationPlanner<'a> {
    configuration: &'a InstallationConfiguration,
    config_processor: ConfigurationManagerProcessor,
}

impl<'a> InstallationPlanner<'a> {
    pub fn new(
        configuration: &'a InstallationConfiguration,
        config_processor: ConfigurationManagerProcessor,
    ) -> Self {
        Self {
            configuration,
            config_processor,
        }
    }

    /// Determines the high-level action to be taken for a specific tool.
    /// This is the primary decision-making method.
    pub fn determine_required_action(
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

                // Perform comprehensive configuration evaluation
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
                            "[SDB::Tools] Error evaluating configuration for {}: {}. Assuming update needed.",
                            tool.name,
                            e
                        );
                        // Create a default evaluation that assumes update is needed
                        Some(ConfigurationEvaluationResult {
                            needs_update: true,
                            current_source_sha: String::new(),
                            current_destination_sha: None,
                            reason: Some(format!("[SDB::Tools] Evaluation error: {e}")),
                        })
                    }
                };

                // Convert evaluation result to ConfigurationAction
                let config_action = match &config_evaluation {
                    Some(eval) if eval.needs_update => ConfigurationAction::Update,
                    Some(eval) => {
                        ConfigurationAction::Skip(eval.reason.clone().unwrap_or_else(|| {
                            "[SDB::Tools::Configuration] Configuration up-to-date".to_string()
                        }))
                    }
                    None => ConfigurationAction::Skip(
                        "[SDB::Tools::Configuration] Configuration disabled".to_string(),
                    ),
                };

                // Combine the individual actions into a final `ToolAction`.
                let final_action = self.combine_actions(version_action, config_action);
                (final_action, config_evaluation)
            }
        }
    }

    /// Analyzes the version requirements for a tool to determine if an update is needed.
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
        if is_latest_version_scenario
            && !self.configuration.force_update_enabled
            && current_state
                .last_updated
                .as_ref()
                .map(|ts| {
                    !is_timestamp_older_than(ts, &self.configuration.update_threshold_duration.0)
                })
                .unwrap_or(false)
        {
            let last_updated_timestamp = current_state.last_updated.as_ref().unwrap(); // Safe unwrap
            let time_since_update =
                time_since(last_updated_timestamp).unwrap_or_else(|| "recently".to_string());
            let threshold_description =
                format_duration(&self.configuration.update_threshold_duration.0);

            return VersionAction::Skip(format!(
                "[SDB::Tools] Version 'latest' updated {time_since_update} (within {threshold_description} threshold)"
            ));
        }

        // The tool is older than the threshold, so it needs an update.
        if is_latest_version_scenario && !self.configuration.force_update_enabled {
            return VersionAction::Update;
        }

        // Handle specific version logic.
        if let Some(required_version) = &tool.version {
            let normalized_required = Self::normalize_version(required_version);
            let normalized_current = Self::normalize_version(&current_state.version);

            if required_version != "latest" && normalized_current == normalized_required {
                // Skip because the specified version is already installed.
                return VersionAction::Skip(
                    "[SDB::Tools] specified version already installed".to_string(),
                );
            }
        } else if !current_state.version.is_empty() && current_state.version != "latest" {
            // A tool with no specified version is already installed with a specific version.
            return VersionAction::Skip(
                "[SDB::Tools] Version not specified but tool is installed".to_string(),
            );
        }

        // Default case: a specific version is required and not installed, or it's a forced update.
        VersionAction::Update
    }

    /// Combines the individual `VersionAction` and `ConfigurationAction` into a single `ToolAction`.
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

    /// Helper function to normalize a version string by removing a leading 'v'.
    fn normalize_version(version: &str) -> &str {
        version.strip_prefix('v').unwrap_or(version)
    }
}

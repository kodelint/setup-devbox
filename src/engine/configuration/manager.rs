use crate::schemas::config_manager::ConfigurationManagerState;
use crate::schemas::tools_types::{InstallationConfiguration, ToolConfig};
use crate::schemas::tools_enums::SdbDuration;
use chrono::Duration;

// ============================================================================
// INSTALLATION CONFIGURATION IMPLEMENTATION
// ============================================================================

impl InstallationConfiguration {
    pub(crate) fn new(tools_config: &ToolConfig, force_update: bool) -> Self {
        let update_threshold_duration = if force_update {
            Duration::seconds(0)
        } else {
            tools_config
                .update_latest_only_after
                .as_ref()
                .map(|d| d.0)
                .unwrap_or_else(|| Duration::days(0))
        };

        Self {
            update_threshold_duration: SdbDuration(update_threshold_duration),
            force_update_enabled: force_update,
        }
    }
}

impl ConfigurationManagerState {
    pub fn new(
        enabled: bool,
        tools_configuration_paths: Vec<String>,
        source_sha: String,
        destination_sha: String,
    ) -> Self {
        Self {
            enabled,
            tools_configuration_paths,
            source_configuration_sha: source_sha,
            destination_configuration_sha: destination_sha,
        }
    }
}
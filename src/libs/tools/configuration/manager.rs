use crate::libs::utilities::timestamps::parse_duration;
use crate::schemas::configuration_management::{ConfigurationManager, ConfigurationManagerState};
use crate::schemas::tools::{InstallationConfiguration, ToolConfig};
use chrono::Duration;
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
// CONFIGURATION MANAGER IMPLEMENTATIONS
// ============================================================================

/// ### Struct Implementations
/// ConfigurationManager struct defines the core configuration for a single tool's configuration management.
/// It's typically part of a larger schema for a tool and controls whether configuration
/// syncing is enabled and where the destination file should be placed.
impl Default for ConfigurationManager {
    /// Provides a default state for the `ConfigurationManager`, with configuration
    /// disabled by default.
    ///
    /// ## Default Behavior
    /// - `enabled: false` - Configuration management is disabled
    /// - `tools_configuration_paths: Vec::new()` - Empty path list
    ///
    /// This ensures that tools without explicit configuration management settings
    /// won't have their configuration files managed by the system.
    fn default() -> Self {
        Self {
            enabled: false,
            tools_configuration_paths: Vec::new(),
        }
    }
}

/// ConfigurationManagerState struct holds the state of a configuration after it has been processed.
/// The SHA hashes are crucial for detecting changes in both source and destination
/// files between runs.
impl ConfigurationManagerState {
    /// Creates a new `ConfigurationManagerState` with the provided details.
    ///
    /// ## Parameters
    /// - `enabled`: Whether configuration management is enabled for this tool
    /// - `tools_configuration_paths`: List of destination configuration file paths
    /// - `source_sha`: SHA-256 hash of the source configuration file content
    /// - `destination_sha`: SHA-256 hash of the destination configuration file content
    ///
    /// ## Returns
    /// A new `ConfigurationManagerState` instance with the provided values.
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

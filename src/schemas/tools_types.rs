use crate::engine::configuration::processor::{
    ConfigurationManager, ConfigurationManagerProcessor,
};
use crate::schemas::state_file::DevBoxState;
use crate::schemas::tools_enums::{SdbDuration, SourceType, ToolEntryError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    pub update_latest_only_after: Option<SdbDuration>,
    pub tools: Vec<ToolEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ToolEntry {
    pub name: String,
    pub version: Option<String>,
    pub source: SourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rename_to: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_path_after_extract: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_installation_hooks: Option<Vec<String>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "ConfigurationManager::is_default")]
    pub configuration_manager: ConfigurationManager,
}

impl ToolEntry {
    pub fn validate(&self) -> Result<(), ToolEntryError> {
        if self.name.trim().is_empty() {
            return Err(ToolEntryError::MissingField("name"));
        }
        Ok(())
    }
}

pub struct ToolInstallationOrchestrator<'a> {
    pub state: &'a mut DevBoxState,
    pub configuration: &'a InstallationConfiguration,
    pub config_processor: ConfigurationManagerProcessor,
}

#[derive(Debug)]
pub struct InstallationConfiguration {
    pub update_threshold_duration: SdbDuration,
    pub force_update_enabled: bool,
}

pub struct InstallationSummary {
    pub installed_tools: Vec<String>,
    pub updated_tools: Vec<String>,
    pub configuration_updated_tools: Vec<String>,
    pub skipped_tools: Vec<(String, String)>,
    pub configuration_skipped_tools: Vec<(String, String)>,
    pub failed_tools: Vec<(String, String)>,
}

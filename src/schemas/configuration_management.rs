use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationManager {
    pub enabled: bool,
    pub tools_configuration_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationManagerState {
    pub enabled: bool,
    pub tools_configuration_path: String,
    pub source_configuration_sha: String,
    pub destination_configuration_sha: String,
}

pub struct ConfigurationManagerProcessor {
    pub(crate) config_base_path: PathBuf,
}

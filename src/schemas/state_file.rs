// Application State File Schema (`state.json`)
// Defines the structure of `setup-devbox`'s internal state file,
// used to track installed tools and applied configurations.
// This file is automatically managed by the application and should not be manually edited.

use crate::libs::configuration_manager::ConfigurationManagerState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The complete structure of `state.json`, representing `setup-devbox`'s persistent memory.
/// This file tracks what has been installed and configured to enable updates and management.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DevBoxState {
    /// Records information about installed tools (keyed by tool name)
    pub tools: HashMap<String, ToolState>,
    /// Records applied system settings (keyed by setting domain:key)
    pub settings: HashMap<String, SettingState>,
    /// Stores information about installed fonts (keyed by font name)
    pub fonts: HashMap<String, FontState>,
}

/// Stores detailed information about each installed tool.
/// This information is used for updates, reinstalls, and cleanup operations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolState {
    /// Exact version of the installed tool
    pub version: String,
    /// File system path where the tool is installed
    pub install_path: String,
    /// True if managed by `setup-devbox` (vs pre-installed or manually installed)
    pub installed_by_devbox: bool,
    /// Method used for installation (e.g., "github-release", "brew", "cargo")
    pub install_method: String,
    /// New name if the executable was renamed during installation
    pub renamed_to: Option<String>,
    /// Type of package (e.g., "binary", "go-module", "rust-binary")
    pub package_type: String,
    /// GitHub repository if applicable (for GitHub source installations)
    pub repo: Option<String>,
    /// Specific GitHub tag/release if applicable (for GitHub source installations)
    pub tag: Option<String>,
    /// Options passed to the installer during installation
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// Original download URL for direct URL installations
    #[serde(default)]
    pub url: Option<String>,
    /// Timestamp of when the tool was last updated
    pub last_updated: Option<String>,
    /// Path to executable within extracted archive, relative to `install_path`
    pub executable_path_after_extract: Option<String>,
    /// Records any additional commands that were executed during installation.
    /// This is stored for reference and potential cleanup/uninstall operations.
    #[serde(default)]
    pub additional_cmd_executed: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_manager: Option<ConfigurationManagerState>,
}

impl ToolState {
    // Helper method to update configuration manager state
    pub fn set_configuration_manager(&mut self, config_state: ConfigurationManagerState) {
        self.configuration_manager = Some(config_state);
    }

    // Helper method to get configuration manager state
    pub fn get_configuration_manager(&self) -> Option<&ConfigurationManagerState> {
        self.configuration_manager.as_ref()
    }
}

/// Records the state of an applied system setting.
/// This information is used to track what settings have been configured.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingState {
    /// Setting's domain (e.g., "com.apple.finder")
    pub domain: String,
    /// Setting's key within the domain
    pub key: String,
    /// Value that was set for this setting
    pub value: String,
    /// Type of the value (e.g., "bool", "string")
    pub value_type: String,
}

/// Records the state of an installed font.
/// This information is used for font management and updates.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontState {
    /// Name of the font family
    pub name: String,
    /// Original download URL for the font
    pub url: String,
    /// List of installed font files (filenames)
    pub files: Vec<String>,
    /// Installed version of the font
    pub version: String,
    /// GitHub repository if applicable (for GitHub source installations)
    #[serde(default)]
    pub repo: Option<String>,
    /// Specific GitHub tag/release if applicable (for GitHub source installations)
    #[serde(default)]
    pub tag: Option<String>,
}

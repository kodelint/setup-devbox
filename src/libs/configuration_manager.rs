use crate::libs::utilities::misc_utils::expand_tilde;
pub(crate) use crate::schemas::sdb_schema::{
    ConfigurationManager, ConfigurationManagerProcessor, ConfigurationManagerState,
};
use crate::{log_debug, log_info, log_warn};
use colored::Colorize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::{env, fs};
use toml::Value as TomlValue;

impl Default for ConfigurationManager {
    fn default() -> Self {
        Self {
            enabled: false,
            tools_configuration_path: String::new(),
        }
    }
}

impl ConfigurationManagerState {
    pub fn new(
        enabled: bool,
        tools_configuration_path: String,
        source_sha: String,
        destination_sha: String,
    ) -> Self {
        Self {
            enabled,
            tools_configuration_path,
            source_configuration_sha: source_sha,
            destination_configuration_sha: destination_sha,
        }
    }
}

impl ConfigurationManagerProcessor {
    /// Creates a new configuration manager processor with proper path resolution
    pub fn new(config_base_path: Option<PathBuf>) -> Self {
        let base_path = Self::resolve_config_base_path(config_base_path);
        Self {
            config_base_path: base_path,
        }
    }

    /// Resolves the configuration base path using environment variables and fallbacks
    fn resolve_config_base_path(config_base_path: Option<PathBuf>) -> PathBuf {
        // Priority 1: Environment variable SDB_TOOLS_SOURCE_CONFIG_PATH
        if let Ok(env_path) = env::var("SDB_TOOLS_SOURCE_CONFIG_PATH") {
            match Self::expand_path(&env_path) {
                Ok(expanded_path) => return expanded_path,
                Err(_) => {
                    log_warn!(
                        "[Tools] Failed to expand \"{}\", using fallback",
                        "SDB_TOOLS_SOURCE_CONFIG_PATH".blue()
                    );
                }
            }
        }

        // Priority 2: Environment variable SDB_CONFIG_PATH
        if let Ok(env_path) = env::var("SDB_CONFIG_PATH") {
            match Self::expand_path(&env_path) {
                Ok(expanded_path) => {
                    log_debug!(
                        "[Tools] Using \"{}\" as SDB Tools Configuration folder",
                        "SDB_CONFIG_PATH".blue()
                    );
                    return expanded_path.join("configs").join("tools");
                }
                Err(_) => {
                    log_warn!(
                        "[Tools] Failed to expand \"{}\", using fallback",
                        "SDB_CONFIG_PATH".blue()
                    );
                }
            }
        }

        // Priority 3: Provided parameter
        if let Some(path) = config_base_path {
            return path;
        }

        // Priority 4: Default home directory path
        dirs::home_dir()
            .map(|home| home.join(".setup-devbox").join("configs").join("tools"))
            // Priority 5: Fallback relative path
            .unwrap_or_else(|| PathBuf::from(".setup-devbox/configs/tools"))
    }

    /// Main entry point for processing tool configuration
    pub fn process_tool_configuration(
        &self,
        tool_name: &str,
        config_manager: &ConfigurationManager,
        existing_state: Option<&ConfigurationManagerState>,
    ) -> Result<Option<ConfigurationManagerState>, Box<dyn std::error::Error>> {
        if !config_manager.enabled {
            log_debug!(
                "[Tools] Configuration manager disabled for tool: {}",
                tool_name.cyan()
            );
            return Ok(None);
        }

        let source_path = self.build_source_path(tool_name);
        let destination_path = Self::expand_path(&config_manager.tools_configuration_path)?;

        if !source_path.exists() {
            log_warn!(
                "[Tools] Source configuration not found for {}: {}",
                tool_name.red(),
                source_path.display().to_string().yellow()
            );
            return Ok(None);
        }

        let current_source_sha = self.calculate_file_sha(&source_path)?;

        if !self.needs_configuration_update(
            &current_source_sha,
            &destination_path,
            existing_state,
        )? {
            log_info!(
                "[Tools] Configuration for {} is up to date, skipping",
                tool_name.green()
            );
            return Ok(existing_state.cloned());
        }

        self.update_configuration_file(&source_path, &destination_path)?;

        let destination_sha = self.calculate_file_sha(&destination_path)?;

        Ok(Some(ConfigurationManagerState::new(
            true,
            config_manager.tools_configuration_path.clone(),
            current_source_sha,
            destination_sha,
        )))
    }

    /// Determines if a configuration needs updating based on current state and file hashes
    pub fn evaluate_configuration_change_needed(
        &self,
        tool_name: &str,
        config_manager: &ConfigurationManager,
        existing_state: Option<&ConfigurationManagerState>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Configuration disabled - return false (no change needed)
        if !config_manager.enabled {
            return Ok(false);
        }

        // Configuration newly enabled
        if existing_state.is_none() {
            log_debug!(
                "[Tools] Configuration newly enabled for {} - change detected",
                tool_name
            );
            return Ok(true);
        }

        let existing_config = existing_state.unwrap();

        // Check if configuration path changed
        if existing_config.tools_configuration_path != config_manager.tools_configuration_path {
            log_debug!(
                "[Tools] Configuration path changed for {} - change detected",
                tool_name
            );
            return Ok(true);
        }

        // Check if destination file exists
        let destination_path = Self::expand_path(&config_manager.tools_configuration_path)?;
        if !destination_path.exists() {
            log_debug!(
                "[Tools] Destination file missing for {} - change detected",
                tool_name
            );
            return Ok(true);
        }

        // Check if source file exists and compare hashes
        let source_path = self.build_source_path(tool_name);
        if !source_path.exists() {
            log_debug!(
                "[Tools] Source file missing for {} - no change needed",
                tool_name
            );
            return Ok(false);
        }

        let current_source_sha = self.calculate_file_sha(&source_path)?;
        self.needs_configuration_update(&current_source_sha, &destination_path, existing_state)
    }

    /// Builds the source configuration file path for a tool
    pub(crate) fn build_source_path(&self, tool_name: &str) -> PathBuf {
        self.config_base_path.join(format!("{}.toml", tool_name))
    }

    /// Determines if configuration update is needed based on SHA comparison
    pub(crate) fn needs_configuration_update(
        &self,
        current_source_sha: &str,
        destination_path: &Path,
        existing_state: Option<&ConfigurationManagerState>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let existing_state = match existing_state {
            Some(state) => state,
            None => {
                log_debug!("[Tools] No existing configuration state, update needed");
                return Ok(true);
            }
        };

        // Check if source file changed
        if current_source_sha != existing_state.source_configuration_sha {
            log_debug!(
                "[Tools] Source file changed - recorded: {}, current: {}",
                existing_state.source_configuration_sha.red(),
                current_source_sha.green()
            );
            return Ok(true);
        }

        // Check if destination file exists
        if !destination_path.exists() {
            log_warn!(
                "[Tools] Destination file missing: {}, will recreate",
                destination_path.display().to_string().yellow()
            );
            return Ok(true);
        }

        // Check if destination file was modified externally
        let current_destination_sha = self.calculate_file_sha(destination_path)?;
        if current_destination_sha != existing_state.destination_configuration_sha {
            log_debug!(
                "[Tools] Destination file modified - recorded: {}, current: {}",
                existing_state.destination_configuration_sha.red(),
                current_destination_sha.yellow()
            );
            return Ok(true);
        }

        Ok(false)
    }

    /// Updates the configuration file from source to destination
    fn update_configuration_file(
        &self,
        source_path: &Path,
        destination_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log_debug!(
            "[Tools] Updating configuration from {} to {}",
            source_path.display().to_string().blue(),
            destination_path.display().to_string().blue()
        );

        // Create destination directory if needed
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read and convert source configuration
        let source_content = fs::read_to_string(source_path)?;
        let toml_value: TomlValue = toml::from_str(&source_content)?;
        let converted_content =
            self.convert_toml_to_target_format(&toml_value, destination_path)?;

        // Write converted content
        fs::write(destination_path, converted_content)?;
        log_info!(
            "[Tools] Configuration written to: {}",
            destination_path.display().to_string().green()
        );

        Ok(())
    }

    /// Convert TOML to target format based on file extension
    fn convert_toml_to_target_format(
        &self,
        toml_value: &TomlValue,
        destination_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let extension = destination_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "json" => {
                let json_value = self.toml_to_json(toml_value)?;
                Ok(serde_json::to_string_pretty(&json_value)?)
            }
            "yaml" | "yml" => {
                let yaml_value = self.toml_to_yaml(toml_value)?;
                Ok(serde_yaml::to_string(&yaml_value)?)
            }
            "toml" => Ok(toml::to_string_pretty(toml_value)?),
            _ => Ok(self.toml_to_key_value(toml_value)),
        }
    }

    /// Convert TOML to JSON
    fn toml_to_json(
        &self,
        toml_value: &TomlValue,
    ) -> Result<JsonValue, Box<dyn std::error::Error>> {
        let json_str = serde_json::to_string(toml_value)?;
        Ok(serde_json::from_str(&json_str)?)
    }

    /// Convert TOML to YAML
    fn toml_to_yaml(
        &self,
        toml_value: &TomlValue,
    ) -> Result<YamlValue, Box<dyn std::error::Error>> {
        let json_value = self.toml_to_json(toml_value)?;
        Ok(serde_yaml::to_value(json_value)?)
    }

    /// Convert TOML to KEY=VALUE format with smart quoting
    fn toml_to_key_value(&self, toml_value: &TomlValue) -> String {
        let mut result = Vec::new();
        self.flatten_toml_to_key_value(toml_value, String::new(), &mut result);
        result.join("\n")
    }

    /// Recursively flatten TOML to KEY=VALUE pairs with smart quoting
    fn flatten_toml_to_key_value(
        &self,
        value: &TomlValue,
        prefix: String,
        result: &mut Vec<String>,
    ) {
        match value {
            TomlValue::Table(table) => {
                for (key, val) in table {
                    let new_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}_{}", prefix, key)
                    };
                    self.flatten_toml_to_key_value(val, new_prefix, result);
                }
            }
            TomlValue::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_prefix = format!("{}_{}", prefix, i);
                    self.flatten_toml_to_key_value(val, new_prefix, result);
                }
            }
            _ => {
                let value_str = match value {
                    TomlValue::String(s) => {
                        if self.needs_quotes(s) {
                            format!("\"{}\"", s)
                        } else {
                            s.clone()
                        }
                    }
                    TomlValue::Integer(i) => i.to_string(),
                    TomlValue::Float(f) => f.to_string(),
                    TomlValue::Boolean(b) => b.to_string(),
                    TomlValue::Datetime(dt) => dt.to_string(),
                    _ => String::new(),
                };
                result.push(format!("{}={}", prefix, value_str));
            }
        }
    }

    /// Determine if a string value needs quotes
    fn needs_quotes(&self, value: &str) -> bool {
        if value.is_empty() {
            return true;
        }

        // Check for whitespace
        if value.contains(char::is_whitespace) {
            return true;
        }

        // Check for special characters that might need quoting
        if value.contains(|c: char| {
            matches!(
                c,
                '=' | '#' | '"' | '\'' | '\\' | ':' | ';' | ',' | '[' | ']' | '{' | '}'
            )
        }) {
            return true;
        }

        // Check if it looks like a number or boolean
        if value.parse::<f64>().is_ok()
            || value.parse::<i64>().is_ok()
            || matches!(value, "true" | "false" | "yes" | "no")
        {
            return true;
        }

        // Check for special prefixes/suffixes
        value.starts_with('%') || value.ends_with('%') || value.starts_with('#')
    }

    /// Calculate SHA256 hash of a file
    pub fn calculate_file_sha(&self, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let content = fs::read(path)?;
        Ok(self.calculate_data_sha(&content))
    }

    /// Calculate SHA256 hash of data
    fn calculate_data_sha(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Expand environment variables and tilde in path
    pub fn expand_path(path: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Handle $HOME environment variable
        let expanded = if path.starts_with("$HOME") {
            if let Some(home_dir) = dirs::home_dir() {
                path.replace("$HOME", home_dir.to_str().unwrap_or(""))
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };

        // Handle tilde expansion
        let expanded_path = expand_tilde(&expanded);

        // Handle other environment variables
        if expanded.contains('$') {
            let path_string = expanded_path.to_string_lossy().to_string();
            let fully_expanded = shellexpand::full(&path_string)?;
            Ok(PathBuf::from(fully_expanded.as_ref()))
        } else {
            Ok(expanded_path)
        }
    }
}

//! This module is responsible for managing and synchronizing tool configuration files.
//! It handles tasks such as locating source configuration files, comparing their state
//! with existing destination files using SHA256 hashes, and updating the destination
//! file only when a change is detected. It also supports converting configuration
//! files between different formats (e.g., TOML to JSON, YAML, or a custom KEY=VALUE format).

use colored::Colorize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::{env, fs};
use toml::Value as TomlValue;
// The `expand_tilde` function is imported from a separate utilities library
use crate::libs::utilities::misc_utils::expand_tilde;
pub(crate) use crate::schemas::configuration_management::{
    ConfigurationManager, ConfigurationManagerProcessor, ConfigurationManagerState,
};
use crate::{log_debug, log_info, log_warn};

/// ### Struct Implementations
/// This struct defines the core configuration for a single tool's configuration management.
/// It's typically part of a larger schema for a tool and controls whether configuration
/// syncing is enabled and where the destination file should be placed.
impl Default for ConfigurationManager {
    /// Provides a default state for the `ConfigurationManager`, with configuration
    /// disabled by default.
    fn default() -> Self {
        Self { enabled: false, tools_configuration_path: String::new() }
    }
}

/// This struct holds the state of a configuration after it has been processed.
/// The SHA hashes are crucial for detecting changes in both source and destination
/// files between runs.
impl ConfigurationManagerState {
    /// Creates a new `ConfigurationManagerState` with the provided details.
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

/// The `ConfigurationManagerProcessor` is the primary component for all configuration
/// management logic. It contains methods for resolving paths, processing configurations,
/// and handling file updates and conversions.
impl ConfigurationManagerProcessor {
    /// Creates a new configuration manager processor with a properly resolved base path.
    ///
    /// The `config_base_path` parameter allows a custom path to be provided, but if not,
    /// the method will fall back to environment variables and default paths.
    pub fn new(config_base_path: Option<PathBuf>) -> Self {
        let base_path = Self::resolve_config_base_path(config_base_path);
        Self { config_base_path: base_path }
    }

    /// Resolves the configuration base path using a prioritized search order.
    ///
    /// The resolution logic follows these steps:
    /// 1. Check for the `SDB_TOOLS_SOURCE_CONFIG_PATH` environment variable.
    /// 2. Check for the `SDB_CONFIG_PATH` environment variable and append `configs/tools`.
    /// 3. Use the `config_base_path` parameter if it was provided.
    /// 4. Fall back to the default home directory path: `~/.setup-devbox/configs/tools`.
    /// 5. As a last resort, use a relative path: `.setup-devbox/configs/tools`.
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
                },
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
                    // The tools configurations are located in a subdirectory
                    return expanded_path.join("configs").join("tools");
                },
                Err(_) => {
                    log_warn!(
                        "[Tools] Failed to expand \"{}\", using fallback",
                        "SDB_CONFIG_PATH".blue()
                    );
                },
            }
        }

        // Priority 3: Provided parameter
        if let Some(path) = config_base_path {
            return path;
        }

        // Priority 4: Default home directory path
        // This is a common convention for configuration files on Linux/macOS
        dirs::home_dir()
            .map(|home| home.join(".setup-devbox").join("configs").join("tools"))
            // Priority 5: Fallback relative path
            .unwrap_or_else(|| PathBuf::from(".setup-devbox/configs/tools"))
    }

    /// The main entry point for processing a tool's configuration.
    ///
    /// This method orchestrates the entire process: it checks if a configuration needs
    /// an update and performs the update if necessary. It returns the new state of the
    /// configuration, which can be stored for future comparisons.
    ///
    /// - `tool_name`: The name of the tool, used to find the source file (e.g., "my-tool.toml").
    /// - `config_manager`: The configuration settings for the tool.
    /// - `existing_state`: The previous state of the configuration, used for change detection.
    pub fn process_tool_configuration(
        &self,
        tool_name: &str,
        config_manager: &ConfigurationManager,
        existing_state: Option<&ConfigurationManagerState>,
    ) -> Result<Option<ConfigurationManagerState>, Box<dyn std::error::Error>> {
        // If configuration is disabled, there's nothing to do.
        if !config_manager.enabled {
            log_debug!("[Tools] Configuration manager disabled for tool: {}", tool_name.cyan());
            return Ok(None);
        }

        let source_path = self.build_source_path(tool_name);
        // Expand the destination path to handle tildes and environment variables
        let destination_path = Self::expand_path(&config_manager.tools_configuration_path)?;

        // If the source file doesn't exist, we can't do anything.
        if !source_path.exists() {
            log_warn!(
                "[Tools] Source configuration not found for {}: {}",
                tool_name.red(),
                source_path.display().to_string().yellow()
            );
            return Ok(None);
        }

        // Calculate the SHA of the source file to compare against the recorded state.
        let current_source_sha = self.calculate_file_sha(&source_path)?;

        // Check if an update is needed. This is the core of the change detection logic.
        if !self.needs_configuration_update(
            &current_source_sha,
            &destination_path,
            existing_state,
        )? {
            log_info!("[Tools] Configuration for {} is up to date, skipping", tool_name.green());
            // If no update is needed, return the existing state.
            return Ok(existing_state.cloned());
        }

        // If an update is needed, perform the file copy and conversion.
        self.update_configuration_file(&source_path, &destination_path)?;

        // Calculate the SHA of the *new* destination file.
        let destination_sha = self.calculate_file_sha(&destination_path)?;

        // Return the new state so it can be saved for the next run.
        Ok(Some(ConfigurationManagerState::new(
            true,
            config_manager.tools_configuration_path.clone(),
            current_source_sha,
            destination_sha,
        )))
    }

    /// Determines if a configuration needs to be updated. This is a public method
    /// that can be used to check for changes without performing the update.
    ///
    /// The checks are similar to `process_tool_configuration`, but without the file
    /// modification step.
    pub fn evaluate_configuration_change_needed(
        &self,
        tool_name: &str,
        config_manager: &ConfigurationManager,
        existing_state: Option<&ConfigurationManagerState>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Configuration disabled means no changes are ever needed.
        if !config_manager.enabled {
            return Ok(false);
        }

        // If there's no existing state, it's a new configuration, so an update is needed.
        if existing_state.is_none() {
            log_debug!("[Tools] Configuration newly enabled for {} - change detected", tool_name);
            return Ok(true);
        }

        let existing_config = existing_state.unwrap();

        // Check if the destination path has been changed in the tool's config.
        if existing_config.tools_configuration_path != config_manager.tools_configuration_path {
            log_debug!("[Tools] Configuration path changed for {} - change detected", tool_name);
            return Ok(true);
        }

        // Check if the destination file exists. It might have been deleted externally.
        let destination_path = Self::expand_path(&config_manager.tools_configuration_path)?;
        if !destination_path.exists() {
            log_debug!("[Tools] Destination file missing for {} - change detected", tool_name);
            return Ok(true);
        }

        // Check if the source file exists. It's a required prerequisite.
        let source_path = self.build_source_path(tool_name);
        if !source_path.exists() {
            log_debug!("[Tools] Source file missing for {} - no change needed", tool_name);
            return Ok(false);
        }

        // Finally, delegate to the core change detection logic.
        let current_source_sha = self.calculate_file_sha(&source_path)?;
        self.needs_configuration_update(&current_source_sha, &destination_path, existing_state)
    }

    /// Constructs the full path to the source configuration file for a given tool.
    ///
    /// Source files are expected to be in TOML format and named after the tool.
    /// For example, a tool named "my-tool" would have its configuration at
    /// `[config_base_path]/my-tool.toml`.
    pub(crate) fn build_source_path(&self, tool_name: &str) -> PathBuf {
        self.config_base_path.join(format!("{}.toml", tool_name))
    }

    /// The central method for determining if an update is required.
    ///
    /// It compares the current state of the source and destination files with the
    /// recorded state from the `existing_state` struct.
    ///
    /// The logic checks for three conditions that would necessitate an update:
    /// 1. There is no existing state (first run).
    /// 2. The source file's SHA has changed, meaning the source configuration was modified.
    /// 3. The destination file is missing.
    /// 4. The destination file's SHA has changed, meaning it was modified externally.
    pub(crate) fn needs_configuration_update(
        &self,
        current_source_sha: &str,
        destination_path: &Path,
        existing_state: Option<&ConfigurationManagerState>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Condition 1: If there's no existing state, an update is always needed.
        let existing_state = match existing_state {
            Some(state) => state,
            None => {
                log_debug!("[Tools] No existing configuration state, update needed");
                return Ok(true);
            },
        };

        // Condition 2: Check if source file changed by comparing SHAs.
        if current_source_sha != existing_state.source_configuration_sha {
            log_debug!(
                "[Tools] Source file changed - recorded: {}, current: {}",
                existing_state.source_configuration_sha.red(),
                current_source_sha.green()
            );
            return Ok(true);
        }

        // Condition 3: Check if destination file exists.
        if !destination_path.exists() {
            log_warn!(
                "[Tools] Destination file missing: {}, will recreate",
                destination_path.display().to_string().yellow()
            );
            return Ok(true);
        }

        // Condition 4: Check if destination file was modified externally by comparing SHAs.
        let current_destination_sha = self.calculate_file_sha(destination_path)?;
        if current_destination_sha != existing_state.destination_configuration_sha {
            log_debug!(
                "[Tools] Destination file modified - recorded: {}, current: {}",
                existing_state.destination_configuration_sha.red(),
                current_destination_sha.yellow()
            );
            return Ok(true);
        }

        // If none of the above conditions are met, the configuration is up-to-date.
        Ok(false)
    }

    /// Updates the destination configuration file from the source file.
    ///
    /// This method performs a read-convert-write operation. It reads the TOML source,
    /// converts it to the format specified by the destination file's extension,
    /// and writes the result.
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

        // Ensure the parent directory for the destination file exists.
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read the source TOML content.
        let source_content = fs::read_to_string(source_path)?;
        let toml_value: TomlValue = toml::from_str(&source_content)?;

        // Convert the TOML data to the target format based on the destination's file extension.
        let converted_content =
            self.convert_toml_to_target_format(&toml_value, destination_path)?;

        // Write the converted content to the destination file.
        fs::write(destination_path, converted_content)?;
        log_info!(
            "[Tools] Configuration written to: {}",
            destination_path.display().to_string().green()
        );

        Ok(())
    }

    /// Converts a TOML value into a target format based on the file extension.
    ///
    /// Supported formats:
    /// - `.json`: Converts TOML to pretty-printed JSON.
    /// - `.yaml` or `.yml`: Converts TOML to YAML.
    /// - `.toml`: Re-serializes the TOML value with pretty printing.
    /// - Any other extension: Flattens the TOML into a simple `KEY=VALUE` format.
    fn convert_toml_to_target_format(
        &self,
        toml_value: &TomlValue,
        destination_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let extension = destination_path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension.to_lowercase().as_str() {
            "json" => {
                let json_value = self.toml_to_json(toml_value)?;
                Ok(serde_json::to_string_pretty(&json_value)?)
            },
            "yaml" | "yml" => {
                let yaml_value = self.toml_to_yaml(toml_value)?;
                Ok(serde_yaml::to_string(&yaml_value)?)
            },
            "toml" => Ok(toml::to_string_pretty(toml_value)?),
            _ => Ok(self.toml_to_key_value(toml_value)),
        }
    }

    /// Converts a `TomlValue` into a `JsonValue`.
    ///
    /// This is done by serializing the TOML value to an intermediary JSON string
    /// and then deserializing it into a `serde_json::Value`. This approach leverages
    /// the `serde` framework's robust TOML-to-JSON mapping.
    fn toml_to_json(
        &self,
        toml_value: &TomlValue,
    ) -> Result<JsonValue, Box<dyn std::error::Error>> {
        // Serialize the TOML value to a JSON string.
        let json_str = serde_json::to_string(toml_value)?;
        // Deserialize the JSON string into a `JsonValue`.
        Ok(serde_json::from_str(&json_str)?)
    }

    /// Converts a `TomlValue` into a `YamlValue`.
    ///
    /// The conversion is done via an intermediary JSON representation. This is a common
    /// pattern when using `serde` for conversions between formats, as JSON is often
    /// the most straightforward intermediary format.
    fn toml_to_yaml(
        &self,
        toml_value: &TomlValue,
    ) -> Result<YamlValue, Box<dyn std::error::Error>> {
        let json_value = self.toml_to_json(toml_value)?;
        Ok(serde_yaml::to_value(json_value)?)
    }

    /// Converts a `TomlValue` into a flattened `KEY=VALUE` string.
    ///
    /// This is useful for simple configurations or shell scripts that
    /// expect environment-variable-style key-value pairs.
    /// It recursively flattens nested tables and arrays.
    fn toml_to_key_value(&self, toml_value: &TomlValue) -> String {
        let mut result = Vec::new();
        // Start the recursive flattening process with an empty prefix.
        self.flatten_toml_to_key_value(toml_value, String::new(), &mut result);
        result.join("\n")
    }

    /// A recursive helper function to flatten a `TomlValue` into key-value pairs.
    ///
    /// It handles different `TomlValue` types:
    /// - `Table`: Recurses on each key-value pair, appending the key to the prefix.
    /// - `Array`: Recurses on each element, appending the index to the prefix.
    /// - Other primitives: Formats the value into a string and pushes the final `KEY=VALUE` line.
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
            },
            TomlValue::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_prefix = format!("{}_{}", prefix, i);
                    self.flatten_toml_to_key_value(val, new_prefix, result);
                }
            },
            _ => {
                let value_str = match value {
                    TomlValue::String(s) => {
                        // Apply quoting if the string contains special characters or whitespace.
                        if self.needs_quotes(s) {
                            format!("\"{}\"", s)
                        } else {
                            s.clone()
                        }
                    },
                    TomlValue::Integer(i) => i.to_string(),
                    TomlValue::Float(f) => f.to_string(),
                    TomlValue::Boolean(b) => b.to_string(),
                    TomlValue::Datetime(dt) => dt.to_string(),
                    // Unhandled types will result in an empty string.
                    _ => String::new(),
                };
                result.push(format!("{}={}", prefix, value_str));
            },
        }
    }

    /// Determines if a string value should be wrapped in quotes when converted to `KEY=VALUE` format.
    ///
    /// This is a "smart quoting" function that checks for:
    /// - Empty strings
    /// - Whitespace
    /// - Special characters that might be misinterpreted by a shell (`=`, `#`, `"`, etc.)
    /// - Values that might be mistaken for numbers or booleans
    /// - Special prefixes or suffixes (`%` or `#`)
    fn needs_quotes(&self, value: &str) -> bool {
        if value.is_empty() {
            return true;
        }

        // Check for whitespace.
        if value.contains(char::is_whitespace) {
            return true;
        }

        // Check for special characters that might need quoting.
        if value.contains(|c: char| {
            matches!(c, '=' | '#' | '"' | '\'' | '\\' | ':' | ';' | ',' | '[' | ']' | '{' | '}')
        }) {
            return true;
        }

        // Check if it looks like a number or boolean.
        if value.parse::<f64>().is_ok()
            || value.parse::<i64>().is_ok()
            || matches!(value, "true" | "false" | "yes" | "no")
        {
            return true;
        }

        // Check for special prefixes/suffixes common in shell scripts.
        value.starts_with('%') || value.ends_with('%') || value.starts_with('#')
    }

    /// Calculates the SHA256 hash of a file's content.
    ///
    /// This is used to create a unique fingerprint of a file, which is essential
    /// for robust change detection.
    pub fn calculate_file_sha(&self, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        // Read the entire file content into memory.
        let content = fs::read(path)?;
        Ok(self.calculate_data_sha(&content))
    }

    /// Calculates the SHA256 hash of a byte slice.
    ///
    /// This is a utility function that performs the actual hashing logic.
    fn calculate_data_sha(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        // Format the resulting hash as a hexadecimal string.
        format!("{:x}", hasher.finalize())
    }

    /// Expands environment variables and tilde (`~`) in a given path string.
    ///
    /// This is a critical utility for ensuring that user-provided paths (like
    /// `~/config/my-file.txt` or `$HOME/config/my-file.txt`) are correctly resolved.
    pub fn expand_path(path: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Handle $HOME environment variable first.
        let expanded = if path.starts_with("$HOME") {
            if let Some(home_dir) = dirs::home_dir() {
                path.replace("$HOME", home_dir.to_str().unwrap_or(""))
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };

        // Handle tilde expansion. The `expand_tilde` function is from a utility library.
        let expanded_path = expand_tilde(&expanded);

        // Handle other environment variables using `shellexpand`.
        if expanded.contains('$') {
            let path_string = expanded_path.to_string_lossy().to_string();
            let fully_expanded = shellexpand::full(&path_string)?;
            Ok(PathBuf::from(fully_expanded.as_ref()))
        } else {
            Ok(expanded_path)
        }
    }
}

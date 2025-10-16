//! # Configuration Management System Implementation
//!
//! This module provides the core implementation for managing and synchronizing tool configuration files.
//! It handles the complete lifecycle of configuration management including discovery, change detection,
//! format conversion, and file synchronization between source and destination locations.
//!
//! ## Core Functionality
//!
//! - **Source File Discovery**: Locates configuration source files in designated directories
//! - **Change Detection**: Uses SHA-256 hashing to detect changes in both source and destination files
//! - **Format Conversion**: Converts TOML source files to various target formats (JSON, YAML, TOML, KEY=VALUE)
//! - **Smart Synchronization**: Only updates files when changes are detected to minimize I/O operations
//! - **Path Expansion**: Supports environment variables and tilde expansion in file paths
//! - **State Tracking**: Maintains persistent state to optimize future processing
//!
//! ## Performance Optimizations
//!
//! - SHA-256 hashing prevents unnecessary file I/O operations
//! - Cached evaluation results avoid duplicate hash calculations
//! - Only processes files when actual changes are detected
//! - Efficient format conversion with minimal intermediate representations

pub(crate) use crate::schemas::configuration_management::ConfigurationEvaluationResult;
pub(crate) use crate::schemas::configuration_management::{
    ConfigurationManager, ConfigurationManagerProcessor, ConfigurationManagerState,
};
use crate::schemas::path_resolver::PathResolver;
use crate::{log_debug, log_info, log_warn};
use colored::Colorize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;
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

// ============================================================================
// CONFIGURATION PROCESSOR IMPLEMENTATION
// ============================================================================

/// The `ConfigurationManagerProcessor` is the primary component for all configuration
/// management logic. It contains methods for resolving paths, processing configurations,
/// and handling file updates and conversions.
impl ConfigurationManagerProcessor {
    /// Creates a new configuration manager processor using the PathResolver.
    ///
    /// ## Parameters
    /// - `paths`: Reference to the PathResolver for accessing configuration paths
    ///
    /// ## Returns
    /// A new `ConfigurationManagerProcessor` instance with resolved base path.
    pub fn new(paths: &PathResolver) -> Self {
        let base_path = paths.tools_config_dir().to_path_buf();
        log_debug!(
            "[SDB::Tools::Configuration::ConfigurationManager] Configuration manager using tools config dir: {}",
            base_path.display()
        );
        Self {
            config_base_path: base_path,
        }
    }

    /// Comprehensive evaluation that returns cached results to avoid duplicate SHA calculations.
    /// This method replaces the separate `evaluate_configuration_change_needed` method.
    ///
    /// ## Parameters
    /// - `tool_name`: Name of the tool being processed
    /// - `config_manager`: Configuration settings for the tool
    /// - `existing_state`: Previous state information for change detection
    ///
    /// ## Returns
    /// `ConfigurationEvaluationResult` containing all evaluation information
    ///
    /// ## Errors
    /// Returns error if file reading or SHA calculation fails
    pub fn evaluate_configuration_requirements(
        &self,
        tool_name: &str,
        config_manager: &ConfigurationManager,
        existing_state: Option<&ConfigurationManagerState>,
    ) -> Result<ConfigurationEvaluationResult, Box<dyn std::error::Error>> {
        // Configuration disabled means no changes are ever needed.
        if !config_manager.enabled {
            return Ok(ConfigurationEvaluationResult {
                needs_update: false,
                current_source_sha: String::new(),
                current_destination_sha: None,
                reason: Some("configuration disabled".to_string()),
            });
        }

        let source_paths = self
            .build_configuration_source_paths(&config_manager.tools_configuration_paths, tool_name);
        let destination_paths =
            PathResolver::expand_paths(&config_manager.tools_configuration_paths)?;

        // If the source file doesn't exist, we can't do anything.
        let existing_source_paths: Vec<&PathBuf> =
            source_paths.iter().filter(|path| path.exists()).collect();

        if existing_source_paths.is_empty() {
            log_warn!(
                "[SDB::Tools::Configuration::ConfigurationManager] Source configuration not found for {}: {}",
                tool_name.red(),
                source_paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
                    .yellow()
            );
            return Ok(ConfigurationEvaluationResult {
                needs_update: false,
                current_source_sha: String::new(),
                current_destination_sha: None,
                reason: Some("source file not found".to_string()),
            });
        }

        // Calculate current SHA values once
        let current_source_sha = self.calculate_combined_files_sha(&source_paths)?;

        // Check if destination files exist and calculate their SHA
        let existing_destination_paths: Vec<&PathBuf> = destination_paths
            .iter()
            .filter(|path| path.exists())
            .collect();

        let current_destination_sha = if !existing_destination_paths.is_empty() {
            Some(self.calculate_combined_files_sha(&destination_paths)?)
        } else {
            None
        };

        // Perform the actual evaluation logic
        let (needs_update, reason) = self.evaluate_update_necessity(
            &current_source_sha,
            &current_destination_sha,
            &destination_paths,
            existing_state,
            tool_name,
        )?;

        Ok(ConfigurationEvaluationResult {
            needs_update,
            current_source_sha,
            current_destination_sha,
            reason,
        })
    }

    /// Internal method that contains the core logic for determining if an update is needed.
    /// This replaces the previous `needs_configuration_update` method with better organization.
    ///
    /// ## Parameters
    /// - `current_source_sha`: Current SHA-256 hash of source file content
    /// - `current_destination_sha`: Current SHA-256 hash of destination file content (if exists)
    /// - `destination_paths`: List of destination file paths
    /// - `existing_state`: Previous state information for comparison
    /// - `tool_name`: Name of the tool for logging purposes
    ///
    /// ## Returns
    /// Tuple of `(bool, Option<String>)` where:
    /// - `bool`: Whether update is needed
    /// - `Option<String>`: Reason for the decision
    ///
    /// ## Errors
    /// Returns error if state comparison or validation fails
    fn evaluate_update_necessity(
        &self,
        current_source_sha: &str,
        current_destination_sha: &Option<String>,
        destination_paths: &[PathBuf],
        existing_state: Option<&ConfigurationManagerState>,
        tool_name: &str,
    ) -> Result<(bool, Option<String>), Box<dyn std::error::Error>> {
        // If there's no existing state, an update is always needed.
        let existing_state = match existing_state {
            Some(state) => state,
            None => {
                log_debug!(
                    "[SDB::Tools::Configuration::ConfigurationManager] No existing configuration state for {}, update needed",
                    tool_name
                );
                return Ok((true, Some("no existing state".to_string())));
            }
        };

        // Check if source file changed by comparing SHAs.
        if current_source_sha != existing_state.source_configuration_sha {
            log_debug!(
                "[SDB::Tools::Configuration::ConfigurationManager] Source file changed for {} - recorded: {}, current: {}",
                tool_name,
                existing_state.source_configuration_sha.red(),
                current_source_sha.green()
            );
            return Ok((true, Some("source file changed".to_string())));
        }

        // Check if destination file exists.
        if current_destination_sha.is_none() {
            log_warn!(
                "[SDB::Tools::Configuration::ConfigurationManager] Destination file missing for {}: {}, will recreate",
                tool_name,
                destination_paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
                    .yellow()
            );
            return Ok((true, Some("destination file missing".to_string())));
        }

        // Check if destination file was modified externally by comparing SHAs.
        if let Some(dest_sha) = current_destination_sha {
            if *dest_sha != existing_state.destination_configuration_sha {
                log_debug!(
                    "[SDB::Tools::Configuration::ConfigurationManager] Destination file modified for {} - recorded: {}, current: {}",
                    tool_name,
                    existing_state.destination_configuration_sha.red(),
                    dest_sha.yellow()
                );
                return Ok((true, Some("destination file modified".to_string())));
            }
        }

        // If none of the above conditions are met, the configuration is up-to-date.
        Ok((false, Some("configuration up-to-date".to_string())))
    }

    /// The main entry point for processing a tool's configuration with cached evaluation.
    /// This method now accepts pre-computed evaluation results to avoid duplicate work.
    ///
    /// ## Parameters
    /// - `tool_name`: The name of the tool, used to find the source file (e.g., "my-tool.toml").
    /// - `config_manager`: The configuration settings for the tool.
    /// - `existing_state`: The previous state of the configuration, used for change detection.
    /// - `cached_evaluation`: Optional pre-computed evaluation to avoid duplicate SHA calculations.
    ///
    /// ## Returns
    /// `Ok(Some(ConfigurationManagerState))` if state was updated
    /// `Ok(None)` if no update was needed or configuration is disabled
    /// `Err` if processing failed
    ///
    /// ## Errors
    /// Returns error if file operations, format conversion, or SHA calculation fails
    pub fn process_tool_configuration(
        &self,
        tool_name: &str,
        config_manager: &ConfigurationManager,
        existing_state: Option<&ConfigurationManagerState>,
        cached_evaluation: Option<ConfigurationEvaluationResult>,
    ) -> Result<Option<ConfigurationManagerState>, Box<dyn std::error::Error>> {
        // If configuration is disabled, there's nothing to do.
        if !config_manager.enabled {
            log_debug!(
                "[SDB::Tools::Configuration::ConfigurationManager] Configuration manager disabled for tool: {}",
                tool_name.cyan()
            );
            return Ok(None);
        }

        // Use cached evaluation if available, otherwise perform fresh evaluation
        let evaluation = match cached_evaluation {
            Some(eval) => eval,
            None => {
                self.evaluate_configuration_requirements(tool_name, config_manager, existing_state)?
            }
        };

        // If no update is needed, return the existing state.
        if !evaluation.needs_update {
            if let Some(reason) = &evaluation.reason {
                log_info!(
                    "[SDB::Tools::Configuration::ConfigurationManager] Configuration for {} is up to date ({}), skipping",
                    tool_name.green(),
                    reason
                );
            }
            return Ok(existing_state.cloned());
        }

        // If an update is needed, perform the file copy and conversion.
        let source_paths = self
            .build_configuration_source_paths(&config_manager.tools_configuration_paths, tool_name);
        let destination_paths =
            PathResolver::expand_paths(&config_manager.tools_configuration_paths)?;

        self.update_configuration_file(&source_paths, &destination_paths)?;

        // Use the cached destination SHA if available, otherwise calculate it
        let destination_sha = match evaluation.current_destination_sha {
            Some(sha) if !evaluation.needs_update => sha,
            _ => self.calculate_combined_files_sha(&destination_paths)?,
        };

        // Return the new state so it can be saved for the next run.
        Ok(Some(ConfigurationManagerState::new(
            true,
            config_manager.tools_configuration_paths.clone(),
            evaluation.current_source_sha,
            destination_sha,
        )))
    }

    /// Constructs the full path to the source configuration file for a given tool.
    ///
    /// Source files are expected to be in TOML format and named after the tool.
    /// For example, a tool named "tool_name" would have its configuration at
    /// `[config_base_path]/tool_name/config_name.toml`.
    ///
    /// ## Parameters
    /// - `destination_paths`: List of destination configuration file paths
    /// - `tool_name`: Name of the tool being processed
    ///
    /// ## Returns
    /// Vector of `PathBuf` objects representing source file paths
    fn build_configuration_source_paths(
        &self,
        destination_paths: &[String],
        tool_name: &str,
    ) -> Vec<PathBuf> {
        destination_paths
            .iter()
            .map(|destination_path| {
                // Refinement: Use rsplit().next() instead of split().last() for efficiency.
                let source_config_filename = destination_path
                    .rsplit('/') // Start splitting from the end
                    .next() // Get the first item from the reverse iteration (the filename)
                    .expect("Destination path must contain a filename");

                let filename = if source_config_filename.contains('.') {
                    source_config_filename.split('.').next().unwrap()
                } else {
                    source_config_filename
                };

                self.config_base_path
                    .join(tool_name)
                    .join(format!("{filename}.toml"))
            })
            .collect()
    }

    /// Updates the destination configuration file from the source file.
    ///
    /// This method performs a read-convert-write operation. It reads the TOML source,
    /// converts it to the format specified by the destination file's extension,
    /// and writes the result.
    ///
    /// ## Parameters
    /// - `source_paths`: List of source file paths to read from
    /// - `destination_paths`: List of destination file paths to write to
    ///
    /// ## Returns
    /// `Ok(())` if successful, `Err` if any operation fails
    ///
    /// ## Errors
    /// Returns error if file operations or format conversion fails
    fn update_configuration_file(
        &self,
        source_paths: &[PathBuf],
        destination_paths: &[PathBuf],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure we have the same number of source and destination paths
        if source_paths.len() != destination_paths.len() {
            return Err(format!(
                "Mismatched number of source ({}) and destination ({}) paths",
                source_paths.len(),
                destination_paths.len()
            )
            .into());
        }

        // Process each source-destination pair
        for (source_path, destination_path) in source_paths.iter().zip(destination_paths.iter()) {
            log_debug!(
                "[SDB::Tools::Configuration::ConfigurationManager] Updating configuration from {} to {}",
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
                "[SDB::Tools::Configuration] Configuration written to: {}",
                destination_path
                    .display()
                    .to_string()
                    .bright_cyan()
                    .italic()
            );
        }

        Ok(())
    }

    /// Converts a TOML value into a target format based on the file extension.
    ///
    /// Supported formats:
    /// - `.json`: Converts TOML to pretty-printed JSON.
    /// - `.yaml` or `.yml`: Converts TOML to YAML.
    /// - `.toml`: Re-serializes the TOML value with pretty printing.
    /// - Any other extension: Flattens the TOML into a simple `KEY=VALUE` format.
    ///
    /// ## Parameters
    /// - `toml_value`: TOML value to convert
    /// - `destination_path`: Destination file path (determines target format)
    ///
    /// ## Returns
    /// `Ok(String)` with converted content, `Err` if conversion fails
    ///
    /// ## Errors
    /// Returns error if format conversion or serialization fails
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

    /// Converts a `TomlValue` into a `JsonValue`.
    ///
    /// This is done by serializing the TOML value to an intermediary JSON string
    /// and then deserializing it into a `serde_json::Value`. This approach leverages
    /// the `serde` framework's robust TOML-to-JSON mapping.
    ///
    /// ## Parameters
    /// - `toml_value`: TOML value to convert
    ///
    /// ## Returns
    /// `Ok(JsonValue)` with converted JSON value, `Err` if conversion fails
    ///
    /// ## Errors
    /// Returns error if JSON serialization or deserialization fails
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
    ///
    /// ## Parameters
    /// - `toml_value`: TOML value to convert
    ///
    /// ## Returns
    /// `Ok(YamlValue)` with converted YAML value, `Err` if conversion fails
    ///
    /// ## Errors
    /// Returns error if JSON or YAML conversion fails
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
    ///
    /// ## Parameters
    /// - `toml_value`: TOML value to convert
    ///
    /// ## Returns
    /// `String` with flattened key-value pairs
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
    ///
    /// ## Parameters
    /// - `value`: TOML value to flatten
    /// - `prefix`: Current key prefix for nested structures
    /// - `result`: Mutable vector to accumulate results
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
                        format!("{prefix}_{key}")
                    };
                    self.flatten_toml_to_key_value(val, new_prefix, result);
                }
            }
            TomlValue::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_prefix = format!("{prefix}_{i}");
                    self.flatten_toml_to_key_value(val, new_prefix, result);
                }
            }
            _ => {
                let value_str = match value {
                    TomlValue::String(s) => {
                        // Apply quoting if the string contains special characters or whitespace.
                        if self.needs_quotes(s) {
                            format!("\"{s}\"")
                        } else {
                            s.clone()
                        }
                    }
                    TomlValue::Integer(i) => i.to_string(),
                    TomlValue::Float(f) => f.to_string(),
                    TomlValue::Boolean(b) => b.to_string(),
                    TomlValue::Datetime(dt) => dt.to_string(),
                    // Unhandled types will result in an empty string.
                    _ => String::new(),
                };
                result.push(format!("{prefix}={value_str}"));
            }
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
    ///
    /// ## Parameters
    /// - `value`: String value to check for quoting needs
    ///
    /// ## Returns
    /// `true` if the value should be quoted, `false` otherwise
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
            matches!(
                c,
                '=' | '#' | '"' | '\'' | '\\' | ':' | ';' | ',' | '[' | ']' | '{' | '}'
            )
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

    /// Calculates the SHA256 hash of a multiple file's content and produce single SHA
    /// all files together.
    ///
    /// This is used to create a unique fingerprint of a file, which is essential
    /// for robust change detection.
    ///
    /// ## Parameters
    /// - `paths`: List of file paths to calculate combined SHA for
    ///
    /// ## Returns
    /// `Ok(String)` with SHA-256 hash, `Err` if file reading fails
    ///
    /// ## Errors
    /// Returns error if any file cannot be read
    pub fn calculate_combined_files_sha(
        &self,
        paths: &[PathBuf],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut hasher = Sha256::new();

        for path in paths {
            let content = fs::read(path)?;
            hasher.update(&content);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

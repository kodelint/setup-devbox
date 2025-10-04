//! # Add Command Implementation
//!
//! This module provides the core logic for adding or updating various configuration items
//! (tools, fonts, system settings, shell aliases) within the setup-devbox configuration files.
//!
//! ## Install or Update Strategy
//!
//! The update strategy ensures **format preservation** and **deep merging**:
//!
//! 1. **Install or Update**: Determine if new tool to be installed or existing tool to be updated. 
//! 2. **Deep Merge (Diff & Update)**: If an item exists, a programmatic deep merge is performed.
//!    Only the configuration fields explicitly provided by the user (the 'new' configuration, which
//!    are non-null) overwrite the existing configuration. All other existing fields are preserved.
//! 3. **Format Preservation**: The file content is manipulated line-by-line to replace only the specific
//!    item's YAML block, ensuring that surrounding comments, blank lines, and file structure are preserved.
//! 4. **Apply Changes**: After successful configuration update, the internal `now::run` function is executed
//!    to immediately trigger the installation/application of the changes.
//!
//! ## Supported Configuration Types
//!
//! - **Tools**: Development tools and applications with various installation sources
//! - **Fonts**: Font families and their installation configurations
//! - **OS Settings**: System-level configuration settings (macOS focused)
//! - **Shell Aliases**: Command line aliases and shortcuts
//!
//! ## Error Handling
//!
//! Provides detailed error messages with color-coded output:
//! - **Info**: General operation progress
//! - **Debug**: Detailed execution flow (enabled with `--debug` flag)
//! - **Warn**: Non-fatal issues or recommendations
//! - **Error**: Critical failures with specific error context

use crate::now;
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::{
    configuration_management::ConfigurationManager, fonts::FontEntry, os_settings::SettingEntry,
    shell_configuration::AliasEntry, tools::ToolEntry,
};
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use serde::{Serialize, de::DeserializeOwned};
use serde_yaml::{self, Value};
use std::fs;
use std::path::PathBuf;

// ============================================================================
// CONFIGURATION UPDATER STRUCTURE
// ============================================================================

/// Central coordinator for configuration file operations
///
/// The `ConfigurationUpdater` handles the complete lifecycle of configuration updates:
/// - Resolving configuration file paths from environment variables
/// - Reading existing configuration files
/// - Performing deep merges of configuration items
/// - Writing updated configurations back to files while preserving formatting
struct ConfigurationUpdater {
    /// The base path where all configuration YAML files reside (e.g., `$SDB_CONFIG_PATH/configs`).
    config_base_path: PathBuf,
}

impl ConfigurationUpdater {
    /// Creates a new `ConfigurationUpdater` instance by resolving the base configuration path
    /// from the `SDB_CONFIG_PATH` environment variable.
    /// 
    /// # Arguments
    /// * `paths`: A string slice (`&PathResolver`) representing the path.
    ///
    /// # Returns
    /// - `Ok(Self)`: Successfully initialized ConfigurationUpdater
    /// - `Err(String)`: Error message if SDB_CONFIG_PATH is not set or invalid
    ///
    /// # Example
    /// ```
    /// let updater = ConfigurationUpdater::new()?;
    /// ```
    pub fn new(paths: &PathResolver) -> Result<Self, String> {
        log_debug!("[Updater] Initializing ConfigurationUpdater...");

        let config_base = paths.configs_dir();

        log_debug!(
            "[Updater] Using config directory: {}",
            config_base.display().to_string().cyan()
        );

        // Configuration files are stored in the 'configs' subdirectory
        Ok(ConfigurationUpdater {
            config_base_path: config_base,
        })
    }

    /// Resolves the full filesystem path for a given configuration filename
    /// by combining the base config directory with the provided filename.
    ///
    /// # Arguments
    /// * `filename` - The name of the YAML configuration file (e.g., "tools.yaml")
    ///
    /// # Returns
    /// Full `PathBuf` to the configuration file
    fn get_config_path(&self, filename: &str) -> PathBuf {
        self.config_base_path.join(filename)
    }

    /// Core logic for updating or adding list items in YAML configuration files
    ///
    /// This is the main workhorse method that handles:
    /// - File I/O operations (reading/writing YAML files)
    /// - Item identification and location within files
    /// - Deep merging of existing and new configurations
    /// - Block replacement while preserving file formatting and comments
    ///
    /// # Type Parameters
    /// * `T`: The configuration struct type that implements required traits:
    ///   - `Serialize`: For converting to YAML
    ///   - `DeserializeOwned`: For parsing from YAML
    ///   - `Clone`: For creating copies during merge operations
    ///   - `PartialEq`: For equality comparisons
    ///   - `Debug`: For logging and debugging
    ///
    /// # Arguments
    /// * `filename`: Target configuration file name (e.g., "tools.yaml")
    /// * `section_key`: Top-level YAML key for the list section (e.g., "tools:")
    /// * `item_key`: Primary key used for item identification (e.g., "name:")
    /// * `item_identifier`: Unique identifier value for the item (e.g., tool name)
    /// * `new_item`: New or updated configuration data provided by user
    ///
    /// # Returns
    /// - `Ok(true)`: Item was found and updated (merge operation)
    /// - `Ok(false)`: Item was not found and was added (append operation)
    /// - `Err(String)`: Error message describing what went wrong
    ///
    /// # Algorithm Overview
    /// 1. Read existing file content or create basic structure if missing
    /// 2. Parse file line-by-line to locate target section and item
    /// 3. If item exists: extract block, deep merge, replace block
    /// 4. If item doesn't exist: append new item with proper formatting
    /// 5. Write updated content back to file
    fn update_or_add_list_item<T>(
        &self,
        filename: &str,
        section_key: &str,
        item_key: &str,
        item_identifier: &str,
        new_item: &T,
    ) -> Result<bool, String>
    where
        T: Serialize + DeserializeOwned + Clone + PartialEq + std::fmt::Debug,
    {
        let config_path = self.get_config_path(filename);
        log_debug!("[Updater] Target config path: {:?}", config_path);

        // 1. Read existing content or create a basic structure if missing
        let content = if config_path.exists() {
            fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config file {filename}: {e}"))?
        } else {
            log_warn!(
                "Config file {} not found. Creating a new one.",
                filename.cyan()
            );
            format!("{section_key}\n")
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut i = 0;
        let mut found_section = false;
        let mut item_updated = false;
        let item_start_prefix = format!("- {item_key}");

        log_debug!(
            "[Updater] Starting line-by-line search for section: {}",
            section_key.cyan()
        );

        // 2. Process file line-by-line to locate and update target item
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim_start();
            let current_indent = line.len() - trimmed.len();

            // Detect the start of the target configuration section (e.g., "tools:")
            if trimmed.starts_with(section_key) {
                log_debug!("[Updater] Found section: {}", section_key.cyan());
                found_section = true;
                result.push(line.to_string());
                i += 1;
                continue;
            }

            // If we are in the target section, check for the start of a list item
            if found_section && trimmed.starts_with(&item_start_prefix) {
                // Potential start of a list item (e.g., "- name: tool_name")
                if let Some(name_value) = extract_yaml_value(trimmed, item_key) {
                    if name_value == item_identifier {
                        log_info!(
                            "[SDB:Add] Existing item '{}' found. Performing deep merge...",
                            item_identifier.cyan()
                        );
                        let start_indent = current_indent;

                        // Extract and Deep Merge
                        let old_block_yaml = self.extract_item_block(&lines, i, start_indent)?;
                        log_debug!(
                            "[Updater] Old YAML block extracted: \n{}",
                            old_block_yaml.yellow()
                        );

                        // Merge the existing data with the new (non-null) data
                        let merged_yaml_raw =
                            self.deep_merge_and_serialize(&old_block_yaml, new_item)?;
                        log_debug!(
                            "[Updater] New (Merged) YAML block (raw): \n{}",
                            merged_yaml_raw.green()
                        );

                        // Replace the block with proper indentation
                        let base_indent = start_indent + 2;
                        let merged_yaml_formatted = indent_yaml(&merged_yaml_raw, base_indent);

                        // Add the merged block to result
                        for line in merged_yaml_formatted.lines() {
                            result.push(line.to_string());
                        }

                        item_updated = true;

                        // Skip the old lines that were just replaced
                        let next_item_start_index = self.find_next_item_start(
                            &lines,
                            i + 1,
                            start_indent,
                            &item_start_prefix,
                        );

                        // Preserve blank line separator if it existed
                        if next_item_start_index < lines.len() {
                            if next_item_start_index > 0
                                && lines[next_item_start_index - 1].trim().is_empty()
                            {
                                result.push("".to_string());
                            }
                        }

                        // Advance to the next item
                        i = next_item_start_index;
                        continue;
                    }
                }
            }

            // Check if we exited the section without finding a match
            if found_section && !trimmed.is_empty() && !trimmed.starts_with("#") {
                if current_indent == 0
                    && !trimmed.starts_with(&item_start_prefix)
                    && !trimmed.starts_with(section_key)
                {
                    found_section = false;
                    log_debug!(
                        "[Updater] Exited section without finding match. Current line: {}",
                        line
                    );
                }
            }

            // Keep the current line if no match found
            result.push(line.to_string());
            i += 1;
        }

        // 3. If the item was not found, append the new item
        if !item_updated {
            log_info!(
                "[SDB:Add] Item '{}' not found. Appending to configuration...",
                item_identifier.cyan()
            );

            let base_indent = 4; // Standard indent for top-level lists

            // Serialize the new item, ensuring nulls/defaults are removed
            let new_item_yaml = serialize_without_nulls(new_item)?;

            if new_item_yaml.trim().is_empty() {
                log_warn!("New item YAML is empty after stripping nulls. Skipping append.");
            } else {
                // Ensure a blank line before appending for readability
                if !result.is_empty() && result.last().map_or(false, |l| !l.is_empty()) {
                    result.push("".to_string());
                }

                let indented = indent_yaml(&new_item_yaml, base_indent);
                result.push(indented);
            }
        }

        // 4. Write updated content back to file
        let final_content = result.join("\n");
        fs::write(&config_path, final_content)
            .map_err(|e| format!("Failed to write config file {filename}: {e}"))?;

        log_debug!(
            "[Updater] File {} successfully written. Item updated: {}",
            filename,
            item_updated
        );
        Ok(item_updated)
    }

    /// Extracts a complete YAML block for a single list item starting at the specified line
    ///
    /// This method captures all lines that belong to the current item by analyzing indentation
    /// levels. It stops when it encounters a line with equal or lesser indentation that isn't
    /// a comment or empty line.
    ///
    /// # Arguments
    /// * `lines`: Array of lines from the configuration file
    /// * `start_index`: Line index where the item block starts
    /// * `start_indent`: Indentation level of the starting line
    ///
    /// # Returns
    /// - `Ok(String)`: Raw YAML string of the item block
    /// - `Err(String)`: Error message if extraction fails
    fn extract_item_block(
        &self,
        lines: &[&str],
        start_index: usize,
        start_indent: usize,
    ) -> Result<String, String> {
        let mut block = String::new();
        let base_trim_amount = start_indent + 2;

        let mut i = start_index;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim_start();
            let current_indent = line.len() - trimmed.len();

            // Stop condition: check for next item starting after the current one
            if i > start_index {
                if current_indent <= start_indent
                    && !trimmed.is_empty()
                    && !trimmed.starts_with("#")
                {
                    if trimmed.starts_with("- ") || current_indent == 0 {
                        break;
                    }
                }
            }

            // Only process lines that are part of the current item block
            if current_indent >= start_indent || trimmed.is_empty() {
                if i == start_index {
                    // First line: Remove the list marker
                    let stripped = trimmed.replacen("- ", "", 1);
                    block.push_str(&stripped);
                } else {
                    // Subsequent lines: Trim to preserve relative hierarchy
                    if current_indent >= base_trim_amount {
                        block.push_str(&line[base_trim_amount..]);
                    } else {
                        block.push_str(line.trim_end());
                    }
                }
                block.push('\n');
            }

            i += 1;
        }

        Ok(block.trim_end().to_string())
    }

    /// Locates the starting index of the next item or section in the configuration file
    ///
    /// This is used during block replacement to determine how many lines to skip
    /// after replacing an existing item block.
    ///
    /// # Arguments
    /// * `lines`: Array of lines from the configuration file
    /// * `start_index`: Index to start searching from
    /// * `start_indent`: Base indentation level of the current section
    /// * `item_start_prefix`: Pattern that identifies the start of an item (e.g., "- name:")
    ///
    /// # Returns
    /// Index of the next item start, or the end of file if no more items found
    fn find_next_item_start(
        &self,
        lines: &[&str],
        start_index: usize,
        start_indent: usize,
        item_start_prefix: &str,
    ) -> usize {
        for i in start_index..lines.len() {
            let line = lines[i];
            let trimmed = line.trim_start();
            let current_indent = line.len() - trimmed.len();

            // If we hit a line at or less than the starting indent, we've exited the block
            if current_indent <= start_indent && !trimmed.is_empty() && !trimmed.starts_with("#") {
                if trimmed.starts_with(item_start_prefix) || current_indent == 0 {
                    log_debug!(
                        "[Updater] Found next item/section start at line index {}: {}",
                        i,
                        line.cyan()
                    );
                    return i;
                }
            }
        }
        lines.len() // End of file reached
    }

    /// Performs a deep merge between existing YAML configuration and new item values
    ///
    /// This is the core function for updating existing configurations while preserving
    /// unspecified fields. It implements the "only change/update the diffed value" requirement.
    ///
    /// # Arguments
    /// * `existing_yaml_block`: Raw YAML string of the existing item's configuration
    /// * `new_item`: New configuration data with fields potentially set to defaults/nulls
    ///
    /// # Returns
    /// Serialized YAML string of the merged configuration with nulls/defaults stripped
    fn deep_merge_and_serialize<T: Serialize + DeserializeOwned + Clone + PartialEq>(
        &self,
        existing_yaml_block: &str,
        new_item: &T,
    ) -> Result<String, String>
    where
        T: std::fmt::Debug,
    {
        // 1. Parse the existing block into a Value first to allow manipulation
        let mut existing_value: Value = serde_yaml::from_str(existing_yaml_block).map_err(|e| {
            format!(
                "Failed to parse existing item YAML into generic Value: {e}")
        })?;

        // FIX: Handle configuration_manager missing 'enabled' field for backward compatibility
        if let Some(map) = existing_value.as_mapping_mut() {
            if let Some(config_mgr) =
                map.get_mut(Value::String("configuration_manager".to_string()))
            {
                if let Some(config_map) = config_mgr.as_mapping_mut() {
                    if config_map
                        .get(Value::String("enabled".to_string()))
                        .is_none()
                    {
                        log_warn!(
                            "[Merge::Fix] Found configuration_manager without 'enabled'. Defaulting to 'enabled: false' for merge safety."
                        );
                        config_map.insert(Value::String("enabled".to_string()), Value::Bool(false));
                    }
                }
            }
        }

        // 2. Deserialize the (fixed) existing block into the struct
        let existing_struct: T = serde_yaml::from_value(existing_value.clone())
            .map_err(|e| format!("Failed to parse existing item into struct for merge: {e}"))?;
        log_debug!("[Merge] Existing struct: {:?}", existing_struct);

        // 3. Convert back to YAML Value for programmatic deep merging
        let mut existing_value = serde_yaml::to_value(existing_struct)
            .map_err(|e| format!("Failed to convert existing struct to YAML Value: {e}"))?;

        // Serialize new item and remove nulls/defaults
        let new_value = serialize_without_nulls_to_value(new_item)?;
        log_debug!(
            "[Merge] New (non-null) Value: {}",
            serde_yaml::to_string(&new_value).unwrap_or_default().trim()
        );

        // 4. Perform Deep Merge (New overwrites existing only if the new value is present/non-null)
        merge_yaml_values(&mut existing_value, &new_value);
        log_debug!(
            "[Merge] Merged Value before final strip: {}",
            serde_yaml::to_string(&existing_value)
                .unwrap_or_default()
                .trim()
        );

        // 5. Perform final strip of any nulls/defaults
        remove_nulls(&mut existing_value);

        let final_yaml = serde_yaml::to_string(&existing_value)
            .map_err(|e| format!("Failed to serialize final merged item: {e}"))?;

        Ok(final_yaml)
    }
}

/// Public interface for adding or updating a tool entry in the tools configuration
///
/// This is the main entry point for the `setup-devbox add tool` command. It:
/// 1. Validates the tool configuration based on installation source
/// 2. Constructs the ToolEntry struct from command line arguments
/// 3. Calls the generic update logic to modify the configuration file
/// 4. Triggers immediate installation/application of changes
///
/// # Arguments
/// * `name`: Unique identifier for the tool
/// * `version`: Tool version specification
/// * `source`: Installation source type (github, url, etc.)
/// * `url`: Direct download URL (for url sources)
/// * `repo`: GitHub repository path (for github sources)
/// * `tag`: Release tag or version (for github sources)
/// * `rename_to`: Optional filename rename after download
/// * `options`: Additional installation options
/// * `executable_path_after_extract`: Path to executable in archive
/// * `post_installation_hooks`: Commands to run after installation
/// * `enable_config_manager`: Whether to enable configuration management
/// * `config_paths`: Paths to configuration files for management
#[allow(clippy::too_many_arguments)]
pub fn add_tool(
    name: String,
    version: String,
    source: String,
    url: Option<String>,
    repo: Option<String>,
    tag: Option<String>,
    rename_to: Option<String>,
    options: Option<Vec<String>>,
    executable_path_after_extract: Option<String>,
    post_installation_hooks: Option<Vec<String>>,
    enable_config_manager: bool,
    config_paths: Vec<String>,
) {
    log_info!("[SDB::Add::Tool] Preparing to add tool: {name}...");

    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!("[SDB::Add::Tool] Failed to initialize path resolver: {}", e);
        std::process::exit(1);
    });

    let updater = ConfigurationUpdater::new(&paths).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Add::Tool] Failed to initialize configuration updater: {}",
            e
        );
        std::process::exit(1);
    });

    // Construct the ToolEntry struct from the arguments
    let new_tool = ToolEntry {
        name: name.clone(),
        version: Some(version),
        source,
        url,
        repo,
        tag,
        rename_to,
        options,
        executable_path_after_extract,
        post_installation_hooks,
        configuration_manager: ConfigurationManager {
            enabled: enable_config_manager,
            tools_configuration_paths: config_paths,
        },
    };

    // Validate the tool configuration based on its source
    if let Err(e) = validate_tool_restrictions(&new_tool) {
        log_error!(
            "[SDB::Add::Tool] Validation failed for tool {}: {}",
            name.cyan(),
            e
        );
        std::process::exit(1);
    }

    // Call the generic list item update/add logic
    match updater.update_or_add_list_item(
        "tools.yaml",
        "tools:",
        "name:",
        &new_tool.name,
        &new_tool,
    ) {
        Ok(is_update) => {
            log_info!(
                "[SDB::Add::Tool] Successfully {} tool '{}' in configuration",
                if is_update { "updated" } else { "added" },
                name.cyan()
            );
        }
        Err(e) => {
            log_error!(
                "[SDB::Add::Tool] Failed to update configuration for tool {}: {}",
                name.cyan(),
                e
            );
            std::process::exit(1);
        }
    }

    // Run the installation command after update
    run_now_command();
}

// ============================================================================
// FONT ADDITION IMPLEMENTATION
// ============================================================================

/// Public interface for adding or updating a font entry in the fonts configuration
///
/// This function handles font installations, typically from GitHub repositories
/// containing font files.
///
/// # Arguments
/// * `name`: Font family or specific font name
/// * `version`: Font version or release tag
/// * `source`: Installation source (typically "github")
/// * `repo`: GitHub repository containing the font files
/// * `tag`: Specific release tag or version
/// * `install_only`: Specific font files to install from the repository
pub fn add_font(
    name: String,
    version: String,
    source: String,
    repo: String,
    tag: String,
    install_only: Vec<String>,
) {
    log_info!("[SDB::Add::Fonts] Preparing to add font: {name}...");

    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!("[SDB::Add::Tool] Failed to initialize path resolver: {}", e);
        std::process::exit(1);
    });

    let updater = ConfigurationUpdater::new(&paths).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Add::Tool] Failed to initialize configuration updater: {}",
            e
        );
        std::process::exit(1);
    });

    // Construct the FontEntry struct from arguments
    let new_font = FontEntry {
        name: name.clone(),
        version: Some(version),
        source,
        repo: Some(repo),
        tag: Some(tag),
        install_only: if install_only.is_empty() {
            None
        } else {
            Some(install_only)
        },
    };

    // Call the generic list item update/add logic for fonts
    match updater.update_or_add_list_item(
        "fonts.yaml",
        "fonts:",
        "name:",
        &new_font.name,
        &new_font,
    ) {
        Ok(is_update) => {
            log_info!(
                "[SDB::Add::Fonts] Successfully {} font '{}' in configuration",
                if is_update { "updated" } else { "added" },
                name.cyan()
            );
        }
        Err(e) => {
            log_error!(
                "[SDB::Add::Fonts] Failed to update configuration for font {}: {}",
                name.cyan(),
                e
            );
            std::process::exit(1);
        }
    }

    run_now_command();
}

// ============================================================================
// SETTING ADDITION IMPLEMENTATION
// ============================================================================

/// Public interface for adding or updating an OS setting entry
///
/// Currently specialized for macOS settings due to their nested YAML structure.
/// This function handles system-level configuration settings.
///
/// # Arguments
/// * `domain`: Setting domain or category (e.g., "NSGlobalDomain")
/// * `key`: Specific setting key within the domain
/// * `value`: Value to set for the configuration
/// * `value_type`: Data type of the value (bool, string, int, float)
pub fn add_setting(domain: String, key: String, value: String, value_type: String) {
    let setting_name = format!("{domain}.{key}");
    log_info!("[SDB::Add::Setting] Preparing to add setting: {setting_name}...");

    // Validate the input type against supported types
    if !is_valid_value_type(&value_type) {
        log_error!(
            "[SDB::Add::Setting] Invalid value type '{}'. Supported types: bool, string, int, float",
            value_type.cyan()
        );
        std::process::exit(1);
    }

    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!("[SDB::Add::Tool] Failed to initialize path resolver: {}", e);
        std::process::exit(1);
    });

    let updater = ConfigurationUpdater::new(&paths).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Add::Setting] Failed to initialize configuration updater: {}",
            e
        );
        std::process::exit(1);
    });

    // Construct the SettingEntry struct
    let new_setting = SettingEntry {
        domain: domain.clone(),
        key: key.clone(),
        value,
        value_type,
    };

    // Settings use a nested structure and need specialized parsing
    match update_or_add_macos_setting(&updater, &new_setting) {
        Ok(is_update) => {
            log_info!(
                "[SDB::Add::Setting] Successfully {} setting '{}' in configuration",
                if is_update { "updated" } else { "added" },
                setting_name.cyan()
            );
        }
        Err(e) => {
            log_error!(
                "[SDB::Add::Setting] Failed to update configuration for setting {}: {}",
                setting_name.cyan(),
                e
            );
            std::process::exit(1);
        }
    }

    run_now_command();
}

// ============================================================================
// ALIAS ADDITION IMPLEMENTATION
// ============================================================================

/// Public interface for adding or updating a shell alias entry
///
/// Shell aliases are simple key-value pairs that create command shortcuts
/// in the user's shell configuration.
///
/// # Arguments
/// * `name`: Alias name (the shortcut command)
/// * `value`: Full command that the alias expands to
pub fn add_alias(name: String, value: String) {
    log_info!("[SDB::Add:Alias] Preparing to add alias: {name}...");

    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!("[SDB::Add::Tool] Failed to initialize path resolver: {}", e);
        std::process::exit(1);
    });

    let updater = ConfigurationUpdater::new(&paths).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Add::Alias] Failed to initialize configuration updater: {}",
            e
        );
        std::process::exit(1);
    });

    // Construct the AliasEntry struct
    let new_alias = AliasEntry {
        name: name.clone(),
        value,
    };

    // Call the generic list item update/add logic for aliases
    match updater.update_or_add_list_item(
        "shellrc.yaml",
        "aliases:",
        "name:",
        &new_alias.name,
        &new_alias,
    ) {
        Ok(is_update) => {
            log_info!(
                "[SDB::Add:Alias] Successfully {} alias '{}' in configuration",
                if is_update { "updated" } else { "added" },
                name.cyan()
            );
        }
        Err(e) => {
            log_error!(
                "[SDB::Add:Alias] Failed to update configuration for alias {}: {}",
                name.cyan(),
                e
            );
            std::process::exit(1);
        }
    }

    run_now_command();
}

// ============================================================================
// TOOL ADDITION IMPLEMENTATION
// ============================================================================

/// Validates tool-specific restrictions based on the installation source
///
/// Different installation sources have different required fields:
/// - GitHub sources require both repository and tag information
/// - URL sources require a direct download URL
/// - Other sources may have their own validation rules
///
/// # Arguments
/// * `new_tool`: The `ToolEntry` struct containing the tool configuration
///
/// # Returns
/// - `Ok(())`: Validation passed successfully
/// - `Err(String)`: Error message describing missing required fields
fn validate_tool_restrictions(new_tool: &ToolEntry) -> Result<(), String> {
    log_debug!(
        "[SDB::Add::Validation] Starting validation for tool: {}",
        new_tool.name.cyan()
    );

    match new_tool.source.to_lowercase().as_str() {
        "github" => {
            if new_tool.repo.is_none() || new_tool.tag.is_none() {
                return Err(format!(
                    "Source is '{}', but requires both {} and {} to be provided.",
                    "github".cyan(),
                    "repo".cyan(),
                    "tag".cyan()
                ));
            }
        }
        "url" => {
            if new_tool.url.is_none() {
                return Err(format!(
                    "Source is '{}', but requires {} to be provided.",
                    "url".cyan(),
                    "url".cyan()
                ));
            }
        }
        _ => {
            log_debug!(
                "[SDB::Add::Validation] No specific restrictions found for source: {}",
                new_tool.source
            );
        }
    }
    Ok(())
}

/// Specialized handler for macOS settings due to nested YAML structure
///
/// macOS settings have a specific structure in the YAML file that requires
/// specialized handling compared to other configuration types.
///
/// # Arguments
/// * `updater`: Configuration updater instance
/// * `setting`: Setting entry to add or update
///
/// # Returns
/// - `Ok(true)`: Setting was updated
/// - `Ok(false)`: Setting was added
/// - `Err(String)`: Error message
fn update_or_add_macos_setting(
    updater: &ConfigurationUpdater,
    setting: &SettingEntry,
) -> Result<bool, String> {
    let filename = "settings.yaml";
    let config_path = updater.get_config_path(filename);

    let content = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file {filename}: {e}"))?
    } else {
        log_warn!(
            "[SDB::Add] Config file {} not found. Creating a new one with basic structure.",
            filename.cyan()
        );
        "settings:\n  macos:\n".to_string()
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    let mut in_macos_section = false;
    let mut setting_updated = false;

    log_debug!(
        "[Setting::Updater] Starting line-by-line search for setting: {}.{}",
        setting.domain.cyan(),
        setting.key.cyan()
    );

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // 1. Detect the start of the 'macos:' section
        if trimmed.starts_with("macos:") {
            in_macos_section = true;
            result.push(line.to_string());
            i += 1;
            continue;
        }

        // 2. Look for the target domain within the 'macos:' section
        if in_macos_section && trimmed.starts_with("- domain:") {
            let current_indent = line.len() - trimmed.len();
            if let Some(domain_value) = extract_yaml_value(trimmed, "domain:") {
                if domain_value == setting.domain {
                    // Check for the key within this domain block
                    if find_setting_key_in_block(&lines, i + 1, current_indent, &setting.key)
                        .is_some()
                    {
                        log_info!(
                            "Existing setting '{}.{}' found. Performing deep merge...",
                            setting.domain.cyan(),
                            setting.key.cyan()
                        );

                        // Extract and merge the existing block
                        let old_block_yaml =
                            updater.extract_item_block(&lines, i, current_indent)?;
                        let merged_yaml =
                            updater.deep_merge_and_serialize(&old_block_yaml, setting)?;

                        // Replace the block
                        result.push(merged_yaml);
                        setting_updated = true;

                        // Skip the old lines that were just replaced
                        i = updater.find_next_item_start(
                            &lines,
                            i + 1,
                            current_indent,
                            "- domain:",
                        );
                        continue;
                    }
                }
            }
        }

        // 3. Handle leaving the macos section
        if in_macos_section && !trimmed.is_empty() && !trimmed.starts_with("#") {
            let current_indent = line.len() - trimmed.len();
            if current_indent < 2 && !trimmed.starts_with("macos:") {
                in_macos_section = false;
                log_debug!("[Setting::Updater] Exited macos section.");
            }
        }

        result.push(line.to_string());
        i += 1;
    }

    // 4. If not updated, append the new setting
    if !setting_updated {
        log_info!(
            "[SDB::Add::Settings] Setting '{}.{}' not found. Appending to configuration...",
            setting.domain.cyan(),
            setting.key.cyan()
        );

        let base_indent = 6; // Special indent for settings under macos

        let new_item_yaml = serialize_without_nulls(setting)?;

        // Ensure a blank line before appending
        if !result.is_empty() && result.last().map_or(false, |l| !l.is_empty()) {
            result.push("".to_string());
        }

        let indented = indent_yaml(&new_item_yaml, base_indent);
        result.push(indented);
    }

    // 5. Write back to file
    let final_content = result.join("\n");
    fs::write(&config_path, final_content)
        .map_err(|e| format!("[SDB::Add] Failed to write config file {filename}: {e}"))?;

    Ok(setting_updated)
}

/// Helper to search for a specific `key:` within a YAML block
///
/// This is used in setting updates to verify the exact key exists within the domain block.
fn find_setting_key_in_block(
    lines: &[&str],
    start_index: usize,
    block_indent: usize,
    target_key: &str,
) -> Option<usize> {
    for i in start_index..lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        let current_indent = line.len() - trimmed.len();

        // If we exit the block's indentation, stop searching
        if current_indent <= block_indent && !trimmed.is_empty() && !trimmed.starts_with("#") {
            return None;
        }

        if let Some(key_value) = extract_yaml_value(trimmed, "key:") {
            if key_value == target_key {
                return Some(i);
            }
        }
    }
    None
}

// ============================================================================
// YAML MANIPULATION UTILITIES
// ============================================================================

/// Indents serialized YAML content by a specified number of spaces
///
/// This ensures proper formatting when inserting YAML blocks into existing
/// configuration files with specific indentation requirements.
///
/// # Arguments
/// * `yaml`: Raw YAML string to indent
/// * `indent`: Number of spaces to indent each line
///
/// # Returns
/// Indented YAML string with proper list item formatting
fn indent_yaml(yaml: &str, indent: usize) -> String {
    let lines: Vec<&str> = yaml.lines().collect();
    let mut result = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();

        if trimmed.is_empty() {
            continue;
        }

        if idx == 0 {
            // First line gets the list item prefix with adjusted indentation
            let line_indent = if indent >= 2 { indent - 2 } else { 0 };
            result.push(format!("{}- {}", " ".repeat(line_indent), trimmed));
        } else {
            // Subsequent lines get full indentation
            let original_indent = line.len() - line.trim_start().len();
            result.push(format!(
                "{}{}",
                " ".repeat(indent + original_indent),
                trimmed
            ));
        }
    }

    result.join("\n")
}

/// Extracts a string value from a simple YAML key-value line
///
/// Handles potential quotes around the value and returns the trimmed content.
///
/// # Arguments
/// * `line`: YAML line to parse (e.g., "name: tool_name")
/// * `key`: Key to extract value for (e.g., "name:")
///
/// # Returns
/// - `Some(String)`: Extracted value if key matches and value exists
/// - `None`: If key doesn't match or no value present
fn extract_yaml_value(line: &str, key: &str) -> Option<String> {
    let full_key = if key.ends_with(':') {
        key.to_string()
    } else {
        format!("{key}:")
    };

    if let Some(pos) = line.find(&full_key) {
        let value_part = &line[pos + full_key.len()..];
        let value = value_part.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

/// Serializes a struct to YAML string while removing null/default values
///
/// This ensures that only explicitly provided fields are included in the
/// output YAML, maintaining clean configuration files.
///
/// # Arguments
/// * `value`: Struct implementing Serialize trait
///
/// # Returns
/// - `Ok(String)`: YAML string with nulls removed
/// - `Err(String)`: Error message if serialization fails
fn serialize_without_nulls<T: Serialize>(value: &T) -> Result<String, String> {
    let yaml_value = serialize_without_nulls_to_value(value)?;
    serde_yaml::to_string(&yaml_value).map_err(|e| format!("Failed to serialize to string: {e}"))
}

/// Serializes a struct to YAML Value while removing null/default values
///
/// This is a variant that returns a YAML Value instead of a string, used
/// internally for programmatic merging operations.
fn serialize_without_nulls_to_value<T: Serialize>(value: &T) -> Result<Value, String> {
    let mut yaml_value =
        serde_yaml::to_value(value).map_err(|e| format!("Failed to serialize to value: {e}"))?;
    remove_nulls(&mut yaml_value);
    Ok(yaml_value)
}

/// Recursively removes null values from a YAML Value structure
///
/// This function implements the core "strict null exclusion" principle by
/// traversing the YAML structure and removing any keys that have null values,
/// empty sequences, or empty maps.
///
/// # Arguments
/// * `value`: Mutable reference to YAML Value to clean
fn remove_nulls(value: &mut Value) {
    match value {
        Value::Mapping(map) => {
            // 1. Retain only entries whose values are not null
            map.retain(|_, v| !v.is_null());

            // 2. Special handling for configuration_manager: remove if 'enabled' is false
            if let Some(config_mgr_key) = map
                .keys()
                .find(|k| k.as_str() == Some("configuration_manager"))
                .cloned()
            {
                if let Some(config_mgr) = map.get(&config_mgr_key) {
                    if let Some(enabled) = config_mgr.get("enabled") {
                        if enabled.as_bool() == Some(false) {
                            map.remove(&config_mgr_key);
                        }
                    }
                }
            }

            // 3. Recursively process remaining values and remove empty sequences/maps
            let mut keys_to_remove = Vec::new();
            for (k, v) in map.iter_mut() {
                remove_nulls(v);

                // After recursive removal, check if the value became empty
                if v.is_sequence() && v.as_sequence().map_or(false, |s| s.is_empty()) {
                    keys_to_remove.push(k.clone());
                } else if v.is_mapping() && v.as_mapping().map_or(false, |m| m.is_empty()) {
                    keys_to_remove.push(k.clone());
                }
            }

            for k in keys_to_remove {
                map.remove(&k);
            }
        }
        Value::Sequence(seq) => {
            // Recursively process sequence items
            for item in seq.iter_mut() {
                remove_nulls(item);
            }
        }
        _ => {}
    }
}

/// Programmatic deep merge for two YAML Values
///
/// `new_value` overwrites or adds to `existing_value` only for fields where `new_value`
/// is non-null, non-empty, and non-default. This preserves existing optional fields
/// if the user didn't explicitly provide a new value for them.
///
/// # Arguments
/// * `existing_value`: Mutable reference to existing YAML value (will be modified)
/// * `new_value`: New YAML value containing updates
fn merge_yaml_values(existing_value: &mut Value, new_value: &Value) {
    if let Value::Mapping(existing_map) = existing_value {
        if let Value::Mapping(new_map) = new_value {
            for (key, new_v) in new_map {
                // Ensure the new value is meaningful before merging
                let is_meaningful = !new_v.is_null()
                    && !(new_v.is_sequence() && new_v.as_sequence().map_or(true, |s| s.is_empty()))
                    && !(new_v.is_mapping() && new_v.as_mapping().map_or(true, |m| m.is_empty()));

                if is_meaningful {
                    if let Some(existing_v) = existing_map.get_mut(key) {
                        // Recursively merge if both are maps
                        if existing_v.is_mapping() && new_v.is_mapping() {
                            merge_yaml_values(existing_v, new_v);
                        } else {
                            // Overwrite existing value
                            existing_map.insert(key.clone(), new_v.clone());
                        }
                    } else {
                        // Insert new key/value
                        existing_map.insert(key.clone(), new_v.clone());
                    }
                }
            }
        }
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Validates that a value type string is supported
///
/// # Arguments
/// * `value_type`: Type string to validate
///
/// # Returns
/// `true` if valid, `false` otherwise
fn is_valid_value_type(value_type: &str) -> bool {
    matches!(
        value_type.to_lowercase().as_str(),
        "bool" | "string" | "int" | "float"
    )
}

/// Executes the `now` command to immediately apply configuration changes
///
/// This function calls the internal `now::run()` function to trigger the
/// installation and application of the newly added or updated configuration
/// items.
fn run_now_command() {
    log_info!(
        "[SDB::Now] Running '{}' to apply configuration changes...",
        "setup-devbox now".cyan()
    );
    log_debug!("[SDB::Now] Executing now::run(None, None, false);");

    // Call the internal function to reload and apply the configuration
    match PathResolver::new(None, None) {
        Ok(paths) => now::run(&paths, false),
        Err(e) => {
            log_error!("Failed to initialize path resolver: {}", e.red());
            std::process::exit(1);
        }
    }
}

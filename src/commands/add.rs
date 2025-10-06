//! # Add Command Implementation
//!
//! This module provides the core logic for adding or updating various configuration items
//! (tools, fonts, system settings, shell aliases) within the setup-devbox configuration files.
//!
//! ## Strategy
//!
//! Uses a structural YAML approach:
//! 1. Parse the entire YAML file into a data structure
//! 2. Locate and update/add items programmatically
//! 3. Serialize back to YAML with consistent formatting
//!
//! This ensures valid YAML output and eliminates formatting bugs at the cost of
//! not preserving custom formatting or comments.

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

/// Handles updating and adding configuration items to YAML files
///
/// This struct provides the core functionality for modifying configuration files
/// using a structural approach that preserves YAML validity while allowing
/// for deep merging of existing configuration items.
struct ConfigurationUpdater {
    /// Base directory where configuration files are stored
    config_base_path: PathBuf,
}

impl ConfigurationUpdater {
    /// Creates a new ConfigurationUpdater instance
    ///
    /// # Arguments
    /// * `paths` - PathResolver instance containing directory configuration
    ///
    /// # Returns
    /// * `Result<Self, String>` - New ConfigurationUpdater instance or error message
    pub fn new(paths: &PathResolver) -> Result<Self, String> {
        log_debug!("[Updater] Initializing ConfigurationUpdater...");

        let config_base = paths.configs_dir();

        log_debug!(
            "[Updater] Using config directory: {}",
            config_base.display().to_string().cyan()
        );

        Ok(ConfigurationUpdater {
            config_base_path: config_base,
        })
    }

    /// Constructs the full path to a configuration file
    ///
    /// # Arguments
    /// * `filename` - Name of the configuration file (e.g., "tools.yaml")
    ///
    /// # Returns
    /// * `PathBuf` - Full path to the configuration file
    fn get_config_path(&self, filename: &str) -> PathBuf {
        self.config_base_path.join(filename)
    }

    /// Core logic for updating or adding list items in YAML configuration files
    ///
    /// This uses a structural approach: parse YAML -> modify data structure -> serialize
    /// The function handles both adding new items and updating existing ones with deep merging.
    ///
    /// # Type Parameters
    /// * `T` - The type of item being added/updated, must be serializable/deserializable
    ///
    /// # Arguments
    /// * `filename` - Name of the YAML configuration file
    /// * `section_key` - Key of the section containing the list (e.g., "tools:")
    /// * `item_key` - Key used to identify items within the list (e.g., "name:")
    /// * `item_identifier` - Value of the identifier to search for
    /// * `new_item` - New item to add or merge with existing
    ///
    /// # Returns
    /// * `Result<bool, String>` - `true` if item was updated, `false` if added new, or error
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

        // Read and parse the entire file
        let content = if config_path.exists() {
            // File exists - read its contents
            fs::read_to_string(&config_path).map_err(|e| {
                format!(
                    "Failed to read {}: {}",
                    filename.to_string().red(),
                    e.to_string().red()
                )
            })?
        } else {
            // File doesn't exist - create initial structure
            log_warn!(
                "Config file {} not found. Creating new one.",
                filename.cyan()
            );
            format!("{}:\n  []\n", section_key.trim_end_matches(':'))
        };

        // Parse YAML content into a structured document
        let mut doc: Value = serde_yaml::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse {}: {}",
                filename.to_string().red(),
                e.to_string().red()
            )
        })?;

        // Navigate to the section containing our items
        let section_name = section_key.trim_end_matches(':');
        let items = doc
            .get_mut(section_name)
            .and_then(|v| v.as_sequence_mut())
            .ok_or_else(|| {
                format!(
                    "Section '{}' not found or not a list",
                    section_name.to_string().red()
                )
            })?;

        // Find existing item by identifier
        let item_key_trimmed = item_key.trim_end_matches(':');
        let existing_idx = items.iter().position(|item| {
            item.get(item_key_trimmed).and_then(|v| v.as_str()) == Some(item_identifier)
        });

        let was_update = if let Some(idx) = existing_idx {
            // Item exists - perform deep merge with existing data
            log_info!(
                "[SDB:Add] Existing item '{}' found. Performing deep merge...",
                item_identifier.cyan()
            );

            // Merge existing with new
            let existing = &items[idx];
            let mut merged = existing.clone();

            let new_value = serde_yaml::to_value(new_item)
                .map_err(|e| format!("Failed to serialize new item: {}", e.to_string().red()))?;

            // Perform deep merge to preserve existing fields not in new_item
            deep_merge_values(&mut merged, &new_value);
            // Clean up null values after merge
            remove_nulls(&mut merged);

            // Replace the existing item with merged version
            items[idx] = merged;
            true // Indicates this was an update operation
        } else {
            // Item doesn't exist - add as new
            log_info!(
                "[SDB:Add] Item '{}' not found. Adding to configuration...",
                item_identifier.cyan()
            );

            // Add new item
            let mut new_value = serde_yaml::to_value(new_item)
                .map_err(|e| format!("Failed to serialize new item: {}", e.to_string().red()))?;
            // Clean up null values before adding
            remove_nulls(&mut new_value);

            items.push(new_value);
            false // Indicates this was an add operation
        };

        // Write back with consistent formatting
        let output = serde_yaml::to_string(&doc)
            .map_err(|e| format!("Failed to serialize config: {}", e.to_string().red()))?;

        fs::write(&config_path, output).map_err(|e| {
            format!(
                "Failed to write {}: {}",
                filename.to_string().red(),
                e.to_string().red()
            )
        })?;

        log_debug!("[Updater] File {} successfully written", filename);
        Ok(was_update)
    }

    /// Specialized handler for macOS settings due to nested structure
    ///
    /// macOS settings have a nested structure under `settings.macos` which requires
    /// special handling compared to other configuration types.
    ///
    /// # Arguments
    /// * `setting` - The setting entry to add or update
    ///
    /// # Returns
    /// * `Result<bool, String>` - `true` if setting was updated, `false` if added new, or error
    fn update_or_add_setting(&self, setting: &SettingEntry) -> Result<bool, String> {
        let filename = "settings.yaml";
        let config_path = self.get_config_path(filename);

        let content = if config_path.exists() {
            // Read existing settings file
            fs::read_to_string(&config_path).map_err(|e| {
                format!(
                    "Failed to read {}: {}",
                    filename.to_string().red(),
                    e.to_string().red()
                )
            })?
        } else {
            // Create new settings file with proper nested structure
            log_warn!(
                "Config file {} not found. Creating new one.",
                filename.cyan()
            );
            "settings:\n  macos:\n    []\n".to_string()
        };

        let mut doc: Value = serde_yaml::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse {}: {}",
                filename.to_string().red(),
                e.to_string().red()
            )
        })?;

        // Navigate to macos section within settings
        let macos_items = doc
            .get_mut("settings")
            .and_then(|v| v.get_mut("macos"))
            .and_then(|v| v.as_sequence_mut())
            .ok_or_else(|| "settings.macos section not found or not a list".to_string())?;

        // Find existing setting by domain and key combination
        let existing_idx = macos_items.iter().position(|item| {
            item.get("domain").and_then(|v| v.as_str()) == Some(&setting.domain)
                && item.get("key").and_then(|v| v.as_str()) == Some(&setting.key)
        });

        let was_update = if let Some(idx) = existing_idx {
            // Setting exists - update with merge
            log_info!(
                "[SDB:Add] Existing setting '{}.{}' found. Updating...",
                setting.domain.cyan(),
                setting.key.cyan()
            );

            let existing = &macos_items[idx];
            let mut merged = existing.clone();

            let new_value = serde_yaml::to_value(setting)
                .map_err(|e| format!("Failed to serialize setting: {}", e.to_string().red()))?;

            // Merge existing setting with new values
            deep_merge_values(&mut merged, &new_value);
            remove_nulls(&mut merged);

            macos_items[idx] = merged;
            true
        } else {
            // Setting doesn't exist - add as new
            log_info!(
                "[SDB:Add] Setting '{}.{}' not found. Adding...",
                setting.domain.cyan(),
                setting.key.cyan()
            );

            let mut new_value = serde_yaml::to_value(setting)
                .map_err(|e| format!("Failed to serialize setting: {}", e.to_string().red()))?;
            remove_nulls(&mut new_value);

            macos_items.push(new_value);
            false
        };

        // Write updated settings back to file
        let output = serde_yaml::to_string(&doc)
            .map_err(|e| format!("Failed to serialize config: {}", e.to_string().red()))?;

        fs::write(&config_path, output).map_err(|e| {
            format!(
                "Failed to write {}: {}",
                filename.to_string().red(),
                e.to_string().red()
            )
        })?;

        Ok(was_update)
    }
}

// ============================================================================
// PUBLIC API FUNCTIONS
// ============================================================================

/// Adds or updates a tool configuration in the tools.yaml file
///
/// This is the main entry point for adding tools to the configuration.
/// It validates tool restrictions, creates the tool entry, and updates
/// the configuration file.
///
/// # Arguments
/// * `name` - Name of the tool
/// * `version` - Version of the tool
/// * `source` - Source type ("github", "url", etc.)
/// * `url` - Download URL (required for "url" source)
/// * `repo` - GitHub repository (required for "github" source)
/// * `tag` - GitHub tag/version (required for "github" source)
/// * `rename_to` - Optional rename for the executable
/// * `options` - Additional installation options
/// * `executable_path_after_extract` - Path to executable after extraction
/// * `post_installation_hooks` - Commands to run after installation
/// * `enable_config_manager` - Whether to enable configuration management
/// * `config_paths` - Paths to configuration files for this tool
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
    log_info!("[SDB::Add::Tool] Preparing to add tool: {}...", name.cyan());

    // Initialize path resolver for configuration directory access
    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Add::Tool] Failed to initialize path resolver: {}",
            e.to_string().red()
        );
        std::process::exit(1);
    });

    // Initialize configuration updater
    let updater = ConfigurationUpdater::new(&paths).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Add::Tool] Failed to initialize updater: {}",
            e.to_string().red()
        );
        std::process::exit(1);
    });

    // Create new tool entry from provided parameters
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

    // Validate tool restrictions based on source type
    if let Err(e) = validate_tool_restrictions(&new_tool) {
        log_error!(
            "[SDB::Add::Tool] Validation failed for tool {}: {}",
            name.cyan(),
            e
        );
        std::process::exit(1);
    }

    // Update or add the tool in configuration
    match updater.update_or_add_list_item("tools.yaml", "tools:", "name:", &name, &new_tool) {
        Ok(is_update) => {
            log_info!(
                "[SDB::Add::Tool] Successfully {} tool '{}'",
                if is_update { "updated" } else { "added" },
                name.cyan()
            );
        }
        Err(e) => {
            log_error!("[SDB::Add::Tool] Failed to update config: {}", e);
            std::process::exit(1);
        }
    }

    // Apply changes immediately
    run_now_command();
}

/// Adds or updates a font configuration in the fonts.yaml file
///
/// # Arguments
/// * `name` - Name of the font
/// * `version` - Version of the font
/// * `source` - Source type (typically "github")
/// * `repo` - GitHub repository containing the font
/// * `tag` - GitHub tag/version
/// * `install_only` - Specific font files to install (empty for all)
pub fn add_font(
    name: String,
    version: String,
    source: String,
    repo: String,
    tag: String,
    install_only: Vec<String>,
) {
    log_info!("[SDB::Add::Font] Preparing to add font: {}...", name.cyan());

    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!("[SDB::Add::Font] Failed to initialize path resolver: {}", e);
        std::process::exit(1);
    });

    let updater = ConfigurationUpdater::new(&paths).unwrap_or_else(|e| {
        log_error!("[SDB::Add::Font] Failed to initialize updater: {}", e);
        std::process::exit(1);
    });

    // Create font entry, converting empty install_only to None
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

    match updater.update_or_add_list_item("fonts.yaml", "fonts:", "name:", &name, &new_font) {
        Ok(is_update) => {
            log_info!(
                "[SDB::Add::Font] Successfully {} font '{}'",
                if is_update { "updated" } else { "added" },
                name.cyan()
            );
        }
        Err(e) => {
            log_error!("[SDB::Add::Font] Failed to update config: {}", e);
            std::process::exit(1);
        }
    }

    run_now_command();
}

/// Adds or updates a macOS system setting in the settings.yaml file
///
/// # Arguments
/// * `domain` - Settings domain (e.g., "NSGlobalDomain")
/// * `key` - Setting key within the domain
/// * `value` - Value to set
/// * `value_type` - Type of value ("bool", "string", "int", "float")
pub fn add_setting(domain: String, key: String, value: String, value_type: String) {
    let setting_name = format!("{domain}.{key}");
    log_info!(
        "[SDB::Add::Setting] Preparing to add setting: {}...",
        setting_name.cyan()
    );

    // Validate value type before proceeding
    if !is_valid_value_type(&value_type) {
        log_error!(
            "[SDB::Add::Setting] Invalid value type '{}'. Supported: bool, string, int, float",
            value_type.cyan()
        );
        std::process::exit(1);
    }

    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Add::Setting] Failed to initialize path resolver: {}",
            e
        );
        std::process::exit(1);
    });

    let updater = ConfigurationUpdater::new(&paths).unwrap_or_else(|e| {
        log_error!("[SDB::Add::Setting] Failed to initialize updater: {}", e);
        std::process::exit(1);
    });

    let new_setting = SettingEntry {
        domain: domain.clone(),
        key: key.clone(),
        value,
        value_type,
    };

    // Use specialized settings handler due to nested structure
    match updater.update_or_add_setting(&new_setting) {
        Ok(is_update) => {
            log_info!(
                "[SDB::Add::Setting] Successfully {} setting '{}'",
                if is_update { "updated" } else { "added" },
                setting_name.cyan()
            );
        }
        Err(e) => {
            log_error!("[SDB::Add::Setting] Failed to update config: {}", e);
            std::process::exit(1);
        }
    }

    run_now_command();
}

/// Adds or updates a shell alias in the shellrc.yaml file
///
/// # Arguments
/// * `name` - Name of the alias
/// * `value` - Command/value of the alias
pub fn add_alias(name: String, value: String) {
    log_info!(
        "[SDB::Add::Alias] Preparing to add alias: {}...",
        name.cyan()
    );

    let paths = PathResolver::new(None, None).unwrap_or_else(|e| {
        log_error!(
            "[SDB::Add::Alias] Failed to initialize path resolver: {}",
            e
        );
        std::process::exit(1);
    });

    let updater = ConfigurationUpdater::new(&paths).unwrap_or_else(|e| {
        log_error!("[SDB::Add::Alias] Failed to initialize updater: {}", e);
        std::process::exit(1);
    });

    let new_alias = AliasEntry {
        name: name.clone(),
        value,
    };

    match updater.update_or_add_list_item("shellrc.yaml", "aliases:", "name:", &name, &new_alias) {
        Ok(is_update) => {
            log_info!(
                "[SDB::Add::Alias] Successfully {} alias '{}'",
                if is_update { "updated" } else { "added" },
                name.cyan()
            );
        }
        Err(e) => {
            log_error!("[SDB::Add::Alias] Failed to update config: {}", e);
            std::process::exit(1);
        }
    }

    run_now_command();
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Validates tool configuration based on source type restrictions
///
/// Different source types have different required fields:
/// - "github" requires both `repo` and `tag`
/// - "url" requires `url`
///
/// # Arguments
/// * `tool` - Tool entry to validate
///
/// # Returns
/// * `Result<(), String>` - Ok if valid, error message if invalid
fn validate_tool_restrictions(tool: &ToolEntry) -> Result<(), String> {
    match tool.source.to_lowercase().as_str() {
        "github" => {
            // GitHub sources require repository and tag information
            if tool.repo.is_none() || tool.tag.is_none() {
                return Err("Source is 'github', but requires both 'repo' and
                    'tag' to be provided"
                    .to_owned());
            }
        }
        "url" => {
            // URL sources require a download URL
            if tool.url.is_none() {
                return Err("Source is 'url', but requires 'url' to be provided".to_owned());
            }
        }
        _ => {
            // Other source types don't have specific restrictions
        }
    }
    Ok(())
}

/// Checks if a value type string is valid for settings
///
/// # Arguments
/// * `value_type` - Type string to validate
///
/// # Returns
/// * `bool` - True if valid, false otherwise
fn is_valid_value_type(value_type: &str) -> bool {
    matches!(
        value_type.to_lowercase().as_str(),
        "bool" | "string" | "int" | "float"
    )
}

/// Executes the 'setup-devbox now' command to apply configuration changes
///
/// This ensures that changes made by add commands are immediately applied
/// to the system rather than waiting for manual execution.
fn run_now_command() {
    log_info!(
        "[SDB::Now] Running '{}' to apply changes...",
        "setup-devbox now".cyan()
    );

    match PathResolver::new(None, None) {
        Ok(paths) => now::run(&paths, false),
        Err(e) => {
            log_error!("Failed to initialize path resolver: {}", e.red());
            std::process::exit(1);
        }
    }
}

// ============================================================================
// YAML MANIPULATION UTILITIES
// ============================================================================

/// Deep merge YAML values. Source overwrites target only for non-null values.
///
/// This function performs a recursive merge of two YAML values, with the source
/// values taking precedence over target values. Null values in the source are
/// skipped to avoid overwriting existing data with nulls.
///
/// # Arguments
/// * `target` - The target YAML value to merge into (modified in-place)
/// * `source` - The source YAML value to merge from
fn deep_merge_values(target: &mut Value, source: &Value) {
    // Only perform merge if both values are mappings (dictionaries)
    if let (Some(target_map), Some(source_map)) = (target.as_mapping_mut(), source.as_mapping()) {
        for (key, value) in source_map {
            // Skip null values - don't overwrite with null
            if value.is_null() {
                continue;
            }

            // Skip empty sequences and mappings to avoid clearing existing data
            if value.is_sequence() && value.as_sequence().is_some_and(|s| s.is_empty()) {
                continue;
            }
            if value.is_mapping() && value.as_mapping().is_some_and(|m| m.is_empty()) {
                continue;
            }

            if let Some(target_value) = target_map.get_mut(key) {
                // Recursively merge nested mappings to preserve nested structure
                if target_value.is_mapping() && value.is_mapping() {
                    deep_merge_values(target_value, value);
                } else {
                    // Overwrite for all other types (scalars, sequences, etc.)
                    *target_value = value.clone();
                }
            } else {
                // Insert new key that doesn't exist in target
                target_map.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Recursively remove null values, empty sequences, and empty mappings from YAML
///
/// This function cleans up YAML structures by removing:
/// - Null values
/// - Empty sequences (arrays)
/// - Empty mappings (objects)
/// - Configuration manager sections when disabled
///
/// # Arguments
/// * `value` - The YAML value to clean (modified in-place)
fn remove_nulls(value: &mut Value) {
    match value {
        Value::Mapping(map) => {
            // First pass: remove all null values
            map.retain(|_, v| !v.is_null());

            // Special handling: remove configuration_manager if enabled is false
            // This cleans up configuration manager sections that are disabled
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

            // Second pass: recursively process remaining values and mark empty containers for removal
            let mut keys_to_remove = Vec::new();
            for (k, v) in map.iter_mut() {
                // Recursively clean nested structures
                remove_nulls(v);

                // Mark empty sequences and mappings for removal
                if v.as_sequence().is_some_and(|s| s.is_empty())
                    || v.as_mapping().is_some_and(|m| m.is_empty())
                {
                    keys_to_remove.push(k.clone());
                }
            }

            // Remove all marked empty containers
            for k in keys_to_remove {
                map.remove(&k);
            }
        }
        Value::Sequence(seq) => {
            // Recursively clean all items in the sequence
            for item in seq.iter_mut() {
                remove_nulls(item);
            }
        }
        _ => {
            // Scalars (strings, numbers, booleans) don't need processing
        }
    }
}

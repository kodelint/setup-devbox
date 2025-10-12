// ============================================================================
// RESULT TYPES
// ============================================================================

use crate::schemas::path_resolver::PathResolver;
use crate::{log_debug, log_info, log_warn};
use colored::Colorize;
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Represents the outcome of a single removal operation.
///
/// This enum provides clear feedback about what happened during removal,
/// allowing the orchestrator to build appropriate summaries for the user.
#[derive(Debug, Clone)]
pub enum RemovalResult {
    /// The item was successfully removed from the system.
    Removed,

    /// The item was not found in state or configuration files.
    /// This is not necessarily an error - the user may have specified
    /// an incorrect name or the item may have been manually removed.
    NotFound,

    /// The removal operation failed with a specific error message.
    /// The String contains details about what went wrong.
    Failed(String),
}

/// Aggregates the results of all removal operations in a single command execution.
///
/// This structure is used to build a comprehensive summary that gets displayed
/// to the user after all removal operations complete.
#[derive(Debug, Default)]
pub struct RemovalSummary {
    /// Names of tools that were successfully removed
    pub removed_tools: Vec<String>,

    /// Names of fonts that were successfully removed
    pub removed_fonts: Vec<String>,

    /// Names of items that could not be found
    pub not_found_items: Vec<String>,

    /// Items that failed to remove, along with the reason for failure
    pub failed_removals: Vec<(String, String)>,
}

/// Contains metadata about an item being removed.
///
/// This acts as a data transfer object, carrying all necessary information
/// from the state file to the appropriate uninstaller.
#[derive(Debug)]
pub struct ItemToBeRemoved {
    /// Original name of the item in the state file
    pub item_name: String,

    /// Optional alias/renamed name for the item
    pub items_renamed_to: Option<String>,

    /// Installation method used (cargo, pip, brew, etc.)
    pub item_source: String,

    /// Version string of the installed item
    pub item_version: String,

    /// File system path where the item is installed
    pub item_path: String,
}

// ============================================================================
//                   UNINSTALLER TRAIT & IMPLEMENTATIONS
// ============================================================================

/// Defines the contract for uninstalling tools based on their installation method.
///
/// This trait uses the Strategy pattern to encapsulate installation-method-specific
/// logic. Each installer type (cargo, pip, brew) gets its own implementation that
/// knows exactly how to remove tools installed via that method.
///
/// ## Design Benefits
///
/// - **Separation of Concerns**: Installation-specific logic is isolated
/// - **Extensibility**: New installation methods can be added without modifying existing code
/// - **Testability**: Each uninstaller can be tested independently
pub(crate) trait ToolUninstaller {
    /// Executes the uninstallation process for the given item.
    ///
    /// # Arguments
    ///
    /// * `uninstall_item` - Metadata about the item to remove
    ///
    /// # Returns
    ///
    /// * `Ok(())` if uninstallation succeeded
    /// * `Err(String)` with an error message if uninstallation failed
    ///
    /// # Implementation Notes
    ///
    /// Implementations should:
    /// - Check if the item exists before attempting removal
    /// - Log their actions for debugging
    /// - Return descriptive errors that help users understand what went wrong
    fn uninstall(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String>;
}

/// Removes tools installed as standalone binaries from GitHub releases or direct URLs.
///
/// These tools are typically single executable files downloaded and placed in a bin directory.
/// Removal is straightforward: just delete the file at the recorded installation path.
pub(crate) struct BinaryUninstaller;

impl ToolUninstaller for BinaryUninstaller {
    fn uninstall(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String> {
        let path = PathBuf::from(&uninstall_item.item_path);
        log_debug!(
            "[SDB::Remove::Tool::Binary] Attempting to remove file at: {}",
            path.display()
        );

        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove binary at {}: {}", path.display(), e))?;
            log_info!(
                "[SDB::Remove::Tool::Binary] Deleted: {}",
                path.display().to_string().cyan()
            );
        } else {
            log_warn!(
                "[SDB::Remove::Tool::Binary] File not found at expected location: {}",
                path.display().to_string().yellow()
            );
        }

        Ok(())
    }
}

/// Removes Rust packages installed via `cargo install`.
///
/// Uses the `cargo uninstall` command, which handles removing the binary
/// and any associated files from the cargo installation directory.
pub(crate) struct CargoUninstaller;

impl ToolUninstaller for CargoUninstaller {
    fn uninstall(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String> {
        log_info!(
            "[SDB::Remove::Tool::Cargo] Uninstalling: {}",
            uninstall_item.item_name.cyan()
        );

        let output = Command::new("cargo")
            .args(["uninstall", &uninstall_item.item_name])
            .output()
            .map_err(|e| format!("Failed to execute cargo uninstall: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cargo uninstall failed: {}", stderr));
        }

        log_info!("[SDB::Remove::Tool::Cargo] Successfully uninstalled package");
        Ok(())
    }
}

/// Removes Rust toolchains installed via `rustup`.
///
/// Rust toolchains (like stable, nightly, or specific versions) are managed
/// by rustup. This uninstaller delegates to `rustup toolchain uninstall`.
pub(crate) struct RustupUninstaller;

impl ToolUninstaller for RustupUninstaller {
    fn uninstall(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String> {
        log_info!(
            "[SDB::Remove::Tool::Rustup] Uninstalling toolchain: {}",
            uninstall_item.item_version.cyan()
        );

        let output = Command::new("rustup")
            .args(["toolchain", "uninstall", &uninstall_item.item_version])
            .output()
            .map_err(|e| format!("Failed to execute rustup: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("rustup toolchain uninstall failed: {}", stderr));
        }

        log_info!("[SDB::Remove::Tool::Rustup] Successfully uninstalled toolchain");
        Ok(())
    }
}

/// Removes Go packages installed via `go install`.
///
/// Go doesn't provide a built-in uninstall command, so we manually locate
/// and delete the binary from $GOPATH/bin.
pub(crate) struct GoUninstaller;

impl ToolUninstaller for GoUninstaller {
    fn uninstall(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String> {
        // Determine GOPATH, falling back to $HOME/go if not set
        let gopath = std::env::var("GOPATH")
            .or_else(|_| std::env::var("HOME").map(|h| format!("{}/go", h)))
            .map_err(|_| "Failed to determine GOPATH".to_string())?;

        // Use the renamed name if available, otherwise use original name
        let binary_name = uninstall_item
            .items_renamed_to
            .as_ref()
            .unwrap_or(&uninstall_item.item_name);

        let binary_path = PathBuf::from(gopath).join("bin").join(binary_name);
        log_debug!(
            "[SDB::Remove::Tool::Go] Target binary: {}",
            binary_path.display()
        );

        if binary_path.exists() {
            fs::remove_file(&binary_path).map_err(|e| {
                format!(
                    "Failed to remove Go binary at {}: {}",
                    binary_path.display(),
                    e
                )
            })?;
            log_info!(
                "[SDB::Remove::Tool::Go] Deleted: {}",
                binary_path.display().to_string().cyan()
            );
        } else {
            log_warn!(
                "[SDB::Remove::Tool::Go] Binary not found at: {}",
                binary_path.display().to_string().yellow()
            );
        }

        Ok(())
    }
}

/// Removes Python packages installed via `pip3`.
///
/// Uses `pip3 uninstall -y` to remove packages without prompting for confirmation.
pub(crate) struct PipUninstaller;

impl ToolUninstaller for PipUninstaller {
    fn uninstall(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String> {
        log_info!(
            "[SDB::Remove::Tool::Pip] Uninstalling: {}",
            uninstall_item.item_name.cyan()
        );

        let output = Command::new("pip3")
            .args(["uninstall", "-y", &uninstall_item.item_name])
            .output()
            .map_err(|e| format!("Failed to execute pip3 uninstall: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pip3 uninstall failed: {}", stderr));
        }

        log_info!("[SDB::Remove::Tool::Pip] Successfully uninstalled package");
        Ok(())
    }
}

/// Removes Python tools installed via `uv tool install`.
///
/// UV is a modern Python package installer. This uninstaller uses
/// `uv tool uninstall` to remove tools from the UV-managed environment.
pub(crate) struct UvUninstaller;

impl ToolUninstaller for UvUninstaller {
    fn uninstall(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String> {
        log_info!(
            "[SDB::Remove::Tool::UV] Uninstalling: {}",
            uninstall_item.item_name.cyan()
        );

        let output = Command::new("uv")
            .args(["tool", "uninstall", &uninstall_item.item_name])
            .output()
            .map_err(|e| format!("Failed to execute uv tool uninstall: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("uv tool uninstall failed: {}", stderr));
        }

        log_info!("[SDB::Remove::Tool::UV] Successfully uninstalled tool");
        Ok(())
    }
}

/// Removes packages installed via Homebrew.
///
/// Uses `brew uninstall` to remove formulas. Homebrew handles dependency
/// management and cleanup automatically.
pub(crate) struct BrewUninstaller;

impl ToolUninstaller for BrewUninstaller {
    fn uninstall(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String> {
        log_info!(
            "[SDB::Remove::Tool::Brew] Uninstalling: {}",
            uninstall_item.item_name.cyan()
        );

        let output = Command::new("brew")
            .args(["uninstall", &uninstall_item.item_name])
            .output()
            .map_err(|e| format!("Failed to execute brew uninstall: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Homebrew returns an error if the formula isn't installed
            // We treat "No such keg" as a non-fatal condition
            if !stderr.contains("No such keg") {
                return Err(format!("brew uninstall failed: {}", stderr));
            }
        }

        log_info!("[SDB::Remove::Tool::Brew] Successfully uninstalled formula");
        Ok(())
    }
}

// ============================================================================
//                         CONFIGURATION CLEANER
// ============================================================================

/// Manages removal of items from YAML configuration files.
///
/// This component encapsulates all YAML manipulation logic, providing a clean
/// interface for removing tools, fonts, aliases, and settings from their
/// respective configuration files.
///
/// ## Supported Operations
///
/// - Remove tools from tools.yaml
/// - Remove fonts from fonts.yaml
/// - Remove aliases from shellrc.yaml
/// - Remove settings from settings.yaml (nested structure)
///
/// ## File Format Expectations
///
/// The cleaner expects YAML files with the following structures:
///
/// ```yaml
/// # tools.yaml, fonts.yaml, shellrc.yaml
/// items:
///   - name: item1
///     # other fields...
///   - name: item2
///     # other fields...
///
/// # settings.yaml
/// settings:
///   macos:
///     - domain: com.apple.dock
///       key: autohide
///       # other fields...
/// ```
pub(crate) struct ConfigurationCleaner {
    /// Base directory where all configuration YAML files are stored
    config_base_path: PathBuf,
}

impl ConfigurationCleaner {
    /// Creates a new ConfigurationCleaner instance.
    ///
    /// # Arguments
    ///
    /// * `paths` - PathResolver to locate the configuration directory
    ///
    /// # Returns
    ///
    /// * `Ok(ConfigurationCleaner)` if initialization succeeded
    /// * `Err(String)` if the configuration directory could not be determined
    pub fn new(paths: &PathResolver) -> Result<Self, String> {
        log_debug!("[SDB::Remove::Config] Initializing ConfigurationCleaner");
        let config_base = paths.configs_dir();
        log_debug!(
            "[SDB::Remove::Config] Using directory: {}",
            config_base.display()
        );

        Ok(ConfigurationCleaner {
            config_base_path: config_base,
        })
    }

    /// Constructs the full path to a configuration file.
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the configuration file (e.g., "tools.yaml")
    ///
    /// # Returns
    ///
    /// Full path to the configuration file
    fn get_config_path(&self, filename: &str) -> PathBuf {
        self.config_base_path.join(filename)
    }

    /// Reads and parses a YAML configuration file.
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the configuration file to read
    ///
    /// # Returns
    ///
    /// * `Ok((PathBuf, String, Value))` - Tuple of (file path, raw content, parsed YAML)
    /// * `Err(String)` - If the file doesn't exist or parsing fails
    ///
    /// # Implementation Notes
    ///
    /// This method returns the raw content string along with the parsed YAML
    /// to enable advanced use cases like preserving comments or formatting.
    /// Currently, we only use the parsed Value, but the content is available
    /// for future enhancements.
    fn read_yaml_file(&self, filename: &str) -> Result<(PathBuf, String, Value), String> {
        let config_path = self.get_config_path(filename);
        log_debug!("[SDB::Remove::Config] Reading: {:?}", config_path);

        if !config_path.exists() {
            return Err("File not found".to_string());
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read {}: {}", filename, e))?;

        let doc: Value = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", filename, e))?;

        Ok((config_path, content, doc))
    }

    /// Helper to read a YAML file and handle the common "file not found" case.
    ///
    /// Many removal operations treat a missing configuration file as a non-error
    /// (the item wasn't configured, so there's nothing to remove). This helper
    /// encapsulates that pattern.
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the configuration file to read
    ///
    /// # Returns
    ///
    /// * `Ok(Some((PathBuf, String, Value)))` - File exists and was parsed successfully
    /// * `Ok(None)` - File not found (not an error)
    /// * `Err(String)` - File exists but parsing failed
    fn read_yaml_file_optional(
        &self,
        filename: &str,
    ) -> Result<Option<(PathBuf, String, Value)>, String> {
        match self.read_yaml_file(filename) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e == "File not found" => {
                log_warn!("[SDB::Remove::Config] File not found: {}", filename);
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Writes a modified YAML document back to disk.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file
    /// * `doc` - Modified YAML document to write
    /// * `filename` - Name of the file (for error messages)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if to write succeeded
    /// * `Err(String)` if serialization or writing failed
    fn write_yaml_file(
        &self,
        config_path: &Path,
        doc: &Value,
        filename: &str,
    ) -> Result<(), String> {
        let output = serde_yaml::to_string(&doc)
            .map_err(|e| format!("Failed to serialize {}: {}", filename, e))?;

        fs::write(config_path, output)
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;

        log_debug!("[SDB::Remove::Config] Wrote updated file: {}", filename);
        Ok(())
    }

    /// Removes an item from a list within a YAML configuration file.
    ///
    /// This is a generic method that works for tools, fonts, and aliases,
    /// all of which use a similar list structure in their YAML files.
    ///
    /// # Arguments
    ///
    /// * `filename` - Configuration file name (e.g., "tools.yaml")
    /// * `section_key` - Top-level key containing the list (e.g., "tools:")
    /// * `item_key` - Key used to identify items in the list (e.g., "name:")
    /// * `item_identifier` - Value to match for removal (e.g., "git")
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Item was found and removed
    /// * `Ok(false)` - Item was not found
    /// * `Err(String)` - File operation or parsing failed
    ///
    /// # Example
    ///
    /// ```rust
    /// // Remove the tool "git" from tools.yaml
    /// cleaner.remove_list_item("tools.yaml", "tools:", "name:", "git")?;
    /// ```
    pub fn remove_list_item(
        &self,
        filename: &str,
        section_key: &str,
        item_key: &str,
        item_identifier: &str,
    ) -> Result<bool, String> {
        // Try to read the file; if it doesn't exist, that's not an error
        let (config_path, _, mut doc) = match self.read_yaml_file_optional(filename)? {
            Some(data) => data,
            None => return Ok(false),
        };

        // Navigate to the target section (strip trailing colon if present)
        let section_name = section_key.trim_end_matches(':');
        let items = doc
            .get_mut(section_name)
            .and_then(|v| v.as_sequence_mut())
            .ok_or_else(|| {
                format!(
                    "Section '{}' not found or invalid in {}",
                    section_name, filename
                )
            })?;

        // Find the item by matching the specified key
        let item_key_trimmed = item_key.trim_end_matches(':');
        let item_idx = items.iter().position(|item| {
            item.get(item_key_trimmed).and_then(|v| v.as_str()) == Some(item_identifier)
        });

        if let Some(idx) = item_idx {
            // Remove the item and write the updated file
            items.remove(idx);
            log_info!(
                "[SDB::Remove::Config] Removed '{}' from {}",
                item_identifier.cyan(),
                filename
            );

            self.write_yaml_file(&config_path, &doc, filename)?;
            Ok(true)
        } else {
            log_warn!(
                "[SDB::Remove::Config] Item '{}' not found in {}",
                item_identifier.yellow(),
                filename
            );
            Ok(false)
        }
    }

    /// Removes a macOS system setting from settings.yaml.
    ///
    /// Settings are stored in a nested structure: settings.macos contains a list
    /// of setting objects, each with a domain and key field.
    ///
    /// # Arguments
    ///
    /// * `domain` - The macOS defaults domain (e.g., "com.apple.dock")
    /// * `key` - The setting key within the domain (e.g., "autohide")
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Setting was found and removed
    /// * `Ok(false)` - Setting was not found
    /// * `Err(String)` - File operation or parsing failed
    ///
    /// # Example
    ///
    /// ```rust
    /// // Remove the dock autohide setting
    /// cleaner.remove_setting("com.apple.dock", "autohide")?;
    /// ```
    pub fn remove_setting(&self, domain: &str, key: &str) -> Result<bool, String> {
        let filename = "settings.yaml";
        let (config_path, _, mut doc) = match self.read_yaml_file_optional(filename)? {
            Some(data) => data,
            None => return Ok(false),
        };

        // Navigate to settings.macos list
        let macos_items = doc
            .get_mut("settings")
            .and_then(|v| v.get_mut("macos"))
            .and_then(|v| v.as_sequence_mut())
            .ok_or_else(|| "settings.macos section not found in settings.yaml".to_string())?;

        // Find the setting by matching both domain and key
        let item_idx = macos_items.iter().position(|item| {
            item.get("domain").and_then(|v| v.as_str()) == Some(domain)
                && item.get("key").and_then(|v| v.as_str()) == Some(key)
        });

        if let Some(idx) = item_idx {
            macos_items.remove(idx);
            log_info!(
                "[SDB::Remove::Config] Removed setting '{}.{}' from {}",
                domain.cyan(),
                key.cyan(),
                filename
            );

            self.write_yaml_file(&config_path, &doc, filename)?;
            Ok(true)
        } else {
            log_warn!(
                "[SDB::Remove::Config] Setting '{}.{}' not found in {}",
                domain.yellow(),
                key.yellow(),
                filename
            );
            Ok(false)
        }
    }
}

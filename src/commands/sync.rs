//! # Configuration Synchronization Module
//!
//! This module implements the `sync` command for setup-devbox, which regenerates
//! configuration YAML files from the application's state file (state.json).
//!
//! ## Purpose
//!
//! The sync command serves as a recovery and migration mechanism that allows users to:
//! - Restore configuration files if they become corrupted or deleted
//! - Migrate from state-based storage to config-based management
//! - Generate initial configuration files from an existing installation
//!
//! ## Architecture
//!
//! The module follows a clean separation of concerns:
//!
//! 1. **State Reading**: Parse state.json into typed structures
//! 2. **Transformation**: Convert state data to configuration format
//! 3. **File Generation**: Write properly formatted YAML files
//!
//! ## Generated Files
//!
//! The sync process generates five configuration files:
//! - `config.yaml`: Main configuration file with references to all others
//! - `tools.yaml`: Development tools and installation configurations
//! - `fonts.yaml`: Font installation configurations
//! - `settings.yaml`: OS-specific system settings (macOS)
//! - `shellrc.yaml`: Shell initialization template (not tracked in state)

use crate::libs::configuration_manager::ConfigurationManagerState;
use crate::schemas::configuration_management::ConfigurationManager;
use crate::schemas::fonts::FontEntry;
use crate::schemas::os_settings::{OsSpecificSettings, SettingEntry, SettingsConfig};
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::shell_configuration::AliasEntry;
use crate::schemas::state_file::{FontState, SettingState, ToolState};
use crate::schemas::tools::ToolEntry;
use crate::{log_debug, log_error, log_info};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// Comprehensive error types for sync operations
///
/// Each variant represents a specific failure mode that can occur during
/// the synchronization process, allowing for targeted error handling and
/// clear user feedback.
#[derive(Debug)]
pub enum SyncError {
    /// File system operation failed (read/write/create directory)
    Io(std::io::Error),

    /// Failed to parse JSON from state file
    Json(serde_json::Error),

    /// Failed to serialize data to YAML format
    Yaml(serde_yaml::Error),

    /// Path resolution or validation failed
    PathError(String),
}

impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        SyncError::Io(e)
    }
}

impl From<serde_json::Error> for SyncError {
    fn from(e: serde_json::Error) -> Self {
        SyncError::Json(e)
    }
}

impl From<serde_yaml::Error> for SyncError {
    fn from(e: serde_yaml::Error) -> Self {
        SyncError::Yaml(e)
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Io(e) => write!(f, "I/O Error: {}", e.to_string().red()),
            SyncError::Json(e) => write!(f, "JSON Parsing Error: {}", e.to_string().red()),
            SyncError::Yaml(e) => write!(f, "YAML Serialization Error: {}", e.to_string().red()),
            SyncError::PathError(s) => write!(f, "Path Error: {}", s.to_string().red()),
        }
    }
}

/// Result type alias for sync operations
pub type SyncResult<T> = Result<T, SyncError>;

// ============================================================================
// STATE STRUCTURES
// ============================================================================

/// Root application state structure
///
/// This represents the deserialized state.json file, which contains the
/// current installation state of all managed resources. The state file
/// is the source of truth for what's actually installed on the system.
#[derive(Debug, Deserialize)]
pub struct AppState {
    /// Installed development tools indexed by tool name
    pub tools: HashMap<String, ToolState>,

    /// Applied OS settings indexed by "domain.key" format
    pub settings: HashMap<String, SettingState>,

    /// Installed fonts indexed by font name
    pub fonts: HashMap<String, FontState>,
    // Note: Shell configuration (aliases, run_commands) is not tracked in state
}

// ============================================================================
// CONFIGURATION STRUCTURES
// ============================================================================

/// Main configuration file structure (config.yaml)
///
/// This file serves as the entry point, containing absolute paths to all
/// other configuration files. This design allows the application to locate
/// all configuration resources from a single reference file.
#[derive(Debug, Serialize)]
pub struct MainConfig {
    /// Absolute path to tools.yaml
    pub tools: String,

    /// Absolute path to settings.yaml
    pub settings: String,

    /// Absolute path to shellrc.yaml
    pub shellrc: String,

    /// Absolute path to fonts.yaml
    pub fonts: String,
}

/// Tools configuration file structure (tools.yaml)
#[derive(Debug, Serialize)]
pub struct ToolConfig {
    /// Optional update check interval (e.g., "7 days")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_latest_only_after: Option<String>,

    /// List of all tool configurations
    pub tools: Vec<ToolEntry>,
}

/// Fonts configuration file structure (fonts.yaml)
#[derive(Debug, Serialize)]
pub struct FontsConfig {
    /// List of all font configurations
    pub fonts: Vec<FontEntry>,
}

/// Shell configuration file structure (shellrc.yaml)
#[derive(Debug, Serialize)]
pub struct ShellConfig {
    /// Shell-specific commands to run at startup
    pub run_commands: ShellCommands,

    /// Command aliases for convenience shortcuts
    pub aliases: Vec<AliasEntry>,
}

/// Shell-specific command configuration
#[derive(Debug, Serialize)]
pub struct ShellCommands {
    /// The shell interpreter (e.g., "zsh", "bash")
    pub shell: String,

    /// List of commands to execute during shell startup
    pub run_commands: Vec<ShellCommand>,
}

/// Individual shell command with categorization
#[derive(Debug, Serialize)]
pub struct ShellCommand {
    /// The actual command to execute
    pub command: String,

    /// Logical grouping (e.g., "Exports", "PATH", "Initialization")
    pub section: String,
}

// ============================================================================
// TYPE CONVERSIONS
// ============================================================================

impl From<&FontState> for FontEntry {
    /// Converts font state to configuration format
    ///
    /// This is a straightforward mapping since font configurations are simple
    /// and don't require complex transformation logic.
    fn from(font_state: &FontState) -> Self {
        FontEntry {
            name: font_state.name.clone(),
            source: font_state.install_method.clone(),
            version: Some(font_state.version.clone()),
            repo: font_state.repo.clone(),
            tag: font_state.tag.clone(),
            install_only: font_state.install_only.clone(),
        }
    }
}

impl ToolEntry {
    /// Constructs a ToolEntry from state data with business rule transformations
    ///
    /// This method encapsulates all the complex logic for converting state
    /// representation to configuration representation, including:
    ///
    /// ## Transformations Applied
    ///
    /// 1. **Source Type Normalization**: Maps verbose state install methods
    ///    to concise config source types (e.g., "uv-python" -> "uv")
    ///
    /// 2. **URL Filtering**: GitHub sources omit URLs since they're derived
    ///    from the repository field
    ///
    /// 3. **Version Handling**: Special rules for version fields:
    ///    - Brew tools with "latest" omit the version entirely
    ///    - Version keywords (stable/nightly) are normalized
    ///    - Specific versions are preserved as-is
    ///
    /// 4. **Field Omission**: Empty vectors and disabled features are omitted
    ///    to keep configuration files clean and readable
    ///
    /// 5. **Configuration Manager**: Only included if explicitly enabled
    ///
    /// # Arguments
    ///
    /// * `name` - The tool's unique identifier
    /// * `tool_state` - The tool's current state from state.json
    ///
    /// # Returns
    ///
    /// A ToolEntry suitable for serialization to tools.yaml
    pub fn from_state(name: String, tool_state: &ToolState) -> Self {
        // Normalize the installation method to a shorter source type
        let source = ToolState::normalize_source_type(&tool_state.install_method);

        // Determine if URL should be included based on source type
        let url_for_config = Self::resolve_url_for_source(&source, &tool_state.url, &name);

        // Only include configuration manager if enabled
        let config_manager = Self::resolve_configuration_manager(&tool_state.configuration_manager);

        ToolEntry {
            name,
            version: Self::normalize_version(&tool_state.version, &tool_state.install_method),
            source,
            url: url_for_config,
            repo: tool_state.repo.clone(),
            tag: tool_state.tag.clone(),
            rename_to: tool_state.renamed_to.clone(),
            options: Self::filter_empty_vec(tool_state.options.clone()),
            executable_path_after_extract: tool_state.executable_path_after_extract.clone(),
            post_installation_hooks: Self::filter_empty_vec(
                tool_state.executed_post_installation_hooks.clone(),
            ),
            configuration_manager: config_manager.unwrap_or_else(|| ConfigurationManager {
                enabled: false,
                tools_configuration_paths: Vec::new(),
            }),
        }
    }

    /// Normalizes version strings according to configuration conventions
    ///
    /// This method implements version field logic to keep configurations clean
    /// and meaningful. Different installation sources have different conventions
    /// for version specifications.
    ///
    /// ## Version Handling Rules
    ///
    /// 1. **Brew + "latest"**: Version field is omitted entirely
    ///    - Rationale: Brew always installs latest by default
    ///
    /// 2. **Other sources + "latest"**: Kept as "latest"
    ///    - Rationale: Explicitly requests latest version
    ///
    /// 3. **Versions containing "stable"**: Normalized to "stable"
    ///    - Example: "stable-2024" -> "stable"
    ///
    /// 4. **Versions containing "nightly"**: Normalized to "nightly"
    ///    - Example: "nightly-x86_64" -> "nightly"
    ///
    /// 5. **Specific versions**: Preserved exactly
    ///    - Example: "1.2.3" -> "1.2.3"
    ///
    /// # Arguments
    ///
    /// * `version` - The version string from state
    /// * `install_method` - The installation method (affects version handling)
    ///
    /// # Returns
    ///
    /// `None` to omit the field, or `Some(String)` with the normalized version
    fn normalize_version(version: &str, install_method: &str) -> Option<String> {
        match version {
            "latest" if install_method == "brew" => {
                log_debug!("[Sync::Tool] Omitting version field for brew tool with 'latest'");
                None
            }
            "latest" => Some("latest".to_string()),
            v if v.contains("stable") => Some("stable".to_string()),
            v if v.contains("nightly") => Some("nightly".to_string()),
            v => Some(v.to_string()),
        }
    }

    /// Determines if URL field should be included based on source type
    ///
    /// GitHub sources derive their download URLs from repository and tag fields,
    /// so including a URL would be redundant. This method implements that logic.
    ///
    /// # Arguments
    ///
    /// * `source` - The normalized source type
    /// * `url` - The URL from state (if any)
    /// * `name` - The tool name (for debug logging)
    ///
    /// # Returns
    ///
    /// `None` for GitHub sources (URL is redundant), otherwise the state's URL
    fn resolve_url_for_source(source: &str, url: &Option<String>, name: &str) -> Option<String> {
        match source {
            "github" => {
                log_debug!("[Sync::Tool] Omitting URL for GitHub tool: {}", name);
                None
            }
            _ => url.clone(),
        }
    }

    /// Filters out empty vectors to avoid cluttering YAML output
    ///
    /// Empty arrays in configuration files add visual noise without providing
    /// any value. This method ensures fields like `options: []` are omitted.
    ///
    /// # Arguments
    ///
    /// * `vec_opt` - Optional vector that may be empty
    ///
    /// # Returns
    ///
    /// `None` if input is `None` or empty vector, otherwise `Some(vec)`
    fn filter_empty_vec(vec_opt: Option<Vec<String>>) -> Option<Vec<String>> {
        vec_opt.filter(|v| !v.is_empty())
    }

    /// Resolves configuration manager settings, omitting if disabled
    ///
    /// The configuration manager feature should only appear in YAML files
    /// when it's explicitly enabled. This prevents cluttering configurations
    /// with disabled features.
    ///
    /// # Arguments
    ///
    /// * `config_manager` - Optional configuration manager state
    ///
    /// # Returns
    ///
    /// `None` if disabled or absent, otherwise `Some(ConfigurationManager)`
    fn resolve_configuration_manager(
        config_manager: &Option<ConfigurationManagerState>,
    ) -> Option<ConfigurationManager> {
        config_manager
            .as_ref()
            .filter(|mgr| mgr.enabled)
            .map(|mgr| ConfigurationManager {
                enabled: mgr.enabled,
                tools_configuration_paths: mgr.tools_configuration_paths.clone(),
            })
    }
}

// ============================================================================
// FILE GENERATION
// ============================================================================

/// Responsible for writing Rust data structures to disk as formatted YAML
///
/// The FileWriter ensures consistent formatting across all generated
/// configuration files, making them readable and maintainable.
pub struct FileWriter;

impl FileWriter {
    pub fn new() -> Self {
        FileWriter
    }

    /// Writes data as properly formatted YAML to the specified path
    ///
    /// This method handles the complete file writing process:
    /// 1. Serialize data structure to YAML
    /// 2. Apply formatting enhancements for readability
    /// 3. Write to disk with proper error handling
    ///
    /// ## Formatting Applied
    ///
    /// - Blank lines between top-level list items for visual separation
    /// - Consistent indentation (3 spaces for list item content)
    /// - Quotes around specific values that might be misinterpreted
    /// - Trailing newline for POSIX compliance
    ///
    /// # Arguments
    ///
    /// * `path` - Target file path for writing
    /// * `data` - Data structure to serialize (must implement Serialize)
    ///
    /// # Errors
    ///
    /// Returns `SyncError::Yaml` if serialization fails, or `SyncError::Io`
    /// if file writing fails
    pub fn write_yaml<T: Serialize>(&self, path: &Path, data: &T) -> SyncResult<()> {
        log_debug!(
            "[Sync::FileWriter] Serializing data for: {}",
            path.display()
        );

        // Serialize to YAML string
        let mut yaml = serde_yaml::to_string(data)?;

        // Apply formatting enhancements
        yaml = self.format_yaml_output(&yaml);

        // Ensure file ends with newline (POSIX standard)
        if !yaml.ends_with('\n') {
            yaml.push('\n');
        }

        // Write to disk
        fs::write(path, yaml)?;

        log_debug!("[Sync::FileWriter] Successfully wrote: {}", path.display());
        Ok(())
    }

    /// Applies formatting rules to improve YAML readability
    ///
    /// This method transforms the raw YAML output from serde_yaml into a more
    /// human-friendly format with better visual structure.
    ///
    /// ## Transformations
    ///
    /// 1. **Blank line separation**: Adds blank line before each list item
    ///    - Before: `- name: tool1\n- name: tool2`
    ///    - After: `- name: tool1\n\n- name: tool2`
    ///
    /// 2. **Consistent indentation**: Adjusts list item content indentation
    ///    - Ensures content is indented 3 spaces from margin
    ///
    /// 3. **Quote time values**: Adds quotes to time-related values
    ///    - Example: `update_latest_only_after: "7 days"`
    ///
    /// # Arguments
    ///
    /// * `yaml` - Raw YAML string from serialization
    ///
    /// # Returns
    ///
    /// Formatted YAML string ready for writing to file
    fn format_yaml_output(&self, yaml: &str) -> String {
        let mut formatted = yaml.to_string();

        // Add blank line before each list item for visual separation
        formatted = formatted.replace("\n- ", "\n\n - ");

        // Adjust indentation: content should be 3 spaces from margin
        formatted = formatted.replace("\n  ", "\n   ");

        // Quote time-related values to prevent misinterpretation
        if formatted.contains("update_latest_only_after:") {
            formatted = formatted
                .lines()
                .map(|line| {
                    if line.trim_start().starts_with("update_latest_only_after:") {
                        self.quote_value_after_colon(line)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
        }

        formatted
    }

    /// Adds quotes around a YAML value if not already quoted
    ///
    /// # Arguments
    ///
    /// * `line` - YAML line containing key: value
    ///
    /// # Returns
    ///
    /// Line with quoted value if necessary
    fn quote_value_after_colon(&self, line: &str) -> String {
        if let Some(colon_pos) = line.find(':') {
            let key_part = &line[..=colon_pos];
            let value_part = line[colon_pos + 1..].trim();

            // Only add quotes if not already quoted
            if !value_part.starts_with('"') && !value_part.starts_with('\'') {
                return format!("{key_part} \"{value_part}\"");
            }
        }
        line.to_string()
    }
}

// ============================================================================
// CONFIGURATION GENERATION
// ============================================================================

/// Core engine for converting application state to configuration files
///
/// The ConfigGenerator encapsulates all business logic for transforming
/// state data into properly formatted configuration YAML files. It handles
/// the complete generation workflow for all configuration types.
pub struct ConfigGenerator {
    /// Directory where all configuration files will be written
    output_dir: PathBuf,

    /// File writer for YAML serialization and disk I/O
    writer: FileWriter,
}

impl ConfigGenerator {
    /// Creates a new ConfigGenerator targeting the specified output directory
    ///
    /// # Arguments
    ///
    /// * `output_dir` - Path to directory where configs will be generated
    pub fn new(output_dir: PathBuf) -> Self {
        ConfigGenerator {
            output_dir,
            writer: FileWriter::new(),
        }
    }

    /// Primary entry point for configuration generation workflow
    ///
    /// This method orchestrates the complete configuration generation process,
    /// creating all five configuration files in the correct order:
    ///
    /// ## Generation Order
    ///
    /// 1. **tools.yaml**: Tool installation configurations
    /// 2. **fonts.yaml**: Font installation configurations
    /// 3. **settings.yaml**: OS-specific system settings
    /// 4. **shellrc.yaml**: Shell initialization template
    /// 5. **config.yaml**: Main config referencing all others (must be last)
    ///
    /// The main config must be generated last because it needs the absolute
    /// paths of all other files, which may not exist until they're created.
    ///
    /// # Arguments
    ///
    /// * `app_state` - Deserialized application state from state.json
    ///
    /// # Returns
    ///
    /// Vector of paths to all successfully generated configuration files
    ///
    /// # Errors
    ///
    /// Returns `SyncError` if any file generation step fails
    pub fn generate_configs(&self, app_state: &AppState) -> SyncResult<Vec<PathBuf>> {
        log_info!("[Sync] Beginning configuration generation from state...");

        // Define all target paths
        let tools_path = self.output_dir.join("tools.yaml");
        let fonts_path = self.output_dir.join("fonts.yaml");
        let settings_path = self.output_dir.join("settings.yaml");
        let shellrc_path = self.output_dir.join("shellrc.yaml");
        let main_config_path = self.output_dir.join("config.yaml");

        // Generate configuration files
        self.generate_tools_config(&app_state.tools, &tools_path)?;
        self.generate_fonts_config(&app_state.fonts, &fonts_path)?;
        self.generate_settings_config(&app_state.settings, &settings_path)?;
        self.generate_shellrc_config(&shellrc_path)?;

        // Generate main config last (requires all other paths to exist)
        self.generate_main_config(
            &main_config_path,
            &tools_path,
            &settings_path,
            &shellrc_path,
            &fonts_path,
        )?;

        log_info!("[Sync] Configuration generation completed successfully");

        Ok(vec![
            main_config_path,
            tools_path,
            fonts_path,
            settings_path,
            shellrc_path,
        ])
    }

    /// Generates the main config.yaml with references to all other configs
    ///
    /// The main configuration file serves as the application's entry point,
    /// containing absolute paths to all other configuration files. This design
    /// allows the application to locate all resources from a single file.
    ///
    /// ## Path Resolution
    ///
    /// Uses `canonicalize()` to convert relative paths to absolute paths,
    /// which provides several benefits:
    /// - Eliminates ambiguity about file locations
    /// - Makes configs portable across directory structures
    /// - Simplifies debugging (clear where files are)
    ///
    /// # Arguments
    ///
    /// * `target_path` - Where config.yaml will be written
    /// * `tools_path` - Path to tools.yaml
    /// * `settings_path` - Path to settings.yaml
    /// * `shell_config_path` - Path to shellrc.yaml
    /// * `fonts_path` - Path to fonts.yaml
    fn generate_main_config(
        &self,
        target_path: &Path,
        tools_path: &Path,
        settings_path: &Path,
        shell_config_path: &Path,
        fonts_path: &Path,
    ) -> SyncResult<()> {
        log_debug!("[Sync] Generating main config.yaml...");

        let main_config = MainConfig {
            tools: self.path_to_string(tools_path),
            settings: self.path_to_string(settings_path),
            shellrc: self.path_to_string(shell_config_path),
            fonts: self.path_to_string(fonts_path),
        };

        self.writer.write_yaml(target_path, &main_config)?;

        log_info!(
            "[Sync] Generated {} at {}",
            "config.yaml".cyan(),
            target_path.display()
        );

        Ok(())
    }

    /// Generates tools.yaml from tool state data
    ///
    /// Transforms the state's tool records into configuration format, applying
    /// all necessary business rules for source types, versions, and optional fields.
    ///
    /// ## Configuration Structure
    ///
    /// ```yaml
    /// update_latest_only_after: "7 days"
    /// tools:
    ///   - name: tool1
    ///     version: "1.0.0"
    ///     source: github
    ///     repo: user/repo
    ///     tag: v1.0.0
    /// ```
    ///
    /// # Arguments
    ///
    /// * `tools` - HashMap of tool names to their state data
    /// * `target_path` - Where tools.yaml will be written
    fn generate_tools_config(
        &self,
        tools: &HashMap<String, ToolState>,
        target_path: &Path,
    ) -> SyncResult<()> {
        log_debug!("[Sync] Generating tools.yaml from {} tools...", tools.len());

        // Transform state data to configuration format
        let tool_entries: Vec<ToolEntry> = tools
            .iter()
            .map(|(name, state)| ToolEntry::from_state(name.clone(), state))
            .collect();

        let tool_config = ToolConfig {
            // Default update check interval
            update_latest_only_after: Some("7 days".to_string()),
            tools: tool_entries,
        };

        self.writer.write_yaml(target_path, &tool_config)?;

        log_info!(
            "[Sync] Generated {} with {} tools at {}",
            "tools.yaml".cyan(),
            tools.len(),
            target_path.display()
        );

        Ok(())
    }

    /// Generates fonts.yaml from font state data
    ///
    /// Creates a fonts configuration file from state entries. Font configurations
    /// are simpler than tools and require minimal transformation.
    ///
    /// ## Configuration Structure
    ///
    /// ```yaml
    /// fonts:
    ///   - name: FiraCode
    ///     source: github
    ///     version: "6.2"
    ///     repo: tonsky/FiraCode
    ///     tag: "6.2"
    /// ```
    ///
    /// # Arguments
    ///
    /// * `fonts` - HashMap of font names to their state data
    /// * `target_path` - Where fonts.yaml will be written
    fn generate_fonts_config(
        &self,
        fonts: &HashMap<String, FontState>,
        target_path: &Path,
    ) -> SyncResult<()> {
        log_debug!("[Sync] Generating fonts.yaml from {} fonts...", fonts.len());

        let font_entries: Vec<FontEntry> = fonts.values().map(FontEntry::from).collect();

        let fonts_config = FontsConfig {
            fonts: font_entries,
        };

        self.writer.write_yaml(target_path, &fonts_config)?;

        log_info!(
            "[Sync] Generated {} with {} fonts at {}",
            "fonts.yaml".cyan(),
            fonts.len(),
            target_path.display()
        );

        Ok(())
    }

    /// Generates settings.yaml from settings state data
    ///
    /// Transforms the flat state structure (keyed by "domain.key") into a
    /// hierarchical YAML structure grouped by platform (macOS).
    ///
    /// ## State Format (input)
    ///
    /// ```json
    /// {
    ///   "NSGlobalDomain.KeyRepeat": {
    ///     "domain": "NSGlobalDomain",
    ///     "key": "KeyRepeat",
    ///     "value": "1",
    ///     "value_type": "int"
    ///   }
    /// }
    /// ```
    ///
    /// ## Config Format (output)
    ///
    /// ```yaml
    /// settings:
    ///   macos:
    ///     - domain: NSGlobalDomain
    ///       key: KeyRepeat
    ///       value: "1"
    ///       value_type: int
    /// ```
    ///
    /// # Arguments
    ///
    /// * `settings` - HashMap of "domain.key" to setting state
    /// * `target_path` - Where settings.yaml will be written
    fn generate_settings_config(
        &self,
        settings: &HashMap<String, SettingState>,
        target_path: &Path,
    ) -> SyncResult<()> {
        log_debug!(
            "[Sync] Generating settings.yaml from {} settings...",
            settings.len()
        );

        // Transform flat state into structured config entries
        let macos_settings: Vec<SettingEntry> = settings
            .values()
            .map(|state| SettingEntry {
                domain: state.domain.clone(),
                key: state.key.clone(),
                value: state.value.clone(),
                value_type: state.value_type.clone(),
            })
            .collect();

        let settings_config = SettingsConfig {
            settings: OsSpecificSettings {
                macos: macos_settings,
            },
        };

        self.writer.write_yaml(target_path, &settings_config)?;

        log_info!(
            "[Sync] Generated {} with {} settings at {}",
            "settings.yaml".cyan(),
            settings.len(),
            target_path.display()
        );

        Ok(())
    }

    /// Generates a template shellrc.yaml file
    ///
    /// Shell configuration (run commands and aliases) is not tracked in the
    /// state file because it's user-specific and doesn't represent installed
    /// software. This function creates an empty but properly structured template
    /// that users can populate manually.
    ///
    /// ## Template Structure
    ///
    /// ```yaml
    /// run_commands:
    ///   shell: "zsh"
    ///   run_commands: []
    /// aliases: []
    /// ```
    ///
    /// # Arguments
    ///
    /// * `target_path` - Where shellrc.yaml will be written
    fn generate_shellrc_config(&self, target_path: &Path) -> SyncResult<()> {
        log_debug!("[Sync] Generating shellrc.yaml template...");

        // Create empty but properly structured template
        let shellrc_config = ShellConfig {
            run_commands: ShellCommands {
                shell: "zsh".to_string(), // Default to zsh, user can change
                run_commands: Vec::new(), // Empty - user adds manually
            },
            aliases: Vec::new(), // Empty - user adds manually
        };

        self.writer.write_yaml(target_path, &shellrc_config)?;

        log_info!(
            "[Sync] Generated {} template at {}",
            "shellrc.yaml".cyan(),
            target_path.display()
        );

        log_debug!(
            "[Sync] Note: shellrc is not tracked in state, generated file is an empty template"
        );

        Ok(())
    }

    /// Converts a path to an absolute string representation
    ///
    /// Attempts to canonicalize the path to get an absolute path. If that fails
    /// (e.g., file doesn't exist yet), falls back to the original path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to convert
    ///
    /// # Returns
    ///
    /// String representation of the path (absolute if possible)
    fn path_to_string(&self, path: &Path) -> String {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .into_owned()
    }
}

// ============================================================================
// SYNCHRONIZATION ORCHESTRATION
// ============================================================================

/// Orchestrates the complete synchronization workflow
///
/// The SyncOrchestrator is the high-level coordinator that manages the entire
/// sync process from start to finish:
///
/// 1. Path validation and setup
/// 2. State file reading and parsing
/// 3. Configuration generation
/// 4. Error handling and reporting
///
/// This separation of concerns keeps the orchestration logic clean and testable.
pub struct SyncOrchestrator {
    /// Path to the state.json file (source of truth)
    state_file_path: PathBuf,

    /// Directory where configuration files will be generated
    config_dir_path: PathBuf,
}

impl SyncOrchestrator {
    /// Initializes the orchestrator with path validation
    ///
    /// This method performs pre-flight checks to ensure the sync operation
    /// can proceed successfully:
    ///
    /// ## Validation Steps
    ///
    /// 1. **State file existence**: Ensures state.json exists and is readable
    /// 2. **Config directory setup**: Creates config directory if needed
    /// 3. **Path conversion**: Converts paths to owned PathBuf for safe storage
    ///
    /// # Arguments
    ///
    /// * `state_file` - Path to state.json file
    /// * `config_dir` - Directory for generated configuration files
    ///
    /// # Returns
    ///
    /// Initialized SyncOrchestrator ready to run synchronization
    ///
    /// # Errors
    ///
    /// - `SyncError::PathError` if state file doesn't exist
    /// - `SyncError::Io` if config directory cannot be created
    pub fn init(state_file: &Path, config_dir: &Path) -> SyncResult<Self> {
        log_info!("[Sync] Initializing sync orchestrator...");

        // Validate state file exists
        if !state_file.exists() {
            return Err(SyncError::PathError(format!(
                "State file not found: {}",
                state_file.display()
            )));
        }

        log_debug!("[Sync] State file verified: {}", state_file.display());

        // Ensure config directory exists
        if !config_dir.exists() {
            log_debug!(
                "[Sync] Config directory doesn't exist, creating: {}",
                config_dir.display()
            );
            fs::create_dir_all(config_dir)?;
        }

        log_debug!("[Sync] Config directory verified: {}", config_dir.display());

        Ok(SyncOrchestrator {
            state_file_path: state_file.to_path_buf(),
            config_dir_path: config_dir.to_path_buf(),
        })
    }

    /// Executes the complete synchronization workflow
    ///
    /// This is the main execution method that performs the entire sync process:
    ///
    /// ## Workflow Steps
    ///
    /// 1. **Read state file**: Load state.json from disk
    /// 2. **Parse JSON**: Deserialize into AppState structure
    /// 3. **Generate configs**: Transform state to configuration files
    /// 4. **Write files**: Persist all configuration YAML files
    ///
    /// # Returns
    ///
    /// Vector of paths to all successfully generated configuration files
    ///
    /// # Errors
    ///
    /// Can return errors from:
    /// - File I/O operations (reading state.json)
    /// - JSON deserialization (malformed state file)
    /// - Configuration generation (YAML serialization, file writing)
    pub fn run_sync(&self) -> SyncResult<Vec<PathBuf>> {
        log_info!(
            "[Sync] Starting sync from state file: {}",
            self.state_file_path.display()
        );

        // Read state file from disk
        let state_content = fs::read_to_string(&self.state_file_path)?;
        log_debug!("[Sync] Read {} bytes from state file", state_content.len());

        // Parse JSON into structured data
        let app_state: AppState = serde_json::from_str(&state_content)?;
        log_debug!(
            "[Sync] Parsed state: {} tools, {} fonts, {} settings",
            app_state.tools.len(),
            app_state.fonts.len(),
            app_state.settings.len()
        );

        // Generate all configuration files
        let generator = ConfigGenerator::new(self.config_dir_path.clone());
        let generated_files = generator.generate_configs(&app_state)?;

        log_info!(
            "[Sync] Successfully generated {} configuration files",
            generated_files.len()
        );

        Ok(generated_files)
    }
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Main entry point for the sync command
///
/// This function serves as the CLI command handler for synchronization operations.
/// It coordinates the entire workflow and provides user-friendly feedback for both
/// success and failure scenarios.
///
/// ## Workflow
///
/// 1. Extract paths from PathResolver
/// 2. Initialize SyncOrchestrator with validation
/// 3. Execute synchronization process
/// 4. Report results to user
///
/// ## User Feedback
///
/// Success output includes:
/// - Confirmation message
/// - List of all generated files with full paths
/// - Visual separator for clarity
///
/// Error output includes:
/// - Clear error message with details
/// - Helpful guidance for resolution
/// - Exit with non-zero status code
///
/// # Arguments
///
/// * `paths` - PathResolver containing application paths
///
/// # Exit Behavior
///
/// - **Success**: Returns normally after printing success messages
/// - **Failure**: Calls `std::process::exit(1)` after printing error details
///
/// # Example Output (Success)
///
/// ```text
/// [INFO] Synchronization process completed successfully!
///
/// ================================================================================
/// Generated Configuration file: /path/to/config.yaml
/// Generated Configuration file: /path/to/tools.yaml
/// Generated Configuration file: /path/to/fonts.yaml
/// Generated Configuration file: /path/to/settings.yaml
/// Generated Configuration file: /path/to/shellrc.yaml
/// ================================================================================
///
/// All configuration files have been regenerated from state.
/// ```
///
/// # Example Output (Failure)
///
/// ```text
/// [ERROR] Synchronization failed: State file not found: /path/to/state.json
///
/// ✗ Sync failed: State file not found: /path/to/state.json
///   Check the logs above for more details.
/// ```
pub fn run(paths: PathResolver) {
    log_debug!("[Sync] Sync command invoked");

    // Extract required paths from resolver
    let state_file_path = paths.state_file();
    let config_dir_path = paths.configs_dir();

    log_debug!("[Sync] State file: {}", state_file_path.display());
    log_debug!("[Sync] Config directory: {}", config_dir_path.display());

    // Initialize orchestrator with path validation
    match SyncOrchestrator::init(state_file_path, &config_dir_path) {
        Ok(orchestrator) => {
            // Execute synchronization process
            match orchestrator.run_sync() {
                Ok(generated_files) => {
                    // Success - report generated files
                    log_info!(
                        "[Sync] {}",
                        "Synchronization process completed successfully!"
                            .bold()
                            .green()
                    );

                    println!("\n{}", "=".repeat(80).blue());
                    for file in generated_files {
                        println!(
                            "Generated Configuration file: {}",
                            file.display().to_string().green()
                        );
                    }
                    println!("{}\n", "=".repeat(80).blue());

                    println!(
                        "{}",
                        "All configuration files have been regenerated from state.".cyan()
                    );

                    log_debug!("[Sync] Sync command completed successfully");
                }
                Err(e) => {
                    // Sync execution failed
                    log_error!("[Sync] Synchronization failed: {}", e);

                    eprintln!(
                        "\n{} {}",
                        "✗ Sync failed:".red().bold(),
                        e.to_string().red()
                    );
                    eprintln!("{}", "  Check the logs above for more details.".yellow());

                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            // Initialization failed
            log_error!("[Sync] Failed to initialize orchestrator: {}", e);

            eprintln!(
                "\n{} {}",
                "✗ Initialization failed:".red().bold(),
                e.to_string().red()
            );
            eprintln!(
                "{}",
                "  Unable to resolve paths or validate requirements.".yellow()
            );

            std::process::exit(1);
        }
    }

    log_debug!("[Sync] Sync command execution completed");
}

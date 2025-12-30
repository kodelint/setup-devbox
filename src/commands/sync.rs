//! # Configuration Synchronization Module
//!
//! This module implements the `sync` command for setup-devbox, which regenerates
//! configuration YAML files from the application's state file (state.json) or
//! fetches them from a remote Gist.
//!
//! ## Purpose
//!
//! The sync command serves as a recovery and migration mechanism that allows users to:
//! - Restore configuration files if they become corrupted or deleted
//! - Migrate from state-based storage to config-based management
//! - Generate initial configuration files from an existing installation
//! - Sync configuration across machines using GitHub Gists
//!
//! ## Architecture
//!
//! The module follows a clean separation of concerns:
//!
//! 1. **State Reading**: Parse state.json into typed structures (or Gist JSON)
//! 2. **Transformation**: Convert state data to configuration format
//! 3. **File Generation**: Write properly formatted YAML files

use crate::engine::configuration::processor::ConfigurationManagerState;
use crate::schemas::config_manager::ConfigurationManager;
use crate::schemas::fonts::FontEntry;
use crate::schemas::os_settings::{OsSpecificSettings, SettingEntry, SettingsConfig};
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::shell_configuration::AliasEntry;
use crate::schemas::state_file::{FontState, SettingState, ToolState};
use crate::schemas::tools_enums::SourceType;
use crate::schemas::tools_types::ToolEntry;
use crate::{log_debug, log_error, log_info};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

// ============================================================================
//                               ERROR HANDLING
// ============================================================================

/// Comprehensive error types for sync operations
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

    /// Network error (e.g., Gist fetching)
    Network(String),
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
            SyncError::Network(s) => write!(f, "Network Error: {}", s.to_string().red()),
        }
    }
}

/// Result type alias for sync operations
pub type SyncResult<T> = Result<T, SyncError>;

// ============================================================================
//                              STATE STRUCTURES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AppState {
    pub tools: HashMap<String, ToolState>,
    pub settings: HashMap<String, SettingState>,
    pub fonts: HashMap<String, FontState>,
}

// ============================================================================
//                           GIST STRUCTURES
// ============================================================================

#[derive(Debug, Deserialize)]
struct GistResponse {
    files: HashMap<String, GistFile>,
}

#[derive(Debug, Deserialize)]
struct GistFile {
    content: String,
}

// ============================================================================
//                           CONFIGURATION STRUCTURES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct MainConfig {
    pub tools: String,
    pub settings: String,
    pub shellrc: String,
    pub fonts: String,
}

#[derive(Debug, Serialize)]
pub struct ToolConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_latest_only_after: Option<String>,
    pub tools: Vec<ToolEntry>,
}

#[derive(Debug, Serialize)]
pub struct FontsConfig {
    pub fonts: Vec<FontEntry>,
}

#[derive(Debug, Serialize)]
pub struct ShellConfig {
    pub run_commands: ShellCommands,
    pub aliases: Vec<AliasEntry>,
}

#[derive(Debug, Serialize)]
pub struct ShellCommands {
    pub shell: String,
    pub run_commands: Vec<ShellCommand>,
}

#[derive(Debug, Serialize)]
pub struct ShellCommand {
    pub command: String,
    pub section: String,
}

// ============================================================================
//                              TYPE CONVERSIONS
// ============================================================================

impl From<&FontState> for FontEntry {
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
    pub fn from_state(name: String, tool_state: &ToolState) -> Self {
        let source_str = ToolState::normalize_source_type(&tool_state.install_method);
        let source: SourceType = source_str.parse().unwrap_or(SourceType::Url);

        let url_for_config = Self::resolve_url_for_source(&source, &tool_state.url, &name);
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

    fn resolve_url_for_source(
        source: &SourceType,
        url: &Option<String>,
        name: &str,
    ) -> Option<String> {
        match source {
            SourceType::Github => {
                log_debug!("[Sync::Tool] Omitting URL for GitHub tool: {}", name);
                None
            }
            _ => url.clone(),
        }
    }

    fn filter_empty_vec(vec_opt: Option<Vec<String>>) -> Option<Vec<String>> {
        vec_opt.filter(|v| !v.is_empty())
    }

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
//                                FILE GENERATION
// ============================================================================

pub struct FileWriter;

impl FileWriter {
    pub fn new() -> Self {
        FileWriter
    }

    pub fn write_yaml<T: Serialize>(&self, path: &Path, data: &T) -> SyncResult<()> {
        log_debug!(
            "[Sync::FileWriter] Serializing data for: {}",
            path.display()
        );

        let mut yaml = serde_yaml::to_string(data)?;
        yaml = self.format_yaml_output(&yaml);

        if !yaml.ends_with('\n') {
            yaml.push('\n');
        }

        fs::write(path, yaml)?;

        log_debug!("[Sync::FileWriter] Successfully wrote: {}", path.display());
        Ok(())
    }

    fn format_yaml_output(&self, yaml: &str) -> String {
        let mut formatted = yaml.to_string();
        formatted = formatted.replace("\n- ", "\n\n - ");
        formatted = formatted.replace("\n  ", "\n   ");

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

    fn quote_value_after_colon(&self, line: &str) -> String {
        if let Some(colon_pos) = line.find(':') {
            let key_part = &line[..=colon_pos];
            let value_part = line[colon_pos + 1..].trim();

            if !value_part.starts_with('"') && !value_part.starts_with('\'') {
                return format!("{key_part} \"{value_part}\"");
            }
        }
        line.to_string()
    }
}

// ============================================================================
//                         CONFIGURATION GENERATION
// ============================================================================

pub struct ConfigGenerator {
    output_dir: PathBuf,
    writer: FileWriter,
}

impl ConfigGenerator {
    pub fn new(output_dir: PathBuf) -> Self {
        ConfigGenerator {
            output_dir,
            writer: FileWriter::new(),
        }
    }

    pub fn generate_configs(&self, app_state: &AppState) -> SyncResult<Vec<PathBuf>> {
        log_info!("[Sync] Beginning configuration generation from state...");

        let tools_path = self.output_dir.join("tools.yaml");
        let fonts_path = self.output_dir.join("fonts.yaml");
        let settings_path = self.output_dir.join("settings.yaml");
        let shellrc_path = self.output_dir.join("shellrc.yaml");
        let main_config_path = self.output_dir.join("config.yaml");

        self.generate_tools_config(&app_state.tools, &tools_path)?;
        self.generate_fonts_config(&app_state.fonts, &fonts_path)?;
        self.generate_settings_config(&app_state.settings, &settings_path)?;
        self.generate_shellrc_config(&shellrc_path)?;

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
        Ok(())
    }

    fn generate_tools_config(
        &self,
        tools: &HashMap<String, ToolState>,
        target_path: &Path,
    ) -> SyncResult<()> {
        let tool_entries: Vec<ToolEntry> = tools
            .iter()
            .map(|(name, state)| ToolEntry::from_state(name.clone(), state))
            .collect();

        let tool_config = ToolConfig {
            update_latest_only_after: Some("7 days".to_string()),
            tools: tool_entries,
        };

        self.writer.write_yaml(target_path, &tool_config)?;
        Ok(())
    }

    fn generate_fonts_config(
        &self,
        fonts: &HashMap<String, FontState>,
        target_path: &Path,
    ) -> SyncResult<()> {
        let font_entries: Vec<FontEntry> = fonts.values().map(FontEntry::from).collect();
        let fonts_config = FontsConfig {
            fonts: font_entries,
        };
        self.writer.write_yaml(target_path, &fonts_config)?;
        Ok(())
    }

    fn generate_settings_config(
        &self,
        settings: &HashMap<String, SettingState>,
        target_path: &Path,
    ) -> SyncResult<()> {
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
                linux: vec![],
            },
        };

        self.writer.write_yaml(target_path, &settings_config)?;
        Ok(())
    }

    fn generate_shellrc_config(&self, target_path: &Path) -> SyncResult<()> {
        let shellrc_config = ShellConfig {
            run_commands: ShellCommands {
                shell: "zsh".to_string(),
                run_commands: Vec::new(),
            },
            aliases: Vec::new(),
        };

        self.writer.write_yaml(target_path, &shellrc_config)?;
        Ok(())
    }

    fn path_to_string(&self, path: &Path) -> String {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .into_owned()
    }
}

// ============================================================================
//                    SYNCHRONIZATION ORCHESTRATION
// ============================================================================

pub struct SyncOrchestrator {
    state_file_path: PathBuf,
    config_dir_path: PathBuf,
}

impl SyncOrchestrator {
    pub fn init(state_file: &Path, config_dir: &Path) -> SyncResult<Self> {
        log_info!("[Sync] Initializing sync orchestrator...");

        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
        }

        Ok(SyncOrchestrator {
            state_file_path: state_file.to_path_buf(),
            config_dir_path: config_dir.to_path_buf(),
        })
    }

    /// Fetches configuration files from a GitHub Gist
    pub fn fetch_from_gist(
        &self,
        gist_id: &str,
        token: Option<String>,
    ) -> SyncResult<Vec<PathBuf>> {
        log_info!(
            "[Sync] Fetching configuration from Gist: {}",
            gist_id.cyan()
        );

        let url = format!("https://api.github.com/gists/{}", gist_id);
        let mut request = ureq::get(&url)
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "setup-devbox");

        if let Some(t) = token {
            request = request.set("Authorization", &format!("token {}", t));
        }

        let response = request
            .call()
            .map_err(|e| SyncError::Network(format!("Gist fetch failed: {}", e)))?;

        let gist: GistResponse = response
            .into_json()
            .map_err(|e| SyncError::Network(format!("Failed to parse Gist JSON: {}", e)))?;

        let mut generated_files = Vec::new();

        for (filename, file_data) in gist.files {
            let target_path = self.config_dir_path.join(&filename);
            log_info!("[Sync] Writing Gist file: {}", filename.cyan());
            fs::write(&target_path, file_data.content)?;
            generated_files.push(target_path);
        }

        Ok(generated_files)
    }

    pub fn run_sync(&self) -> SyncResult<Vec<PathBuf>> {
        log_info!(
            "[Sync] Starting sync from state file: {}",
            self.state_file_path.display()
        );

        if !self.state_file_path.exists() {
            return Err(SyncError::PathError(format!(
                "State file not found: {}",
                self.state_file_path.display()
            )));
        }

        let state_content = fs::read_to_string(&self.state_file_path)?;
        let app_state: AppState = serde_json::from_str(&state_content)?;

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
//                                   PUBLIC API
// ============================================================================

pub fn run(paths: PathResolver, gist: Option<String>, github_token: Option<String>) {
    log_debug!("[Sync] Sync command invoked");

    let state_file_path = paths.state_file();
    let config_dir_path = paths.configs_dir();

    match SyncOrchestrator::init(state_file_path, &config_dir_path) {
        Ok(orchestrator) => {
            // Check if we are syncing from Gist or State
            let result = if let Some(gist_id) = gist {
                orchestrator.fetch_from_gist(&gist_id, github_token)
            } else {
                orchestrator.run_sync()
            };

            match result {
                Ok(generated_files) => {
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
                        "All configuration files have been regenerated.".cyan()
                    );
                }
                Err(e) => {
                    log_error!("[Sync] Synchronization failed: {}", e);
                    eprintln!(
                        "\n{} {}",
                        "✗ Sync failed:".red().bold(),
                        e.to_string().red()
                    );
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            log_error!("[Sync] Failed to initialize orchestrator: {}", e);
            eprintln!(
                "\n{} {}",
                "✗ Initialization failed:".red().bold(),
                e.to_string().red()
            );
            std::process::exit(1);
        }
    }
}

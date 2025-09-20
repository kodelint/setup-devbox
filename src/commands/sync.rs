// This file implements the `sync` command for `setup-devbox`.
// Its primary function is to regenerate configuration YAML files from the application's
// internal state file (`state.json`), or vice-versa.

use crate::libs::state_management::read_devbox_state; // Function to read application state.
use crate::libs::utilities::misc_utils::get_devbox_dir;
use crate::schemas::common::MainConfig; // Schema definitions for config and state.
use crate::schemas::fonts::{FontConfig, FontEntry};
use crate::schemas::os_settings::{OsSpecificSettings, SettingEntry, SettingsConfig};
use crate::schemas::tools::{ToolConfig, ToolEntry};
use crate::{log_debug, log_error, log_info, log_warn}; // Custom logging macros.
use clap::Args; // Clap macro for argument parsing.
use colored::Colorize; // For colored terminal output.
use std::fs; // File system operations.
use std::path::{Path, PathBuf}; // Path manipulation utilities.

/// Arguments for the `sync config` subcommand.
#[derive(Debug, Args)]
pub struct SyncConfigArgs {
    /// Path to the DevBox state file. Defaults to `~/.setup-devbox/state.json`.
    #[arg(long)]
    pub state: Option<PathBuf>,
    /// Path to the directory where config files should be generated.
    /// Defaults to `~/.setup-devbox/configs/`.
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
}

/// Executes the `sync` command, primarily for syncing state to config files.
pub fn run(args: SyncConfigArgs) {
    eprint!("\n"); // Add a blank line for readability.
    log_warn!(
        "{}",
        "[Sync] This 'sync' command is an EMERGENCY command and \
    should be avoided if possible."
            .bold()
    ); // Warns user about command usage.
    log_warn!(
        "{}",
        "[Sync] It is primarily for recovering or generating \
    configuration from/to the state."
            .bold()
    ); // Explains purpose of sync.
    log_warn!(
        "{}",
        "[Sync] Please use the 'now' command for regular setup \
    and updates."
            .bold()
    ); // Recommends 'now' for regular use.
    eprint!("\n"); // Add another blank line.

    // Resolve default paths for state and configs.
    let devbox_dir = get_devbox_dir();
    let default_state_path = devbox_dir.join("state.json");
    let default_config_dir = devbox_dir.join("configs");

    log_info!("[Sync] Syncing config files from state file.");
    // Use provided paths or fall back to defaults.
    let state_path = args.state.unwrap_or(default_state_path);
    let output_dir = args.output_dir.unwrap_or(default_config_dir);
    log_debug!("[Sync] Syncing from state: {}", state_path.display());
    log_debug!("[Sync] Outputting to config directory: {}", output_dir.display());
    sync_state_to_configs(&state_path, &output_dir);
    log_info!("[Sync] Synchronization process completed.");
}

/// Helper function to serialize data to YAML and write to a file.
fn write_yaml_file<T: serde::Serialize>(path: &PathBuf, data: &T) -> Result<(), String> {
    log_debug!("[Sync:File] Writing YAML file: {}", path.display().to_string().cyan());
    // Create parent directories if they don't exist.
    fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).map_err(|e| {
        format!(
            "Failed to create directory for {}: \
        {}",
            path.display(),
            e
        )
    })?;
    // Serialize data to YAML string.
    let content = serde_yaml::to_string(data).map_err(|e| {
        format!(
            "Failed to serialize data to YAML for {}: \
        {}",
            path.display(),
            e
        )
    })?;
    // Write content to file.
    fs::write(path, content)
        .map_err(|e| format!("Failed to write to {}: {}", path.display(), e))?;
    Ok(())
}

/// Reads `DevBoxState` from `state_path` and generates corresponding YAML config files
/// (`config.yaml`, `tools.yaml`, `settings.yaml`, `fonts.yaml`) in `output_dir`.
fn sync_state_to_configs(state_path: &PathBuf, output_dir: &PathBuf) {
    log_info!(
        "[Sync:Config] Attempting to sync config files from state: {}",
        state_path.display().to_string().cyan()
    );

    // Load DevBoxState.
    let devbox_state = match read_devbox_state(state_path) {
        Ok(state) => state,
        Err(e) => {
            log_error!(
                "[Sync:Config] Failed to read DevBox state from {}: {}",
                state_path.display().to_string().red(),
                e
            );
            return; // Exit if state cannot be read.
        },
    };
    log_info!("[Sync:Config] DevBox state loaded.");

    // Create output directory if it doesn't exist.
    if let Err(e) = fs::create_dir_all(output_dir) {
        log_error!(
            "[Sync:Config] Failed to create output directory {}: {}",
            output_dir.display().to_string().red(),
            e
        );
        return;
    }

    // 1. Generate `tools.yaml` from `devbox_state.tools`.
    let mut tool_entries: Vec<ToolEntry> = Vec::new();
    for (name, tool_state) in devbox_state.tools {
        // Construct ToolEntry from ToolState.
        let tool_entry = ToolEntry {
            name,
            version: Some(tool_state.version),
            source: tool_state.install_method,
            url: tool_state.url,
            repo: tool_state.repo,
            tag: tool_state.tag,
            rename_to: tool_state.renamed_to,
            options: tool_state.options,
            executable_path_after_extract: tool_state.executable_path_after_extract,
            additional_cmd: tool_state.additional_cmd_executed,
            configuration_manager: Default::default(),
        };
        tool_entry
            .validate()
            .map_err(|e| {
                log_warn!("[Sync:Config] Validation warning for tool '{}': {}", tool_entry.name, e);
            })
            .ok(); // Log validation warnings but continue.
        tool_entries.push(tool_entry);
    }
    let tool_config = ToolConfig { update_latest_only_after: None, tools: tool_entries };

    let tools_yaml_path = output_dir.join("tools.yaml");
    if let Err(e) = write_yaml_file(&tools_yaml_path, &tool_config) {
        log_error!(
            "[Sync:Config] Failed to write tools.yaml to {}: {}",
            tools_yaml_path.display().to_string().red(),
            e
        );
    } else {
        log_info!(
            "[Sync:Config] tools.yaml generated at {}.",
            tools_yaml_path.display().to_string().green()
        );
    }

    // 2. Generate `settings.yaml` from `devbox_state.settings`.
    let mut macos_settings: Vec<SettingEntry> = Vec::new();
    // Assuming all stored settings are macOS for now.
    for (_, setting_state) in devbox_state.settings {
        let setting_entry = SettingEntry {
            domain: setting_state.domain,
            key: setting_state.key,
            value: setting_state.value,
            value_type: setting_state.value_type,
        };
        macos_settings.push(setting_entry);
    }
    let settings_config = SettingsConfig { settings: OsSpecificSettings { macos: macos_settings } };

    let settings_yaml_path = output_dir.join("settings.yaml");
    if let Err(e) = write_yaml_file(&settings_yaml_path, &settings_config) {
        log_error!(
            "[Sync:Config] Failed to write settings.yaml to {}: {}",
            settings_yaml_path.display().to_string().red(),
            e
        );
    } else {
        log_info!(
            "[Sync:Config] settings.yaml generated at {}.",
            settings_yaml_path.display().to_string().green()
        );
    }

    // 3. Generate `fonts.yaml` from `devbox_state.fonts`.
    let mut font_entries: Vec<FontEntry> = Vec::new();
    for (name, font_state) in devbox_state.fonts {
        let font_entry = FontEntry {
            name,
            version: Some(font_state.version),
            source: "github".to_string(), // Currently assumes GitHub for all fonts.
            repo: font_state.repo,
            tag: font_state.tag,
            install_only: None, // `install_only` is not stored in state; defaults to None.
        };
        font_entries.push(font_entry);
    }
    let font_config = FontConfig { fonts: font_entries };

    let fonts_yaml_path = output_dir.join("fonts.yaml");
    if let Err(e) = write_yaml_file(&fonts_yaml_path, &font_config) {
        log_error!(
            "[Sync:Config] Failed to write fonts.yaml to {}: {}",
            fonts_yaml_path.display().to_string().red(),
            e
        );
    } else {
        log_info!(
            "[Sync:Config] fonts.yaml generated at {}.",
            fonts_yaml_path.display().to_string().green()
        );
    }

    // 4. Create and write the main `config.yaml`, referencing the generated files.
    // Paths are relative to `output_dir` for `MainConfig`.
    let main_config = MainConfig {
        tools: Some(tools_yaml_path.to_str().unwrap().to_string()),
        settings: Some(settings_yaml_path.to_str().unwrap().to_string()),
        fonts: Some(fonts_yaml_path.to_str().unwrap().to_string()),
        shellrc: None, // ShellRC content is not currently stored in the state file.
    };

    let main_config_path = output_dir.join("config.yaml");
    if let Err(e) = write_yaml_file(&main_config_path, &main_config) {
        log_error!(
            "[Sync:Config] Failed to write config.yaml to {}: {}",
            main_config_path.display().to_string().red(),
            e
        );
    } else {
        log_info!(
            "[Sync:Config] config.yaml generated at {}.",
            main_config_path.display().to_string().green()
        );
    }
}

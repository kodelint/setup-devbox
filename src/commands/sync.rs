// Importing schema definitions. These structs (e.g., `ToolConfig`, `FontConfig`) define
// the expected data structure for each type of YAML configuration file, enabling `serde`
// to correctly parse them. `MainConfig` specifically defines the structure of the primary
// `config.yaml` file that links to other configuration files.
use crate::schema::{FontConfig, FontEntry, MainConfig, OsSpecificSettings, SettingEntry, SettingsConfig, ToolConfig, ToolEntry};
// Bring in our custom logging macros.
// These macros (`log_debug`, `log_error`, `log_info`, `log_warn`) provide
// a standardized way to output messages to the console with different severity levels,
// making it easier to track the application's flow and diagnose issues.
use crate::{log_debug, log_error, log_info, log_warn};
use crate::libs::state_management::read_devbox_state;
use clap::Args;
// The 'colored' crate helps us make our console output look pretty and readable.
use colored::Colorize;
// 'std::fs' is our toolkit for interacting with the file system – creating directories, creating files, etc.
use std::fs;
// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::{Path, PathBuf};
/// Returns the canonical path to the DevBox directory, typically `~/.setup-devbox`.
/// This is the base directory where `state.json` and the `config` folder reside.
/// If the home directory cannot be determined, it will log an error and
/// fall back to the current directory, which might lead to unexpected behavior.
use crate::libs::utilities::path_helpers::get_devbox_dir;

/// Arguments for the `sync config` subcommand.
///
/// `#[derive(Debug, Args)]` tells Clap to generate argument parsing for these arguments.
#[derive(Debug, Args)]
pub struct SyncConfigArgs {
    /// Path to the DevBox state file.
    /// Defaults to `~/.setup-devbox/state.json`.
    #[arg(long)]
    pub state: Option<PathBuf>,
    /// Path to the directory where config files should be generated.
    /// Defaults to `~/.setup-devbox/configs/`.
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
}

pub fn run(args: SyncConfigArgs) {
    eprint!("\n"); // Add a blank line for readability.
    log_warn!("{}", "[Sync] This 'sync' command is an EMERGENCY command and should be avoided if possible.".bold());
    log_warn!("{}", "[Sync] It is primarily for recovering or generating configuration from/to the state.".bold());
    log_warn!("{}", "[Sync] Please use the 'now' command for regular setup and updates.".bold());
    eprint!("\n"); // Add another blank line.

    // Define default paths based on the `~/.setup-devbox` directory
    let devbox_dir = get_devbox_dir();
    let default_state_path = devbox_dir.join("state.json");
    let default_config_dir = devbox_dir.join("configs");

    // The logic below now directly executes sync_state_to_configs.
    // This was previously inside the `SyncCommands::SyncConfig(args)` arm.
    log_info!("[Sync] Syncing config files from state file.");
    let state_path = args.state.unwrap_or(default_state_path);
    let output_dir = args.output_dir.unwrap_or(default_config_dir);
    log_debug!("[Sync] Syncing from state: {}", state_path.display());
    log_debug!("[Sync] Outputting to config directory: {}", output_dir.display());
    sync_state_to_configs(&state_path, &output_dir);
    log_info!("[Sync] Synchronization process completed.");
}

// Helper function to write data of type `T` to a YAML file.
fn write_yaml_file<T: serde::Serialize>(path: &PathBuf, data: &T) -> Result<(), String> {
    log_debug!("[Sync:File] Writing YAML file: {}", path.display().to_string().cyan());
    // Ensure the parent directory exists before writing the file
    fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))
        .map_err(|e| format!("Failed to create directory for {}: {}", path.display(), e))?;
    let content = serde_yaml::to_string(data)
        .map_err(|e| format!("Failed to serialize data to YAML for {}: {}", path.display(), e))?;
    fs::write(path, content)
        .map_err(|e| format!("Failed to write to {}: {}", path.display(), e))?;
    Ok(())
}

/// Synchronizes configuration files from a DevBox state file.
/// This function reads the `DevBoxState` and generates `config.yaml`, `tools.yaml`,
/// `settings.yaml`, and `fonts.yaml` based on its contents.
fn sync_state_to_configs(state_path: &PathBuf, output_dir: &PathBuf) {
    log_info!("[Sync:Config] Attempting to sync config files from state: {}", state_path.display().to_string().cyan());

    // 1. Read the DevBoxState from the specified path
    let devbox_state = match read_devbox_state(state_path) {
        Ok(state) => state,
        Err(e) => {
            log_error!("[Sync:Config] Failed to read DevBox state from {}: {}", state_path.display().to_string().red(), e);
            return; // Exit if state cannot be read
        }
    };

    log_info!("[Sync:Config] DevBox state loaded. Generating config files in: {}", output_dir.display().to_string().green());

    // 2. Ensure the output directory exists
    if let Err(e) = fs::create_dir_all(output_dir) {
        log_error!("[Sync:Config] Failed to create output directory {}: {}", output_dir.display().to_string().red(), e);
        return; // Exit if directory cannot be created
    }

    // Prepare vectors/maps to hold the converted config structs
    let mut tools_entries: Vec<ToolEntry> = Vec::new();
    let mut fonts_entries: Vec<FontEntry> = Vec::new();

    // 3. Convert ToolState entries to ToolEntry
    // Fix applied here: Use `tool_name` (the HashMap key) for `ToolEntry.name`
    for (tool_name, tool_state) in devbox_state.tools {
        tools_entries.push(ToolEntry {
            name: tool_name, // Use the key from the HashMap as the tool's name
            version: Some(tool_state.version),   // `ToolState.version` maps to `ToolEntry.version`
            source: tool_state.install_method,   // `ToolState.install_method` maps to `ToolEntry.source`
            url: None,                           // Todo: Fix me
            repo: tool_state.repo,               // Repo Name
            tag: tool_state.tag,                 // Git Tag usually
            rename_to: tool_state.renamed_to,    // Maps directly
            options: tool_state.options.clone(), // Pass the options if any
            executable_path_after_extract: None, // Todo: Fix me
        });
    }
    let tool_config = ToolConfig { tools: tools_entries };


    // 4. Convert SettingState entries to SettingEntry
    // Note: This conversion is lossy. SettingState doesn't have an OS key,
    // and SettingEntry requires `value_type`.
    // Assumption: Group all settings under a "macos" key for generation.
    let mut macos_settings: Vec<SettingEntry> = Vec::new();
    for (_, setting_state) in devbox_state.settings {
        macos_settings.push(SettingEntry {
            domain: setting_state.domain,
            key: setting_state.key,
            value: setting_state.value,
            value_type: setting_state.value_type, // Now correctly populated from SettingState!
        });
    }

    // CORRECTED: Create an OsSpecificSettings instance
    let os_specific_settings = OsSpecificSettings {
        macos: macos_settings,
        // Add other OS vectors here if you expand OsSpecificSettings
        // linux: Vec::new(), // Example for future expansion
    };

    // Assign the correctly structured OsSpecificSettings to SettingsConfig
    let settings_config = SettingsConfig { settings: os_specific_settings };


    // 5. Convert FontState entries to FontEntry
    // Note: This conversion is lossy, as FontEntry has `repo`, `tag` which are not in FontState.
    // `FontState.url` maps to `FontEntry.source`.
    for (_, font_state) in devbox_state.fonts {
        // Determine the 'source' for FontEntry based on FontState's URL
        let source_type = if font_state.url.contains("github.com") && font_state.url.contains("/releases/") {
            // If the URL is from GitHub releases, set the source as "GitHub"
            "github".to_string()
        } else {
            // Otherwise, use the full URL as the source
            font_state.url.clone()
        };
        fonts_entries.push(FontEntry {
            name: font_state.name,
            version: Some(font_state.version),
            source: source_type,
            repo: font_state.repo,
            tag: font_state.tag,
            // Todo: Fix below
            install_only: None,
        });
    }
    let font_config = FontConfig { fonts: fonts_entries };


    // 6. Write the individual YAML configuration files
    let tools_yaml_path = output_dir.join("tools.yaml");
    if let Err(e) = write_yaml_file(&tools_yaml_path, &tool_config) {
        log_error!("[Sync:Config] Failed to write tools.yaml to {}: {}", tools_yaml_path.display().to_string().red(), e);
    } else {
        log_info!("[Sync:Config] tools.yaml generated at {}.", tools_yaml_path.display().to_string().green());
    }

    let settings_yaml_path = output_dir.join("settings.yaml");
    if let Err(e) = write_yaml_file(&settings_yaml_path, &settings_config) {
        log_error!("[Sync:Config] Failed to write settings.yaml to {}: {}", settings_yaml_path.display().to_string().red(), e);
    } else {
        log_info!("[Sync:Config] settings.yaml generated at {}.", settings_yaml_path.display().to_string().green());
    }

    let fonts_yaml_path = output_dir.join("fonts.yaml");
    if let Err(e) = write_yaml_file(&fonts_yaml_path, &font_config) {
        log_error!("[Sync:Config] Failed to write fonts.yaml to {}: {}", fonts_yaml_path.display().to_string().red(), e);
    } else {
        log_info!("[Sync:Config] fonts.yaml generated at {}.", fonts_yaml_path.display().to_string().green());
    }

    // 7. Create and write the main config.yaml, referencing the generated files
    // Paths are relative to the `output_dir` for MainConfig.
    let main_config = MainConfig {
        tools: Some(tools_yaml_path.to_str().unwrap().to_string()),
        settings: Some(settings_yaml_path.to_str().unwrap().to_string()),
        fonts: Some(fonts_yaml_path.to_str().unwrap().to_string()),
        shellrc: None, // ShellRC file content is not preserved in the state-file, so null
    };

    let main_config_path = output_dir.join("config.yaml");
    if let Err(e) = write_yaml_file(&main_config_path, &main_config) {
        log_error!("[Sync:Config] Failed to write config.yaml to {}: {}", main_config_path.display().to_string().red(), e);
    } else {
        log_info!("[Sync:Config] config.yaml generated at {}.", main_config_path.display().to_string().green());
    }

    log_info!("[Sync:Config] Configuration files generation complete.");
}
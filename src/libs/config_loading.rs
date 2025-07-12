use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use crate::{log_debug, log_error, log_info, log_warn};
use crate::schema::{FontConfig, MainConfig, SettingsConfig, ShellConfig, ToolConfig};
use crate::utils::expand_tilde;

/// A struct to hold all the parsed configuration data.
/// This makes it easier to pass around configuration items between functions
/// without having a massive number of arguments.
pub struct ParsedConfigs {
    pub(crate) tools: Option<ToolConfig>,
    pub(crate) settings: Option<SettingsConfig>,
    pub(crate) shell: Option<ShellConfig>,
    pub(crate) fonts: Option<FontConfig>,
}

/// Helper function to load an individual configuration file (e.g., tools.yaml).
///
/// This function abstracts the repetitive logic of reading a YAML file,
/// expanding the tilde, and deserializing its content into a specific struct.
/// It provides clear logging for success, file not found, or parsing errors.
///
/// # Type Parameters
/// * `T`: The type of the configuration struct (e.g., `ToolConfig`, `FontConfig`)
///        that implements `serde::de::DeserializeOwned` for deserialization.
///
/// # Arguments
/// * `path_option`: An `Option<String>` containing the path to the config file (from `MainConfig`).
/// * `config_name`: A string slice representing the human-readable name of the config (e.g., "tools", "settings").
/// * `bold_name`: A string slice for the bolded section name in logs (e.g., "[Tools]").
///
/// # Returns
/// An `Option<T>` containing the parsed configuration struct if successful, otherwise `None`.
pub fn load_individual_config<T>(path_option: Option<&String>, config_name: &str, bold_name: &str) -> Option<T>
where
    T: serde::de::DeserializeOwned + std::fmt::Debug,
{
    if let Some(path_str) = path_option {
        let path = expand_tilde(path_str);
        log_debug!("Attempting to load {} config from: {:?}", config_name, path.display());
        match fs::read_to_string(&path) {
            Ok(contents) => {
                match serde_yaml::from_str::<T>(&contents) {
                    Ok(cfg) => {
                        log_debug!("{} Successfully loaded {} configuration from {}", bold_name.bold(), config_name, path.display().to_string().green());
                        Some(cfg)
                    },
                    Err(e) => {
                        log_error!("Failed to parse {}.yaml at {:?}: {}. Please check its YAML syntax.", config_name, path.display().to_string().red(), e);
                        None
                    }
                }
            },
            Err(_) => {
                log_warn!("{} configuration file not found or unreadable at {}. Skipping {} setup.", config_name.yellow(), path.display().to_string().yellow(), config_name);
                None
            }
        }
    } else {
        None
    }
}

/// Loads all configurations from the master `config.yaml` and its linked files.
///
/// This function orchestrates the reading of the main `config.yaml` and then
/// uses helper functions to load linked `tools.yaml`, `settings.yaml`, etc.
/// It provides robust error handling for missing or malformed YAML files.
///
/// # Arguments
/// * `config_path_resolved`: The `PathBuf` to the main `config.yaml` file.
///
/// # Returns
/// A `ParsedConfigs` struct containing `Option`s for each type of configuration.
/// Exits the application if the main `config.yaml` cannot be read or parsed.
pub fn load_master_configs(config_path_resolved: &PathBuf) -> ParsedConfigs {
    log_debug!("Entering load_master_configs() function.");
    log_debug!("Loading configurations as per master config file: {}", config_path_resolved.display().to_string().blue());

    let main_cfg_content = match fs::read_to_string(config_path_resolved) {
        Ok(c) => c,
        Err(e) => {
            log_error!(
                "Failed to read main config.yaml at {}: {}. Please ensure the file exists and is readable.",
                config_path_resolved.display().to_string().red(),
                e
            );
            std::process::exit(1); // Critical error, exit
        }
    };

    let main_cfg: MainConfig = match serde_yaml::from_str(&main_cfg_content) {
        Ok(cfg) => cfg,
        Err(e) => {
            log_error!(
                "Failed to parse main config.yaml at {:?}: {}. Please check your YAML syntax for errors.",
                config_path_resolved.display().to_string().red(),
                e
            );
            std::process::exit(1); // Critical error, exit
        }
    };
    log_debug!("MainConfig loaded: {:?}", main_cfg);

    // Use the new helper function for each individual config type.
    let tools_config = load_individual_config(main_cfg.tools.as_ref(), "tools", "[Tools]");
    let settings_config = load_individual_config(main_cfg.settings.as_ref(), "settings", "[Settings]");
    let shell_config = load_individual_config(main_cfg.shellrc.as_ref(), "shell config", "[Shell Config]"); // Renamed for clarity in log
    let fonts_config = load_individual_config(main_cfg.fonts.as_ref(), "fonts", "[Fonts]");

    log_debug!("Exiting load_master_configs() function.");
    ParsedConfigs {
        tools: tools_config,
        settings: settings_config,
        shell: shell_config,
        fonts: fonts_config,
    }
}

/// Loads a single configuration file directly, bypassing `config.yaml`.
///
/// This function handles scenarios where the user provides a direct path
/// to a specific config file (e.g., `tools.yaml`). It attempts to parse the
/// file based on its filename.
///
/// # Arguments
/// * `config_path_resolved`: The `PathBuf` to the single config file.
/// * `config_filename`: The filename (e.g., "tools.yaml") to determine the type.
///
/// # Returns
/// A `ParsedConfigs` struct with only the relevant configuration `Option` set.
/// Exits the application if the file cannot be read or its type is unsupported.
pub fn load_single_config(config_path_resolved: &PathBuf, config_filename: &str) -> ParsedConfigs {
    log_debug!("Entering load_single_config() function.");
    log_info!("Loading configuration from single file: {}", config_path_resolved.display().to_string().blue());
    log_debug!("Attempting to load configuration directly from: {:?}", config_path_resolved.display());

    let contents = match fs::read_to_string(config_path_resolved) {
        Ok(c) => c,
        Err(e) => {
            log_error!(
                "Failed to read single config file {:?}: {}. Please check its existence and permissions.",
                config_path_resolved.display().to_string().red(),
                e
            );
            std::process::exit(1); // Critical error, exit
        }
    };

    let mut parsed_configs = ParsedConfigs {
        tools: None,
        settings: None,
        shell: None,
        fonts: None,
    };

    match config_filename {
        "tools.yaml" => {
            log_debug!("Identified as tools.yaml. Attempting to parse...");
            parsed_configs.tools = match serde_yaml::from_str(&contents) {
                Ok(cfg) => { log_info!("[Tools] Successfully parsed tools.yaml."); Some(cfg) },
                Err(e) => { log_error!("Failed to parse tools.yaml: {}", e); None },
            }
        },
        "settings.yaml" => {
            log_debug!("Identified as settings.yaml. Attempting to parse...");
            parsed_configs.settings = match serde_yaml::from_str(&contents) {
                Ok(cfg) => { log_info!("[Settings] Successfully parsed settings.yaml."); Some(cfg) },
                Err(e) => { log_error!("Failed to parse settings.yaml: {}", e); None },
            }
        },
        "shellrc.yaml" | "shellac.yaml" => {
            log_debug!("Identified as shell config file. Attempting to parse...");
            parsed_configs.shell = match serde_yaml::from_str(&contents) {
                Ok(cfg) => { log_info!("[ShellRC] Successfully parsed shell config."); Some(cfg) },
                Err(e) => { log_error!("Failed to parse shell config: {}", e); None },
            }
        },
        "fonts.yaml" => {
            log_debug!("Identified as fonts.yaml. Attempting to parse...");
            parsed_configs.fonts = match serde_yaml::from_str(&contents) {
                Ok(cfg) => { log_info!("[Fonts] Successfully parsed fonts.yaml."); Some(cfg) },
                Err(e) => { log_error!("Failed to parse fonts.yaml: {}", e); None },
            }
        },
        other => {
            log_error!(
                "Unsupported single config file type: '{}'. Expected 'tools.yaml', 'settings.yaml', 'shellrc.yaml', or 'fonts.yaml'.",
                other.red()
            );
            std::process::exit(1); // Critical error, exit
        }
    }
    log_debug!("Exiting load_single_config() function.");
    parsed_configs
}
use colored::Colorize;
use std::env;
use std::path::{Path, PathBuf};

use crate::{log_debug, log_info, log_warn};

/// # PathResolver
///
/// Central path resolution service for the application.
///
/// This struct is responsible for determining the file system locations for:
/// 1. The **base configuration directory**.
/// 2. The main **configuration file**.
/// 3. The application **state file**.
/// 4. The **tools configuration directory**.
///
/// It handles environment variable overrides (`SDB_CONFIG_PATH`, `SDB_STATE_FILE_PATH`, etc.)
/// and provides sensible defaults, including tilde (`~`) expansion for user paths.
///
/// Initialize once at application startup using `PathResolver::new()` and pass around as needed.
#[derive(Debug, Clone)]
pub struct PathResolver {
    /// Base configuration directory (determined from `SDB_CONFIG_PATH` or a default, e.g., `~/.setup-devbox`).
    base_config_dir: PathBuf,
    /// Full path to the main configuration file (e.g., `~/.setup-devbox/configs/config.yaml`).
    config_file: PathBuf,
    /// The filename of the main configuration file (e.g., `"config.yaml"`).
    config_filename: String,
    /// Full path to the application state file (e.g., `~/.setup-devbox/state.json`).
    state_file: PathBuf,
    /// Directory containing tools configuration files.
    #[allow(dead_code)]
    tools_config_dir: PathBuf,
}

impl PathResolver {
    /// Initializes the path resolver by determining all key application paths.
    ///
    /// The resolution order for paths is generally:
    /// 1. Explicit argument (`config_path` or `state_path`).
    /// 2. Specific Environment Variable (e.g., `SDB_STATE_FILE_PATH`).
    /// 3. General Environment Variable (e.g., `SDB_CONFIG_PATH`).
    /// 4. Default path relative to the resolved base configuration directory.
    ///
    /// # Arguments
    /// * `config_path` - Optional override for the **main config file path**. This takes the highest priority.
    /// * `state_path` - Optional override for the **state file path**. This takes the highest priority.
    ///
    /// # Returns
    /// A `Result` containing the initialized `PathResolver` on success, or a `String` error message on failure.
    pub fn new(config_path: Option<String>, state_path: Option<String>) -> Result<Self, String> {
        log_debug!("Initializing PathResolver");

        // First, determine the base config directory. This acts as the root for default paths.
        let base_config_dir = Self::resolve_base_config_dir();
        log_debug!("[SDB] Base config directory: {}", base_config_dir.display());

        // Resolve main configuration file path based on overrides and base directory.
        let config_file = Self::resolve_config_file(&base_config_dir, config_path)?;

        // Extract and validate the configuration filename.
        let config_filename = config_file
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("Invalid config filename")? // Return error if path ends unexpectedly (e.g., is a directory).
            .to_string();

        // Resolve state file path.
        let state_file = Self::resolve_state_file(&base_config_dir, state_path)?;

        // Resolve tools config directory.
        let tools_config_dir = Self::resolve_tools_config_dir(&base_config_dir);

        // Log final resolved paths for debugging and user information.
        log_info!(
            "[SDB] Using configuration file: {}",
            config_file.display().to_string().cyan() // Highlight important path in log.
        );
        log_debug!(
            "[SDB] Managing application state in: {}",
            state_file.display().to_string().yellow() // Highlight state file path.
        );
        log_debug!(
            "[SDB] Tools config directory: {}",
            tools_config_dir.display()
        );
        log_debug!(
            "[SDB] Detected config filename: '{}'",
            config_filename.blue() // Highlight filename.
        );

        Ok(PathResolver {
            base_config_dir,
            config_file,
            config_filename,
            state_file,
            tools_config_dir,
        })
    }

    /// Gets a reference to the base configuration directory path.
    #[allow(dead_code)]
    pub fn base_config_dir(&self) -> &Path {
        &self.base_config_dir
    }

    /// Gets a reference to the main config file's full path.
    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    /// Gets a reference to the main config file's name (e.g., "config.yaml").
    pub fn config_filename(&self) -> &str {
        &self.config_filename
    }

    /// Gets a reference to the state file's full path.
    pub fn state_file(&self) -> &Path {
        &self.state_file
    }

    /// Gets a reference to the tools configuration directory path.
    #[allow(dead_code)]
    pub fn tools_config_dir(&self) -> &Path {
        &self.tools_config_dir
    }

    /// Constructs and returns the path to the 'configs' subdirectory within the base config directory.
    ///
    /// This is typically used by components like `ConfigurationUpdater`.
    ///
    /// # Returns
    /// A `PathBuf` representing the path (e.g., `~/.setup-devbox/configs`).
    pub fn configs_dir(&self) -> PathBuf {
        self.base_config_dir.join("configs")
    }

    /// Returns the key paths in a legacy tuple format for backwards compatibility.
    ///
    /// # Returns
    /// A tuple of `(config_file, config_filename, state_file)`.
    #[allow(dead_code)]
    pub fn as_tuple(&self) -> (PathBuf, String, PathBuf) {
        (
            self.config_file.clone(),
            self.config_filename.clone(),
            self.state_file.clone(),
        )
    }

    /// Determines the **base configuration directory**.
    ///
    /// Resolution priority:
    /// 1. `SDB_CONFIG_PATH` environment variable.
    /// 2. Default: `~/.setup-devbox`.
    fn resolve_base_config_dir() -> PathBuf {
        if let Ok(env_path) = env::var("SDB_CONFIG_PATH") {
            log_debug!("[SDB] Using SDB_CONFIG_PATH: {}", env_path.blue());
            // Expand tilde in the environment path if present.
            return Self::expand_tilde(&env_path);
        }

        // Default fallback path, expanding '~' to the user's home directory.
        Self::expand_tilde("~/.setup-devbox")
    }

    /// Determines the main **configuration file path**.
    ///
    /// Resolution priority:
    /// 1. `user_override` argument.
    /// 2. Path derived from `SDB_CONFIG_PATH` (`$SDB_CONFIG_PATH/configs/config.yaml`).
    /// 3. Default path derived from `base_dir` (`$base_dir/configs/config.yaml`).
    fn resolve_config_file(
        base_dir: &Path,
        user_override: Option<String>,
    ) -> Result<PathBuf, String> {
        let path = if let Some(user_path) = user_override {
            // Priority 1: User-provided path takes highest priority.
            Self::expand_tilde(&user_path)
        } else if let Ok(env_path) = env::var("SDB_CONFIG_PATH") {
            // Priority 2: Use SDB_CONFIG_PATH (even if the variable was only used for the base dir).
            // Note: This logic assumes $SDB_CONFIG_PATH is the *base* directory.
            Self::expand_tilde(&format!("{env_path}/configs/config.yaml"))
        } else {
            // Priority 3: Default path relative to the resolved base config directory.
            base_dir.join("configs").join("config.yaml")
        };

        // Basic validation: ensure the resulting path is not empty.
        if path.as_os_str().is_empty() {
            return Err("Resolved config path is empty".to_string());
        }

        Ok(path)
    }

    /// Determines the application **state file path**.
    ///
    /// Resolution priority:
    /// 1. `user_override` argument.
    /// 2. `SDB_STATE_FILE_PATH` environment variable (`$SDB_STATE_FILE_PATH/state.json`).
    /// 3. `SDB_CONFIG_PATH` environment variable (`$SDB_CONFIG_PATH/state.json`).
    /// 4. Default path derived from `base_dir` (`$base_dir/state.json`).
    fn resolve_state_file(
        base_dir: &Path,
        user_override: Option<String>,
    ) -> Result<PathBuf, String> {
        let path = if let Some(user_path) = user_override {
            // Priority 1: User-provided path takes highest priority.
            Self::expand_tilde(&user_path)
        } else if let Ok(env_path) = env::var("SDB_STATE_FILE_PATH") {
            // Priority 2: Dedicated state file environment variable.
            log_debug!(
                "[SDB] Using {} for state file",
                "SDB_STATE_FILE_PATH".cyan()
            );
            Self::expand_tilde(&format!("{env_path}/state.json"))
        } else if let Ok(env_path) = env::var("SDB_CONFIG_PATH") {
            // Priority 3: Fallback to the general config path environment variable.
            log_debug!("[SDB] Using {} for state file", "SDB_CONFIG_PATH".cyan());
            Self::expand_tilde(&format!("{env_path}/state.json"))
        } else {
            // Priority 4: Default path relative to the resolved base config directory.
            base_dir.join("state.json")
        };

        // Basic validation: ensure the resulting path is not empty.
        if path.as_os_str().is_empty() {
            return Err("Resolved state path is empty".to_string());
        }

        Ok(path)
    }

    /// Determines the **tools configuration directory** path.
    ///
    /// Resolution priority:
    /// 1. `SDB_TOOLS_SOURCE_CONFIG_PATH` environment variable (full path).
    /// 2. Default: `$base_dir/configs/tools`.
    fn resolve_tools_config_dir(base_dir: &Path) -> PathBuf {
        // Priority 1: SDB_TOOLS_SOURCE_CONFIG_PATH
        if let Ok(env_path) = env::var("SDB_TOOLS_SOURCE_CONFIG_PATH") {
            match Self::expand_path(&env_path) {
                Ok(expanded) => {
                    log_debug!("[Tools] Using {}", "SDB_TOOLS_SOURCE_CONFIG_PATH".cyan());
                    return expanded;
                }
                Err(_) => {
                    // Log warning if expansion/validation fails for the environment path.
                    log_warn!(
                        "[Tools] Failed to expand {}, using fallback",
                        "SDB_TOOLS_SOURCE_CONFIG_PATH".cyan()
                    );
                }
            }
        }

        // Priority 2: Base config dir + configs/tools
        base_dir.join("configs").join("tools")
    }

    /// A super useful function to resolve paths that start with a tilde `~`.
    /// On Unix-like systems, `~` is a shortcut for the user's home directory.
    /// This function expands that `~` into the full, absolute path, like `/Users/yourusername/`.
    /// This is crucial for user-friendly path inputs.
    ///
    /// # Arguments
    /// * `path`: A string slice (`&str`) representing the path, which might start with `~`.
    ///
    /// # Returns
    /// * `PathBuf`: The fully resolved path if `~` was present and the home directory
    ///   could be determined. Otherwise, it returns the original path unchanged.
    ///
    /// # Examples
    /// ```
    /// use path_resolver::PathResolver;
    ///
    /// let expanded = PathResolver::expand_tilde("~/Documents/file.txt");
    /// // On Unix: PathBuf("/Users/username/Documents/file.txt")
    ///
    /// let unchanged = PathResolver::expand_tilde("/absolute/path");
    /// // Returns: PathBuf("/absolute/path")
    /// ```
    pub fn expand_tilde(path: &str) -> PathBuf {
        // Check if the input path string actually begins with a tilde character.
        if path.starts_with("~") {
            // Attempt to retrieve the current user's home directory.
            // `dirs::home_dir()` is a cross-platform way to get this path.
            if let Some(home) = dirs::home_dir() {
                // If the home directory was successfully found:
                // 1. Convert the home directory `PathBuf` into a string slice (`to_string_lossy()`)
                //    which safely handles non-UTF8 characters by replacing them.
                // 2. Use `replacen` to replace only the *first* occurrence of `~` with the home path.
                //    This ensures paths like `~/Documents/~/file.txt` are handled correctly.
                return PathBuf::from(path.replacen("~", &home.to_string_lossy(), 1));
            }
        }
        // If the path does not start with `~`, or if `dirs::home_dir()` failed to find
        // the home directory, simply convert the original input path string into a `PathBuf`
        // and return it as is.
        PathBuf::from(path)
    }

    /// Expands a path (including tilde expansion) and checks if the resulting path is empty.
    ///
    /// Used specifically for environment variable paths where a non-empty, valid path is expected.
    ///
    /// # Arguments
    /// * `path` - The path string to expand.
    ///
    /// # Returns
    /// A `Result` containing the expanded `PathBuf` or an error if the result is empty.
    fn expand_path(path: &str) -> Result<PathBuf, String> {
        let expanded = Self::expand_tilde(path);
        if expanded.as_os_str().is_empty() {
            return Err("Expanded path is empty".to_string());
        }
        Ok(expanded)
    }
}

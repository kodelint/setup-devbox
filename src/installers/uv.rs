// This module provides the installation logic for Python packages using `uv`.
// It acts as a specialized installer within the `setup-devbox` application,
// handling the nuances of `uv` commands, different installation modes, and options.

// For working with file paths in an OS-agnostic manner.
use crate::libs::utilities::assets::current_timestamp;
use std::path::PathBuf;
// Internal module imports:
// `ToolEntry`: Represents a single tool's configuration as defined in your `tools.yaml` file.
//              It's a struct that contains all possible configuration fields for a tool,
//              such as name, version, source, URL, repository, etc.
// `ToolState`: Represents the actual state of an *installed* tool. This struct is used to
//              persist information about installed tools in the application's `state.json` file.
//              It helps `setup-devbox` track what's installed, its version, and where it's located.
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;
// Imports custom logging macros from the crate root.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to terminal output.
use colored::Colorize;
// For executing external commands and capturing their output.
use crate::libs::tool_installer::execute_post_installation_hooks;
use crate::libs::utilities::misc_utils::{default_python_path, extract_version_number};
use serde_json::Value;
use std::process::{Command, Output};

/// Installs a Python package using `uv`.
///
/// This function is the primary entry point for installing tools using the `uv` package manager.
/// It encapsulates the entire process, from validating the environment to executing the command
/// and recording the final state of the installation.
///
/// `uv` supports three primary installation modes, each with distinct behavior:
/// 1. `uv tool install`: For installing command-line tools that are also Python packages (e.g., `black`, `isort`).
/// 2. `uv pip install`: For installing Python packages into a virtual environment or user site.
/// 3. `uv python install`: For installing specific Python interpreter versions.
///
/// # Workflow:
/// 1. **UV Executable Detection**: Checks if the `uv` command is available on the system's PATH.
/// 2. **Installation Mode Detection**: Determines the correct `uv` subcommand (`tool`, `pip`, or `python`)
///    based on the `ToolEntry` configuration, with `tool` as the default.
/// 3. **Command Construction**: Builds the complete `uv` command, including the package specifier
///    (`package==version`) and any additional user-provided options.
/// 4. **Execution**: Runs the constructed command in a new process.
/// 5. **Error Handling**: Captures the process output and status code, providing clear, detailed
///    logs for success, warnings, or failures.
/// 6. **Path Determination**: Calculates the likely installation path based on the chosen mode.
/// 7. **State Recording**: Creates and returns a `ToolState` object containing all relevant
///    installation details for persistent tracking.
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing the tool's configuration.
///   - `tool_entry.name`: The name of the Python package or interpreter (mandatory).
///   - `tool_entry.version`: Optional version string to install.
///   - `tool_entry.options`: Optional `Vec<String>` with additional arguments to pass to `uv`.
///     The special option `--mode=tool|pip|python` can be used to override the default behavior.
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if the installation was successful.
/// * `None` if `uv` is not found or the installation fails for any other reason.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!(
        "[UV Installer] Attempting to install Python package: {}",
        tool_entry.name.bold()
    );

    // 1. Check if uv command is available
    if Command::new("uv").arg("--version").output().is_err() {
        log_error!(
            "[UV Installer] 'uv' command not found. Please install uv first. Visit: https://github.com/astral-sh/uv"
        );
        return None;
    }

    // 2. Validate options before proceeding
    if !validate_uv_options(tool_entry) {
        log_error!(
            "[UV Installer] Invalid options for tool entry: {}",
            tool_entry.name
        );
        return None;
    }

    // 3. Determine installation mode and construct command
    let (subcommand, mut command_args) = determine_installation_mode(tool_entry);

    log_debug!(
        "[UV Installer] Using installation mode: {}",
        subcommand.cyan().bold()
    );

    // 4. Add the package specifier (for tool and pip modes)
    if subcommand != "python" {
        let package_specifier = if let Some(version) = &tool_entry.version {
            format!("{}=={}", tool_entry.name, version)
        } else {
            tool_entry.name.clone()
        };
        command_args.push(package_specifier.clone());
    } else {
        // For python mode, the "name" is just dummy
        // For python mode, the "version" is actually the Python version
        command_args.push(tool_entry.version.clone()?);
    }

    // 5. Process additional options from tool_entry
    if let Some(options) = &tool_entry.options {
        for opt in options {
            // Skip our custom --mode option as it's already processed
            if !opt.starts_with("--mode=") {
                command_args.push(opt.clone());
            }
        }
    }

    // 6. Log the full command being executed
    log_info!(
        "[UV Installer] Executing: {} {} {}",
        "uv".cyan().bold(),
        subcommand.cyan().bold(),
        command_args.join(" ").cyan()
    );

    // 7. Execute the uv command
    let mut cmd = Command::new("uv");
    cmd.arg(&subcommand).args(&command_args);

    let output: Output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            log_error!(
                "[UV Installer] Failed to execute 'uv {}' command for '{}': {}",
                subcommand.bold().red(),
                tool_entry.name.bold().red(),
                e
            );
            return None;
        }
    };

    // 8. Check command exit status
    if output.status.success() {
        log_info!(
            "[UV Installer] Successfully installed: {} using {} {}",
            tool_entry.name.bold().green(),
            "uv".green().bold(),
            subcommand.green()
        );

        // Log outputs for debugging
        if !output.stdout.is_empty() {
            log_debug!(
                "[UV Installer] Stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        if !output.stderr.is_empty() {
            log_warn!(
                "[UV Installer] Stderr (might contain warnings): {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // 9. Determine installation path based on mode
        let install_path = determine_install_path(&subcommand, &tool_entry.name);

        // 10. Execute Additional Commands (if specified)
        // After the main installation is complete, execute any additional commands specified
        // in the tool configuration. These commands are often used for post-installation setup,
        // such as copying configuration files, creating directories, or setting up symbolic links.
        // Optional - failure won't stop installation
        let additional_cmd_working_dir =
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Execute additional commands (optional - failure won't stop installation)
        let executed_post_installation_hooks = execute_post_installation_hooks(
            "[UV Installer]",
            tool_entry,
            &additional_cmd_working_dir,
        );

        // 11. Return ToolState for Tracking
        // Construct a `ToolState` object to record the details of this successful installation.
        // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
        // what tools are installed, where they are, and how they were installed. This is crucial
        // for future operations like uninstallation, updates, or syncing.
        Some(ToolState {
            // The version field for tracking. Defaults to "latest" if not explicitly set in `tools.yaml`.
            version: match subcommand.as_str() {
                "python" => {
                    // For Python installations, the "version" is the Python version itself
                    tool_entry.version.clone()?
                }
                _ => {
                    // For packages, use the specified version or "latest"
                    tool_entry
                        .version
                        .clone()
                        .unwrap_or_else(|| "latest".to_string())
                }
            },
            // The canonical path where the tool's executable was installed. This is the path
            // that will be recorded in the `state.json` file.
            install_path,
            // Flag indicating that this tool was installed by `setup-devbox`. This helps distinguish
            // between tools managed by our system and those installed manually.
            installed_by_devbox: true,
            // The method of installation, useful for future diagnostics or differing update logic.
            // In this module, it's always "uv".
            // e.g., "uv-tool", "uv-python", "uv-pip"
            install_method: format!("uv-{subcommand}"),
            // Records if the binary was renamed during installation, storing the new name.
            renamed_to: tool_entry.rename_to.clone(),
            repo: None,
            tag: None, // Package version, not uv version
            // The actual package type detected by the `file` command or inferred. This is for diagnostic
            // purposes, providing the most accurate type even if the installation logic
            // used a filename-based guess (e.g., "binary", "macos-pkg-installer").
            package_type: match subcommand.as_str() {
                "python" => "python-interpreter".to_string(),
                _ => "python-package".to_string(),
            },
            // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
            options: tool_entry.options.clone(),
            // For direct URL installations: The original URL from which the tool was downloaded.
            // This is important for re-downloading or verifying in the future.
            url: tool_entry.url.clone(),
            // Record the timestamp when the tool was installed or updated
            last_updated: Some(current_timestamp()),
            // This field is currently `None` but could be used to store the path to an executable
            // *within* an extracted archive if `install_path` points to the archive's root.
            executable_path_after_extract: None,
            // Record any additional commands that were executed during installation.
            // This is useful for tracking what was done and potentially for cleanup during uninstall.
            // additional_cmd_executed: tool_entry.additional_cmd.clone(),
            executed_post_installation_hooks,
            configuration_manager: None,
        })
    } else {
        // 11. Handle installation failure
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        log_error!(
            "[UV Installer] Failed to install '{}' using uv {}. Exit code: {}",
            tool_entry.name.bold().red(),
            subcommand.red(),
            output.status.code().unwrap_or(-1)
        );

        if !stderr.is_empty() {
            log_error!("[UV Installer] Stderr: {}", stderr.red());
        }
        if !stdout.is_empty() {
            log_debug!("[UV Installer] Stdout: {}", stdout);
        }

        None
    }
}

/// Determines the appropriate `uv` installation mode and constructs base command arguments.
///
/// This function inspects the `tool_entry`'s `options` for a `--mode=` flag. If found, it
/// uses that mode; otherwise, it defaults to `uv tool install`. This provides a flexible
/// way for users to control the installation behavior.
///
/// # Installation Modes:
/// - `tool`: Uses `uv tool install` - for global CLI tools (default).
/// - `pip`: Uses `uv pip install` - a pip-compatible interface, typically for libraries.
/// - `python`: Uses `uv python install` - for installing Python versions.
///
/// # Arguments
/// * `tool_entry`: A reference to the tool's configuration.
///
/// # Returns
/// A tuple of `(subcommand, base_args)` where:
/// - `subcommand`: The `uv` subcommand to use (e.g., "tool", "pip", "python").
/// - `base_args`: The initial arguments for the subcommand, which is always `["install"]` in this case.
fn determine_installation_mode(tool_entry: &ToolEntry) -> (String, Vec<String>) {
    // Check for explicit mode override in options
    if let Some(options) = &tool_entry.options {
        for opt in options {
            if let Some(mode) = opt.strip_prefix("--mode=") {
                match mode {
                    "tool" => return ("tool".to_string(), vec!["install".to_string()]),
                    "pip" => return ("pip".to_string(), vec!["install".to_string()]),
                    "python" => return ("python".to_string(), vec!["install".to_string()]),
                    _ => {
                        log_warn!(
                            "[UV Installer] Unknown installation mode '{}', falling back to 'tool'",
                            mode
                        );
                    }
                }
            }
        }
    }

    // Default to tool installation for global CLI tools
    ("tool".to_string(), vec!["install".to_string()])
}

/// Determines the likely installation path based on the `uv` subcommand used.
///
/// This function provides a best-effort guess for the final location of the installed binary or package.
/// The exact path can vary based on the user's system and `uv` configuration, but these paths are
/// the most common and serve as a reliable default for recording the `ToolState`.
///
/// # Arguments
/// * `subcommand`: The `uv` subcommand that was used ("tool", "pip", "python").
/// * `package_name`: The name of the installed package or Python version.
///
/// # Returns
/// A string representing the likely absolute installation path.
fn determine_install_path(subcommand: &str, package_name: &str) -> String {
    match subcommand {
        "tool" => {
            // uv tool install typically installs to ~/.local/bin or similar
            // uv manages its own tool installation directory
            if let Ok(home) = std::env::var("HOME") {
                // uv tool installs typically go to ~/.local/bin
                format!("{home}/.local/bin/{package_name}")
            } else {
                log_warn!(
                    "[UV Installer] Could not determine HOME directory for tool installation path"
                );
                format!("/usr/local/bin/{package_name}")
            }
        }
        "pip" => {
            // uv pip install behavior depends on active environment
            // Default to user site-packages location
            if let Ok(home) = std::env::var("HOME") {
                format!("{home}/.local/lib/python/site-packages/{package_name}")
            } else {
                "/usr/local/lib/python/site-packages/".to_string()
            }
        }
        "python" => {
            // uv python install installs to uv's managed Python directory
            // Use `uv python list --only-installed --json` to find the exact path
            match Command::new("uv")
                .args([
                    "python",
                    "list",
                    "--only-installed",
                    "--output-format",
                    "json",
                ])
                .output()
            {
                Ok(output) if output.status.success() => {
                    let output_str = String::from_utf8_lossy(&output.stdout);

                    // Parse the JSON output
                    match serde_json::from_str::<Vec<serde_json::Value>>(&output_str) {
                        Ok(python_installations) => {
                            for installation in python_installations {
                                if let Some(key) = installation.get("key").and_then(|k| k.as_str())
                                {
                                    // Check if the key approximately matches the package_name
                                    if key.contains(package_name) {
                                        if let Some(path) =
                                            installation.get("path").and_then(|p| p.as_str())
                                        {
                                            log_debug!(
                                                "[UV Installer] Found Python installation for {}: {}",
                                                package_name,
                                                path
                                            );
                                            return path.to_string();
                                        }
                                    }
                                }
                            }

                            // Fallback if no matching installation found
                            log_warn!(
                                "[UV Installer] No matching Python installation found for {}, using default path",
                                package_name
                            );
                            default_python_path(package_name)
                        }
                        Err(e) => {
                            log_warn!(
                                "[UV Installer] Failed to parse uv python list output: {}, using default path",
                                e
                            );
                            default_python_path(package_name)
                        }
                    }
                }
                Ok(output) => {
                    let error = String::from_utf8_lossy(&output.stderr);
                    log_warn!(
                        "[UV Installer] uv python list command failed: {}, using default path",
                        error
                    );
                    default_python_path(package_name)
                }
                Err(e) => {
                    log_warn!(
                        "[UV Installer] Failed to execute uv python list: {}, using default path",
                        e
                    );
                    default_python_path(package_name)
                }
            }
        }
        _ => {
            log_warn!(
                "[UV Installer] Unknown subcommand '{}', using generic path",
                subcommand
            );
            if let Ok(home) = std::env::var("HOME") {
                format!("{home}/.local/bin/{package_name}")
            } else {
                "/usr/local/bin/".to_string()
            }
        }
    }
}

/// Validates `uv`-specific options and provides helpful warnings.
///
/// This function performs a basic validation of the user-provided options to ensure they are
/// compatible with the chosen installation mode. It checks for typos or invalid options that
/// might cause the `uv` command to fail.
///
/// # Arguments
/// * `tool_entry`: A reference to the tool configuration to validate.
///
/// # Returns
/// `true` if the configuration appears valid, `false` if there are critical issues.
pub fn validate_uv_options(tool_entry: &ToolEntry) -> bool {
    let mut is_valid = true;

    if let Some(options) = &tool_entry.options {
        let mut install_mode = "tool"; // default

        // Extract the installation mode first
        for opt in options {
            if let Some(mode) = opt.strip_prefix("--mode=") {
                if !matches!(mode, "tool" | "pip" | "python") {
                    log_error!(
                        "[UV Installer] Invalid installation mode '{}'. Supported modes: tool, pip, python",
                        mode
                    );
                    is_valid = false;
                } else {
                    install_mode = mode;
                }
                break;
            }
        }

        // Define valid options for each mode based on uv help output
        let valid_options = match install_mode {
            "tool" => vec![
                // Cache options
                "-n",
                "--no-cache",
                "--cache-dir",
                // Python options
                "--managed-python",
                "--no-managed-python",
                "--no-python-downloads",
                // Global options
                "-q",
                "--quiet",
                "-v",
                "--verbose",
                "--color",
                "--native-tls",
                "--offline",
                "--allow-insecure-host",
                "--no-progress",
                "--directory",
                "--project",
                "--config-file",
                "--no-config",
                // Tool install specific options (from uv tool install -h)
                "--with",
                "--python",
                "--force",
                "--editable",
                "-e",
            ],
            "pip" => vec![
                // Cache options
                "-n",
                "--no-cache",
                "--cache-dir",
                // Python options
                "--managed-python",
                "--no-managed-python",
                "--no-python-downloads",
                // Global options
                "-q",
                "--quiet",
                "-v",
                "--verbose",
                "--color",
                "--native-tls",
                "--offline",
                "--allow-insecure-host",
                "--no-progress",
                "--directory",
                "--project",
                "--config-file",
                "--no-config",
                // Pip install specific options (from uv pip install -h)
                "-r",
                "--requirement",
                "-c",
                "--constraint",
                "-e",
                "--editable",
                "--extra-index-url",
                "--find-links",
                "--no-index",
                "--index-url",
                "--keyring-provider",
                "--no-build-isolation",
                "--no-deps",
                "--upgrade",
                "--upgrade-package",
                "--reinstall",
                "--reinstall-package",
                "--no-build",
                "--no-binary",
                "--only-binary",
                "--python",
                "--system",
                "--break-system-packages",
                "--target",
                "--prefix",
                "--legacy-setup-py",
                "--no-build-isolation-package",
                "--config-settings",
                "--no-compile-bytecode",
                "--link-mode",
                "--compile-bytecode",
            ],
            "python" => vec![
                // Cache options
                "-n",
                "--no-cache",
                "--cache-dir",
                // Python options
                "--managed-python",
                "--no-managed-python",
                "--no-python-downloads",
                // Global options
                "-q",
                "--quiet",
                "-v",
                "--verbose",
                "--color",
                "--native-tls",
                "--offline",
                "--allow-insecure-host",
                "--no-progress",
                "--directory",
                "--project",
                "--config-file",
                "--no-config",
                // Python install specific options (from uv python install -h)
                "--force",
                "--default",
            ],
            _ => vec![],
        };

        // Validate each option
        for opt in options {
            // Skip our custom --mode option
            if opt.starts_with("--mode=") {
                continue;
            }

            // Extract the option name (without value)
            let opt_name = if opt.starts_with("--") {
                opt.split('=').next().unwrap_or(opt)
            } else if opt.starts_with('-') && opt.len() == 2 {
                opt
            } else {
                // Could be a value for a previous option, skip validation
                continue;
            };

            // Check if the option is valid for the current mode
            if !valid_options.contains(&opt_name) {
                log_warn!(
                    "[UV Installer] Option '{}' may not be valid for 'uv {}' mode",
                    opt,
                    install_mode
                );
            }
        }
        // Mode-specific validations
        match install_mode {
            "python" => {
                // For python mode, validate that the "name" looks like a Python version
                if !is_valid_python_version(&tool_entry.name) {
                    log_warn!(
                        "[UV Installer] '{}' doesn't look like a Python version (expected format like '3.11', '3.12.1', etc.)",
                        tool_entry.version.clone().unwrap().bright_yellow()
                    );
                }
            }
            _ => {}
        }
    }

    is_valid
}

/// Checks if a Python version string matches any available version from uv.
/// Only matches the version number part (e.g., "3.13.7") ignoring any prefix.
///
/// # Arguments
/// * `version_input` - The version string to check (e.g., "3.13.7", "cpython3.13.7", "cpython-3.13.7")
///
/// # Returns
/// `bool` - `true` if a matching version is found, `false` otherwise
fn is_valid_python_version(version_input: &str) -> bool {
    // Extract just the version number part (e.g., "3.13.7")
    let version_number = extract_version_number(version_input);

    // Execute the uv command
    let output = match Command::new("uv")
        .args(&["python", "list", "--output-format", "json"])
        .output()
    {
        Ok(output) => output,
        Err(_) => return false, // Command failed
    };

    if !output.status.success() {
        return false;
    }

    // Parse JSON output
    let json_output: Value = match serde_json::from_slice(&output.stdout) {
        Ok(json) => json,
        Err(_) => return false, // JSON parsing failed
    };

    // Extract all available version strings from the JSON
    let available_versions: Vec<String> = match json_output.as_array() {
        Some(array) => array
            .iter()
            .filter_map(|entry| entry["version"].as_str().map(String::from))
            .collect(),
        None => return false, // Not a JSON array
    };

    // Check if the extracted version number matches any available version
    available_versions
        .iter()
        .any(|available_version| available_version == &version_number)
}

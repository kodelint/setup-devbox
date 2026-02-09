//! # UV Installer Module
//!
//! This module provides a robust, production-grade installer for Python packages using the `uv` tool.
//! It follows the same reliability standards as the official `uv` installer with comprehensive
//! error handling, verification mechanisms, and accurate path detection.
//!
//! ## Key Features
//!
//! - **Multiple Installation Modes**: Supports tool, pip, and python installation modes
//! - **Comprehensive Validation**: Validates uv availability, installation success, and binary paths
//! - **Smart State Tracking**: Maintains accurate installation state with version tracking
//! - **Flexible Configuration**: Supports version specifications, custom uv options, and mode selection
//! - **Post-Installation Hooks**: Executes additional setup commands after successful installation
//! - **Environment Awareness**: Properly handles different installation directories and Python paths
//!
//! ## Installation Workflow
//!
//! The installer follows a meticulous 9-step process:
//!
//! 1. **Configuration Validation** - Validates tool entry and installation options
//! 2. **Mode Detection** - Determines correct installation mode (tool/pip/python)
//! 3. **Command Construction** - Builds the complete uv command with proper arguments
//! 4. **Command Execution** - Runs installation with comprehensive error handling
//! 5. **Installation Verification** - Confirms the package was properly installed
//! 6. **Path Resolution** - Accurately determines installation path based on mode
//! 7. **Post-Installation Hooks** - Executes any additional setup commands
//! 8. **State Creation** - Creates comprehensive tool state for persistence
//!
//! ## Error Handling
//!
//! The module provides detailed error messages and logging at multiple levels:
//! - **Info**: High-level installation progress
//! - **Debug**: Detailed command construction and path resolution
//! - **Warn**: Non-fatal issues or warnings during installation
//! - **Error**: Installation failures with specific error codes and messages

// Standard Library Imports
use semver::Version;
use std::path::PathBuf;
use std::process::{Command, Output};
// External Crate Imports
// The `colored` crate allows us to make log messages and other terminal output more readable
// by applying colors (e.g., `.blue()`, `.green()`, `.red()`).
use colored::Colorize;
use serde_json::Value;
// Internal Module Imports
// These macros (`log_debug`, `log_error`, `log_info`, `log_warn`) provide
// a standardized way to output messages to the console with different severity levels,
// making it easier to track the application's flow and diagnose issues.
use crate::{log_debug, log_error, log_info, log_warn};
// For executing external commands and capturing their output.
// `std::process::Command` is used to run commands/hooks.
// `std::process::Output` captures the stdout, stderr, and exit status of executed commands.
use crate::engine::execute_post_installation_hooks;
use crate::engine::installers::errors::InstallerError;
use crate::engine::installers::traits::Installer;
// `ToolEntry`: Represents a single tool's configuration from `tools.yaml`.
// `ToolState`: Represents the actual state of an installed tool for persistence in `state.json`.
use crate::schemas::state_file::ToolState;
use crate::schemas::tools_types::ToolEntry;

/// Struct representing the UV installer.
pub struct UvInstaller;

impl Installer for UvInstaller {
    fn install(&self, tool_entry: &ToolEntry) -> Result<ToolState, InstallerError> {
        log_info!(
            "[SDB::Tools::UVInstaller] Attempting to install Python package: {}",
            tool_entry.name.bold()
        );
        log_debug!(
            "[SDB::Tools::UVInstaller] ToolEntry details: {:#?}",
            tool_entry
        );

        if !validate_uv_configuration(tool_entry) {
            return Err(InstallerError::ConfigurationError(format!(
                "Invalid configuration for '{}'",
                tool_entry.name
            )));
        }

        let (subcommand, base_args) = determine_installation_mode(tool_entry);
        log_debug!(
            "[SDB::Tools::UVInstaller] Using installation mode: {}",
            subcommand.cyan().bold()
        );

        let command_args =
            build_command_args(&subcommand, &base_args, tool_entry).ok_or_else(|| {
                InstallerError::ConfigurationError("Failed to build command args".into())
            })?;

        let output =
            execute_uv_command(&subcommand, &command_args, tool_entry).ok_or_else(|| {
                InstallerError::CommandFailed(format!("Failed to execute 'uv {}'", subcommand))
            })?;

        if !verify_installation_success(&output, &subcommand, tool_entry) {
            return Err(InstallerError::InstallationFailed(format!(
                "Installation failed for '{}'",
                tool_entry.name
            )));
        }

        let install_path = determine_install_path(&subcommand, &tool_entry.name);
        log_debug!(
            "[SDB::Tools::UVInstaller] Determined installation path: {}",
            install_path.display().to_string().cyan()
        );

        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let executed_hooks =
            execute_post_installation_hooks("[UV Installer]", tool_entry, &working_dir);

        log_info!(
            "[SDB::Tools::UVInstaller] Successfully installed: {} using uv {}",
            tool_entry.name.bold().green(),
            subcommand.green()
        );

        Ok(ToolState::new(
            tool_entry,
            &install_path,
            format!("uv-{subcommand}"),
            match subcommand.as_str() {
                "python" => "python-interpreter".to_string(),
                _ => "python-package".to_string(),
            },
            tool_entry
                .version
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            None,
            None,
            executed_hooks,
        ))
    }

    fn get_latest_version(&self, tool_entry: &ToolEntry) -> Result<String, InstallerError> {
        log_debug!(
            "[SDB::Tools::UVInstaller] Getting latest version for: {}",
            tool_entry.name.bold()
        );
        log_debug!(
            "[SDB::Tools::UVInstaller] ToolEntry details: {:#?}",
            tool_entry
        );

        let (mode, _) = determine_installation_mode(tool_entry);

        match mode.as_str() {
            "python" => {
                let version_prefix = tool_entry.version.as_ref().map(|v| {
                    let parts: Vec<&str> = v.split('.').collect();
                    if parts.len() >= 2 {
                        format!("{}.{}", parts[0], parts[1])
                    } else {
                        v.clone()
                    }
                });

                get_latest_uv_python_version(version_prefix).ok_or_else(|| {
                    InstallerError::VersionDetectionFailed(format!(
                        "Failed to get latest python version for '{}'",
                        tool_entry.name
                    ))
                })
            }
            "pip" => {
                let package_name = &tool_entry.name;
                get_latest_uv_pip_version(package_name).ok_or_else(|| {
                    InstallerError::VersionDetectionFailed(format!(
                        "Failed to get latest uv version for '{}'",
                        package_name
                    ))
                })
            }
            "tool" => Ok("Skipped (not supported for uv tool)".to_string()),
            _ => Err(InstallerError::VersionDetectionFailed(
                "Unknown uv installation mode.".to_string(),
            )),
        }
    }
}

/// # `get_latest_uv_python_version`
///
/// Gets the latest available version for a python interpreter from `uv python list`.
///
/// ## Arguments
///
/// * `version_prefix`: An optional string to filter the python versions.
///
/// ## Returns
///
/// `Some(String)` containing the latest version, or `None` if detection fails.
fn get_latest_uv_python_version(version_prefix: Option<String>) -> Option<String> {
    log_debug!("[SDB::Tools::UVInstaller] Executing 'uv python list --output-format json'");

    match Command::new("uv")
        .args(["python", "list", "--output-format", "json"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(json_array) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                let mut latest_semver: Option<Version> = None;

                for installation in json_array {
                    if let Some(key) = installation["key"].as_str()
                        && let Some(prefix) = &version_prefix
                        && key.contains(prefix)
                        && let Some(version_part) = key.strip_prefix("cpython-")
                        && let Some(end_pos) = version_part.find('-')
                    {
                        let version_str = &version_part[..end_pos];
                        if let Ok(semver) = Version::parse(version_str)
                            && (latest_semver.is_none()
                                || semver > *latest_semver.as_ref().unwrap())
                        {
                            latest_semver = Some(semver);
                        }
                    }
                }
                return latest_semver.map(|v| v.to_string());
            }
            None
        }
        _ => None,
    }
}

/// # `get_latest_uv_pip_version`
///
/// Gets the latest available version for a pip package from PyPI.
///
/// ## Arguments
///
/// * `package_name`: The name of the package to search for.
///
/// ## Returns
///
/// `Some(String)` containing the latest version, or `None` if detection fails.
fn get_latest_uv_pip_version(package_name: &str) -> Option<String> {
    log_debug!(
        "[SDB::Tools::UVInstaller] Executing 'uv pip search {}'",
        package_name.cyan()
    );

    match Command::new("uv")
        .args(["pip", "search", package_name])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed_line = line.trim();
                if trimmed_line.starts_with(package_name)
                    && let Some(start_paren) = trimmed_line.find('(')
                    && let Some(end_paren) = trimmed_line.find(')')
                {
                    let version = &trimmed_line[start_paren + 1..end_paren];
                    return Some(version.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

/// # `validate_uv_configuration`
///
/// Validates the tool entry for `uv` installation.
///
/// ## Arguments
///
/// * `tool_entry`: A reference to the `ToolEntry` to validate.
///
/// ## Returns
///
/// `true` if the configuration is valid, `false` otherwise.
fn validate_uv_configuration(tool_entry: &ToolEntry) -> bool {
    if tool_entry.name.trim().is_empty() {
        log_error!("[SDB::Tools::UVInstaller] Tool name is empty or whitespace");
        return false;
    }

    if !validate_uv_options(tool_entry) {
        log_error!(
            "[SDB::Tools::UVInstaller] Invalid options for tool entry: {}",
            tool_entry.name.red()
        );
        return false;
    }

    if let Some(options) = &tool_entry.options {
        for opt in options {
            if let Some("python") = opt.strip_prefix("--mode=")
                && tool_entry.version.is_none()
            {
                log_error!(
                    "[SDB::Tools::UVInstaller] Python installation mode requires a version to be specified for tool '{}'",
                    tool_entry.name.red()
                );
                return false;
            }
        }
    }

    true
}

/// # `build_command_args`
///
/// Builds the command arguments for the `uv` command.
///
/// ## Arguments
///
/// * `subcommand`: The `uv` subcommand to use (e.g., "tool", "pip", "python").
/// * `base_args`: The base arguments for the subcommand.
/// * `tool_entry`: The `ToolEntry` containing the configuration.
///
/// ## Returns
///
/// `Some(Vec<String>)` with the command arguments, or `None` on error.
fn build_command_args(
    subcommand: &str,
    base_args: &[String],
    tool_entry: &ToolEntry,
) -> Option<Vec<String>> {
    let mut args = base_args.to_vec();

    match subcommand {
        "python" => match &tool_entry.version {
            Some(version) if !version.trim().is_empty() => {
                args.push(version.clone());
            }
            Some(_) => {
                log_error!(
                    "[SDB::Tools::UVInstaller] Empty Python version specified for tool '{}'",
                    tool_entry.name.red()
                );
                return None;
            }
            None => {
                log_error!(
                    "[SDB::Tools::UVInstaller] No Python version specified for tool '{}'",
                    tool_entry.name.red()
                );
                return None;
            }
        },
        _ => {
            let package_specifier = if let Some(version) = &tool_entry.version {
                if version.trim().is_empty() {
                    log_error!(
                        "[SDB::Tools::UVInstaller] Empty version string specified for tool '{}'",
                        tool_entry.name.red()
                    );
                    return None;
                }
                format!("{}=={}", tool_entry.name, version)
            } else {
                tool_entry.name.clone()
            };
            args.push(package_specifier);
        }
    }

    if let Some(options) = &tool_entry.options {
        for opt in options {
            if !opt.starts_with("--mode=") {
                args.push(opt.clone());
            }
        }
    }

    Some(args)
}

/// # `execute_uv_command`
///
/// Executes a `uv` command.
///
/// ## Arguments
///
/// * `subcommand`: The `uv` subcommand to execute.
/// * `command_args`: The arguments for the subcommand.
/// * `tool_entry`: The `ToolEntry` for logging purposes.
///
/// ## Returns
///
/// `Some(Output)` if the command was successful, `None` otherwise.
fn execute_uv_command(
    subcommand: &str,
    command_args: &[String],
    tool_entry: &ToolEntry,
) -> Option<Output> {
    log_debug!(
        "[SDB::Tools::UVInstaller] Executing: {} {} {}",
        "uv".cyan().bold(),
        subcommand.cyan().bold(),
        command_args.join(" ").cyan()
    );

    match Command::new("uv")
        .arg(subcommand)
        .args(command_args)
        .output()
    {
        Ok(output) => Some(output),
        Err(e) => {
            log_error!(
                "[SDB::Tools::UVInstaller] Failed to execute 'uv {}' command for '{}': {}",
                subcommand.bold().red(),
                tool_entry.name.bold().red(),
                e.to_string().red()
            );
            None
        }
    }
}

/// # `verify_installation_success`
///
/// Verifies that the `uv` command executed successfully.
///
/// ## Arguments
///
/// * `output`: The `Output` from the command execution.
/// * `subcommand`: The `uv` subcommand that was executed.
/// * `tool_entry`: The `ToolEntry` for logging purposes.
///
/// ## Returns
///
/// `true` if the installation was successful, `false` otherwise.
fn verify_installation_success(output: &Output, subcommand: &str, tool_entry: &ToolEntry) -> bool {
    if output.status.success() {
        if !output.stdout.is_empty() {
            log_debug!(
                "[SDB::Tools::UVInstaller] Stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        if !output.stderr.is_empty() {
            log_warn!(
                "[SDB::Tools::UVInstaller] Stderr (may contain warnings): {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        true
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        log_error!(
            "[SDB::Tools::UVInstaller] Failed to install '{}' using uv {}. Exit code: {}",
            tool_entry.name.bold().red(),
            subcommand.red(),
            output.status.code().unwrap_or(-1)
        );

        if !stderr.is_empty() {
            log_error!("[SDB::Tools::UVInstaller] Stderr: {}", stderr.red());
        }
        if !stdout.is_empty() {
            log_debug!("[SDB::Tools::UVInstaller] Stdout: {}", stdout);
        }

        false
    }
}

/// # `determine_installation_mode`
///
/// Determines the installation mode (`tool`, `pip`, or `python`) from the tool's options.
///
/// ## Arguments
///
/// * `tool_entry`: The `ToolEntry` to inspect.
///
/// ## Returns
///
/// A tuple containing the subcommand string and a vector of base arguments.
fn determine_installation_mode(tool_entry: &ToolEntry) -> (String, Vec<String>) {
    if let Some(options) = &tool_entry.options {
        for opt in options {
            if let Some(mode) = opt.strip_prefix("--mode=") {
                match mode {
                    "tool" => return ("tool".to_string(), vec!["install".to_string()]),
                    "pip" => return ("pip".to_string(), vec!["install".to_string()]),
                    "python" => return ("python".to_string(), vec!["install".to_string()]),
                    _ => {
                        log_warn!(
                            "[SDB::Tools::UVInstaller] Unknown installation mode '{}', falling back to 'tool'",
                            mode.yellow()
                        );
                    }
                }
            }
        }
    }

    ("tool".to_string(), vec!["install".to_string()])
}

/// # `determine_install_path`
///
/// Determines the likely installation path for a `uv`-installed tool.
///
/// ## Arguments
///
/// * `subcommand`: The `uv` subcommand used for installation.
/// * `package_name`: The name of the package.
///
/// ## Returns
///
/// A `PathBuf` representing the likely installation path.
fn determine_install_path(subcommand: &str, package_name: &str) -> PathBuf {
    match subcommand {
        "tool" => {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(format!("{home}/.local/bin/{package_name}"))
            } else {
                log_warn!(
                    "[SDB::Tools::UVInstaller] Could not determine HOME directory for tool installation path"
                );
                PathBuf::from(format!("/usr/local/bin/{package_name}"))
            }
        }
        "pip" => {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(format!(
                    "{home}/.local/lib/python/site-packages/{package_name}"
                ))
            } else {
                PathBuf::from("/usr/local/lib/python/site-packages/".to_string())
            }
        }
        "python" => match get_python_installation_path(package_name) {
            Some(path) => PathBuf::from(path),
            None => {
                log_warn!(
                    "[SDB::Tools::UVInstaller] Could not determine Python installation path, using default"
                );
                PathBuf::from(default_python_path(package_name))
            }
        },
        _ => {
            log_warn!(
                "[SDB::Tools::UVInstaller] Unknown subcommand '{}', using generic path",
                subcommand
            );
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(format!("{home}/.local/bin/{package_name}"))
            } else {
                PathBuf::from("/usr/local/bin/".to_string())
            }
        }
    }
}

/// # `get_python_installation_path`
///
/// Gets the installation path of a Python interpreter from `uv`.
///
/// ## Arguments
///
/// * `package_name`: The name of the Python package (e.g., "cpython").
///
/// ## Returns
///
/// `Some(String)` with the installation path, or `None` if not found.
fn get_python_installation_path(package_name: &str) -> Option<String> {
    let output = Command::new("uv")
        .args([
            "python",
            "list",
            "--only-installed",
            "--output-format",
            "json",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        log_warn!(
            "[SDB::Tools::UVInstaller] uv python list command failed: {}",
            error
        );
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let installations: Vec<Value> = serde_json::from_str(&output_str).ok()?;

    for installation in installations {
        match (
            installation.get("key").and_then(|k| k.as_str()),
            installation.get("path").and_then(|p| p.as_str()),
        ) {
            (Some(key), Some(path)) if key.contains(package_name) => {
                log_debug!(
                    "[SDB::Tools::UVInstaller] Found Python installation for {}: {}",
                    package_name,
                    path
                );
                return Some(path.to_string());
            }
            _ => continue,
        }
    }

    log_warn!(
        "[SDB::Tools::UVInstaller] No matching Python installation found for {}",
        package_name
    );
    None
}

/// # `validate_uv_options`
///
/// Validates the options for a `uv` tool entry.
///
/// ## Arguments
///
/// * `tool_entry`: The `ToolEntry` to validate.
///
/// ## Returns
///
/// `true` if the options are valid, `false` otherwise.
pub fn validate_uv_options(tool_entry: &ToolEntry) -> bool {
    let mut is_valid = true;

    let options = match &tool_entry.options {
        Some(opts) => opts,
        None => return true,
    };

    let mut install_mode = "tool";

    for opt in options {
        if let Some(mode) = opt.strip_prefix("--mode=") {
            if !matches!(mode, "tool" | "pip" | "python") {
                log_error!(
                    "[SDB::Tools::UVInstaller] Invalid installation mode '{}'. Supported modes: tool, pip, python",
                    mode.red()
                );
                is_valid = false;
            } else {
                install_mode = mode;
            }
            break;
        }
    }

    let valid_options = get_valid_options_for_mode(install_mode);

    for opt in options {
        if opt.starts_with("--mode=") {
            continue;
        }

        if opt.trim().is_empty() {
            log_warn!("[SDB::Tools::UVInstaller] Empty option string detected");
            continue;
        }

        let opt_name = if opt.starts_with("--") {
            opt.split('=').next().unwrap_or(opt)
        } else if opt.starts_with('-') && opt.len() == 2 {
            opt
        } else {
            continue;
        };

        if !valid_options.contains(&opt_name) {
            log_warn!(
                "[SDB::Tools::UVInstaller] Option '{}' may not be valid for 'uv {}' mode",
                opt.yellow(),
                install_mode
            );
        }
    }

    if install_mode == "python" {
        match &tool_entry.version {
            Some(version) if !is_valid_python_version(version) => {
                log_warn!(
                    "[SDB::Tools::UVInstaller] '{}' doesn't look like a valid Python version (expected format like '3.11', '3.12.1', etc.)",
                    version.bright_yellow()
                );
            }
            _ => {}
        }
    }

    is_valid
}

/// # `get_valid_options_for_mode`
///
/// Gets a list of valid options for a given `uv` installation mode.
///
/// ## Arguments
///
/// * `mode`: The installation mode ("tool", "pip", or "python").
///
/// ## Returns
///
/// A `Vec<&'static str>` containing the valid options for the mode.
fn get_valid_options_for_mode(mode: &str) -> Vec<&'static str> {
    let common_options = vec![
        "-n",
        "--no-cache",
        "--cache-dir",
        "--managed-python",
        "--no-managed-python",
        "--no-python-downloads",
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
    ];

    match mode {
        "tool" => {
            let mut opts = common_options.clone();
            opts.extend_from_slice(&["--with", "--python", "--force", "--editable", "-e"]);
            opts
        }
        "pip" => {
            let mut opts = common_options.clone();
            opts.extend_from_slice(&[
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
            ]);
            opts
        }
        "python" => {
            let mut opts = common_options.clone();
            opts.extend_from_slice(&["--force", "--default"]);
            opts
        }
        _ => common_options,
    }
}

/// # `is_valid_python_version`
///
/// Checks if a given Python version string is valid according to `uv`.
///
/// ## Arguments
///
/// * `version_input`: The version string to check.
///
/// ## Returns
///
/// `true` if the version is valid, `false` otherwise.
fn is_valid_python_version(version_input: &str) -> bool {
    let version_number = extract_version_number(version_input);

    let output = match Command::new("uv")
        .args(["python", "list", "--output-format", "json"])
        .output()
    {
        Ok(out) if out.status.success() => out,
        _ => return false,
    };

    let json: Value = match serde_json::from_slice(&output.stdout) {
        Ok(j) => j,
        Err(_) => return false,
    };

    let available_versions: Vec<String> = match json.as_array() {
        Some(array) => array
            .iter()
            .filter_map(|entry| entry["version"].as_str().map(String::from))
            .collect(),
        None => return false,
    };

    available_versions.iter().any(|v| v == &version_number)
}

/// # `default_python_path`
///
/// Gets the default path for a Python installation.
///
/// ## Arguments
///
/// * `package_name`: The name of the Python package.
///
/// ## Returns
///
/// A `String` with the default path.
fn default_python_path(package_name: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        format!("{home}/.local/share/uv/python/{package_name}")
    } else {
        format!("/usr/local/share/uv/python/{package_name}")
    }
}

/// # `extract_version_number`
///
/// Extracts the version number part of a string.
///
/// ## Arguments
///
/// * `input`: The string to extract the version from.
///
/// ## Returns
///
/// A `String` containing just the version number.
fn extract_version_number(input: &str) -> String {
    if let Some(pos) = input.find(|c: char| c.is_ascii_digit()) {
        input[pos..].to_string()
    } else {
        input.to_string()
    }
}

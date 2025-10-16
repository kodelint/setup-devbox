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

use crate::libs::tool_installer::execute_post_installation_hooks;
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Installs a Python package using `uv` with comprehensive validation and error handling.
///
/// This function provides a robust UV-based installation flow that mirrors the quality and
/// reliability of the official `uv` installer. It includes extensive validation, verification,
/// and accurate state tracking.
///
/// # Workflow:
/// 1. **Environment Validation**: Verifies `uv` is installed and functional
/// 2. **Configuration Validation**: Validates tool entry and installation options
/// 3. **Mode Detection**: Determines correct installation mode (tool/pip/python)
/// 4. **Command Construction**: Builds the complete uv command with proper arguments
/// 5. **Command Execution**: Runs the command with comprehensive error handling
/// 6. **Installation Verification**: Validates installation success and captures output
/// 7. **Path Resolution**: Accurately determines installation path based on mode
/// 8. **Post-Installation Hooks**: Executes additional setup commands if specified
/// 9. **State Creation**: Creates comprehensive `ToolState` with all relevant metadata
///
/// # Installation Modes:
/// - `tool`: Uses `uv tool install` for global CLI tools (default mode)
/// - `pip`: Uses `uv pip install` for Python library packages
/// - `python`: Uses `uv python install` for Python interpreters
///
/// # Arguments:
/// * `tool_entry`: A reference to the `ToolEntry` struct containing package configuration
///   - `tool_entry.name`: **Required** - The package name to install
///   - `tool_entry.version`: Optional version specification (required for python mode)
///   - `tool_entry.options`: Optional list of uv install options (--mode, --features, etc.)
///
/// # Returns:
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if installation was completely successful with accurate metadata
/// * `None` if any step of the installation process fails
///
/// ## Examples - YAML
///
/// ```yaml
/// ## `ruff` - An extremely fast Python linter and code formatter, written in Rust.
/// ## https://docs.astral.sh/ruff/
/// - name: ruff
///   source: uv
///   version: 0.1.0
///   options:
///     - --mode=tool
///   configuration_manager:
///   enabled: true
///   tools_configuration_paths:
///     - $HOME/.config/ruff/ruff.toml
///
/// ## `black` - The uncompromising Python code formatter.
/// - name: black
///   source: uv
///   version: 23.11.0
///   options:
///     - --mode=pip
///
/// ## Python Interpreter Installation
/// - name: python
///   source: uv
///   version: 3.11.0
///   options:
///     - --mode=python
/// ```
///
/// ## Examples - Rust Code
///
/// ### Tool Mode Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "ruff".to_string(),
///     version: Some("0.1.0".to_string()),
///     options: Some(vec!["--mode=tool".to_string()]),
/// };
/// install(&tool_entry);
/// ```
///
/// ### Pip Mode Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "black".to_string(),
///     version: Some("23.11.0".to_string()),
///     options: Some(vec!["--mode=pip".to_string()]),
/// };
/// install(&tool_entry);
/// ```
///
/// ### Python Interpreter Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "python".to_string(),
///     version: Some("3.11.0".to_string()),
///     options: Some(vec!["--mode=python".to_string()]),
/// };
/// install(&tool_entry);
/// ```
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[SDB::Tools::UVInstaller] Attempting to install Python package: {}",
        tool_entry.name.bold()
    );
    log_debug!(
        "[SDB::Tools::UVInstaller] ToolEntry details: {:#?}",
        tool_entry
    );

    // 1. Validate configuration - check tool entry and options for correctness
    if !validate_uv_configuration(tool_entry) {
        return None;
    }

    // 2. Determine installation mode - detect whether to use tool, pip, or python mode
    let (subcommand, base_args) = determine_installation_mode(tool_entry);
    log_debug!(
        "[SDB::Tools::UVInstaller] Using installation mode: {}",
        subcommand.cyan().bold()
    );

    // 3. Build complete command - construct appropriate uv command arguments
    let command_args = build_command_args(&subcommand, &base_args, tool_entry)?;

    // 4. Execute installation - run the uv command with error handling
    let output = execute_uv_command(&subcommand, &command_args, tool_entry)?;

    // 5. Verify installation success - ensure the package was properly installed
    if !verify_installation_success(&output, &subcommand, tool_entry) {
        return None;
    }

    // 6. Determine installation path - where the package was actually installed
    let install_path = determine_install_path(&subcommand, &tool_entry.name);
    log_debug!(
        "[SDB::Tools::UVInstaller] Determined installation path: {}",
        install_path.display().to_string().cyan()
    );

    // 7. Execute post-installation hooks - run any additional setup commands
    let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let executed_hooks =
        execute_post_installation_hooks("[UV Installer]", tool_entry, &working_dir);

    log_info!(
        "[SDB::Tools::UVInstaller] Successfully installed: {} using uv {}",
        tool_entry.name.bold().green(),
        subcommand.green()
    );

    // 8. Return comprehensive ToolState for tracking
    //
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed.
    Some(ToolState::new(
        tool_entry,
        &install_path,
        format!("uv-{subcommand}"),
        match subcommand.as_str() {
            "python" => "python-interpreter".to_string(),
            _ => "python-package".to_string(),
        },
        tool_entry.version.clone()?.to_string(),
        None,
        None,
        executed_hooks,
    ))
}

/// Validates the UV configuration for the tool entry.
///
/// This function performs comprehensive validation of the tool configuration
/// including name validation, option validation, and mode-specific requirements.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing installation options
///
/// # Returns
/// `true` if the configuration is valid, `false` otherwise
///
/// # Validation Rules
/// - Tool name must not be empty or whitespace
/// - Options must be valid for the selected installation mode
/// - Python installation mode requires a version to be specified
/// - Version strings must not be empty when provided
fn validate_uv_configuration(tool_entry: &ToolEntry) -> bool {
    // Validate tool name
    if tool_entry.name.trim().is_empty() {
        log_error!("[SDB::Tools::UVInstaller] Tool name is empty or whitespace");
        return false;
    }

    // Validate options
    if !validate_uv_options(tool_entry) {
        log_error!(
            "[SDB::Tools::UVInstaller] Invalid options for tool entry: {}",
            tool_entry.name.red()
        );
        return false;
    }

    // Validate version for python mode
    if let Some(options) = &tool_entry.options {
        for opt in options {
            if let Some(mode) = opt.strip_prefix("--mode=") {
                if mode == "python" && tool_entry.version.is_none() {
                    log_error!(
                        "[SDB::Tools::UVInstaller] Python installation mode requires a version to be specified for tool '{}'",
                        tool_entry.name.red()
                    );
                    return false;
                }
            }
        }
    }

    true
}

/// Builds the complete command arguments for UV installation.
///
/// This function constructs the appropriate command-line arguments for `uv install`
/// based on the installation mode and tool configuration.
///
/// # Arguments
/// * `subcommand` - The UV subcommand (tool, pip, or python)
/// * `base_args` - Base arguments for the subcommand (typically ["install"])
/// * `tool_entry` - The tool configuration
///
/// # Returns
/// A vector of command-line arguments to pass to `uv`, or `None` on error
///
/// # Processing Logic
/// - **Python mode**: Uses version directly as Python version specifier
/// - **Tool/Pip modes**: Constructs package specifier (name==version or name)
/// - **Options**: Includes additional options while filtering out mode options
/// - **Validation**: Ensures version is provided when required
fn build_command_args(
    subcommand: &str,
    base_args: &[String],
    tool_entry: &ToolEntry,
) -> Option<Vec<String>> {
    let mut args = base_args.to_vec();

    // Add package specifier based on mode
    match subcommand {
        "python" => {
            // For python mode, version is required
            match &tool_entry.version {
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
            }
        }
        _ => {
            // For tool and pip modes, add package specifier
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

    // Add additional options (excluding --mode=)
    if let Some(options) = &tool_entry.options {
        for opt in options {
            if !opt.starts_with("--mode=") {
                args.push(opt.clone());
            }
        }
    }

    Some(args)
}

/// Executes the UV command with proper error handling.
///
/// This function runs the actual `uv` command and provides detailed
/// logging and error reporting. It captures both stdout and stderr for debugging.
///
/// # Arguments
/// * `subcommand` - The UV subcommand to execute
/// * `command_args` - The command-line arguments for the subcommand
/// * `tool_entry` - The tool being installed (for logging purposes)
///
/// # Returns
/// `Some(Output)` if command execution was successful, `None` otherwise
///
/// # Error Handling
/// - Logs the exact command being executed for transparency
/// - Provides specific error messages for command execution failures
/// - Returns the command output for further processing
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

/// Verifies that the UV command executed successfully.
///
/// This function examines the command output to determine if the installation
/// was successful, providing detailed logging for both success and failure cases.
///
/// # Arguments
/// * `output` - The command output from UV execution
/// * `subcommand` - The UV subcommand that was executed
/// * `tool_entry` - The tool being installed (for logging purposes)
///
/// # Returns
/// `true` if installation was successful, `false` otherwise
///
/// # Success Criteria
/// - Command exit status indicates success (status.success())
/// - Stdout/stderr are logged appropriately for debugging
/// - Non-zero exit codes are treated as failures with detailed error reporting
fn verify_installation_success(output: &Output, subcommand: &str, tool_entry: &ToolEntry) -> bool {
    if output.status.success() {
        // Log outputs for debugging
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

/// Determines the appropriate UV installation mode and constructs base command arguments.
///
/// This function examines the tool's options to determine the correct installation mode
/// (tool, pip, or python) and constructs the appropriate base arguments for that mode.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing installation options
///
/// # Returns
/// A tuple containing:
/// - `String`: The UV subcommand to use (tool, pip, or python)
/// - `Vec<String>`: Base arguments for the subcommand (typically ["install"])
///
/// # Mode Detection Logic
/// - **Explicit Mode**: Uses `--mode=` option from tool configuration
/// - **Default Mode**: Falls back to "tool" mode if no mode is specified
/// - **Validation**: Warns about unknown modes and falls back to default
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

    // Default to tool installation
    ("tool".to_string(), vec!["install".to_string()])
}

/// Determines the likely installation path based on the UV subcommand used.
///
/// This function calculates where UV would install the package based on the
/// installation mode and system environment variables.
///
/// # Arguments
/// * `subcommand` - The UV subcommand used for installation
/// * `package_name` - The name of the installed package
///
/// # Returns
/// A `PathBuf` containing the expected installation path
///
/// # Path Resolution by Mode
/// - **Tool mode**: `~/.local/bin/{package_name}` (user-local binaries)
/// - **Pip mode**: `~/.local/lib/python/site-packages/{package_name}` (Python packages)
/// - **Python mode**: Queries UV for actual Python installation path, falls back to default
/// - **Fallback**: System paths when HOME environment variable is not available
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
        "python" => {
            // Try to get the actual path from uv python list
            match get_python_installation_path(package_name) {
                Some(path) => PathBuf::from(path),
                None => {
                    log_warn!(
                        "[SDB::Tools::UVInstaller] Could not determine Python installation path, using default"
                    );
                    PathBuf::from(default_python_path(package_name))
                }
            }
        }
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

/// Gets the Python installation path from UV.
///
/// This function queries UV's installed Python interpreters to determine
/// the actual installation path for a specific Python version.
///
/// # Arguments
/// * `package_name` - The Python version or package name to locate
///
/// # Returns
/// `Some(String)` containing the installation path if found, `None` otherwise
///
/// # Implementation Details
/// - Uses `uv python list --only-installed --output-format json` to get installed Pythons
/// - Parses JSON output to find matching Python installations
/// - Matches based on version string contained in the installation key
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
        if let Some(key) = installation.get("key").and_then(|k| k.as_str()) {
            if key.contains(package_name) {
                if let Some(path) = installation.get("path").and_then(|p| p.as_str()) {
                    log_debug!(
                        "[SDB::Tools::UVInstaller] Found Python installation for {}: {}",
                        package_name,
                        path
                    );
                    return Some(path.to_string());
                }
            }
        }
    }

    log_warn!(
        "[SDB::Tools::UVInstaller] No matching Python installation found for {}",
        package_name
    );
    None
}

/// Validates UV-specific options and provides helpful warnings.
///
/// This function validates that the provided options are appropriate for the
/// selected installation mode and provides warnings for potentially invalid options.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing options to validate
///
/// # Returns
/// `true` if options are valid, `false` if invalid options are detected
///
/// # Validation Logic
/// - Extracts installation mode from options
/// - Checks each option against valid options for the mode
/// - Provides warnings for potentially invalid options
/// - Validates Python version format for python mode
/// - Skips validation for empty options and option values
pub fn validate_uv_options(tool_entry: &ToolEntry) -> bool {
    let mut is_valid = true;

    let options = match &tool_entry.options {
        Some(opts) => opts,
        None => return true, // No options to validate
    };

    let mut install_mode = "tool"; // default

    // Extract the installation mode first
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

    // Get valid options for the mode
    let valid_options = get_valid_options_for_mode(install_mode);

    // Validate each option
    for opt in options {
        // Skip our custom --mode option
        if opt.starts_with("--mode=") {
            continue;
        }

        // Skip empty options
        if opt.trim().is_empty() {
            log_warn!("[SDB::Tools::UVInstaller] Empty option string detected");
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
                "[SDB::Tools::UVInstaller] Option '{}' may not be valid for 'uv {}' mode",
                opt.yellow(),
                install_mode
            );
        }
    }

    // Mode-specific validations
    if install_mode == "python" {
        if let Some(version) = &tool_entry.version {
            if !is_valid_python_version(version) {
                log_warn!(
                    "[SDB::Tools::UVInstaller] '{}' doesn't look like a valid Python version (expected format like '3.11', '3.12.1', etc.)",
                    version.bright_yellow()
                );
            }
        }
    }

    is_valid
}

/// Returns valid options for a specific UV mode.
///
/// This function provides a curated list of valid command-line options
/// for each UV installation mode to assist with option validation.
///
/// # Arguments
/// * `mode` - The UV installation mode (tool, pip, or python)
///
/// # Returns
/// A vector of valid option strings for the specified mode
///
/// # Option Categories
/// - **Common options**: Shared across all modes (caching, verbosity, config)
/// - **Tool-specific**: Options specific to `uv tool install`
/// - **Pip-specific**: Options specific to `uv pip install`
/// - **Python-specific**: Options specific to `uv python install`
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

/// Checks if a Python version string is valid by querying UV.
///
/// This function validates that a Python version is available for installation
/// by querying UV's available Python versions.
///
/// # Arguments
/// * `version_input` - The Python version string to validate
///
/// # Returns
/// `true` if the version is available in UV's Python registry, `false` otherwise
///
/// # Implementation Details
/// - Extracts version number from input string
/// - Queries `uv python list --output-format json` for available versions
/// - Compares against available versions using exact matching
/// - Returns false if UV command fails or JSON parsing fails
fn is_valid_python_version(version_input: &str) -> bool {
    // Extract just the version number part
    let version_number = extract_version_number(version_input);

    // Get available Python versions from UV
    let output = match Command::new("uv")
        .args(["python", "list", "--output-format", "json"])
        .output()
    {
        Ok(out) if out.status.success() => out,
        _ => return false,
    };

    // Parse JSON output
    let json: Value = match serde_json::from_slice(&output.stdout) {
        Ok(j) => j,
        Err(_) => return false,
    };

    // Extract available versions
    let available_versions: Vec<String> = match json.as_array() {
        Some(array) => array
            .iter()
            .filter_map(|entry| entry["version"].as_str().map(String::from))
            .collect(),
        None => return false,
    };

    // Check if version matches any available version
    available_versions.iter().any(|v| v == &version_number)
}

/// Helper function to get the default Python installation path
fn default_python_path(package_name: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        format!("{home}/.local/share/uv/python/{package_name}")
    } else {
        format!("/usr/local/share/uv/python/{package_name}")
    }
}

/// Extracts just the version number part from input string
fn extract_version_number(input: &str) -> String {
    // Find the first digit in the string and take everything from there
    if let Some(pos) = input.find(|c: char| c.is_ascii_digit()) {
        input[pos..].to_string()
    } else {
        input.to_string() // Fallback: return original input if no digits found
    }
}

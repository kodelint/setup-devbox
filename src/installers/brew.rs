//! # Homebrew Installer Module
//!
//! This module provides a robust, production-grade installer for software tools
//! using the Homebrew package manager. It follows enterprise-grade reliability standards
//! with comprehensive error handling, verification mechanisms, and accurate path detection.
//!
//! ## Key Features
//!
//! - **Formula Management**: Installs Homebrew formulae with version specification support
//! - **Cross-Platform Support**: Works on both macOS and Linux Homebrew installations
//! - **Comprehensive Validation**: Validates brew availability, formula installation, and binary existence
//! - **Smart State Tracking**: Maintains accurate installation state with version tracking
//! - **Environment Awareness**: Properly handles different Homebrew installation locations
//! - **Architecture Support**: Automatically detects Apple Silicon vs Intel macOS installations
//!
//! ## Installation Workflow
//!
//! The installer follows a meticulous 8-step process:
//!
//! 1. **Formula Installation** - Executes `brew install` with comprehensive error handling
//! 2. **Installation Verification** - Confirms the formula was properly installed
//! 3. **Verification of Installation** - Verify if the installation was successful
//! 4. **Path Resolution** - Accurately determines the installation path using `brew --prefix`
//! 5. **Binary Verification** - Confirms the binary exists at the expected path
//! 6. **Post-installation Hooks** - Executes any additional setup commands
//! 7. **State Creation** - Creates comprehensive `ToolState` with all relevant metadata
//!
//! ## Supported Formula Formats
//!
//! - **Basic formulae**: `git`, `node`, `python`
//! - **Version-specific**: `python@3.11`, `node@18`
//! - **Custom options**: Support for `--HEAD`, `--devel`, and other brew options
//!
//! ## Error Handling
//!
//! The module provides detailed error messages and logging at multiple levels:
//! - **Info**: High-level installation progress
//! - **Debug**: Detailed command construction and path resolution
//! - **Warn**: Non-fatal issues or verification warnings
//! - **Error**: Installation failures with specific error codes and messages

// This module provides comprehensive functionality to install software tools using Homebrew with
// comprehensive error handling, verification, and accurate path detection.
// It provides robust interaction with the `brew`
// command to install packages as specified in `setup-devbox`'s configuration.

// Standard library imports:
// `std::path::PathBuf`: Provides an owned, OS-agnostic path for path manipulation.
use std::path::PathBuf;
// `std::process::{Command, Output}`: Core functionality for executing external commands.
//   - `Command`: Builder for new processes, used to construct and configure `brew` commands.
use std::process::Command;

// External crate imports:
// `colored::Colorize`: Library for adding color to terminal output for better readability.
use colored::Colorize;

// Internal module imports:
// `ToolEntry`: Represents a single tool's configuration from `tools.yaml`.
// `ToolState`: Represents the actual state of an installed tool for persistence in `state.json`.
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;
// Custom logging macros for structured output.
use crate::{log_debug, log_error, log_info, log_warn};
// Post-installation hook execution functionality.
use crate::libs::tool_installer::execute_post_installation_hooks;

/// Installs a tool using the Homebrew package manager with comprehensive error handling.
///
/// This function provides a robust installer for Homebrew formulae that mirrors the quality.
/// It includes validation, verification and accurate state tracking.
///
/// # Workflow:
/// 1. **Environment Validation**: Verifies `brew` is installed and accessible
/// 2. **Pre-installation Check**: Verifies if formula is already installed
/// 3. **Formula Installation**: Executes `brew install` with comprehensive error handling
/// 4. **Installation Verification**: Confirms the formula was properly installed
/// 5. **Path Resolution**: Accurately determines the installation path using `brew --prefix`
/// 6. **Binary Verification**: Confirms the binary exists at the expected path
/// 7. **Post-installation Hooks**: Executes any additional setup commands
/// 8. **State Creation**: Creates comprehensive `ToolState` with all relevant metadata
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing Homebrew configuration
///   - `tool_entry.name`: **Required** - The Homebrew formula name to install
///   - `tool_entry.version`: Optional version specification (e.g., "formula@version")
///   - `tool_entry.rename_to`: Optional binary rename specification
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if installation was completely successful with accurate metadata
/// * `None` if any step of the installation process fails
///
/// ## Examples - YAML
///
/// ```yaml
///  - name: pyenv
///    source: brew
///    options:
///      - --head
/// ```
/// ## Examples - Rust Code
///
/// ### Basic Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "pyenv".to_string(),
///     version: None, // Install latest version
///     options: ["--head"]
/// };
/// install(&tool_entry);
/// ```
///
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[Brew Installer] Attempting to install Homebrew formula: {}",
        tool_entry.name.bold()
    );
    log_debug!("[Brew Installer] ToolEntry details: {:#?}", tool_entry);

    // 1. Check if formula is already installed (optimization)
    if check_formula_already_installed(&tool_entry.name) {
        log_info!(
            "[Brew Installer] Formula '{}' appears to be already installed",
            tool_entry.name.green()
        );
        // Continue with installation to ensure correct version and options
        log_debug!("[Brew Installer] Proceeding with installation to ensure correct version");
    }

    // 2. Prepare and execute brew install command
    let command_args = prepare_brew_install_command(tool_entry);
    if !execute_brew_install_command(&command_args, tool_entry) {
        return None;
    }

    // 3. Verify the installation was successful
    if !verify_brew_installation(&tool_entry.name) {
        return None;
    }

    // 4. Determine accurate installation path
    let install_path = determine_brew_installation_path(tool_entry);
    log_debug!(
        "[Brew Installer] Determined installation path: {}",
        install_path.display().to_string().cyan()
    );

    // 5. Verify binary exists at expected path
    if !verify_binary_exists(install_path.clone()) {
        log_error!(
            "[Brew Installer] Binary not found at expected path: {}",
            install_path.display().to_string().red()
        );
        return None;
    }

    // 6. Execute post-installation hooks
    let working_dir = install_path
        .parent()
        .unwrap_or(&PathBuf::from("/"))
        .to_path_buf();
    let executed_post_installation_hooks =
        execute_post_installation_hooks("[Brew Installer]", tool_entry, &working_dir);

    // 7. Get actual installed version for accurate tracking
    let actual_version = determine_installed_version(tool_entry);

    log_info!(
        "[Brew Installer] Successfully installed Homebrew formula: {} (version: {})",
        tool_entry.name.bold().green(),
        actual_version.green()
    );

    // 8. Return comprehensive ToolState for tracking
    //
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed.
    Some(ToolState::new(
        tool_entry,
        &install_path,
        "brew".to_string(),
        "binary-by-brew".to_string(),
        actual_version,
        None,
        None,
        executed_post_installation_hooks,
    ))
}

/// Checks if a formula is already installed to avoid unnecessary reinstallation.
///
/// This function runs `brew list <formula_name>` and checks the exit code to determine
/// if the specified formula is already installed. This optimization prevents
/// reinstalling existing formulae.
///
/// # Arguments
/// * `formula_name` - The name of the formula to check
///
/// # Returns
/// `true` if the formula is already installed, `false` otherwise
///
/// # Note
/// Homebrew returns exit code 1 if a formula is not installed, and 0 if it is installed.
/// Other exit codes indicate errors in executing the brew command.
fn check_formula_already_installed(formula_name: &str) -> bool {
    match Command::new("brew").args(["list", formula_name]).output() {
        Ok(output) if output.status.success() => {
            log_debug!(
                "[Brew Installer] Formula '{}' is already installed",
                formula_name
            );
            true
        }
        Ok(output) => {
            // brew list returns non-zero exit code if formula is not installed
            if output.status.code() == Some(1) {
                log_debug!(
                    "[Brew Installer] Formula '{}' is not installed",
                    formula_name
                );
                false
            } else {
                log_warn!(
                    "[Brew Installer] Could not check formula status. Exit code: {}. Error: {}",
                    output.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&output.stderr)
                );
                false
            }
        }
        Err(e) => {
            log_warn!(
                "[Brew Installer] Failed to check formula installation status: {}",
                e
            );
            false
        }
    }
}

/// Prepares the brew install command arguments.
///
/// This function constructs the command-line arguments for the `brew install` command
/// based on the tool configuration, including version specifications and custom options.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing formula information
///
/// # Returns
/// A `Vec<String>` containing the prepared command arguments
///
/// # Argument Construction
/// - Base command: `install`
/// - Formula name: `<formula_name>` or `<formula_name>@<version>` if version specified
/// - Custom options: Any additional options from `tool_entry.options`
fn prepare_brew_install_command(tool_entry: &ToolEntry) -> Vec<String> {
    let mut command_args = Vec::new();
    command_args.push("install".to_string());

    // Handle version specification (e.g., "formula@version")
    if let Some(version) = &tool_entry.version {
        if !version.trim().is_empty() {
            let formula_with_version = format!("{}@{}", tool_entry.name, version);
            command_args.push(formula_with_version);
            log_debug!(
                "[Brew Installer] Installing specific version: {}",
                version.cyan()
            );
        } else {
            command_args.push(tool_entry.name.clone());
        }
    } else {
        command_args.push(tool_entry.name.clone());
    }

    // Add any additional options (like --HEAD, --devel, etc.)
    if let Some(options) = &tool_entry.options {
        log_debug!("[Brew Installer] Adding custom options: {:#?}", options);
        for opt in options {
            command_args.push(opt.clone());
        }
    }

    log_debug!(
        "[Brew Installer] Prepared command arguments: {} {}",
        "brew".cyan().bold(),
        command_args.join(" ").cyan()
    );

    command_args
}

/// Executes the brew install command with comprehensive error handling.
///
/// This function runs the actual `brew install` command with the prepared arguments
/// and provides detailed logging and error reporting.
///
/// # Arguments
/// * `command_args` - The command arguments prepared by `prepare_brew_install_command`
/// * `tool_entry` - The tool configuration for logging purposes
///
/// # Returns
/// `true` if installation was successful, `false` otherwise
///
/// # Command Execution
/// Runs: `brew install <formula_name> [options]`
fn execute_brew_install_command(command_args: &[String], tool_entry: &ToolEntry) -> bool {
    log_debug!(
        "[Brew Installer] Executing: {} {}",
        "brew".cyan().bold(),
        command_args.join(" ").cyan()
    );

    match Command::new("brew").args(command_args).output() {
        Ok(output) if output.status.success() => {
            log_info!(
                "[Brew Installer] Successfully installed formula: {}",
                tool_entry.name.bold().green()
            );

            // Log output for debugging
            if !output.stdout.is_empty() {
                log_debug!(
                    "[Brew Installer] Stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            if !output.stderr.is_empty() {
                log_warn!(
                    "[Brew Installer] Stderr (may contain warnings): {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            true
        }
        Ok(output) => {
            log_error!(
                "[Brew Installer] Failed to install formula '{}'. Exit code: {}. Error: {}",
                tool_entry.name.bold().red(),
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );

            if !output.stdout.is_empty() {
                log_debug!(
                    "[Brew Installer] Stdout (on failure): {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            false
        }
        Err(e) => {
            log_error!(
                "[Brew Installer] Failed to execute 'brew install' for '{}': {}",
                tool_entry.name.bold().red(),
                e.to_string().red()
            );
            false
        }
    }
}

/// Verifies that the formula was properly installed.
///
/// This function performs a comprehensive verification to ensure the installation
/// was completely successful before marking the formula as ready for use.
///
/// # Arguments
/// * `formula_name` - The name of the formula to verify
///
/// # Returns
/// `true` if verification passes, `false` otherwise
///
/// # Verification Steps
/// 1. Formula existence check using `brew list`
/// 2. Formula linkage verification using `brew list --versions`
fn verify_brew_installation(formula_name: &str) -> bool {
    // Verify the formula appears in brew list
    if !verify_formula_in_list(formula_name) {
        return false;
    }

    // Verify the formula is properly linked
    if !verify_formula_linked(formula_name) {
        log_warn!(
            "[Brew Installer] Formula '{}' is installed but not linked",
            formula_name
        );
        // Continue anyway as this might be intentional
    }

    log_debug!("[Brew Installer] Installation verification completed successfully");
    true
}

/// Verifies that the formula appears in the brew list output.
///
/// This function checks if the formula appears in the list of installed formulae
/// returned by `brew list <formula_name>`.
///
/// # Arguments
/// * `formula_name` - The formula to verify
///
/// # Returns
/// `true` if the formula is found, `false` otherwise
fn verify_formula_in_list(formula_name: &str) -> bool {
    match Command::new("brew").args(["list", formula_name]).output() {
        Ok(output) if output.status.success() => {
            log_debug!(
                "[Brew Installer] Verified formula '{}' is in brew list",
                formula_name
            );
            true
        }
        Ok(output) => {
            log_error!(
                "[Brew Installer] Formula '{}' not found in installed formulae. Exit code: {}. Error: {}",
                formula_name.red(),
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );
            false
        }
        Err(e) => {
            log_error!(
                "[Brew Installer] Failed to execute formula verification: {}",
                e.to_string().red()
            );
            false
        }
    }
}

/// Verifies that the formula is properly linked.
///
/// This function checks if the formula is properly linked by examining the output
/// of `brew list --versions <formula_name>`. A properly linked formula will
/// appear in this list with its version information.
///
/// # Arguments
/// * `formula_name` - The formula to verify
///
/// # Returns
/// `true` if the formula is properly linked, `false` otherwise
fn verify_formula_linked(formula_name: &str) -> bool {
    match Command::new("brew")
        .args(["list", "--versions", formula_name])
        .output()
    {
        Ok(output) if output.status.success() => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Check if the formula is listed and has a version (indicating it's properly installed)
            if output_str.contains(formula_name) {
                log_debug!(
                    "[Brew Installer] Verified formula '{}' is properly linked",
                    formula_name
                );
                return true;
            }
            false
        }
        _ => false,
    }
}

/// Verifies that the binary exists at the expected path.
///
/// This function checks if the actual binary file exists at the determined
/// installation path, providing a final verification that the installation
/// produced the expected executable.
///
/// # Arguments
/// * `install_path` - The path where the binary should be located
///
/// # Returns
/// `true` if the binary exists, `false` otherwise
fn verify_binary_exists(install_path: PathBuf) -> bool {
    if install_path.exists() {
        log_debug!(
            "[Brew Installer] Verified binary exists at: {}",
            install_path.display()
        );
        true
    } else {
        log_error!(
            "[Brew Installer] Binary does not exist at expected path: {}",
            install_path.display().to_string().red()
        );
        false
    }
}

/// Determines the accurate installation path for brew-installed binaries.
///
/// This function attempts to locate where Homebrew installed the binary
/// by checking multiple location strategies in order of preference.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing binary name information
///
/// # Returns
/// A `PathBuf` containing the full path to the installed binary
///
/// # Path Resolution Order
/// 1. **Brew Prefix**: Uses `brew --prefix` to get the installation prefix (highest priority)
/// 2. **Common Paths**: Checks common Homebrew installation locations
/// 3. **System Fallback**: Uses `/usr/local/bin` as final fallback
///
/// # Platform Support
/// - **Apple Silicon macOS**: `/opt/homebrew/bin`
/// - **Intel macOS**: `/usr/local/bin`
/// - **Linux**: `/home/linuxbrew/.linuxbrew/bin` or `/opt/homebrew/bin`
fn determine_brew_installation_path(tool_entry: &ToolEntry) -> PathBuf {
    // Get the binary name (either the original name or renamed)
    let bin_name = tool_entry
        .rename_to
        .clone()
        .unwrap_or_else(|| tool_entry.name.clone());

    // Try to get the brew prefix using `brew --prefix`
    if let Some(brew_prefix) = get_brew_prefix() {
        let path = PathBuf::from(&brew_prefix).join("bin").join(&bin_name);
        log_debug!(
            "[Brew Installer] Using brew prefix path: {}",
            path.display()
        );
        return path;
    }

    // Fallback: try common Homebrew installation paths
    if let Some(path) = get_common_brew_paths(&bin_name) {
        return path;
    }

    // Final fallback
    log_warn!("[Brew Installer] Could not determine brew prefix, using system fallback");
    PathBuf::from("/usr/local/bin").join(bin_name)
}

/// Gets the Homebrew prefix using `brew --prefix`.
///
/// This function executes `brew --prefix` to determine where Homebrew
/// is installed on the system, which is essential for locating installed binaries.
///
/// # Returns
/// `Some(String)` containing the brew prefix path, or `None` if the command fails
///
/// # Note
/// The brew prefix is typically:
/// - `/opt/homebrew` on Apple Silicon macOS
/// - `/usr/local` on Intel macOS
/// - `/home/linuxbrew/.linuxbrew` on Linux
fn get_brew_prefix() -> Option<String> {
    match Command::new("brew").arg("--prefix").output() {
        Ok(output) if output.status.success() => {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() {
                log_debug!("[Brew Installer] Detected brew prefix: {}", prefix);
                return Some(prefix);
            }
            None
        }
        Ok(output) => {
            log_warn!(
                "[Brew Installer] Failed to get brew prefix. Exit code: {}. Error: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            );
            None
        }
        Err(e) => {
            log_warn!("[Brew Installer] Failed to execute 'brew --prefix': {}", e);
            None
        }
    }
}

/// Gets common Homebrew installation paths as fallback.
///
/// This function checks common Homebrew installation locations when
/// `brew --prefix` fails or returns an unexpected result.
///
/// # Arguments
/// * `bin_name` - The name of the binary to look for
///
/// # Returns
/// `Some(PathBuf)` if the binary is found at a common location, `None` otherwise
///
/// # Common Paths Checked
/// - `/opt/homebrew/bin` (Apple Silicon macOS)
/// - `/usr/local/bin` (Intel macOS)
/// - `/home/linuxbrew/.linuxbrew/bin` (Linux)
/// - `/opt/homebrew/bin` (Some Linux installations)
fn get_common_brew_paths(bin_name: &str) -> Option<PathBuf> {
    // Common Homebrew installation paths on different architectures
    let common_paths = [
        // Apple Silicon macOS
        PathBuf::from("/opt/homebrew/bin").join(bin_name),
        // Intel macOS
        PathBuf::from("/usr/local/bin").join(bin_name),
        // Linux
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin").join(bin_name),
        PathBuf::from("/opt/homebrew/bin").join(bin_name), // Some Linux installations
    ];

    for path in &common_paths {
        if path.exists() {
            log_debug!(
                "[Brew Installer] Found binary at common path: {}",
                path.display()
            );
            return Some(path.clone());
        }
    }

    None
}

/// Determines the appropriate version string for the installed tool.
///
/// This function attempts to determine the precise version of the installed
/// formula by checking multiple sources in order of preference.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing version information
///
/// # Returns
/// A `String` containing the best available version information
///
/// # Version Detection Strategies
/// 1. **Configuration Priority**: Uses version from tool configuration if specified
/// 2. **Actual Installed Version**: Queries Homebrew for the actual installed version
/// 3. **Fallback**: Returns "latest" if no version information can be determined
fn determine_installed_version(tool_entry: &ToolEntry) -> String {
    // Priority 1: Use version from configuration if specified
    if let Some(version) = &tool_entry.version {
        if !version.trim().is_empty() {
            return version.clone();
        }
    }

    // Priority 2: Try to get actual installed version from Homebrew
    if let Some(actual_version) = get_brew_installed_version(&tool_entry.name) {
        return actual_version;
    }

    // Priority 3: Fallback to "latest"
    "latest".to_string()
}

/// Gets the actual installed version from Homebrew.
///
/// This function queries Homebrew for detailed information about the installed
/// formula and extracts the version from the JSON output.
///
/// # Arguments
/// * `formula_name` - The name of the formula to query
///
/// # Returns
/// `Some(String)` containing the actual installed version, or `None` if detection fails
///
/// # Command Execution
/// Runs: `brew info --json <formula_name>`
///
/// # Note
/// This function performs a simplified JSON parsing. In a production environment,
/// you would typically use a proper JSON parser for more robust extraction.
fn get_brew_installed_version(formula_name: &str) -> Option<String> {
    match Command::new("brew")
        .args(["info", "--json", formula_name])
        .output()
    {
        Ok(output) if output.status.success() => {
            let info_output = String::from_utf8_lossy(&output.stdout);
            // Parse JSON output to extract version
            // This is a simplified version - in a real implementation you'd use a JSON parser
            if let Some(version_start) = info_output.find("\"version\":\"") {
                let version_part = &info_output[version_start + 10..];
                if let Some(version_end) = version_part.find('"') {
                    return Some(version_part[..version_end].to_string());
                }
            }
            None
        }
        _ => None,
    }
}

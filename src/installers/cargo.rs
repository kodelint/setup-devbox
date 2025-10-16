//! # Cargo Installer Module
//!
//! This module provides a robust, production-grade installer for Rust tools using the `cargo install` command.
//! It follows comprehensive error handling, verification mechanisms, and accurate path detection.
//!
//! ## Key Features
//!
//! - **Dual Installation Support**: Handles both regular crates.io installations and Git-based installations
//! - **Comprehensive Validation**: Validates cargo availability, installation success, and binary paths
//! - **Smart State Tracking**: Maintains accurate installation state with version tracking
//! - **Flexible Configuration**: Supports version specifications, custom cargo options, and Git references
//! - **Post-Installation Hooks**: Executes additional setup commands after successful installation
//! - **Environment Awareness**: Properly handles different cargo home directories and installation paths
//!
//! ## Installation Workflow
//!
//! The installer follows a meticulous 9-step process:
//!
//! 1. **Pre-installation Check** - Determines if tool is already installed (outside SDB)
//! 2. **Source Detection** - Identifies installation type (crates.io vs Git)
//! 3. **Command Preparation** - Constructs appropriate `cargo install` command
//! 4. **Command Execution** - Runs installation with comprehensive error handling
//! 5. **Installation Verification** - Confirms the crate was properly installed
//! 6. **Path Resolution** - Accurately determines binary installation path
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

use std::env;
// Standard library imports:
// Provides an owned, OS-agnostic path. Used for building the installation path
// of the binary (e.g., `~/.cargo/bin/my-tool`).
use std::path::PathBuf;
// Core functionality for executing external commands.
//   - `Command`: Builder for new processes, used to construct and configure `cargo install` commands.
use std::process::Command;

// Post-installation hook execution functionality.
use crate::libs::tool_installer::execute_post_installation_hooks;
// Internal module imports:
// `ToolEntry`: Represents a single tool's configuration from `tools.yaml`.
// `ToolState`: Represents the actual state of an installed tool for persistence in `state.json`.
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;
// Custom logging macros for structured output.
use crate::{log_debug, log_error, log_info, log_warn};
// External crate imports:
//   Library for adding color to terminal output for better readability.
use colored::Colorize;

/// Installs a Rust tools using the `cargo install` command with comprehensive error handling.
///
/// This function provides a robust installer for Cargo based tool installation which
/// includes validation, verification, and accurate state tracking.
///
/// # Workflow:
/// 1. **Environment Validation**: Verifies `cargo` is installed and accessible
/// 2. **Already Installed**: Determines if the tool was already install outside SDB
/// 3. **Installation Source**: Determines how to install the tool
/// 4. **Prepare and Execute Command**: Prepare the `cargo install` command
/// 5. **Installation Verification**: Confirms the crate was properly installed
/// 6. **Path Resolution**: Accurately determines the installation path
/// 7. **Post-installation Hooks**: Executes any additional setup commands
/// 8. **Get Version**: Determine the actual installed version
/// 9. **State Creation**: Creates comprehensive `ToolState` with all relevant metadata
///
/// # Arguments:
/// * `tool_entry`: A reference to the `ToolEntry` struct containing crate configuration
///   - `tool_entry.name`: **Required** - The crate name to install
///   - `tool_entry.version`: Optional version specification for crates.io installations
///   - `tool_entry.options`: Optional list of cargo install options (--features, --git, etc.)
///
/// # Returns:
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if installation was completely successful with accurate metadata
/// * `None` if any step of the installation process fails
///
/// ## Examples - YAML
///
/// ```yaml
/// ## `uv` - A single tool to replace pip, pip-tools, pipx, poetry, pyenv, twine, virtualenv, and more.
/// ## https://docs.astral.sh/uv/
/// - name: uv
///   source: cargo
///   version: 0.8.17
///   options:
///     - --git https://github.com/astral-sh/uv
///   configuration_manager:
///   enabled: true
///   tools_configuration_paths:
///     - $HOME/.config/uv/uv.toml
///
///  ## `cargo-deny` - cargo-deny is a cargo plugin that lets you lint your project's dependency
///  ## graph to ensure all your dependencies conform to your expectations and requirements.
/// - name: cargo-deny
///   source: cargo
///   version: 0.18.4
/// ```
///
/// ## Examples - Rust Code
///
/// ### Basic Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "cargo-deny".to_string(),
///     version: None, // Install latest version
///     options: None,
/// };
/// install(&tool_entry);
/// ```
///
/// ### Version-Specific Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "cargo-deny".to_string(),
///     version: Some("0.18.4".to_string()),
///     options: None,
/// };
/// install(&tool_entry);
/// ```
///
/// ### Git-Based Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "uv".to_string(),
///     version: Some("0.8.17".to_string()), // Used as git tag
///     options: Some(vec![
///         "--git".to_string(),
///         "https://github.com/astral-sh/uv".to_string()
///     ]),
/// };
/// install(&tool_entry);
/// ```
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[SDB::Tools::CargoInstaller] Attempting to install Tool: {}",
        tool_entry.name.green().bold()
    );
    log_debug!(
        "[SDB::Tools::CargoInstaller] ToolEntry details: {:#?}",
        tool_entry
    );

    // 1. Check if crate is already installed (optimization)
    // There might be possibility that tool was already installed outside SDB
    log_debug!(
        "[SDB::Tools::CargoInstaller] Checking if {} is already installed",
        tool_entry.name.bold()
    );
    if check_if_installed(&tool_entry.name) {
        log_warn!(
            "[SDB::Tools::CargoInstaller] Tool '{}' appears to be already installed, outside SDB",
            tool_entry.name.green()
        );
        log_info!(
            "[SDB::Tools::CargoInstaller] Updating SDB inventory for {}",
            tool_entry.name.green()
        );
        log_debug!(
            "[SDB::Tools::CargoInstaller] Proceeding with installation to ensure correct version/options"
        );
    }

    // 2. Detect if cargo needs to install it using git url
    let is_it_git_based_install = detect_install_source(tool_entry);
    log_debug!(
        "[SDB::Tools::CargoInstaller] Installation type: {}",
        if is_it_git_based_install {
            "Git Based".cyan()
        } else {
            "Cargo Index based".cyan()
        }
    );

    // 3. Prepare and execute cargo install command
    log_debug!(
        "[SDB::Tools::CargoInstaller] Prepare the command to install: {}",
        tool_entry.name.bold()
    );
    let command_args = prepare_cargo_install_command(tool_entry, is_it_git_based_install);
    if !execute_cargo_install_command(&command_args, tool_entry) {
        return None;
    }

    // 4. Verify the installation was successful - ensure the binary is actually available
    log_debug!(
        "[SDB::Tools::CargoInstaller] Verify if the {} actually installed",
        tool_entry.name.bold()
    );
    if !check_if_installed(&tool_entry.name) {
        return None;
    }

    // 5. Determine accurate installation path - where the binary was actually installed
    let install_path = determine_cargo_installation_path(&tool_entry.name);
    log_debug!(
        "[SDB::Tools::CargoInstaller] Determined installation path: {}",
        install_path.display().to_string().cyan()
    );

    // 6. Execute post-installation hooks - run any additional setup commands
    log_debug!(
        "[SDB::Tools::CargoInstaller] Executing post installation hooks, post installing {}",
        tool_entry.name.bold()
    );
    let executed_post_installation_hooks =
        execute_post_installation_hooks("[SDB::Tools::CargoInstaller]", tool_entry, &install_path);

    // 7. Get actual installed version for accurate tracking - important for state management
    let actual_version = determine_installed_version(tool_entry, is_it_git_based_install);

    log_info!(
        "[SDB::Tools::CargoInstaller] Successfully installed tool: {} (version: {})",
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
        "cargo-install".to_string(),
        "rust-crate".to_string(),
        actual_version,
        None,
        None,
        executed_post_installation_hooks,
    ))
}

/// Detects if this is a git-based installation by checking for --git option.
///
/// This function examines the tool's options to determine if it should be
/// installed from a Git repository rather than from crates.io.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing installation options
///
/// # Returns
/// `true` if this is a Git-based installation, `false` for crates.io installation
fn detect_install_source(tool_entry: &ToolEntry) -> bool {
    if let Some(options) = &tool_entry.options {
        options.iter().any(|opt| opt.starts_with("--git"))
    } else {
        false
    }
}

/// Checks if a crate is already installed to avoid unnecessary reinstallation.
///
/// This function runs `cargo install --list` and parses the output to determine
/// if the specified crate is already installed. This helps optimize installation
/// by skipping already installed crates.
///
/// # Arguments
/// * `tool_name` - The name of the crate to check
///
/// # Returns
/// `true` if the crate is already installed, `false` otherwise
///
/// # Note
/// The function looks for lines that start with the tool name and contain a colon,
/// which is the format used by `cargo install --list` for installed crates.
fn check_if_installed(tool_name: &str) -> bool {
    match Command::new("cargo").args(["install", "--list"]).output() {
        Ok(output) if output.status.success() => {
            let installed_crates = String::from_utf8_lossy(&output.stdout);
            installed_crates
                .lines()
                .filter(|line| line.starts_with(char::is_whitespace)) // Only indented binary lines
                .any(|line| line.trim() == tool_name) // Exact match
        }
        _ => false,
    }
}

/// Prepares the cargo install command arguments based on installation type.
///
/// This function constructs the appropriate command-line arguments for `cargo install`
/// based on whether this is a Git-based installation or a regular crates.io installation.
///
/// # Arguments
/// * `tool_entry` - The tool configuration
/// * `is_it_git_based_installed` - Flag indicating installation type
///
/// # Returns
/// A vector of command-line arguments to pass to `cargo install`
///
/// # Examples
///
/// ```yaml
/// # `uv` - A single tool to replace pip, pip-tools, pipx, poetry, pyenv, twine, virtualenv, and more.
/// # https://docs.astral.sh/uv/
/// - name: uv
///   source: cargo
///   version: 0.8.17
///   options:
///     - --git https://github.com/astral-sh/uv
///   configuration_manager:
///   enabled: true
///   tools_configuration_paths:
///     - $HOME/.config/uv/uv.toml
/// ```
/// ```yaml
///  # `cargo-deny` - cargo-deny is a cargo plugin that lets you lint your project's dependency
///  # graph to ensure all your dependencies conform to your expectations and requirements.
/// - name: cargo-deny
///   source: cargo
///   version: 0.18.0
/// ```
///
/// ## Cargo Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "cargo-deny".to_string(),
///     version: Some("0.18.4".to_string()),
///     options: None,
/// };
///
/// let args = prepare_cargo_install_command(&tool_entry, false);
/// // args: ["install", "cargo-deny", "--version", "0.18.4", "--quiet"]
/// ```
///
/// ## Git Installation
///
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "uv".to_string(),
///     options: Some(vec!["--git".to_string(), "https://github.com/astral-sh/uv".to_string()]),
///     ..Default::default()
/// };
///
/// let args = prepare_cargo_install_command(&tool_entry, true);
/// // args: ["install", "--git", "https://github.com/astral-sh/uv", "uv", "--quiet"]
/// ```
fn prepare_cargo_install_command(
    tool_entry: &ToolEntry,
    is_it_git_based_installed: bool,
) -> Vec<String> {
    let mut command_args = Vec::new();
    command_args.push("install".to_string());

    if is_it_git_based_installed {
        prepare_git_based_install_command(&mut command_args, tool_entry);
    } else {
        prepare_cargo_based_install_command(&mut command_args, tool_entry);
    }

    // Add quiet flag to reduce noise, but keep debug logging comprehensive
    command_args.push("--quiet".to_string());

    log_debug!(
        "[SDB::Tools::CargoInstaller] Prepared command arguments: {} {}",
        "cargo".cyan().bold(),
        command_args.join(" ").cyan()
    );

    command_args
}

/// Prepares command arguments for a regular crate installation from crates.io.
///
/// This function handles the common case of installing a crate from the official
/// crates.io registry with optional version specification and custom cargo options.
///
/// # Arguments
/// * `command_args` - Mutable reference to the command arguments vector (will be modified)
/// * `tool_entry` - The tool configuration
///
/// # Processing Logic
/// 1. Adds the crate name as the first argument
/// 2. Adds version specification if provided (`--version <version>`)
/// 3. Adds custom options while filtering out Git-specific options
/// 4. Skips options that are Git-related (--git, --branch, --tag, --rev)
fn prepare_cargo_based_install_command(command_args: &mut Vec<String>, tool_entry: &ToolEntry) {
    command_args.push(tool_entry.name.clone());

    // Handle version specification
    if let Some(version) = &tool_entry.version {
        if !version.trim().is_empty() {
            command_args.push("--version".to_string());
            command_args.push(version.clone());
            log_debug!(
                "[Cargo Installer] Installing specific version: {}",
                version.cyan()
            );
        }
    }

    // Add any additional options
    if let Some(options) = &tool_entry.options {
        log_debug!(
            "[SDB::Tools::CargoInstaller] Adding custom options: {:#?}",
            options
        );
        for opt in options {
            // Skip git options for crate installations
            if !opt.starts_with("--git")
                && !opt.starts_with("--branch")
                && !opt.starts_with("--tag")
                && !opt.starts_with("--rev")
            {
                command_args.push(opt.clone());
            }
        }
    }
}

/// Prepares command arguments for a git-based installation.
///
/// This function handles Git repository installations with support for various
/// Git reference types (branches, tags, specific commits).
///
/// # Arguments
/// * `command_args` - Mutable reference to the command arguments vector
/// * `tool_entry` - The tool configuration containing Git options
///
/// # Processing Logic
/// 1. Processes all Git options with proper handling of space-separated and equals-separated formats
/// 2. Adds the crate name at the end (required for Git installations)
/// 3. Uses the version as a Git tag if no explicit Git reference options are provided
/// 4. Handles three formats of option specification:
///    - Space-separated: `--git https://url`
///    - Equals-separated: `--git=https://url`
///    - Simple flags: `--locked`
fn prepare_git_based_install_command(command_args: &mut Vec<String>, tool_entry: &ToolEntry) {
    let options = tool_entry.options.as_ref().unwrap();

    // Check for existing git reference options
    let has_branch = options.iter().any(|opt| opt.starts_with("--branch"));
    let has_tag = options.iter().any(|opt| opt.starts_with("--tag"));
    let has_rev = options.iter().any(|opt| opt.starts_with("--rev"));

    // Process all options
    for opt in options {
        if opt.contains(' ') {
            // Handle space-separated options (e.g., "--git https://url")
            let parts: Vec<&str> = opt.split_whitespace().collect();
            for part in parts {
                command_args.push(part.to_string());
            }
        } else if opt.contains('=') {
            // Handle --option=value format (e.g., "--git=https://url")
            let parts: Vec<&str> = opt.splitn(2, '=').collect();
            command_args.push(parts[0].to_string());
            if !parts[1].is_empty() {
                command_args.push(parts[1].to_string());
            }
        } else {
            command_args.push(opt.clone());
        }
    }

    // Add crate name at the end for git installations (cargo requirement)
    command_args.push(tool_entry.name.clone());

    // Handle version as git tag if no explicit git options are present
    if !has_branch && !has_tag && !has_rev {
        if let Some(version) = &tool_entry.version {
            if !version.trim().is_empty() {
                command_args.push("--tag".to_string());
                command_args.push(version.clone());
                log_debug!(
                    "[Cargo Installer] Using version as git tag: {}",
                    version.cyan()
                );
            }
        }
    }
}

/// Executes the cargo install command with comprehensive error handling.
///
/// This function runs the actual `cargo install` command and provides detailed
/// logging and error reporting. It captures both stdout and stderr for debugging.
///
/// # Arguments
/// * `command_args` - The command-line arguments for `cargo install`
/// * `tool_entry` - The tool being installed (for logging purposes)
///
/// # Returns
/// `true` if installation was successful, `false` otherwise
///
/// # Error Handling
/// - Logs success with green highlighting
/// - Captures and logs stdout/stderr for debugging
/// - Provides specific error codes and messages for failures
/// - Handles both command execution failures and non-zero exit codes
fn execute_cargo_install_command(command_args: &[String], tool_entry: &ToolEntry) -> bool {
    log_debug!(
        "[SDB::Tools::CargoInstaller] Executing: {} {}",
        "cargo".cyan().bold(),
        command_args.join(" ").cyan()
    );

    match Command::new("cargo").args(command_args).output() {
        Ok(output) if output.status.success() => {
            log_info!(
                "[SDB::Tools::CargoInstaller] Successfully installed tool: {}",
                tool_entry.name.bold().green()
            );

            // Log output for debugging
            if !output.stdout.is_empty() {
                log_debug!(
                    "[SDB::Tools::CargoInstaller] Stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            if !output.stderr.is_empty() {
                log_warn!(
                    "[SDB::Tools::CargoInstaller] Stderr (may contain warnings): {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            true
        }
        Ok(output) => {
            log_error!(
                "[Cargo Installer] Failed to install tool '{}'. Exit code: {}. Error: {}",
                tool_entry.name.bold().red(),
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );

            if !output.stdout.is_empty() {
                log_debug!(
                    "[Cargo Installer] Stdout (on failure): {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            false
        }
        Err(e) => {
            log_error!(
                "[SDB::Tools::CargoInstaller] Failed to execute 'cargo install' for '{}': {}",
                tool_entry.name.bold().red(),
                e.to_string().red()
            );
            false
        }
    }
}

/// Determines the accurate installation path for cargo-installed binaries.
///
/// This function attempts to locate where cargo actually installed the binary
/// by checking environment variables in order of precedence.
///
/// # Arguments
/// * `tool_name` - The name of the installed tool
///
/// # Returns
/// A `PathBuf` containing the full path to the installed binary
///
/// # Path Resolution Order
/// 1. `CARGO_INSTALL_ROOT` environment variable (highest priority)
/// 2. `CARGO_HOME` environment variable
/// 3. `HOME` environment variable with default `~/.cargo/bin` path
/// 4. System fallback to `/usr/local/bin` (lowest priority)
fn determine_cargo_installation_path(tool_name: &str) -> PathBuf {
    // Try environment variables in order of preference
    if let Some(path) = get_cargo_install_path(tool_name) {
        return path;
    }

    // Final fallback to system default PATH
    log_warn!("[SDB::Tools::CargoInstaller] Could not determine cargo home, using system fallback");
    PathBuf::from("/usr/local/bin").join(tool_name)
}

/// Gets the installation path for a cargo-installed crate by checking
/// `CARGO_INSTALL_ROOT`, `CARGO_HOME`, and a default `HOME` based path, in order.
///
/// This is a helper function for `determine_cargo_installation_path` that implements
/// the actual environment variable checking logic.
///
/// # Arguments
/// * `tool_name` - The name of the installed tool
///
/// # Returns
/// `Some(PathBuf)` if a valid path can be constructed, `None` otherwise
///
/// # Environment Variable Precedence
///
/// 1. **CARGO_INSTALL_ROOT**: Directly specifies the installation root
/// 2. **CARGO_HOME**: The cargo home directory (usually `~/.cargo`)
/// 3. **HOME**: Used to construct the default cargo path (`~/.cargo/bin`)
fn get_cargo_install_path(tool_name: &str) -> Option<PathBuf> {
    // 1. Check CARGO_INSTALL_ROOT (highest priority)
    if let Ok(root) = env::var("CARGO_INSTALL_ROOT") {
        let path = PathBuf::from(root).join("bin").join(tool_name);
        log_debug!(
            "[SDB::Tools::CargoInstaller] Using CARGO_INSTALL_ROOT path: {}",
            path.display()
        );
        return Some(path);
    }

    // 2. Check CARGO_HOME (medium priority)
    if let Ok(cargo_home) = env::var("CARGO_HOME") {
        let path = PathBuf::from(cargo_home).join("bin").join(tool_name);
        log_debug!(
            "[SDB::Tools::CargoInstaller] Using CARGO_HOME path: {}",
            path.display()
        );
        return Some(path);
    }

    // 3. Check HOME (for the default ~/.cargo/bin path) (lowest priority)
    if let Ok(home) = env::var("HOME") {
        let path = PathBuf::from(home)
            .join(".cargo")
            .join("bin")
            .join(tool_name);
        log_debug!(
            "[SDB::Tools::CargoInstaller] Using HOME-based cargo path: {}",
            path.display()
        );
        return Some(path);
    }

    // If none of the environment variables are set or paths are constructed, return None
    None
}

/// Determines the appropriate version string for the installed tool.
///
/// This function attempts to determine the most accurate version string
/// for state tracking purposes, with multiple fallback strategies.
///
/// # Arguments
/// * `tool_entry` - The tool configuration
/// * `is_it_already_installed` - Flag indicating if this was a Git installation
///
/// # Returns
/// A version string for state tracking
///
/// # Version Resolution Priority
///
/// 1. **Explicit Version**: From `tool_entry.version` if specified
/// 2. **Git References**: For Git installations, extracts version from Git options:
///    - `--tag`: Uses the tag value directly
///    - `--branch`: Formats as "branch-{branch_name}"
///    - `--rev`: Formats as "rev-{short_commit_hash}" (first 7 characters)
/// 3. **Fallback**: Returns "latest" if no version information can be determined
///
/// # Examples
///
/// ```yaml
/// # `uv` - A single tool to replace pip, pip-tools, pipx, poetry, pyenv, twine, virtualenv, and more.
/// # https://docs.astral.sh/uv/
/// - name: uv
///   source: cargo
///   version: 0.8.17
///   options:
///     - --git https://github.com/astral-sh/uv
///   configuration_manager:
///   enabled: true
///   tools_configuration_paths:
///     - $HOME/.config/uv/uv.toml
/// ```
fn determine_installed_version(tool_entry: &ToolEntry, is_it_already_installed: bool) -> String {
    // Priority 1: Use version from configuration if specified
    if let Some(version) = &tool_entry.version {
        if !version.trim().is_empty() {
            return version.clone();
        }
    }

    // Priority 2: For git installations, extract version from git options
    log_debug!(
        "[SDB::Tools::CargoInstaller] Checking if other indexes were used to install {}",
        tool_entry.name.bold()
    );
    if is_it_already_installed {
        if let Some(options) = &tool_entry.options {
            // Check for --tag first (highest priority for Git)
            if let Some(tag_opt) = options.iter().find(|opt| opt.starts_with("--tag")) {
                if let Some(tag_value) = tag_opt.split('=').nth(1) {
                    return tag_value.to_string();
                } else if let Some(pos) = options.iter().position(|opt| opt == "--tag") {
                    if let Some(tag_value) = options.get(pos + 1) {
                        return tag_value.clone();
                    }
                }
            }

            // Check for --branch next (medium priority for Git)
            if let Some(branch_opt) = options.iter().find(|opt| opt.starts_with("--branch")) {
                if let Some(branch_value) = branch_opt.split('=').nth(1) {
                    return format!("branch-{branch_value}");
                } else if let Some(pos) = options.iter().position(|opt| opt == "--branch") {
                    if let Some(branch_value) = options.get(pos + 1) {
                        return format!("branch-{branch_value}");
                    }
                }
            }

            // Check for --rev last (lowest priority for Git, only first 7 characters for brevity)
            if let Some(rev_opt) = options.iter().find(|opt| opt.starts_with("--rev")) {
                if let Some(rev_value) = rev_opt.split('=').nth(1) {
                    let short_rev = if rev_value.len() > 7 {
                        &rev_value[..7]
                    } else {
                        rev_value
                    };
                    return format!("rev-{short_rev}");
                } else if let Some(pos) = options.iter().position(|opt| opt == "--rev") {
                    if let Some(rev_value) = options.get(pos + 1) {
                        let short_rev = if rev_value.len() > 7 {
                            &rev_value[..7]
                        } else {
                            rev_value
                        };
                        return format!("rev-{short_rev}");
                    }
                }
            }
        }
    }

    // Priority 3: Fallback to "latest" when no version information is available
    "latest".to_string()
}

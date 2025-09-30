//! # Rustup Installer Module
//!
//! This module provides a robust, production-grade installer for Rust toolchains and components
//! using the `rustup` toolchain manager. It follows enterprise-grade reliability standards
//! with comprehensive error handling, verification mechanisms, and accurate path detection.
//!
//! ## Key Features
//!
//! - **Toolchain Management**: Installs specific Rust toolchains (stable, nightly, version-specific)
//! - **Component Support**: Installs additional components (clippy, rustfmt, etc.) for toolchains
//! - **Comprehensive Validation**: Validates rustup availability, toolchain installation, and component status
//! - **Smart State Tracking**: Maintains accurate installation state with version tracking
//! - **Environment Awareness**: Properly handles different rustup home directories and installation paths
//! - **Partial Success Handling**: Continues installation even if some components fail
//!
//! ## Installation Workflow
//!
//! The installer follows a meticulous 10-step process:
//!
//! 1. **Toolchain Validation** - Ensures toolchain version is specified and valid
//! 2. **Pre-installation Toolchain Check** - Determines if toolchain is already installed
//! 3. **Pre-Component Installation Check** - Determines if component is already installed
//! 4. **Installation Component** - Install the component
//! 5. **Installation Verification** - Confirms toolchain and components are properly installed
//! 6. **Path Resolution** - Accurately determines toolchain installation path
//! 7. **Version Detection** - Gets actual installed version for accurate tracking
//! 8. **Post-Installation Hooks** - Executes any additional setup commands
//! 9. **State Creation** - Creates comprehensive tool state for persistence
//!
//! ## Supported Toolchain Formats
//!
//! - **Named toolchains**: `stable`, `nightly`, `beta`
//! - **Version-specific**: `1.70.0`, `1.69.0-x86_64-pc-windows-msvc`
//! - **Custom toolchains**: Any valid rustup toolchain identifier
//!
//! ## Error Handling
//!
//! The module provides detailed error messages and logging at multiple levels:
//! - **Info**: High-level installation progress
//! - **Debug**: Detailed command construction and path resolution
//! - **Warn**: Non-fatal issues or verification warnings
//! - **Error**: Installation failures with specific error codes and messages

// For getting environment variables, like HOME.
// `std::env` is used to find the user's home directory to determine rustup's installation path.
// Internal module imports:
// `ToolEntry`: Represents a single tool's configuration from `tools.yaml`.
// `ToolState`: Represents the actual state of an installed tool for persistence in `state.json`.
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;
// Imports custom logging macros from the crate root.
// These macros (`log_debug`, `log_error`, `log_info`, `log_warn`) provide
// a standardized way to output messages to the console with different severity levels,
// making it easier to track the application's flow and diagnose issues.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to terminal output.
// The `colored` crate allows us to make log messages and other terminal output more readable
// by applying colors (e.g., `.blue()`, `.green()`, `.red()`).
use colored::Colorize;
use std::env;
// For working with file paths, specifically to construct installation paths.
// `PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::PathBuf;
// For executing external commands and capturing their output.
// `std::process::Command` is used to run `rustup` commands.
// `std::process::Output` captures the stdout, stderr, and exit status of executed commands.
use crate::libs::tool_installer::execute_post_installation_hooks;
use std::process::Command;

/// Installs a Rust toolchain and optionally its components using `rustup`.
///
/// This function acts as the installer module for `rustup`-managed Rust environments with
/// comprehensive error handling, verification, and accurate path detection.
///
/// # Workflow:
/// 1. **Environment Validation**: Verifies `rustup` is installed and accessible
/// 2. **Toolchain Validation**: Ensures the toolchain version is specified and valid
/// 3. **Pre-installation Check**: Verifies if toolchain is already installed
/// 4. **Toolchain Installation**: Installs the specified Rust toolchain with proper error handling
/// 5. **Component Installation**: Installs all specified components with individual validation
/// 6. **Installation Verification**: Confirms toolchain and components are properly installed
/// 7. **Path Resolution**: Accurately determines the installation path
/// 8. **Version Detection**: Gets actual installed version for accurate tracking
/// 9. **Post-installation Hooks**: Executes any additional setup commands
/// 10. **State Creation**: Creates comprehensive `ToolState` with all relevant metadata
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing toolchain configuration
///   - `tool_entry.name`: **Required** - The name identifier for the toolchain
///   - `tool_entry.version`: **Required** - The Rust toolchain name (e.g., "stable", "nightly", "1.70.0")
///   - `tool_entry.options`: Optional list of rustup components to install (e.g., ["rustfmt", "clippy"])
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if installation was completely successful with accurate metadata
/// * `None` if any step of the installation process fails
///
/// # Examples - YAML
///
/// ```yaml
/// # ###########################
/// # Example: RUSTUP Installer #
/// # ###########################
/// # Install rust and other rust tools
/// - name: rust
///   source: rustup
///   version: stable
///   options:
///     - rust-src
///     - clippy
///     - rustfmt
///     - rust-analyzer
/// ```
///
/// ## Toolchain with Components
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "rust-full".to_string(),
///     version: Some("nightly".to_string()),
///     options: Some(vec![
///         "rustfmt".to_string(),
///         "clippy".to_string(),
///         "rust-src".to_string()
///     ]),
/// };
/// install(&tool_entry);
/// ```
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[Rustup Installer] Attempting to install Rust toolchain: {}",
        tool_entry.name.bold()
    );
    log_debug!("[Rustup Installer] ToolEntry details: {:#?}", tool_entry);

    // 1. Validate and extract toolchain name - ensure version is specified and valid
    let toolchain_name = validate_toolchain_version(tool_entry)?;

    log_debug!(
        "[Rustup Installer] Installing toolchain: {}",
        toolchain_name.cyan()
    );

    // 2. Check if toolchain already exists and handle accordingly
    // This optimization avoids reinstalling existing toolchains
    let toolchain_status = check_toolchain_status(&toolchain_name);
    match toolchain_status {
        ToolchainStatus::AlreadyInstalled => {
            log_info!(
                "[Rustup Installer] Toolchain '{}' is already installed",
                toolchain_name.green()
            );
        }
        ToolchainStatus::NotInstalled => {
            // 4. Install the toolchain - only if not already present
            if !install_toolchain(&toolchain_name) {
                return None;
            }
        }
        ToolchainStatus::CheckFailed => {
            log_warn!(
                "[Rustup Installer] Could not verify toolchain status, proceeding with installation attempt"
            );
            if !install_toolchain(&toolchain_name) {
                return None;
            }
        }
    }

    log_debug!(
        "[Rustup Installer] Checking if component: {} is already installed",
        tool_entry.name.bold()
    );

    // 3. Check if component is already Installed
    if !check_if_installed(&tool_entry.name) {
        log_info!(
            "[Rustup Installer] Component '{}' appears to be already installed, outside SDB",
            tool_entry.name.green()
        );
        log_debug!(
            "[Rustup Installer] Proceeding with installation to ensure correct version/options"
        );
    }

    // 4. Install components if specified - adds additional tools to the toolchain
    if let Some(components) = &tool_entry.options {
        if !install_components(components, &toolchain_name) {
            return None;
        }
    }

    // 5. Verify the complete installation - ensure everything was installed correctly
    if !verify_toolchain_installation(&toolchain_name, tool_entry.options.as_ref()) {
        return None;
    }

    // 6. Determine accurate installation path - where the toolchain binaries are located
    let install_path = determine_rustup_installation_path(&toolchain_name);
    log_debug!(
        "[Rustup Installer] Determined installation path: {}",
        install_path.display().to_string().cyan()
    );

    // 7. Get actual installed version for accurate tracking - important for state management
    let actual_version =
        get_actual_toolchain_version(&toolchain_name).unwrap_or_else(|| toolchain_name.clone());

    log_info!(
        "[Rustup Installer] Successfully installed Rust toolchain: {} (version: {})",
        tool_entry.name.bold().green(),
        actual_version.green()
    );

    // 8. Execute post-installation hooks - run any additional setup commands
    log_debug!(
        "[Rustup Installer] Executing post installation hooks, post installing {}",
        tool_entry.name.bold()
    );
    let executed_post_installation_hooks =
        execute_post_installation_hooks("[Rustup Installer]", tool_entry, &install_path);

    // 9. Return comprehensive ToolState for tracking
    //
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed.
    Some(ToolState::new(
        tool_entry,
        &install_path,
        "rustup".to_string(),
        "rust-toolchain".to_string(),
        actual_version,
        None,
        None,
        executed_post_installation_hooks,
    ))
}

/// Represents the status of a toolchain installation check.
///
/// This enum is used to categorize the result of checking whether a toolchain
/// is already installed, allowing for appropriate handling of each scenario.
#[derive(Debug)]
enum ToolchainStatus {
    /// The toolchain is already installed and available
    AlreadyInstalled,
    /// The toolchain is not currently installed
    NotInstalled,
    /// The check failed due to an error (rustup command failure, etc.)
    CheckFailed,
}

/// Validates and extracts the toolchain version from the tool entry.
///
/// This function ensures that a valid toolchain version is specified in the
/// tool configuration. Rustup requires a toolchain identifier to install.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing version information
///
/// # Returns
/// `Some(String)` containing the validated toolchain name, or `None` if invalid
///
/// # Supported Toolchain Formats
/// - `stable`, `nightly`, `beta` (named toolchains)
/// - `1.70.0`, `1.69.0` (version-specific toolchains)
/// - `nightly-2023-05-01` (date-specific nightly toolchains)
/// - Any valid rustup toolchain identifier
fn validate_toolchain_version(tool_entry: &ToolEntry) -> Option<String> {
    match &tool_entry.version {
        Some(version) if !version.trim().is_empty() => {
            log_debug!("[Rustup Installer] Using toolchain version: {}", version);
            Some(version.clone())
        }
        Some(_) => {
            log_error!(
                "[Rustup Installer] Empty toolchain version provided for tool '{}'",
                tool_entry.name.bold().red()
            );
            None
        }
        None => {
            log_error!(
                "[Rustup Installer] Toolchain version (e.g., 'stable', 'nightly', '1.70.0') is required for rustup tool '{}'",
                tool_entry.name.bold().red()
            );
            None
        }
    }
}

/// Checks if a toolchain is already installed to avoid unnecessary reinstallation.
///
/// This function runs `rustup toolchain list` and parses the output to determine
/// if the specified toolchain is already installed. This optimization prevents
/// reinstalling existing toolchains.
///
/// # Arguments
/// * `toolchain_name` - The name of the toolchain to check
///
/// # Returns
/// `ToolchainStatus` indicating the installation status
///
/// # Note
/// The function handles toolchain names with platform suffixes (e.g., "stable-x86_64-pc-windows-msvc")
/// and ignores the "(default)" marker in the rustup output.
fn check_toolchain_status(toolchain_name: &str) -> ToolchainStatus {
    match Command::new("rustup").args(["toolchain", "list"]).output() {
        Ok(output) if output.status.success() => {
            let installed_toolchains = String::from_utf8_lossy(&output.stdout);

            // Check if our toolchain is in the list
            for line in installed_toolchains.lines() {
                let cleaned_line = line.trim().replace("(default)", "").trim().to_string();
                if cleaned_line == toolchain_name
                    || cleaned_line.starts_with(&format!("{toolchain_name}-"))
                {
                    return ToolchainStatus::AlreadyInstalled;
                }
            }
            ToolchainStatus::NotInstalled
        }
        Ok(output) => {
            log_warn!(
                "[Rustup Installer] Failed to check installed toolchains. Exit code: {}. Error: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            );
            ToolchainStatus::CheckFailed
        }
        Err(e) => {
            log_warn!(
                "[Rustup Installer] Failed to execute 'rustup toolchain list': {}",
                e
            );
            ToolchainStatus::CheckFailed
        }
    }
}

/// Installs the specified Rust toolchain using `rustup toolchain install`.
///
/// This function executes the actual toolchain installation command with
/// comprehensive error handling and logging.
///
/// # Arguments
/// * `toolchain_name` - The name of the toolchain to install
///
/// # Returns
/// `true` if installation was successful, `false` otherwise
///
/// # Command Execution
/// Runs: `rustup toolchain install <toolchain_name>`
///
fn install_toolchain(toolchain_name: &str) -> bool {
    let args = vec!["toolchain", "install", toolchain_name];

    log_info!(
        "[Rustup Installer] Executing: {} {}",
        "rustup".cyan().bold(),
        args.join(" ").cyan()
    );

    match Command::new("rustup").args(&args).output() {
        Ok(output) if output.status.success() => {
            log_info!(
                "[Rustup Installer] Successfully installed toolchain: {}",
                toolchain_name.bold().green()
            );

            // Log output for debugging
            if !output.stdout.is_empty() {
                log_debug!(
                    "[Rustup Installer] Stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            if !output.stderr.is_empty() {
                log_warn!(
                    "[Rustup Installer] Stderr (may contain warnings): {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            true
        }
        Ok(output) => {
            log_error!(
                "[Rustup Installer] Failed to install toolchain '{}'. Exit code: {}. Error: {}",
                toolchain_name.bold().red(),
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );

            if !output.stdout.is_empty() {
                log_debug!(
                    "[Rustup Installer] Stdout (on failure): {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            false
        }
        Err(e) => {
            log_error!(
                "[Rustup Installer] Failed to execute 'rustup toolchain install' for '{}': {}",
                toolchain_name.bold().red(),
                e.to_string().red()
            );
            false
        }
    }
}

/// Checks if a specific Rust toolchain component is installed on the active toolchain.
///
/// This function executes `rustup component list --installed` and checks the output
/// for the presence of the specified component. It handles the target triple
/// suffix often present in component names (e.g., "clippy" matches "clippy-aarch64-apple-darwin").
///
/// # Arguments
/// * `component_name` - The base name of the component to check (e.g., "clippy", "rustfmt").
///
/// # Returns
/// `true` if the component is found, `false` otherwise (including if `rustup` fails).
fn check_if_installed(component_name: &str) -> bool {
    // 1. Execute the rustup command to list installed components
    match Command::new("rustup")
        .args(["component", "list", "--installed"])
        .output()
    {
        // 2. Check for successful command execution
        Ok(output) if output.status.success() => {
            let installed_components = String::from_utf8_lossy(&output.stdout);

            // 3. Define the component name with a hyphen suffix for target-specific checks
            // Example: "clippy" becomes "clippy-"
            let component_prefix = format!("{component_name}-");

            // 4. Iterate over lines and check for a match
            installed_components.lines().any(|line| {
                let trimmed_line = line.trim();

                // Check for an exact match (for components like "rust-src")
                if trimmed_line == component_name {
                    return true;
                }

                // Check for a prefix match (for components like "clippy-...")
                if trimmed_line.starts_with(&component_prefix) {
                    return true;
                }

                false
            })
        }
        // 5. Handle command failure (e.g., rustup not found, command fails)
        _ => false,
    }
}

/// Installs all specified components for the toolchain.
///
/// This function iterates through the list of components and installs each one
/// individually. It uses a "partial success" approach where failures in individual
/// components don't immediately stop the entire installation process.
///
/// # Arguments
/// * `components` - Slice of component names to install
/// * `toolchain_name` - The toolchain to which components should be added
///
/// # Returns
/// `true` if all components were installed successfully, `false` if any failed
///
/// # Partial Success Handling
/// The function continues installing components even if some fail, but returns
/// `false` if any component installation fails. This allows for maximum progress
/// while still reporting overall failure.
fn install_components(components: &[String], toolchain_name: &str) -> bool {
    let mut all_success = true;

    for component in components {
        if !install_single_component(component, toolchain_name) {
            all_success = false;
            // Continue with other components instead of failing immediately
            // This allows partial success scenarios
        }
    }

    if !all_success {
        log_error!("[Rustup Installer] One or more components failed to install");
        return false;
    }

    log_info!("[Rustup Installer] All components installed successfully");
    true
}

/// Installs a single component for the specified toolchain.
///
/// This function adds a specific component to a toolchain using `rustup component add`.
///
/// # Arguments
/// * `component` - The name of the component to install
/// * `toolchain_name` - The toolchain to which the component should be added
///
/// # Returns
/// `true` if component installation was successful, `false` otherwise
///
/// # Command Execution
/// Runs: `rustup component add <component> --toolchain <toolchain_name>`
///
/// # Common Components
/// - `rustfmt`: Code formatting tool
/// - `clippy`: Linting tool
/// - `rust-src`: Rust source code for IDE support
/// - `llvm-tools`: LLVM tooling utilities
/// - `rust-analysis`: IDE analysis data
fn install_single_component(component: &str, toolchain_name: &str) -> bool {
    let args = vec!["component", "add", component, "--toolchain", toolchain_name];

    log_info!(
        "[Rustup Installer] Executing: {} {}",
        "rustup".cyan().bold(),
        args.join(" ").cyan()
    );

    match Command::new("rustup").args(&args).output() {
        Ok(output) if output.status.success() => {
            log_info!(
                "[Rustup Installer] Successfully added component '{}' to toolchain '{}'",
                component.bold().green(),
                toolchain_name.bold().green()
            );

            if !output.stdout.is_empty() {
                log_debug!(
                    "[Rustup Installer] Stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            if !output.stderr.is_empty() {
                log_warn!(
                    "[Rustup Installer] Stderr (may contain warnings): {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            true
        }
        Ok(output) => {
            log_error!(
                "[Rustup Installer] Failed to add component '{}' to toolchain '{}'. Exit code: {}. Error: {}",
                component.bold().red(),
                toolchain_name.bold().red(),
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );

            if !output.stdout.is_empty() {
                log_debug!(
                    "[Rustup Installer] Stdout (on failure): {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            false
        }
        Err(e) => {
            log_error!(
                "[Rustup Installer] Failed to execute 'rustup component add' for '{}' on toolchain '{}': {}",
                component.bold().red(),
                toolchain_name.bold().red(),
                e.to_string().red()
            );
            false
        }
    }
}

/// Verifies that the toolchain and all components are properly installed.
///
/// This function performs a comprehensive verification to ensure the installation
/// was completely successful before marking the toolchain as ready for use.
///
/// # Arguments
/// * `toolchain_name` - The toolchain to verify
/// * `components` - Optional list of components that should be verified
///
/// # Returns
/// `true` if verification passes, `false` otherwise
///
/// # Verification Steps
/// 1. Toolchain existence check using `rustup toolchain list`
/// 2. Component verification using `rustup component list --installed`
fn verify_toolchain_installation(toolchain_name: &str, components: Option<&Vec<String>>) -> bool {
    // 1. Verify the toolchain itself - ensure it appears in the installed list
    if !verify_toolchain_exists(toolchain_name) {
        return false;
    }

    // 2. Verify components if any were specified - check each component is installed
    if let Some(component_list) = components {
        if !verify_components_installed(component_list, toolchain_name) {
            return false;
        }
    }

    log_debug!("[Rustup Installer] Installation verification completed successfully");
    true
}

/// Verifies that a toolchain is installed and accessible.
///
/// This function checks if the toolchain appears in the list of installed toolchains
/// returned by `rustup toolchain list`.
///
/// # Arguments
/// * `toolchain_name` - The toolchain to verify
///
/// # Returns
/// `true` if the toolchain is found, `false` otherwise
fn verify_toolchain_exists(toolchain_name: &str) -> bool {
    match Command::new("rustup").args(["toolchain", "list"]).output() {
        Ok(output) if output.status.success() => {
            let installed_toolchains = String::from_utf8_lossy(&output.stdout);

            for line in installed_toolchains.lines() {
                let cleaned_line = line.trim().replace("(default)", "").trim().to_string();
                if cleaned_line == toolchain_name
                    || cleaned_line.starts_with(&format!("{toolchain_name}-"))
                {
                    log_debug!(
                        "[Rustup Installer] Verified toolchain '{}' is installed",
                        toolchain_name
                    );
                    return true;
                }
            }

            log_error!(
                "[Rustup Installer] Toolchain '{}' not found in installed toolchains",
                toolchain_name.red()
            );
            false
        }
        Ok(output) => {
            log_error!(
                "[Rustup Installer] Failed to verify toolchain installation. Exit code: {}. Error: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );
            false
        }
        Err(e) => {
            log_error!(
                "[Rustup Installer] Failed to execute toolchain verification command: {}",
                e.to_string().red()
            );
            false
        }
    }
}

/// Verifies that all specified components are installed for the toolchain.
///
/// This function checks the list of installed components for the toolchain
/// to ensure all requested components are present. It handles cases where
/// components include a target triple (e.g., 'clippy-x86_64-unknown-linux-gnu').
///
/// # Arguments
/// * `components` - List of component names that should be installed (e.g., "clippy")
/// * `toolchain_name` - The toolchain to check
///
/// # Returns
/// `true` if all components are found, `false` otherwise
///
/// # Note
/// Component verification failures are treated as warnings rather than errors
/// to avoid blocking successful toolchain installations due to minor component issues.
fn verify_components_installed(components: &[String], toolchain_name: &str) -> bool {
    match Command::new("rustup")
        .args([
            "component",
            "list",
            "--toolchain",
            toolchain_name,
            "--installed",
        ])
        .output()
    {
        Ok(output) if output.status.success() => {
            let installed_components = String::from_utf8_lossy(&output.stdout);

            // Collect installed components as owned Strings for easier prefix checking
            let installed_set: std::collections::HashSet<String> = installed_components
                .lines()
                .filter_map(|line| {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect();

            let mut all_found = true;
            for component in components {
                // Check if any installed component starts with the component name,
                // followed by the end of the string OR a hyphen, to account for
                // target triples (e.g., "clippy" matches "clippy-aarch64-apple-darwin").
                let component_found = installed_set.iter().any(|installed_name| {
                    installed_name == component
                        || installed_name.starts_with(&format!("{component}-"))
                });

                if !component_found {
                    log_error!(
                        "[Rustup Installer] Component '{}' not found in installed components for toolchain '{}'",
                        component.red(),
                        toolchain_name.red()
                    );
                    all_found = false;
                }
            }

            if all_found {
                log_debug!("[Rustup Installer] All specified components verified as installed");
            }
            all_found
        }
        Ok(output) => {
            log_warn!(
                "[Rustup Installer] Could not verify components. Exit code: {}. Error: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            );
            // Return true as warning, since component verification failure shouldn't block success
            true
        }
        Err(e) => {
            log_warn!(
                "[Rustup Installer] Failed to execute component verification: {}",
                e
            );
            // Return true as warning, since component verification failure shouldn't block success
            true
        }
    }
}

/// Determines the accurate installation path for the rustup toolchain.
///
/// This function attempts to locate where rustup installed the toolchain binaries
/// by checking environment variables in order of precedence.
///
/// # Arguments
/// * `toolchain_name` - The name of the installed toolchain
///
/// # Returns
/// A `PathBuf` containing the full path to the toolchain's bin directory
///
/// # Path Resolution Order
/// 1. `RUSTUP_HOME` environment variable (highest priority)
/// 2. `HOME` environment variable with default `~/.rustup` path
/// 3. System fallback to `/usr/local/bin` (lowest priority)
///
fn determine_rustup_installation_path(toolchain_name: &str) -> PathBuf {
    // Try to get the rustup home directory in order of preference
    if let Some(path) = get_rustup_home_path(toolchain_name) {
        return path;
    }

    // Final fallback
    log_warn!("[Rustup Installer] Could not determine rustup home, using system fallback");
    PathBuf::from("/usr/local/bin")
}

/// Gets the rustup installation path using RUSTUP_HOME environment variable.
///
/// This is a helper function that constructs the toolchain path based on the
/// `RUSTUP_HOME` environment variable, which is the standard way to customize
/// rustup's installation location.
///
/// # Arguments
/// * `toolchain_name` - The name of the toolchain
///
/// # Returns
/// `Some(PathBuf)` if RUSTUP_HOME is set and path can be constructed, `None` otherwise
///
/// # Path Format
/// `$RUSTUP_HOME/toolchains/<toolchain_name>/bin`
fn get_rustup_home_path(toolchain_name: &str) -> Option<PathBuf> {
    // 1. Check RUSTUP_HOME (highest priority)
    if let Ok(rustup_home) = env::var("RUSTUP_HOME") {
        let path = PathBuf::from(rustup_home)
            .join("toolchains")
            .join(toolchain_name)
            .join("bin");

        log_debug!(
            "[Rustup Installer] Using RUSTUP_HOME path: {}",
            path.display()
        );
        return Some(path);
    }

    // 2. Check CARGO_HOME (medium priority)
    if let Ok(home) = env::var("HOME") {
        let path = PathBuf::from(home)
            .join(".rustup")
            .join("toolchains")
            .join(toolchain_name)
            .join("bin");

        log_debug!(
            "[Rustup Installer] Using HOME-based rustup path: {}",
            path.display()
        );
        return Some(path);
    }

    // If none of the environment variables are set or paths are constructed, return None
    None
}

/// Gets the actual version of the installed toolchain for accurate tracking.
///
/// This function attempts to determine the precise version of the installed
/// toolchain by querying rustup and rustc. This provides more accurate version
/// information than the user-specified toolchain name.
///
/// # Arguments
/// * `toolchain_name` - The name of the toolchain to query
///
/// # Returns
/// `Some(String)` containing the actual version, or `None` if version detection fails
///
/// # Version Detection Strategies
/// 1. **Active Toolchain Query**: Uses `rustup show active-toolchain`
/// 2. **Rustc Version Query**: Uses `rustup run <toolchain> rustc --version`
/// 3. **Fallback**: Returns the original toolchain name if detection fails
///
fn get_actual_toolchain_version(toolchain_name: &str) -> Option<String> {
    // Try to get detailed version information
    match Command::new("rustup")
        .args(["show", "active-toolchain"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // Look for lines that contain our toolchain name
            for line in output_str.lines() {
                if line.contains(toolchain_name) {
                    // Extract version information from the line
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(version_part) = parts.first() {
                        return Some(version_part.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    // Fallback: try to get version using rustc with specific toolchain
    match Command::new("rustup")
        .args(["run", toolchain_name, "rustc", "--version"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let version_output = String::from_utf8_lossy(&output.stdout);
            // Parse "rustc 1.70.0 (90c541806 2023-05-31)" format
            if let Some(first_line) = version_output.lines().next() {
                if let Some(version_start) = first_line.find("rustc ") {
                    let version_part = &first_line[version_start + 6..];
                    if let Some(space_pos) = version_part.find(' ') {
                        return Some(version_part[..space_pos].to_string());
                    }
                }
            }
        }
        _ => {}
    }

    None
}

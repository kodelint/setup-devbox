// This module provides comprehensive functionality to install Python packages using `pip`.
// It follows the same robust pattern as the `rustup.rs`, `cargo.rs`, and `brew.rs` installers
// with comprehensive error handling, verification, and accurate path detection.
//
// The installer handles Python package installations with support for different pip variants,
// installation modes (user vs system), version specifications, and comprehensive verification.

use std::env;
// Standard library imports:
// `std::path::PathBuf`: Provides an owned, OS-agnostic path for path manipulation.
use std::path::PathBuf;
// `std::process::{Command, Output}`: Core functionality for executing external commands.
//   - `Command`: Builder for new processes, used to construct and configure `pip` commands.
//   - `Output`: Represents the output of a finished process, containing exit status, stdout, and stderr.
use std::process::Command;
// `std::collections::HashSet`: Used for efficient lookup during verification.
use std::collections::HashSet;

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

/// Represents the type of pip executable detected
#[derive(Debug, Clone)]
enum PipVariant {
    Pip3,
    Pip,
    Python3Module,
    PythonModule,
}

impl PipVariant {
    /// Returns the command string for this pip variant
    fn command(&self) -> &'static str {
        match self {
            PipVariant::Pip3 => "pip3",
            PipVariant::Pip => "pip",
            PipVariant::Python3Module => "python3",
            PipVariant::PythonModule => "python",
        }
    }

    /// Returns the arguments for module execution
    fn module_args(&self) -> Vec<&'static str> {
        match self {
            PipVariant::Python3Module => vec!["-m", "pip"],
            PipVariant::PythonModule => vec!["-m", "pip"],
            _ => vec![],
        }
    }

    /// Returns whether this variant uses module execution
    fn is_module(&self) -> bool {
        matches!(self, PipVariant::Python3Module | PipVariant::PythonModule)
    }
}

/// Installs a Python package using pip with comprehensive error handling.
///
/// This function provides a robust installer for Python packages that mirrors the quality and
/// reliability of the other installers. It includes validation, verification, and accurate
/// state tracking for pip installations.
///
/// # Workflow:
/// 1. **Environment Validation**: Verifies pip/Python environment is available
/// 2. **Pip Variant Detection**: Determines the best available pip executable
/// 3. **Installation Mode Detection**: Determines user vs system installation
/// 4. **Command Preparation**: Constructs appropriate pip install command arguments
/// 5. **Pre-installation Check**: Verifies if package is already installed
/// 6. **Package Installation**: Executes pip install with comprehensive error handling
/// 7. **Installation Verification**: Confirms package was properly installed
/// 8. **Path Resolution**: Accurately determines the installation path
/// 9. **Version Detection**: Retrieves actual installed version for accurate tracking
/// 10. **Post-installation Hooks**: Executes any additional setup commands
/// 11. **State Creation**: Creates comprehensive `ToolState` with all relevant metadata
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing package configuration
///   - `tool_entry.name`: **Required** - The Python package name to install
///   - `tool_entry.version`: Optional version specification (e.g., "package==1.0.0")
///   - `tool_entry.options`: Optional list of pip install options (--user, --upgrade, etc.)
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if installation was completely successful with accurate metadata
/// * `None` if any step of the installation process fails
/// ## Examples - YAML
///
/// ```yaml
/// # ########################
/// # Example: PIP Installer #
/// # ########################
/// - name: pip
///   source: pip
///   version: 25.2
///   options:
///     - --upgrade
/// ```
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[Pip Installer] Attempting to install Python package: {}",
        tool_entry.name.bold()
    );
    log_debug!("[Pip Installer] ToolEntry details: {:#?}", tool_entry);

    // 1. Detect and validate pip executable
    let pip_variant = detect_pip_variant()?;
    log_debug!("[Pip Installer] Using pip variant: {:?}", pip_variant);

    // 2. Validate package configuration
    if !validate_package_configuration(tool_entry) {
        return None;
    }

    // 3. Determine installation mode (user vs system)
    let is_user_install = detect_installation_mode(tool_entry);
    log_debug!(
        "[Pip Installer] Installation mode: {}",
        if is_user_install { "user" } else { "system" }.cyan()
    );

    // 4. Check if package is already installed (optimization)
    if check_package_already_installed(&tool_entry.name, &pip_variant) {
        log_info!(
            "[Pip Installer] Package '{}' appears to be already installed",
            tool_entry.name.green()
        );
        // Continue with installation to ensure correct version and options
        log_debug!("[Pip Installer] Proceeding with installation to ensure correct version");
    }

    // 5. Prepare and execute pip install command
    let command_args = prepare_pip_install_command(tool_entry, &pip_variant);
    if !execute_pip_install_command(&pip_variant, &command_args, tool_entry) {
        return None;
    }

    // 6. Verify the installation was successful
    if !verify_pip_installation(&tool_entry.name, &pip_variant) {
        return None;
    }

    // 7. Determine accurate installation path
    let install_path =
        determine_pip_installation_path(&tool_entry.name, is_user_install, &pip_variant);
    log_debug!(
        "[Pip Installer] Determined installation path: {}",
        install_path.display().to_string().cyan()
    );

    // 8. Verify binary/package exists at expected path
    if !verify_package_accessible(&tool_entry.name, &pip_variant) {
        log_error!(
            "[Pip Installer] Package '{}' is not accessible after installation",
            tool_entry.name.red()
        );
        return None;
    }

    // 9. Execute post-installation hooks
    let working_dir = install_path
        .parent()
        .unwrap_or(&PathBuf::from("/"))
        .to_path_buf();
    let executed_post_installation_hooks =
        execute_post_installation_hooks("[Pip Installer]", tool_entry, &working_dir);

    // 10. Get actual installed version for accurate tracking
    let actual_version = determine_installed_version(&tool_entry.name, &pip_variant)
        .unwrap_or_else(|| {
            tool_entry
                .version
                .clone()
                .unwrap_or_else(|| "latest".to_string())
        });

    log_info!(
        "[Pip Installer] Successfully installed Python package: {} (version: {})",
        tool_entry.name.bold().green(),
        actual_version.green()
    );

    // 11. Return comprehensive ToolState for tracking
    //
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed.
    Some(ToolState::new(
        tool_entry,
        &install_path,
        "pip".to_string(),
        "python-package".to_string(),
        actual_version,
        None,
        None,
        executed_post_installation_hooks,
    ))
}

/// Detects the best available pip variant on the system.
fn detect_pip_variant() -> Option<PipVariant> {
    // Try pip3 first (preferred for Python 3)
    if Command::new("pip3").arg("--version").output().is_ok() {
        log_debug!("[Pip Installer] Found pip3");
        return Some(PipVariant::Pip3);
    }

    // Try pip
    if Command::new("pip").arg("--version").output().is_ok() {
        log_debug!("[Pip Installer] Found pip");
        return Some(PipVariant::Pip);
    }

    // Try python3 -m pip
    if Command::new("python3")
        .args(["-m", "pip", "--version"])
        .output()
        .is_ok()
    {
        log_debug!("[Pip Installer] Found python3 -m pip");
        return Some(PipVariant::Python3Module);
    }

    // Try python -m pip as final fallback
    if Command::new("python")
        .args(["-m", "pip", "--version"])
        .output()
        .is_ok()
    {
        log_debug!("[Pip Installer] Found python -m pip");
        return Some(PipVariant::PythonModule);
    }

    log_error!(
        "[Pip Installer] No pip executable found. Please ensure Python and pip are installed and in your PATH."
    );
    None
}

/// Validates the package configuration for consistency and correctness.
fn validate_package_configuration(tool_entry: &ToolEntry) -> bool {
    // Validate package name
    if tool_entry.name.trim().is_empty() {
        log_error!("[Pip Installer] Package name cannot be empty");
        return false;
    }

    // Validate package name doesn't contain invalid characters
    if tool_entry
        .name
        .contains(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.')
    {
        log_error!(
            "[Pip Installer] Invalid package name '{}'. Package names should only contain alphanumeric characters, hyphens, underscores, and periods.",
            tool_entry.name.red()
        );
        return false;
    }

    // Validate version format if specified
    if let Some(version) = &tool_entry.version {
        if version.trim().is_empty() {
            log_warn!("[Pip Installer] Empty version specified, using latest available");
        }
    }

    true
}

/// Detects the installation mode (user vs system) based on options.
fn detect_installation_mode(tool_entry: &ToolEntry) -> bool {
    tool_entry
        .options
        .as_ref()
        .map(|opts| opts.iter().any(|opt| opt == "--user"))
        .unwrap_or(false)
}

/// Checks if a package is already installed to avoid unnecessary reinstallation.
fn check_package_already_installed(package_name: &str, pip_variant: &PipVariant) -> bool {
    let (command, args) = build_pip_show_command(pip_variant, package_name);

    match Command::new(command).args(&args).output() {
        Ok(output) if output.status.success() => {
            log_debug!(
                "[Pip Installer] Package '{}' is already installed",
                package_name
            );
            true
        }
        Ok(output) => {
            // pip show returns non-zero exit code if package is not installed
            if output.status.code() == Some(1) {
                log_debug!(
                    "[Pip Installer] Package '{}' is not installed",
                    package_name
                );
                false
            } else {
                log_warn!(
                    "[Pip Installer] Could not check package status. Exit code: {}. Error: {}",
                    output.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&output.stderr)
                );
                false
            }
        }
        Err(e) => {
            log_warn!(
                "[Pip Installer] Failed to check package installation status: {}",
                e
            );
            false
        }
    }
}

/// Prepares the pip install command arguments.
fn prepare_pip_install_command(tool_entry: &ToolEntry, pip_variant: &PipVariant) -> Vec<String> {
    let mut command_args = Vec::new();

    // Add module arguments if using python -m pip
    if pip_variant.is_module() {
        command_args.extend(pip_variant.module_args().iter().map(|s| s.to_string()));
    }

    command_args.push("install".to_string());

    // Build package specifier with version if specified
    let package_specifier = if let Some(version) = &tool_entry.version {
        if !version.trim().is_empty() {
            format!("{}=={}", tool_entry.name, version)
        } else {
            tool_entry.name.clone()
        }
    } else {
        tool_entry.name.clone()
    };
    command_args.push(package_specifier);

    // Add any additional options
    if let Some(options) = &tool_entry.options {
        log_debug!("[Pip Installer] Adding custom options: {:#?}", options);
        for opt in options {
            command_args.push(opt.clone());
        }
    }

    log_debug!(
        "[Pip Installer] Prepared command arguments: {} {}",
        pip_variant.command().cyan().bold(),
        command_args.join(" ").cyan()
    );

    command_args
}

/// Executes the pip install command with comprehensive error handling.
fn execute_pip_install_command(
    pip_variant: &PipVariant,
    command_args: &[String],
    tool_entry: &ToolEntry,
) -> bool {
    log_info!(
        "[Pip Installer] Executing: {} {}",
        pip_variant.command().cyan().bold(),
        command_args.join(" ").cyan()
    );

    match Command::new(pip_variant.command())
        .args(command_args)
        .output()
    {
        Ok(output) if output.status.success() => {
            log_info!(
                "[Pip Installer] Successfully installed package: {}",
                tool_entry.name.bold().green()
            );

            // Log output for debugging
            if !output.stdout.is_empty() {
                log_debug!(
                    "[Pip Installer] Stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            if !output.stderr.is_empty() {
                log_warn!(
                    "[Pip Installer] Stderr (may contain warnings): {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            true
        }
        Ok(output) => {
            log_error!(
                "[Pip Installer] Failed to install package '{}'. Exit code: {}. Error: {}",
                tool_entry.name.bold().red(),
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );

            if !output.stdout.is_empty() {
                log_debug!(
                    "[Pip Installer] Stdout (on failure): {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            false
        }
        Err(e) => {
            log_error!(
                "[Pip Installer] Failed to execute 'pip install' for '{}': {}",
                tool_entry.name.bold().red(),
                e.to_string().red()
            );
            false
        }
    }
}

/// Verifies that the package was properly installed.
fn verify_pip_installation(package_name: &str, pip_variant: &PipVariant) -> bool {
    // Verify the package appears in pip list
    if !verify_package_in_list(package_name, pip_variant) {
        return false;
    }

    // Verify the package is accessible via pip show
    if !verify_package_accessible(package_name, pip_variant) {
        return false;
    }

    log_debug!("[Pip Installer] Installation verification completed successfully");
    true
}

/// Verifies that the package appears in the pip list output.
fn verify_package_in_list(package_name: &str, pip_variant: &PipVariant) -> bool {
    let (command, args) = build_pip_list_command(pip_variant);

    match Command::new(command).args(&args).output() {
        Ok(output) if output.status.success() => {
            let installed_packages = String::from_utf8_lossy(&output.stdout);
            let installed_set: HashSet<&str> = installed_packages
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if !parts.is_empty() {
                        Some(parts[0])
                    } else {
                        None
                    }
                })
                .collect();

            if installed_set.contains(&package_name.to_lowercase().as_str()) {
                log_debug!(
                    "[Pip Installer] Verified package '{}' is in pip list",
                    package_name
                );
                true
            } else {
                log_error!(
                    "[Pip Installer] Package '{}' not found in installed packages list",
                    package_name.red()
                );
                false
            }
        }
        Ok(output) => {
            log_warn!(
                "[Pip Installer] Could not verify installation via pip list. Exit code: {}. Error: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            );
            // Return true as warning, since verification failure shouldn't block success
            true
        }
        Err(e) => {
            log_warn!(
                "[Pip Installer] Failed to execute installation verification: {}",
                e
            );
            // Return true as warning, since verification failure shouldn't block success
            true
        }
    }
}

/// Verifies that the package is accessible via pip show.
fn verify_package_accessible(package_name: &str, pip_variant: &PipVariant) -> bool {
    let (command, args) = build_pip_show_command(pip_variant, package_name);

    match Command::new(command).args(&args).output() {
        Ok(output) if output.status.success() => {
            log_debug!(
                "[Pip Installer] Verified package '{}' is accessible via pip show",
                package_name
            );
            true
        }
        Ok(output) => {
            log_error!(
                "[Pip Installer] Package '{}' not accessible via pip show. Exit code: {}. Error: {}",
                package_name.red(),
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );
            false
        }
        Err(e) => {
            log_error!(
                "[Pip Installer] Failed to execute package accessibility check: {}",
                e.to_string().red()
            );
            false
        }
    }
}

/// Builds the command and arguments for pip list.
fn build_pip_list_command(pip_variant: &PipVariant) -> (String, Vec<String>) {
    let command = pip_variant.command().to_string();
    let mut args = Vec::new();

    if pip_variant.is_module() {
        args.extend(pip_variant.module_args().iter().map(|s| s.to_string()));
    }

    args.push("list".to_string());
    args.push("--format=freeze".to_string());

    (command, args)
}

/// Builds the command and arguments for pip show.
fn build_pip_show_command(pip_variant: &PipVariant, package_name: &str) -> (String, Vec<String>) {
    let command = pip_variant.command().to_string();
    let mut args = Vec::new();

    if pip_variant.is_module() {
        args.extend(pip_variant.module_args().iter().map(|s| s.to_string()));
    }

    args.push("show".to_string());
    args.push(package_name.to_string());

    (command, args)
}

/// Determines the accurate installation path for pip-installed packages.
fn determine_pip_installation_path(
    package_name: &str,
    is_user_install: bool,
    pip_variant: &PipVariant,
) -> PathBuf {
    if is_user_install {
        // For user installations, get the user base directory
        if let Some(path) = get_user_installation_path(package_name, pip_variant) {
            return path;
        }
    } else {
        // For system installations, try to get the system path
        if let Some(path) = get_system_installation_path(package_name, pip_variant) {
            return path;
        }
    }

    // Fallback: try common Python installation paths
    if let Some(path) = get_common_python_paths(package_name) {
        return path;
    }

    // Final fallback
    log_warn!("[Pip Installer] Could not determine pip installation path, using system fallback");
    PathBuf::from("/usr/local/bin").join(package_name)
}

/// Gets the user installation path for Python packages.
fn get_user_installation_path(package_name: &str, pip_variant: &PipVariant) -> Option<PathBuf> {
    // Try to get the user base directory using Python
    let python_cmd = if pip_variant.is_module() {
        pip_variant.command()
    } else {
        // Determine appropriate Python command based on pip variant
        match pip_variant {
            PipVariant::Pip3 => "python3",
            PipVariant::Pip => "python",
            _ => "python3", // fallback
        }
    };

    if let Ok(output) = Command::new(python_cmd)
        .args(["-c", "import site; print(site.USER_BASE)"])
        .output()
    {
        if output.status.success() {
            let user_base = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !user_base.is_empty() {
                log_debug!("[Pip Installer] Found user base: {}", user_base);
                return Some(PathBuf::from(user_base).join("bin").join(package_name));
            }
        }
    }

    // Fallback to HOME/.local/bin
    if let Ok(home) = env::var("HOME") {
        log_debug!("[Pip Installer] Using HOME/.local/bin fallback");
        return Some(
            PathBuf::from(home)
                .join(".local")
                .join("bin")
                .join(package_name),
        );
    }

    None
}

/// Gets the system installation path for Python packages.
fn get_system_installation_path(package_name: &str, pip_variant: &PipVariant) -> Option<PathBuf> {
    // Try to get system site-packages directory using the same Python command
    let python_cmd = if pip_variant.is_module() {
        pip_variant.command()
    } else {
        match pip_variant {
            PipVariant::Pip3 => "python3",
            PipVariant::Pip => "python",
            _ => "python3",
        }
    };

    if let Ok(output) = Command::new(python_cmd)
        .args(["-c", "import sys; print(next((p for p in sys.path if 'site-packages' in p and 'local' in p), ''))"])
        .output()
    {
        if output.status.success() {
            let site_packages = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !site_packages.is_empty() {
                // Convert site-packages path to bin path
                let mut path = PathBuf::from(&site_packages);
                // Go up from site-packages to find the bin directory
                if path.pop() && path.pop() && path.pop() {
                    let bin_path = path.join("bin").join(package_name);
                    log_debug!("[Pip Installer] Found system bin path: {}", bin_path.display());
                    return Some(bin_path);
                }
            }
        }
    }

    None
}

/// Gets common Python installation paths as fallback.
fn get_common_python_paths(package_name: &str) -> Option<PathBuf> {
    // Common Python installation paths
    let common_paths = [
        PathBuf::from("/usr/local/bin").join(package_name),
        PathBuf::from("/opt/homebrew/bin").join(package_name), // Homebrew on Apple Silicon
        PathBuf::from("/usr/bin").join(package_name),
        PathBuf::from("/opt/local/bin").join(package_name), // MacPorts
    ];

    for path in &common_paths {
        if path.exists() {
            log_debug!(
                "[Pip Installer] Found executable at common path: {}",
                path.display()
            );
            return Some(path.clone());
        }
    }

    None
}

/// Determines the actual installed version of the package.
fn determine_installed_version(package_name: &str, pip_variant: &PipVariant) -> Option<String> {
    let (command, args) = build_pip_show_command(pip_variant, package_name);

    match Command::new(command).args(&args).output() {
        Ok(output) if output.status.success() => {
            parse_version_from_output(&String::from_utf8_lossy(&output.stdout))
        }
        _ => None,
    }
}

/// Parses the version from pip show output.
fn parse_version_from_output(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.starts_with("Version:") {
            // Manual implementation of split_once functionality
            // Find the first occurrence of ':' in the line
            if let Some(colon_pos) = line.find(':') {
                // Calculate the start position for the version part (after the colon)
                let version_start = colon_pos + 1; // +1 to skip the colon itself
                // Extract the version part from after the colon to the end of the line
                let version_part = &line[version_start..];
                let version = version_part.trim().to_string();

                // Only return if we have a non-empty version
                if !version.is_empty() {
                    return Some(version);
                }
            }
        }
    }
    None
}

// This module provides the installation logic for Python packages using `pip`.
// It acts as a specialized installer within the `setup-devbox` application,
// handling the nuances of `pip` commands, versioning, and additional options.

// Imports necessary schema definitions for tools.
// ToolEntry: Defines how a tool (in this case, a Python package) is configured in `tools.yaml`.
// ToolState: Defines the structure for how the state of an installed tool is recorded in `state.json`.
use crate::schema::{ToolEntry, ToolState};
// Imports custom logging macros from the crate root.
// These macros (`log_debug`, `log_error`, `log_info`, `log_warn`) provide
// a standardized way to output messages to the console with different severity levels,
// essential for tracking progress and diagnosing issues during package installation.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to terminal output.
// The `colored` crate allows us to make log messages and other terminal output more readable
// by applying colors (e.g., `.bold()`, `.cyan()`, `.red()`, `.green()`).
use colored::Colorize;
// For executing external commands (like `pip`) and capturing their output.
// `std::process::Command` is used to build and run system commands.
// `std::process::Output` captures the standard output, standard error, and exit status.
use std::process::{Command, Output};
// For working with file paths in an OS-agnostic manner.
// `PathBuf` is used here primarily for constructing the `install_path` for the `ToolState`.
use std::path::PathBuf;

/// Installs a Python package using `pip` (or `pip3`).
///
/// This function serves as the dedicated installer for Python packages within the DevBox system.
/// It intelligently detects the correct `pip` executable (`pip3` preferred, then `pip`),
/// constructs the appropriate `pip install` command based on the `ToolEntry` configuration,
/// executes it, and records the installation details in a `ToolState` object.
///
/// # Workflow:
/// 1.  **`pip` Executable Detection**: Checks for `pip3` first, then `pip` to ensure compatibility
///     with modern Python environments.
/// 2.  **Command Construction**: Builds the `pip install` command, including the package name,
///     an optional version specifier (e.g., `package==1.2.3`), and any additional `pip` options
///     provided in the `tool_entry`.
/// 3.  **Execution**: Runs the constructed `pip` command and captures its output.
/// 4.  **Error Handling**: Provides detailed logging for successful installations, warnings,
///     and failures, including exit codes and standard error output.
/// 5.  **Path Determination (Approximation)**: Attempts to determine a representative installation
///     path for the installed package. Note that exact paths for Python packages can vary
///     significantly (system-wide, user-site, virtual environments), and this provides a common
///     approximation, especially for executables installed into `~/.local/bin`.
/// 6.  **State Recording**: Creates and returns a `ToolState` object containing the installed
///     version, approximated path, installation method, and other metadata for persistent tracking.
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct. This struct holds the configuration
///   details for the Python package to be installed, as defined in `tools.yaml`.
///   - `tool_entry.name`: The name of the Python package (e.g., "requests", "black"). This is mandatory.
///   - `tool_entry.version`: An optional version string (e.g., "1.2.3"). If present, the
///     package will be installed with `package==version`. If `None`, the latest stable version is installed.
///   - `tool_entry.options`: An optional `Vec<String>` containing additional arguments to pass
///     directly to the `pip install` command (e.g., `"--user"`, `"--upgrade"`, `"--index-url https://..."`).
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if the Python package installation (including command execution and success status)
///     was completely successful. The `ToolState` object contains details about the installed package.
/// * `None` if `pip` (or `pip3`) is not found, or if the `pip install` command fails for any reason
///     (e.g., network error, package not found, installation error).
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!("[Pip Installer] Attempting to install Python package: {}", tool_entry.name.bold());

    // 1. Basic validation: Ensure 'pip' command is available on the system.
    // We prioritize 'pip3' as it's the standard for Python 3 installations.
    // If 'pip3 --version' works, we use 'pip3'.
    // Otherwise, we try 'pip --version'.
    // If neither is found, we log an error and cannot proceed.
    let pip_command = if Command::new("pip3").arg("--version").output().is_ok() {
        // If pip3 is found and executable, use it.
        "pip3"
    } else if Command::new("pip").arg("--version").output().is_ok() {
        // If pip3 isn't found, try 'pip'.
        "pip"
    } else {
        // Neither pip nor pip3 found, critical error.
        log_error!(
            "[Pip Installer] 'pip' or 'pip3' command not found. Please ensure Python and Pip are installed and in your PATH."
        );
        return None; // Cannot proceed without a valid pip executable.
    };

    // Initialize the arguments list for the `pip install` command.
    let mut command_args = vec!["install"];

    // Construct the package specifier: either "package_name" or "package_name==version".
    // This allows users to specify an exact version or get the latest.
    let package_specifier = if let Some(version) = &tool_entry.version {
        // If a version is specified, format as "name==version".
        format!("{}=={}", tool_entry.name, version)
    } else {
        // If no version, just use the package name, pip will install the latest.
        tool_entry.name.clone()
    };
    command_args.push(&package_specifier); // Add the package specifier to the command arguments.

    // Add any additional `pip` options from `tool_entry.options`.
    // This allows users to pass flags like `--user`, `--upgrade`, `--no-cache-dir`, etc.
    if let Some(options) = &tool_entry.options {
        for opt in options {
            command_args.push(opt); // Add each option string to the arguments list.
        }
    }

    // Log the full command being executed for debugging and user visibility.
    // The command and arguments are colored for better readability.
    log_info!("[Pip Installer] Executing: {} {}", pip_command.cyan().bold(), command_args.join(" ").cyan());

    // Execute the `pip install` command.
    // This is a blocking call that waits for the command to complete.
    let output: Output = match Command::new(pip_command) // Use the detected pip command.
        .args(&command_args) // Pass all constructed arguments.
        .output() // Execute and capture stdout, stderr, and exit status.
    {
        Ok(out) => out, // Command executed successfully (not necessarily *installed* successfully).
        Err(e) => {
            // Log an error if the command itself failed to spawn or execute (e.g., permissions issues).
            log_error!("[Pip Installer] Failed to execute '{} install' command for '{}': {}", pip_command.bold().red(), tool_entry.name.bold().red(), e);
            return None; // Indicate an installation failure.
        }
    };

    // 2. Check the command's exit status.
    // A successful exit status (`status.success()`) means the pip command ran without error,
    // indicating a successful package installation.
    if output.status.success() {
        log_info!(
            "[Pip Installer] Successfully installed Python package: {}",
            tool_entry.name.bold().green() // Green color for success.
        );
        // Log standard output if available, usually contains installation progress.
        if !output.stdout.is_empty() {
            log_debug!("[Pip Installer] Stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        // Log standard error if available. Pip sometimes prints warnings to stderr even on success.
        if !output.stderr.is_empty() {
            log_warn!("[Pip Installer] Stderr (might contain warnings): {}", String::from_utf8_lossy(&output.stderr));
        }

        // 3. Determine the installation path for `ToolState`.
        // Determining the exact `install_path` for pip packages can be challenging
        // due to varying installation locations (global site-packages, user site-packages, virtual environments).
        // This attempts to provide a common path, especially relevant for executables installed
        // into the user's local bin directory (e.g., `~/.local/bin/black` for the `black` package).
        // A more robust solution for libraries might involve parsing `pip show <package>` output for 'Location'.
        let install_path = if let Ok(mut home) = std::env::var("HOME") {
            // If the HOME environment variable is found, append the typical user-local bin path.
            // This is where many `pip --user` installed scripts/executables end up.
            home.push_str("/.local/bin/");
            // Construct the full path, joining the base path with the package name (assuming it might be an executable).
            PathBuf::from(home).join(&tool_entry.name).to_string_lossy().into_owned()
        } else {
            // If HOME directory cannot be determined, fall back to a generic system-wide bin path.
            // This is less accurate but provides a placeholder.
            log_warn!("[Pip Installer] Could not determine HOME directory. Using generic fallback for pip package path.");
            "/usr/local/bin/".to_string()
        };


        // 4. Return `ToolState` for Tracking.
        // Create and return a `ToolState` object to record this successful installation
        // in the application's persistent state (`state.json`).
        Some(ToolState {
            // Use the version specified in the config, or default to "latest" if not specified.
            version: tool_entry.version.clone().unwrap_or_else(|| "latest".to_string()),
            // The approximated installation path.
            install_path,
            // Mark that this tool was installed by `setup-devbox`.
            installed_by_devbox: true,
            // The method used for installation.
            install_method: "pip".to_string(),
            // Any `rename_to` specified in the config, passed for schema consistency.
            renamed_to: tool_entry.rename_to.clone(),
            // Categorize the type of package installed.
            package_type: "python-package".to_string(),
            // `repo` and `tag` are not typically applicable for pip installations
            // as pip fetches from PyPI or other package indexes, not directly from Git repositories.
            // Therefore, these fields are set to `None` for clarity and clean state.json.
            repo: None, // Set to None as pip doesn't track direct Git repositories.
            tag: None,  // Set to None as pip doesn't track Git tags.
            // Store the additional options that were used during the `pip install` command.
            options: tool_entry.options.clone(),
            // For direct URL installations: The original URL from which the tool was downloaded.
            url: tool_entry.url.clone(),
            executable_path_after_extract: None,
        })
    } else {
        // 5. Handle Installation Failure.
        // If the `pip` command exited with a non-zero status code, it indicates a failure.
        // Capture and log the standard error output for debugging purposes.
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error!(
            "[Pip Installer] Failed to install Python package '{}'. Exit code: {}. Error: {}",
            tool_entry.name.bold().red(), // Package name, colored red.
            output.status.code().unwrap_or(-1), // Exit code (default to -1 if not available).
            stderr.red() // Standard error output, colored red.
        );
        // Also log stdout on failure, as it might contain useful context.
        if !output.stdout.is_empty() {
            log_debug!("[Pip Installer] Stdout (on failure): {}", String::from_utf8_lossy(&output.stdout));
        }
        // Return `None` to signal that the installation was unsuccessful.
        None
    }
}
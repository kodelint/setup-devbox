// This module provides the installation logic for Python packages using `pip`.
// It acts as a specialized installer within the `setup-devbox` application,
// handling the nuances of `pip` commands, versioning, and additional options.

use std::env;
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
use crate::libs::utilities::assets::current_timestamp;
use std::path::PathBuf;
use crate::libs::tool_installer::execute_post_installation_hooks;

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
///   was completely successful. The `ToolState` object contains details about the installed package.
/// * `None` if `pip` (or `pip3`) is not found, or if the `pip install` command fails for any reason
///   (e.g., network error, package not found, installation error).
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!(
        "[Pip Installer] Attempting to install Python package: {}",
        tool_entry.name.bold()
    );

    // 1. Basic validation: Ensure 'pip' command is available on the system.
    // We prioritize 'pip3' as it's the standard for Python 3 installations.
    // If 'pip3 --version' works, we use 'pip3'.
    // Otherwise, we try 'pip --version'.
    // If neither is found, we log an error and cannot proceed.
    let pip_command = if Command::new("pip3").arg("--version").status().is_ok() {
        // Check if the command can be found and executed.
        "pip3"
    } else if Command::new("pip").arg("--version").status().is_ok() {
        // If pip3 isn't found, try 'pip'.
        "pip"
    } else {
        // Neither pip nor pip3 found, critical error.
        log_error!("[Pip Installer] 'pip' or 'pip3' command not found. Please ensure Python and \
        Pip are installed and in your PATH.");
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
    log_info!(
        "[Pip Installer] Executing: {} {}",
        pip_command.cyan().bold(),
        command_args.join(" ").cyan()
    );

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
            log_debug!(
                "[Pip Installer] Stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        // Log standard error if available. Pip sometimes prints warnings to stderr even on success.
        if !output.stderr.is_empty() {
            log_warn!(
                "[Pip Installer] Stderr (might contain warnings): {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // 3. Determine the installation path for `ToolState`.
        // Determining the exact `install_path` for pip packages can be challenging
        // due to varying installation locations (global site-packages, user site-packages, virtual environments).
        // This attempts to provide a common path, especially relevant for executables installed
        // into the user's local bin directory (e.g., `~/.local/bin/black` for the `black` package).
        // A more robust solution for libraries might involve parsing `pip show <package>` output for 'Location'.
        // Main logic using the new consolidated function
        let install_path = determine_pip_install_path(&tool_entry.name, &tool_entry.options);

        log_debug!("[Pip Installer] Determined installation path: {}",
            format!("{}", install_path.display()).cyan());

        // 4. Execute Additional Commands (if specified)
        // After the main installation is complete, execute any additional commands specified
        // in the tool configuration. These commands are often used for post-installation setup,
        // such as copying configuration files, creating directories, or setting up symbolic links.
        // Optional - failure won't stop installation
        let executed_post_installation_hooks = execute_post_installation_hooks(
            "[GitHub Installer]",
            tool_entry,
            &install_path,
        );
        // If execution reaches this point, the installation was successful.
        log_info!("[GitHub Installer] Installation of {} completed successfully at {}!",
            tool_entry.name.to_string().bold(), install_path.display().to_string().green());

        // 5. Return ToolState for Tracking
        // Construct a `ToolState` object to record the details of this successful installation.
        // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
        // what tools are installed, where they are, and how they were installed. This is crucial
        // for future operations like uninstallation, updates, or syncing.
        Some(ToolState {
            // Use the version specified in the config, or default to "latest" if not specified.
            version: tool_entry
                .version
                .clone()
                .unwrap_or_else(|| "latest".to_string()),
            // The canonical path where the tool's executable was installed. This is the path
            // that will be recorded in the `state.json` file.
            install_path: install_path.to_string_lossy().into_owned(),
            // Flag indicating that this tool was installed by `setup-devbox`. This helps distinguish
            // between tools managed by our system and those installed manually.
            installed_by_devbox: true,
            // The method of installation, useful for future diagnostics or differing update logic.
            // In this module, it's always "pip".
            install_method: "pip".to_string(),
            // Records if the binary was renamed during installation, storing the new name.
            renamed_to: tool_entry.rename_to.clone(),
            // `repo` and `tag` are not typically applicable for pip installations
            // as pip fetches from PyPI or other package indexes, not directly from Git repositories.
            // Therefore, these fields are set to `None` for clarity and clean state.json.
            repo: None, // Set to None as pip doesn't track direct Git repositories.
            tag: None,  // Set to None as pip doesn't track Git tags.
            // The actual package type detected by the `file` command or inferred. This is for diagnostic
            // purposes, providing the most accurate type even if the installation logic
            // used a filename-based guess (e.g., "binary", "macos-pkg-installer").
            package_type: "python-package".to_string(),
            // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
            // Store the additional options that were used during the `pip install` command.
            options: tool_entry.options.clone(),
            // For direct URL installations: The original URL from which the tool was downloaded.
            // This is important for re-downloading or verifying in the future.nal URL from which the tool was downloaded.
            url: tool_entry.url.clone(),
            // Record the timestamp when the tool was installed or updated
            last_updated: Some(current_timestamp()),
            // This field is currently `None` but could be used to store the path to an executable
            // *within* an extracted archive if `install_path` points to the archive's root.
            executable_path_after_extract: None,
            // Record any additional commands that were executed during installation.
            // This is useful for tracking what was done and potentially for cleanup during uninstall.
            executed_post_installation_hooks,
            configuration_manager: None,
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
            stderr.red()                  // Standard error output, colored red.
        );
        // Also log stdout on failure, as it might contain useful context.
        if !output.stdout.is_empty() {
            log_debug!(
                "[Pip Installer] Stdout (on failure): {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        // Return `None` to signal that the installation was unsuccessful.
        None
    }
}

// Main function to determine the installation path based on user options.
fn determine_pip_install_path(tool_name: &str, options: &Option<Vec<String>>) -> PathBuf {
    // Determine if "--user" option is present
    let installing_as_user = if let Some(opts) = options {
        opts.iter().any(|opt| opt == "--user")
    } else {
        false
    };

    // --- 1. USER-LOCAL INSTALL PATH ---
    if installing_as_user {
        log_debug!("[Pip Path] Determining path for '--user' install.");

        // A. Try to query Python for USER_BASE
        for python_cmd in ["python3", "python"] {
            let command = &[python_cmd, "-m", "site"];
            if let Some(path_str) = execute_and_parse_command(command, "USER_BASE:") {
                log_debug!("[Pip Path] Found path via '{} -m site'.", python_cmd);
                // Returns $USER_BASE/bin/<tool_name>
                return PathBuf::from(path_str).join("bin").join(tool_name);
            }
        }

        // B. Fallback for '--user': The manual default path ($HOME/.local/bin)
        if let Ok(home) = env::var("HOME") {
            log_debug!("[Pip Path] Falling back to $HOME/.local/bin/.");
            // Returns $HOME/.local/bin/<tool_name>
            return PathBuf::from(home).join(".local").join("bin").join(tool_name);
        }

    } else {
        // --- 2. SYSTEM-WIDE INSTALL PATH ---
        log_debug!("[Pip Path] Determining path for system-wide install.");

        // A. Try to derive path from 'pip3 show' (best system guess)
        let package_name = tool_name; // Assuming tool_name is the package name
        let command = &["pip3", "show", package_name];
        if let Some(path_str) = execute_and_parse_command(command, "Location:") {
            let mut location_path = PathBuf::from(path_str);

            // Brittle pop() logic to convert Location (/.../lib/site-packages) to Bin path
            if location_path.pop() && location_path.pop() && location_path.pop() {
                log_debug!("[Pip Path] Derived system path from 'pip3 show Location'.");
                return location_path.join("bin").join(tool_name);
            }
        }

        // B. Final System Fallback: Standard system path
        log_debug!("[Pip Path] Defaulting to /usr/local/bin/.");
        return PathBuf::from("/usr/local/bin/").join(tool_name);
    }

    // --- 3. LAST RESORT DEFAULT ---
    // This is reached only if it was a '--user' install AND both Python query and $HOME failed.
    if let Ok(home) = env::var("HOME") {
        // As per your final requirement, default to $HOME/bin/
        log_error!("[Pip Path] CRITICAL: Failed all user-local checks. Defaulting to $HOME/bin/.");
        return PathBuf::from(home).join("bin").join(tool_name);
    }

    // If HOME is not even set (extremely rare)
    log_error!("[Pip Path] CRITICAL: HOME not set. Defaulting to /bin/.");
    PathBuf::from("/bin/").join(tool_name)
}

// Helper function: Executes a command and tries to extract a key/value path (Kept for external command parsing).
fn execute_and_parse_command(command_parts: &[&str], key: &str) -> Option<String> {
    let program = command_parts.get(0)?;
    let args = command_parts.get(1..)?;

    match Command::new(program).args(args).output() {
        Ok(output) if output.status.success() => {
            let output_str = String::from_utf8_lossy(&output.stdout);

            for line in output_str.lines() {
                // Check for the specific key (e.g., "Location:", "USER_BASE:")
                if line.starts_with(key) {
                    let path_str = line.splitn(2, ':').nth(1).unwrap_or("").trim();
                    if !path_str.is_empty() {
                        return Some(path_str.to_string());
                    }
                }
            }
        }
        _ => {}
    }
    None
}
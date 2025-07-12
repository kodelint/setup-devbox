// This module provides the functionality to install software tools using Homebrew.
// Homebrew is a popular package manager for macOS and Linux, simplifying the installation
// of many open-source tools. This installer interacts directly with the `brew` command.

// For pretty, colored output in the terminal logs.
use colored::Colorize;
// Our custom logging macros for different verbosity levels.
use crate::{log_debug, log_error, log_info, log_warn};
// Importing the necessary data structures (schemas) for defining a tool in our config
// and storing its installation state.
use crate::schema::{ToolEntry, ToolState};
// For building and manipulating file paths.
use std::path::PathBuf;
// For executing external commands, specifically the `brew` command.
use std::process::Command;

/// Installs a tool using the Homebrew package manager.
/// This function executes `brew install <tool_name>` and determines the
/// final installation path to record it in the application's state.
///
/// # Arguments
/// * `tool`: A reference to a `ToolEntry` struct, which contains the
///           configuration details for the tool to be installed (e.g., `name`, `version`, `rename_to`).
///
/// # Returns
/// * `Option<ToolState>`:
///   - `Some(ToolState)` if the tool was successfully installed, including its version
///     and the detected installation path.
///   - `None` if the installation failed at any point (e.g., tool name missing, `brew` command fails).
pub fn install(tool: &ToolEntry) -> Option<ToolState> {
    log_debug!("[Brew] Starting installation process for tool: {:?}", tool.name.bold());

    // 1. Validate Tool Name
    // Ensure that a tool name is provided in the configuration.
    let name = &tool.name;
    if name.is_empty() {
        log_error!("[Brew] Tool name is empty in the configuration. Cannot proceed with Homebrew installation.");
        return None; // Cannot install a tool without a name.
    }

    // 2. Prepare and Execute Homebrew Installation Command
    // Create a new `Command` instance to run `brew`.
    let mut cmd = Command::new("brew");

    // Add the "install" argument, followed by the tool's name.
    // Note: Homebrew typically handles finding the latest stable version by default.
    // If specific versioning (e.g., `brew install <formula>@<version>`) were desired,
    // additional logic would be needed here to parse `tool.version` and potentially
    // manage Homebrew taps. For simplicity, we assume latest stable.
    cmd.arg("install").arg(name);

    log_info!("[Brew] Attempting to install {} using Homebrew...", name.bold());
    log_debug!("[Brew] Executing command: {:?}", cmd);

    // Execute the `brew install` command and wait for its output.
    let output = match cmd.output() {
        Ok(output) => output, // Command executed successfully, get its output (stdout, stderr, status).
        Err(err) => {
            // If the `brew` command itself could not be executed (e.g., `brew` not in PATH).
            log_error!("[Brew] Failed to execute `brew` command for {}: {}. Is Homebrew installed and in your system's PATH?", name.red(), err);
            return None;
        }
    };

    // Check if the `brew install` command exited successfully (status code 0).
    if !output.status.success() {
        // If it failed, log the error status and the stderr output from Homebrew.
        log_error!(
            "[Brew] Homebrew installation for {} failed with status: {}. stderr: \n{}",
            name.red(),
            output.status,
            String::from_utf8_lossy(&output.stderr).red() // Display brew's error message.
        );
        return None;
    }

    log_info!("[Brew] Successfully installed {}!", name.green());

    // 3. Determine the Installation Path
    // Homebrew installs binaries into a specific directory, which can vary (e.g., `/usr/local/bin`
    // on Intel macOS or `/opt/homebrew/bin` on Apple Silicon macOS).
    // We query `brew --prefix` to get the base installation path.
    let brew_prefix_output = Command::new("brew")
        .arg("--prefix")
        .output()
        .expect("[Brew] Failed to execute `brew --prefix`. Is Homebrew installed?"); // This `expect` would panic if brew isn't found.

    let brew_prefix = if brew_prefix_output.status.success() {
        // If `brew --prefix` was successful, trim whitespace and convert to string.
        String::from_utf8_lossy(&brew_prefix_output.stdout).trim().to_string()
    } else {
        // If `brew --prefix` failed, log a warning and default to a common path.
        // This fallback might not be correct for all systems/architectures.
        log_warn!("[Brew] Could not reliably determine Homebrew prefix. Defaulting to `/usr/local`. Installation path might be incorrect.");
        "/usr/local".to_string()
    };
    log_debug!("[Brew] Homebrew prefix detected: {}", brew_prefix.blue());

    // Construct the expected full path to the installed binary.
    // Homebrew typically symlinks binaries into a `bin` directory under its prefix.
    let bin_name = tool.rename_to.clone().unwrap_or_else(|| name.clone());
    let install_path = PathBuf::from(format!("{}/bin/{}", brew_prefix, bin_name));

    log_debug!("[Brew] Expected final binary path for {}: {:?}", name.bold(), install_path.display().to_string().cyan());

    // 4. Return ToolState for Tracking
    // Create and return a `ToolState` object to record this successful installation.
    Some(ToolState {
        // Use the version from the config, or default to "latest" since Homebrew handles versioning.
        version: tool.version.clone().unwrap_or_else(|| "latest".to_string()),
        // The detected installation path.
        install_path: install_path.display().to_string(),
        // Mark that this tool was installed by `setup-devbox`.
        installed_by_devbox: true,
        // The installation method used.
        install_method: "brew".to_string(),
        // Any `rename_to` specified in the config.
        renamed_to: tool.rename_to.clone(),
        // We can denote the package type as "brew" for consistency.
        package_type: "brew".to_string(),
        // Not required for `Homebrew` installations
        // This is just placeholder for symmetry
        repo: Option::from("UNKNOWN_FROM_CONFIG".to_string()),
        // Not required for `Homebrew` installations
        // This is just placeholder for symmetry
        tag: Option::from("UNKNOWN_FROM_CONFIG".to_string()),
    })
}
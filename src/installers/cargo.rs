// This module provides the functionality to install Rust crates using the `cargo install` command.
// It acts as a wrapper around the Cargo build system, allowing `devbox` to manage Rust-based tools
// distributed as crates.
//
// The goal is to integrate Rust tool installation seamlessly into the `devbox` ecosystem,
// handling common scenarios like specifying versions and passing additional Cargo options.

// Standard library imports:
use std::path::PathBuf;  // For ergonomic and platform-agnostic path manipulation.
use std::process::{Command, Output}; // For executing external commands (like `cargo`) and capturing their output.

// External crate imports:
use colored::Colorize; // Used for adding color to terminal output, improving log readability.

// Internal module imports:
use crate::schema::{ToolEntry, ToolState};
// `ToolEntry`: Defines the structure for a tool's configuration as read from `tools.yaml`,
//              providing details specific to a Cargo-based installation (e.g., crate name, version).
// `ToolState`: Represents the state of an installed tool, which we persist in `state.json`
//              to track installed tools, their versions, and paths.

use crate::{log_debug, log_error, log_info, log_warn};
// Custom logging macros. These are used throughout the module to provide informative output
// during the installation process, aiding in debugging and user feedback.


/// Installs a Rust crate using the `cargo install` command.
///
/// This is the core function for installing tools that are distributed as Rust crates.
/// It constructs the appropriate `cargo install` command based on the `ToolEntry` configuration,
/// executes it, and handles the outcome (success/failure), including logging and updating
/// the tool's state.
///
/// # Arguments
/// * `tool_entry`: A reference to a `ToolEntry` struct. This `ToolEntry` contains all the
///                 metadata read from the `tools.yaml` configuration file that specifies
///                 how to install this particular tool as a Cargo crate (e.g., `name`,
///                 `version`, `options` like `--features`).
///
/// # Returns
/// * `Option<ToolState>`:
///   - `Some(ToolState)`: Indicates a successful installation. The contained `ToolState`
///     struct provides details like the installed version, the absolute path to the binary,
///     and the installation method, which are then persisted in our internal `state.json`.
///   - `None`: Signifies that the installation failed at some step. Detailed error logging
///     is performed before returning `None` to provide context for the failure.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    // Start the installation process with a debug log, clearly indicating which tool (crate) is being processed.
    log_debug!("[Cargo Installer] Attempting to install Cargo tool: {}", tool_entry.name.bold());

    // 1. Basic Validation: Check if `cargo` command is available.
    // Before attempting any installation, we must ensure that the Rust toolchain (specifically `cargo`)
    // is installed and accessible in the system's PATH. This prevents cryptic errors later.
    if Command::new("cargo").arg("--version").output().is_err() {
        log_error!(
            "[Cargo Installer] 'cargo' command not found. Please ensure Rust/Cargo is installed and in your PATH."
        );
        return None; // Cannot proceed without `cargo`.
    }
    log_debug!("[Cargo Installer] 'cargo' command found in PATH.");

    // 2. Prepare `cargo install` Command Arguments
    // Initialize the base command arguments. `install` and the `tool_entry.name` (the crate name)
    // are always required for `cargo install`.
    let mut command_args = vec!["install", &tool_entry.name];

    // Handle `version` if specified in `tools.yaml`.
    // If a specific version is provided, append `--version <VERSION>` to the command.
    if let Some(version) = &tool_entry.version {
        command_args.push("--version");
        command_args.push(version);
        log_debug!("[Cargo Installer] Installing specific version: {}", version.cyan());
    }

    // Add any additional options from `tool_entry.options`.
    // This allows users to pass arbitrary flags to `cargo install`, such as `--features`, `--locked`, etc.
    if let Some(options) = &tool_entry.options {
        log_debug!("[Cargo Installer] Adding custom options: {:?}", options);
        for opt in options {
            command_args.push(opt);
        }
    }

    // Log the full command that will be executed for debugging and user visibility.
    log_info!("[Cargo Installer] Executing: {} {}", "cargo".cyan().bold(), command_args.join(" ").cyan());

    // 3. Execute `cargo install` Command
    // Spawn the `cargo` command with the prepared arguments and capture its standard output and error.
    let output: Output = match Command::new("cargo")
        .args(&command_args)
        .output() // Execute the command and wait for it to complete, capturing output.
    {
        Ok(out) => out, // Command executed successfully (process started and finished).
        Err(e) => {
            // Log if the command itself failed to spawn (e.g., permissions, `cargo` not found,
            // though the initial check should prevent most of this).
            log_error!("[Cargo Installer] Failed to execute 'cargo install' command for '{}': {}", tool_entry.name.bold().red(), e);
            return None;
        }
    };

    // 4. Check Command Execution Result
    // Evaluate the `output.status.success()` to determine if `cargo install` exited with a zero status code.
    if output.status.success() {
        log_info!(
            "[Cargo Installer] Successfully installed Cargo tool: {}",
            tool_entry.name.bold().green()
        );
        // Log stdout and stderr, even on success, as they might contain useful information or warnings.
        if !output.stdout.is_empty() {
            log_debug!("[Cargo Installer] Stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            log_warn!("[Cargo Installer] Stderr (might contain warnings): {}", String::from_utf8_lossy(&output.stderr));
        }

        // 5. Determine Installation Path
        // Cargo typically installs binaries into `~/.cargo/bin/`. We need to construct this path.
        // We prioritize getting the `HOME` environment variable.
        let install_path = if let Ok(mut home) = std::env::var("HOME") {
            home.push_str("/.cargo/bin/"); // Append the standard Cargo bin directory.
            // Join the bin directory with the tool's name to get the full binary path.
            PathBuf::from(home).join(&tool_entry.name).to_string_lossy().into_owned()
        } else {
            // Fallback path if the HOME environment variable is not set.
            // This is a less reliable default but provides a reasonable guess.
            log_warn!("[Cargo Installer] HOME environment variable not set, defaulting install path to /usr/local/bin/");
            "/usr/local/bin/".to_string()
        };
        log_debug!("[Cargo Installer] Determined installation path: {}", install_path.cyan());

        // 6. Return `ToolState` for Tracking
        // If the installation was successful, construct a `ToolState` object. This object
        // captures all relevant information about the installed tool, allowing `devbox` to
        // track it in its internal state file (`state.json`).
        Some(ToolState {
            // The version recorded for the tool. Uses the specified version or "latest" as a fallback.
            version: tool_entry.version.clone().unwrap_or_else(|| "latest".to_string()),
            // The absolute path where the tool's executable was installed.
            install_path,
            // Flag indicating that this tool was installed by `devbox`.
            installed_by_devbox: true,
            // The method of installation, useful for future diagnostics or differing update logic.
            install_method: "cargo-install".to_string(),
            // Records if the binary was renamed during installation. For `cargo install`, this is
            // usually `None` unless `--bin` or `--example` flags are used in `options`.
            renamed_to: tool_entry.rename_to.clone(),
            // Specifies the type of package.
            package_type: "rust-crate".to_string(),
            // These fields are specific to GitHub releases and are not applicable for `cargo install`.
            repo: None,
            tag: None,
            // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
            options: tool_entry.options.clone(),
            // For direct URL installations: The original URL from which the tool was downloaded.
            url: tool_entry.url.clone(),
            executable_path_after_extract: None,
        })
    } else {
        // 7. Handle Failed Installation
        // If `cargo install` exited with a non-zero status code, it indicates a failure.
        // Capture and log the standard error output for debugging purposes.
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error!(
            "[Cargo Installer] Failed to install Cargo tool '{}'. Exit code: {}. Error: {}",
            tool_entry.name.bold().red(),
            output.status.code().unwrap_or(-1), // Get the actual exit code, or -1 if unavailable.
            stderr.red()
        );
        // Also log standard output on failure, as it might contain context for the error.
        if !output.stdout.is_empty() {
            log_debug!("[Cargo Installer] Stdout (on failure): {}", String::from_utf8_lossy(&output.stdout));
        }
        None // Return `None` to indicate that the installation failed.
    }
}
// Imports necessary schema definitions for tools.
// ToolEntry: Represents the configuration for a single tool as defined in tools.yaml.
// ToolState: Represents the recorded state of an installed tool, stored in state.json.
use crate::schemas::sdb_schema::{ToolEntry, ToolState};
// Imports custom logging macros from the crate root.
// These macros (`log_debug`, `log_error`, `log_info`, `log_warn`) provide
// a standardized way to output messages to the console with different severity levels,
// making it easier to track the application's flow and diagnose issues.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to terminal output.
// The `colored` crate allows us to make log messages and other terminal output more readable
// by applying colors (e.g., `.blue()`, `.green()`, `.red()`).
use colored::Colorize;
// For executing external commands and capturing their output.
// `std::process::Command` is used to run `rustup` commands.
// `std::process::Output` captures the stdout, stderr, and exit status of executed commands.
use std::process::{Command, Output};
// For working with file paths, specifically to construct installation paths.
// `PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::PathBuf;
// For getting environment variables, like HOME.
// `std::env` is used to find the user's home directory to determine rustup's installation path.
use crate::libs::utilities::assets::current_timestamp;
use std::env;

/// Installs a Rust toolchain and optionally its components using `rustup`.
///
/// This function acts as the installer module for `rustup`-managed Rust environments.
/// It performs the following steps:
/// 1. Verifies `rustup` is installed and accessible in the system's PATH.
/// 2. Extracts the desired Rust toolchain version from `tool_entry.version`. This field is mandatory for `rustup` source.
/// 3. Executes `rustup toolchain install <toolchain_name>` to set up the base Rust environment.
/// 4. If `tool_entry.options` are provided, it iterates through them, treating each as a
///    Rust component (e.g., `rust-analyzer`, `clippy`, `rustfmt`) and installs them
///    using `rustup component add <component_name> --toolchain <toolchain_name>`.
/// 5. Determines the expected installation path for the toolchain's binaries (typically in `~/.rustup/toolchains/<toolchain_name>/bin`).
/// 6. Constructs and returns a `ToolState` object to record the successful installation
///    in the application's global state (`state.json`).
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct containing details for the Rust toolchain
///   as defined in `tools.yaml`.
///   - `tool_entry.name`: Used for logging and identification within the DevBox system (e.g., "rust").
///   - `tool_entry.version`: The *required* Rust toolchain name (e.g., "stable", "nightly", "1.70.0").
///   - `tool_entry.options`: An optional list of `rustup` components to install for this toolchain.
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if the installation of the toolchain and all specified components was successful.
///   The `ToolState` includes the installed version, path, and other metadata.
/// * `None` if `rustup` is not found, a toolchain version is missing from the config,
///   or if any `rustup` command (toolchain install or component add) fails.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[Rustup Installer] Attempting to install Rust-related tools based on entry: {}",
        tool_entry.name.bold()
    );
    log_debug!("[Rustup Installer] ToolEntry details: {:#?}", tool_entry);

    // 1. Basic validation: Ensure 'rustup' command is available on the system.
    // We check for its existence by attempting to run `rustup --version`.
    // If it fails, it means rustup is not in the PATH or not installed.
    if Command::new("rustup").arg("--version").output().is_err() {
        log_error!(
            "[Rustup Installer] 'rustup' command not found. Please ensure Rustup is installed and in your PATH."
        );
        return None; // Cannot proceed without rustup.
    }

    // Determine the toolchain to install.
    // The `version` field from `ToolEntry` is crucial here and must be present.
    // If it's missing, we log an error and exit gracefully.
    let toolchain_name = match &tool_entry.version {
        Some(v) => v.clone(), // Clone the string for ownership.
        None => {
            log_error!(
                "[Rustup Installer] Toolchain version (e.g., 'stable', 'nightly', '1.70.0') is required for rustup tool '{}'.",
                tool_entry.name.bold().red() // Highlight the tool name in error.
            );
            return None; // Toolchain version is a mandatory field for rustup installations.
        }
    };

    // 2. Install the primary Rust toolchain.
    // This command downloads and installs the specified Rust toolchain.
    let toolchain_command_args = vec!["toolchain", "install", &toolchain_name];

    log_info!(
        "[Rustup Installer] Executing: {} {}",
        "rustup".cyan().bold(),
        toolchain_command_args.join(" ").cyan()
    );

    let output_toolchain: Output = match Command::new("rustup")
        .args(&toolchain_command_args) // Pass all arguments to the `rustup` command.
        .output() // Execute the command and capture its output.
    {
        Ok(out) => out, // Successfully executed the command.
        Err(e) => {
            // Log error if the command itself failed to execute (e.g., permissions, command not found).
            log_error!("[Rustup Installer] Failed to execute 'rustup toolchain install' for '{}': {}", toolchain_name.bold().red(), e);
            return None; // Indicate installation failure.
        }
    };

    // Check the exit status of the toolchain installation command.
    if !output_toolchain.status.success() {
        let stderr = String::from_utf8_lossy(&output_toolchain.stderr);
        log_error!(
            "[Rustup Installer] Failed to install toolchain '{}'. Exit code: {}. Error: {}",
            toolchain_name.bold().red(), // Toolchain name
            output_toolchain.status.code().unwrap_or(-1), // Exit code (default to -1 if not available)
            stderr.red()                                  // Standard error output from rustup
        );
        // Provide debug info if stdout also has content on failure.
        if !output_toolchain.stdout.is_empty() {
            log_debug!(
                "[Rustup Installer] Stdout (on failure): {}",
                String::from_utf8_lossy(&output_toolchain.stdout)
            );
        }
        return None; // Installation failed.
    } else {
        // Log success message and any stdout/stderr for diagnostic purposes.
        log_info!(
            "[Rustup Installer] Successfully installed toolchain: {}",
            toolchain_name.bold().green()
        );
        if !output_toolchain.stdout.is_empty() {
            log_debug!(
                "[Rustup Installer] Stdout: {}",
                String::from_utf8_lossy(&output_toolchain.stdout)
            );
        }
        if !output_toolchain.stderr.is_empty() {
            log_warn!(
                "[Rustup Installer] Stderr (might contain warnings): {}",
                String::from_utf8_lossy(&output_toolchain.stderr)
            );
        }
    }

    // 3. Install components if specified in `tool_entry.options`.
    // This loop iterates through the list of components (if any) and adds each one
    // to the newly installed toolchain.
    if let Some(components) = &tool_entry.options {
        for component in components {
            // Command to add a component to a specific toolchain.
            let component_command_args = vec![
                "component",
                "add",
                component,
                "--toolchain",
                &toolchain_name,
            ];

            log_info!(
                "[Rustup Installer] Executing: {} {}",
                "rustup".cyan().bold(),
                component_command_args.join(" ").cyan()
            );

            let output_component: Output = match Command::new("rustup")
                .args(&component_command_args)
                .output()
            {
                Ok(out) => out,
                Err(e) => {
                    // Log error if component command failed to execute.
                    log_error!(
                        "[Rustup Installer] Failed to execute 'rustup component add' for '{}' on toolchain '{}': {}",
                        component.bold().red(),      // Component name
                        toolchain_name.bold().red(), // Toolchain name
                        e                            // Error details
                    );
                    // Decided to return None here. If a core component fails,
                    // the entire Rust setup might be considered incomplete/broken.
                    return None;
                }
            };

            if !output_component.status.success() {
                // Log detailed error if adding a component failed.
                log_error!(
                    "[Rustup Installer] Failed to add component '{}'. Exit code: {}. Stderr: {}",
                    component.bold().red(), // Component name (Argument 1)
                    output_component.status.code().unwrap_or(-1), // Exit code (Argument 2)
                    String::from_utf8_lossy(&output_component.stderr)
                        .trim()
                        .red()  // Stderr (Argument 3)
                );
                // Similar to toolchain installation, a failed component addition
                // is treated as a fatal error for this tool's installation.
                // If you want to continue installing other components even if one fails,
                // you would remove this `return None;` and simply log the error.
                return None;
            } else {
                // Log success for component addition.
                log_info!(
                    "[Rustup Installer] Successfully added component '{}' to toolchain '{}'.",
                    component.bold().green(),
                    toolchain_name.bold().green()
                );
                if !output_component.stdout.is_empty() {
                    log_debug!(
                        "[Rustup Installer] Stdout: {}",
                        String::from_utf8_lossy(&output_component.stdout)
                    );
                }
                if !output_component.stderr.is_empty() {
                    log_warn!(
                        "[Rustup Installer] Stderr (might contain warnings): {}",
                        String::from_utf8_lossy(&output_component.stderr)
                    );
                }
            }
        }
    }

    // Determine the installation path.
    // Rustup typically installs toolchains under `~/.rustup/toolchains/<toolchain_name>/bin`.
    // We construct this path to record it in `ToolState` for future reference.
    let install_path = if let Ok(mut home) = env::var("HOME") {
        home.push_str(&format!("/.rustup/toolchains/{}/bin", toolchain_name));
        PathBuf::from(home).to_string_lossy().into_owned()
    } else {
        // Fallback if HOME directory cannot be determined, though this might be less accurate
        // for rustup which heavily relies on the user's home directory structure.
        log_warn!(
            "[Rustup Installer] Could not determine HOME directory. Using generic fallback for toolchain path."
        );
        "/usr/local/bin/".to_string()
    };

    // Return ToolState for Tracking
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed. This is crucial
    // for future operations like uninstallation, updates, or syncing.
    Some(ToolState {
        // The version field for tracking. Defaults to "latest" if not explicitly set in `tools.yaml`.
        // The installed toolchain version.
        version: toolchain_name,
        // The canonical path where the tool's executable was installed. This is the path
        // that will be recorded in the `state.json` file.
        // The path where the toolchain binaries are expected.
        install_path,
        // Flag indicating that this tool was installed by `setup-devbox`. This helps distinguish
        // between tools managed by our system and those installed manually.
        installed_by_devbox: true,
        // The method of installation, useful for future diagnostics or differing update logic.
        // In this module, it's always "rustup".
        install_method: "rustup".to_string(),
        // Records if the binary was renamed during installation, storing the new name.
        // Usually not useful for `rustup` installers
        renamed_to: tool_entry.rename_to.clone(),
        // `repo` and `tag` are not typically applicable for rust installations
        // as rust fetches from crate or other package indexes, not directly from Git repositories.
        // Therefore, these fields are set to `None` for clarity and clean state.json.
        repo: None,
        tag: None,
        // The actual package type detected by the `file` command or inferred. This is for diagnostic
        // purposes, providing the most accurate type even if the installation logic
        // used a filename-based guess (e.g., "binary", "macos-pkg-installer").
        package_type: "rust-toolchain".to_string(),
        // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
        // Store the components that were attempted to be added with `rustup`
        options: tool_entry.options.clone(),
        // For direct URL installations: The original URL from which the tool was downloaded.
        // This is important for re-downloading or verifying in the future.
        // Not used for rustup
        url: tool_entry.url.clone(),
        // Record the timestamp when the tool was installed or updated
        last_updated: Some(current_timestamp()),
        // This field is currently `None` but could be used to store the path to an executable
        // *within* an extracted archive if `install_path` points to the archive's root.
        executable_path_after_extract: None,
        // Record any additional commands that were executed during installation.
        // This is useful for tracking what was done and potentially for cleanup during uninstall.
        additional_cmd_executed: tool_entry.additional_cmd.clone(),
    })
}

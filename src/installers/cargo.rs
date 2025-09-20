// This module provides the functionality to install Rust crates using the `cargo install` command.
// It acts as a wrapper around the Cargo build system, allowing `devbox` to manage Rust-based tools
// distributed as crates.
//
// The goal is to integrate Rust tool installation seamlessly into the `devbox` ecosystem,
// handling common scenarios like specifying versions and passing additional Cargo options.

// Standard library imports:
// `std::path::PathBuf`: Provides an owned, OS-agnostic path. It's used here for building the
//                       installation path of the binary (e.g., `~/.cargo/bin/my-tool`).
use std::path::PathBuf;
// `std::process::{Command, Output}`: This is the core of the module's functionality.
//   - `Command`: A builder for a new process, used to construct and configure the `cargo install` command.
//   - `Output`: Represents the output of a finished process, containing its exit status, stdout, and stderr.
use std::process::{Command, Output};

// External crate imports:
// `colored::Colorize`: A simple library for adding color to strings in the terminal.
//                      Used to make log messages more readable and highlight key information.
use colored::Colorize;

// Internal module imports:
// `ToolEntry`: Represents a single tool's configuration as defined in your `tools.yaml` file.
//              It's a struct that contains all possible configuration fields for a tool,
//              such as name, version, source, URL, repository, etc.
// `ToolState`: Represents the actual state of an *installed* tool. This struct is used to
//              persist information about installed tools in the application's `state.json` file.
//              It helps `setup-devbox` track what's installed, its version, and where it's located.
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;
// A helper function to get the current UNIX timestamp. Used for the `last_updated` field.
use crate::libs::utilities::assets::current_timestamp;
// `crate::{log_debug, log_error, log_info, log_warn}`: Custom logging macros for structured output.
//   - `log_debug!`: For detailed, developer-focused logs.
//   - `log_error!`: For critical failures.
//   - `log_info!`: For general progress updates.
//   - `log_warn!`: For non-critical issues or warnings.
use crate::{log_debug, log_error, log_info, log_warn};

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
    // We try to run `cargo --version`. If this command fails to spawn (`.is_err()`), it means `cargo`
    // is not found, so we log an error and exit gracefully.
    if Command::new("cargo").arg("--version").output().is_err() {
        log_error!(
            "[Cargo Installer] 'cargo' command not found. Please ensure Rust/Cargo is installed and in your PATH."
        );
        return None; // Cannot proceed without `cargo`.
    }
    log_debug!("[Cargo Installer] 'cargo' command found in PATH.");

    // 2. Check if this is a git-based installation
    // We look at the `options` field in `tool_entry` to see if it contains the `--git` flag.
    // This determines which `cargo install` command variant to build.
    let is_git_install = tool_entry
        .options
        .as_ref()
        .map_or(false, |options| options.iter().any(|opt| opt.starts_with("--git")));

    // 3. Prepare `cargo install` Command Arguments
    // We initialize a vector to hold the arguments for the `cargo` command.
    let mut command_args = Vec::new();
    command_args.push("install".to_string());

    if is_git_install {
        // For git installations, handle git-specific options and version logic.
        // This helper function will populate `command_args` with the appropriate arguments.
        prepare_git_install_command(&mut command_args, tool_entry);
    } else {
        // For regular crate installations.
        // This helper function handles the standard `cargo install <crate_name> --version <version>` format.
        prepare_crate_install_command(&mut command_args, tool_entry);
    }

    // Log the full command that will be executed for debugging and user visibility.
    // The `colored` crate is used here to highlight the command for clarity.
    log_info!(
        "[Cargo Installer] Executing: {} {}",
        "cargo".cyan().bold(),
        command_args.join(" ").cyan()
    );

    // 4. Execute `cargo install` Command
    // Spawn the `cargo` command with the prepared arguments and capture its standard output and error.
    let output: Output = match Command::new("cargo")
        .args(&command_args) // Pass the vector of arguments.
        .output() // Execute the command and wait for it to complete, capturing output.
    {
        Ok(out) => out, // Command executed successfully (the process started and finished).
        Err(e) => {
            // Log if the command itself failed to spawn (e.g., permissions, `cargo` not found,
            // though the initial check should prevent most of this).
            log_error!("[Cargo Installer] Failed to execute 'cargo install' command for '{}': {}", tool_entry.name.bold().red(), e);
            return None; // Return `None` to indicate a failure.
        }
    };

    // 5. Check Command Execution Result
    // Evaluate the `output.status.success()` to determine if `cargo install` exited with a zero status code.
    if output.status.success() {
        log_info!(
            "[Cargo Installer] Successfully installed Cargo tool: {}",
            tool_entry.name.bold().green()
        );
        // Log stdout and stderr, even on success, as they might contain useful information or warnings.
        if !output.stdout.is_empty() {
            log_debug!(
                "[Cargo Installer] Stdout: {}",
                String::from_utf8_lossy(&output.stdout) // Convert byte buffer to string for logging.
            );
        }
        if !output.stderr.is_empty() {
            log_debug!(
                "[Cargo Installer] Stderr (might contain warnings): {}",
                String::from_utf8_lossy(&output.stderr) // Convert byte buffer to string.
            );
        }

        // 6. Determine Installation Path
        // Cargo typically installs binaries into `~/.cargo/bin/`. We need to construct this path dynamically.
        // `std::env::var("HOME")` safely gets the user's home directory.
        let install_path = if let Ok(mut home) = std::env::var("HOME") {
            home.push_str("/.cargo/bin/"); // Append the standard Cargo bin directory.
            // Join the bin directory with the tool's name to get the full binary path.
            PathBuf::from(home)
                .join(&tool_entry.name)
                .to_string_lossy() // Converts `PathBuf` to an owned string, handling platform differences.
                .into_owned()
        } else {
            // Fallback path if the HOME environment variable is not set.
            // This is a less reliable default but provides a reasonable guess for common systems.
            log_warn!(
                "[Cargo Installer] HOME environment variable not set, defaulting install path to /usr/local/bin/"
            );
            "/usr/local/bin/".to_string()
        };
        log_debug!("[Cargo Installer] Determined installation path: {}", install_path.cyan());

        // 7. Return ToolState for Tracking
        // Construct a `ToolState` object to record the details of this successful installation.
        // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
        // what tools are installed, where they are, and how they were installed.
        Some(ToolState {
            // The version recorded for the tool. Uses the specified version or "latest" as a fallback.
            version: determine_installed_version(tool_entry, is_git_install),
            // The canonical path where the tool's executable was installed. This is the path
            // that will be recorded in the `state.json` file.
            install_path,
            // Flag indicating that this tool was installed by `setup-devbox`. This helps distinguish
            // between tools managed by our system and those installed manually.
            installed_by_devbox: true,
            // The method of installation, useful for future diagnostics or differing update logic.
            // In this module, it's always "cargo-install".
            install_method: "cargo-install".to_string(),
            // Records if the binary was renamed during installation. For `cargo install`, this is
            // usually `None` unless `--bin` or `--example` flags are used in `options`.
            renamed_to: tool_entry.rename_to.clone(),
            // The actual package type detected by the `file` command or inferred. This is for diagnostic
            // purposes, providing the most accurate type even if the installation logic
            // used a filename-based guess (e.g., "binary", "macos-pkg-installer").
            package_type: "rust-crate".to_string(),
            // These fields are specific to GitHub releases and are not applicable for `cargo install`.
            repo: None,
            tag: None,
            // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
            // For git installations, we convert version to --tag in options
            options: get_updated_cargo_options_for_state(tool_entry, is_git_install),
            // For direct URL installations: The original URL from which the tool was downloaded.
            // This is important for re-downloading or verifying in the future.
            url: tool_entry.url.clone(),
            // Record the timestamp when the tool was installed or updated
            last_updated: Some(current_timestamp()),
            // This field is currently `None` but could be used to store the path to an executable
            // *within* an extracted archive if `install_path` points to the archive's root.
            executable_path_after_extract: None,
            // Record any additional commands that were executed during installation.
            // This is useful for tracking what was done and potentially for cleanup during uninstall.
            additional_cmd_executed: tool_entry.additional_cmd.clone(),
            configuration_manager: None,
        })
    } else {
        // 8. Handle Failed Installation
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
            log_debug!(
                "[Cargo Installer] Stdout (on failure): {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        None // Return `None` to indicate that the installation failed.
    }
}

/// Prepares command arguments for a regular crate installation.
///
/// This function constructs the command arguments for `cargo install` when installing from `crates.io`.
/// It handles the crate name, version, and any additional options.
///
/// # Arguments
/// * `command_args`: A mutable reference to the vector of strings that will hold the arguments.
/// * `tool_entry`: The `ToolEntry` containing the configuration.
fn prepare_crate_install_command(command_args: &mut Vec<String>, tool_entry: &ToolEntry) {
    // Add the crate name as the first argument after "install".
    command_args.push(tool_entry.name.clone());

    // Handle the `--version` argument if a version is specified in the `tools.yaml`.
    if let Some(version) = &tool_entry.version {
        command_args.push("--version".to_string());
        command_args.push(version.clone());
        log_debug!("[Cargo Installer] Installing specific version: {}", version.cyan());
    }

    // Add any additional, user-defined options (like `--features`, `--locked`, etc.).
    if let Some(options) = &tool_entry.options {
        log_debug!("[Cargo Installer] Adding custom options: {:#?}", options);
        for opt in options {
            command_args.push(opt.clone());
        }
    }
}

/// Prepares command arguments for a git-based installation.
///
/// This function handles the more complex logic for `cargo install` from a git repository.
/// It correctly handles `--git`, `--branch`, `--tag`, and `--rev` options.
///
/// # Arguments
/// * `command_args`: A mutable reference to the vector of strings that will hold the arguments.
/// * `tool_entry`: The `ToolEntry` containing the configuration.
fn prepare_git_install_command(command_args: &mut Vec<String>, tool_entry: &ToolEntry) {
    // We can unwrap safely here because `is_git_install` guarantees `options` is `Some`.
    let options = tool_entry.options.as_ref().unwrap();

    // Check for the presence of specific git options to avoid conflicts.
    let has_branch = options.iter().any(|opt| opt.starts_with("--branch"));
    let has_tag = options.iter().any(|opt| opt.starts_with("--tag"));
    let has_rev = options.iter().any(|opt| opt.starts_with("--rev"));

    // Process each option from `tools.yaml`.
    for opt in options {
        // Handle options where the value is separated by a space (e.g., `--git URL`).
        if opt.contains(' ') {
            let parts: Vec<&str> = opt.split_whitespace().collect();
            for part in parts {
                command_args.push(part.to_string());
            }
        } else if opt.contains('=') {
            // Handle `--option=value` format.
            let parts: Vec<&str> = opt.splitn(2, '=').collect();
            command_args.push(parts[0].to_string());
            command_args.push(parts[1].to_string());
        } else {
            command_args.push(opt.clone());
        }
    }

    // Add the tool name at the end for git installations. Cargo requires the crate name to be
    // the last argument when a git URL is provided.
    command_args.push(tool_entry.name.clone());

    // If the user provided a `version` field but no explicit git-specific options,
    // we interpret the version as a git tag by default. This is a user-friendly assumption.
    if !has_branch && !has_tag && !has_rev {
        if let Some(version) = &tool_entry.version {
            command_args.push("--tag".to_string());
            command_args.push(version.clone());
            log_debug!("[Cargo Installer] Using version as git tag: {}", version.cyan());
        }
    } else {
        // If git options are explicitly set (e.g., `--branch`), the `version` field is ignored
        // to prevent conflicting behavior.
        log_debug!(
            "[Cargo Installer] Git-specific options present, ignoring version field: {:?}",
            tool_entry.version
        );
    }
}

/// Determines the appropriate version string for the installed tool.
///
/// This function decides what value to use for the `version` field in the `ToolState` struct,
/// based on the `tool_entry` and the installation method. It prioritizes the explicit
/// `version` field, then git-specific options, and finally falls back to "latest".
///
/// # Arguments
/// * `tool_entry`: The configuration for the tool.
/// * `is_git_install`: A boolean flag indicating if this was a git-based installation.
///
/// # Returns
/// Determines the appropriate version string for the installed tool
fn determine_installed_version(tool_entry: &ToolEntry, is_git_install: bool) -> String {
    // Priority 1: Use version from configuration if specified (for both crate and git)
    if let Some(version) = &tool_entry.version {
        return version.clone();
    }

    // Priority 2: For git installations, use git-specific options
    if is_git_install {
        let options = tool_entry.options.as_ref().unwrap();

        // Check for --tag first
        if let Some(tag_opt) = options.iter().find(|opt| opt.starts_with("--tag")) {
            // Extract tag value (assuming format is "--tag=value" or "--tag value")
            if let Some(tag_value) = tag_opt.split('=').nth(1) {
                return tag_value.to_string();
            } else if let Some(pos) = options.iter().position(|opt| opt == "--tag") {
                if let Some(tag_value) = options.get(pos + 1) {
                    return tag_value.clone();
                }
            }
        }

        // Check for --branch next
        if let Some(branch_opt) = options.iter().find(|opt| opt.starts_with("--branch")) {
            // Extract branch value
            if let Some(branch_value) = branch_opt.split('=').nth(1) {
                return format!("branch-{}", branch_value);
            } else if let Some(pos) = options.iter().position(|opt| opt == "--branch") {
                if let Some(branch_value) = options.get(pos + 1) {
                    return format!("branch-{}", branch_value);
                }
            }
        }

        // Check for --rev last (only first 7 characters)
        if let Some(rev_opt) = options.iter().find(|opt| opt.starts_with("--rev")) {
            // Extract rev value and take first 7 characters
            if let Some(rev_value) = rev_opt.split('=').nth(1) {
                let short_rev = if rev_value.len() > 7 {
                    &rev_value[..7]
                } else {
                    rev_value
                };
                return format!("rev-{}", short_rev);
            } else if let Some(pos) = options.iter().position(|opt| opt == "--rev") {
                if let Some(rev_value) = options.get(pos + 1) {
                    let short_rev = if rev_value.len() > 7 {
                        &rev_value[..7]
                    } else {
                        rev_value
                    };
                    return format!("rev-{}", short_rev);
                }
            }
        }
    }

    // Priority 3: Fallback to "latest"
    "latest".to_string()
}

/// Updates the options in ToolState to include --tag for git installations when version is specified
fn get_updated_cargo_options_for_state(
    tool_entry: &ToolEntry,
    is_git_install: bool,
) -> Option<Vec<String>> {
    if !is_git_install {
        return tool_entry.options.clone();
    }

    let mut updated_options = tool_entry.options.clone().unwrap_or_default();

    // If this is a git installation and version is specified, add --tag to options
    if let Some(version) = &tool_entry.version {
        // Check if git-specific options are already present
        let has_branch = updated_options.iter().any(|opt| opt.starts_with("--branch"));
        let has_tag = updated_options.iter().any(|opt| opt.starts_with("--tag"));
        let has_rev = updated_options.iter().any(|opt| opt.starts_with("--rev"));

        // Only add --tag if no other git-specific options are present
        if !has_branch && !has_tag && !has_rev {
            updated_options.push("--tag".to_string());
            updated_options.push(version.clone());
        }
    }

    Some(updated_options)
}

// This module provides the functionality to install software tools using Homebrew.
// Homebrew is a popular package manager for macOS and Linux, simplifying the installation
// of many open-source tools. This installer interacts directly with the `brew` command
// to install packages as specified in `devbox`'s configuration.
//
// The primary goal is to offer a straightforward and robust method for `devbox` to
// manage tools available through Homebrew.

// Standard library imports:
use std::path::PathBuf; // For ergonomic and platform-agnostic path manipulation.
use std::process::Command; // For executing external commands (like `brew`) and capturing their output.

// External crate imports:
use colored::Colorize; // Used for adding color to terminal output, improving log readability.

// Internal module imports:
use crate::schema::{ToolEntry, ToolState};
// `ToolEntry`: Defines the structure for a tool's configuration as read from `tools.yaml`,
//              providing details specific to a Homebrew installation (e.g., formula name).
// `ToolState`: Represents the state of an installed tool, which we persist in `state.json`
//              to track installed tools, their versions, and paths.

use crate::{log_debug, log_error, log_info, log_warn};
// Custom logging macros. These are used throughout the module to provide informative output
// during the installation process, aiding in debugging and user feedback.

/// Installs a tool using the Homebrew package manager.
///
/// This is the core function for installing tools that are available as Homebrew formulae.
/// It orchestrates the following steps:
/// 1. Validates the tool name from the `ToolEntry`.
/// 2. Executes the `brew install <tool_name>` command.
/// 3. Checks the success of the Homebrew command and logs any errors.
/// 4. Determines the actual installation path of the binary using `brew --prefix`.
/// 5. Constructs and returns a `ToolState` object for persistence.
///
/// # Arguments
/// * `tool`: A reference to a `ToolEntry` struct. This `ToolEntry` contains all the
///           metadata read from the `tools.yaml` configuration file that specifies
///           how to install this particular tool using Homebrew (e.g., `name` of the formula,
///           `rename_to` if the binary needs a different name).
///
/// # Returns
/// * `Option<ToolState>`:
///   - `Some(ToolState)`: Indicates a successful installation. The contained `ToolState`
///     struct provides details like the installed version, the absolute path to the binary,
///     and the installation method, which are then persisted in our internal `state.json`.
///   - `None`: Signifies that the installation failed at any point (e.g., missing tool name,
///     `brew` command not found, `brew install` failed). Detailed error logging is performed
///     before returning `None` to provide context for the failure.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!(
        "[Brew] Starting installation process for tool: {}",
        tool_entry.name.bold()
    );

    // 1. Validate Tool Name
    // Ensure that the `name` field is present in the `ToolEntry` and is not empty.
    // The tool name is essential for the `brew install` command.
    let name = &tool_entry.name;
    if name.is_empty() {
        log_error!(
            "[Brew] Tool name is empty in the configuration. Cannot proceed with Homebrew installation."
        );
        return None; // Abort if the tool name is missing.
    }
    log_debug!("[Brew] Tool name '{}' is validated.", name.blue());

    // 2. Prepare and Execute Homebrew Installation Command
    // Create a new `Command` instance to interact with the `brew` executable.
    let mut cmd = Command::new("brew");

    // Add the "install" subcommand and the tool's name (Homebrew formula).
    // Homebrew by default installs the latest stable version of a formula.
    // If specific versioning (e.g., `brew install <formula>@<version>`) were desired,
    // additional parsing of `tool.version` and logic to manage Homebrew taps would be needed.
    // For this implementation, we assume installation of the latest stable version.
    cmd.arg("install").arg(name);

    log_info!(
        "[Brew] Attempting to install or upgrade {} using Homebrew...",
        name.bold()
    );
    // Log the exact command being executed for debugging purposes.
    log_debug!("[Brew] Executing command: {:#?}", cmd);

    // Execute the `brew install` command and wait for it to complete, capturing its output.
    let output = match cmd.output() {
        Ok(output) => output, // Successfully executed the command (process started and finished).
        Err(err) => {
            // Log an error if the `brew` command itself could not be spawned.
            // This usually means `brew` is not installed or not in the system's PATH.
            log_error!(
                "[Brew] Failed to execute `brew` command for {}: {}. Is Homebrew installed and in your system's PATH?",
                name.red(),
                err
            );
            return None;
        }
    };

    // Check the exit status of the `brew install` command.
    if !output.status.success() {
        // If the command failed (non-zero exit code), log the error.
        // Include Homebrew's standard error output, as it typically contains detailed reasons for failure.
        log_error!(
            "[Brew] Homebrew installation for {} failed with status: {}. stderr: \n{}",
            name.red(),
            output.status, // Display the exit status code.
            String::from_utf8_lossy(&output.stderr).red() // Convert stderr bytes to a string and color it red.
        );
        return None; // Indicate installation failure.
    }

    log_info!("[Brew] Successfully installed {}!", name.green());

    // 3. Determine the Installation Path
    // Homebrew installs binaries into a specific directory, which varies based on macOS architecture
    // (e.g., `/usr/local/bin` on Intel, `/opt/homebrew/bin` on Apple Silicon).
    // The most reliable way to find this is by querying `brew --prefix`.
    let brew_prefix_output = Command::new("brew")
        .arg("--prefix")
        .output()
        // If `brew --prefix` itself fails to execute, it's a critical error indicating Homebrew setup issues.
        .expect("[Brew] Failed to execute `brew --prefix`. Is Homebrew installed?");

    let brew_prefix = if brew_prefix_output.status.success() {
        // If `brew --prefix` was successful, take its stdout, trim whitespace, and convert to a String.
        String::from_utf8_lossy(&brew_prefix_output.stdout)
            .trim()
            .to_string()
    } else {
        // If `brew --prefix` failed (e.g., even if `brew` is found, `--prefix` might error for some reason),
        // log a warning and default to a common, but potentially incorrect, path.
        log_warn!(
            "[Brew] Could not reliably determine Homebrew prefix. Defaulting to `/usr/local`. \
             Installation path might be incorrect. Stderr from --prefix: {}",
            String::from_utf8_lossy(&brew_prefix_output.stderr)
        );
        "/usr/local".to_string() // Fallback path.
    };
    log_debug!("[Brew] Homebrew prefix detected: {}", brew_prefix.blue());

    // Construct the expected full path to the installed binary.
    // Homebrew typically creates symlinks to installed binaries in a `bin` directory
    // located directly under its determined prefix (e.g., `/usr/local/bin/<tool_name>`).
    // The binary name itself might be renamed if `tool.rename_to` is specified.
    let bin_name = tool_entry.rename_to.clone().unwrap_or_else(|| name.clone());
    let install_path = PathBuf::from(format!("{}/bin/{}", brew_prefix, bin_name));

    log_debug!(
        "[Brew] Expected final binary path for {}: {}",
        name.bold(),
        install_path.display().to_string().cyan()
    );

    // 4. Return `ToolState` for Tracking
    // Create and return a `ToolState` object to record this successful installation in `devbox`'s state file.
    Some(ToolState {
        // The version field. Homebrew handles versions, so we can either use the `tool.version`
        // if specified (e.g., for specific formula@version syntax) or default to "latest"
        // to signify it's managed by Homebrew.
        version: tool_entry
            .version
            .clone()
            .unwrap_or_else(|| "latest".to_string()),
        // The detected absolute path to the installed binary.
        install_path: install_path.display().to_string(),
        // Flag indicating that this tool was installed by `devbox`.
        installed_by_devbox: true,
        // The method of installation.
        install_method: "brew".to_string(),
        // Any `rename_to` value specified in the configuration.
        renamed_to: tool_entry.rename_to.clone(),
        // Denotes the package type as "brew" for consistency and potential future filtering.
        package_type: "binary-by-brew".to_string(),
        // `repo` and `tag` fields are specific to GitHub releases and are not applicable for Homebrew.
        repo: None,
        tag: None,
        // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
        options: tool_entry.options.clone(),
        // For direct URL installations: The original URL from which the tool was downloaded.
        url: tool_entry.url.clone(),
        executable_path_after_extract: None,
        additional_cmd_executed: tool_entry.additional_cmd.clone(),
    })
}

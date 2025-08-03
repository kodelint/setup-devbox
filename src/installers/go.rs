// This module provides the installation logic for Go binaries and tools
// using the `go install` command. It's designed to integrate seamlessly
// with the `setup-devbox` application, allowing users to define Go tools
// in their `tools.yaml` configuration.

// Imports necessary schema definitions for tools.
// `ToolEntry`: Defines the structure for how a Go tool is configured in `tools.yaml`.
// `ToolState`: Defines the structure for how the state of an installed Go tool is recorded in `state.json`.
use crate::schema::{ToolEntry, ToolState};
// Imports custom logging macros from the crate root.
// These macros (`log_debug`, `log_error`, `log_info`, `log_warn`) provide
// a standardized way to output messages to the console with different severity levels,
// essential for tracking progress and diagnosing issues during Go tool installation.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to terminal output.
// The `colored` crate allows us to make log messages and other terminal output more readable
// by applying colors (e.g., `.bold()`, `.cyan()`, `.red()`, `.green()`).
use colored::Colorize;
// For executing external commands (like `go`) and capturing their output.
// `std::process::Command` is used to build and run system commands.
// `std::process::Output` captures the standard output, standard error, and exit status.
use std::process::{Command, Output};
// For working with file paths, specifically to construct installation paths.
// `PathBuf` is used here primarily for constructing the `install_path` for the `ToolState`.
use std::path::PathBuf;

/// Installs a Go binary or module using the `go install` command.
///
/// This function acts as the dedicated installer for Go-based tools within the DevBox system.
/// It constructs the appropriate `go install` command, handles optional versioning using
/// Go module syntax (e.g., `module@version`), includes any additional build options,
/// executes the command, and records the installation details.
///
/// This revised version prioritizes using `tool_entry.url` as the source for `go install`
/// if provided, falling back to `tool_entry.name` otherwise.
///
/// # Workflow:
/// 1.  **`go` Executable Check**: Verifies that the `go` command is available in the system's PATH.
/// 2.  **Source Determination**: Decides whether to use `tool_entry.url` or `tool_entry.name`
///     as the Go module path for `go install`.
/// 3.  **Command Construction**: Builds the `go install` command arguments, combining the module path
///     with an optional version specifier (`@version`).
/// 4.  **Additional Options**: Incorporates any extra `go install` options (e.g., `-ldflags`)
///     provided in the `tool_entry.options`.
/// 5.  **Execution**: Runs the constructed `go install` command and captures its standard output,
///     standard error, and exit status.
/// 6.  **Error Handling**: Provides detailed logging for successful installations, warnings,
///     and failures, including the command's exit code and standard error.
/// 7.  **Path Determination**: Attempts to determine the common installation path for Go binaries
///     (typically `GOPATH/bin` or `~/go/bin/`).
/// 8.  **State Recording**: Creates and returns a `ToolState` object to persistently track
///     the installed Go tool within the `state.json` file.
///
/// # Arguments
/// * `tool_entry`: A reference to the `ToolEntry` struct. This struct holds the configuration
///   details for the Go tool to be installed, as defined in `tools.yaml`.
///   - `tool_entry.name`: The Go module path for the tool (e.g., "golang.org/x/tools/cmd/goimports").
///   - `tool_entry.version`: An optional version string (e.g., "v0.1.0"). If present, `go install`
///     will attempt to install that specific version using `@version` syntax. If `None`, the latest
///     stable version is typically installed.
///   - `tool_entry.url`: An optional URL string. If present, and interpretable by `go install` as
///     a module path (e.g., "https://github.com/spf13/cobra"), it will be used as the source.
///   - `tool_entry.options`: An optional `Vec<String>` containing additional arguments to pass
///     directly to the `go install` command (e.g., `"-ldflags='-s -w'"`, `"-buildvcs=false"`).
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if the Go tool installation was successful (command executed with exit code 0).
///   The `ToolState` object contains details about the installed tool.
/// * `None` if the `go` command is not found, or if the `go install` command fails for any reason
///   (e.g., compilation error, network issue, module not found).
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_debug!(
        "[Go Installer] Attempting to install Go tool: {}",
        tool_entry.name.bold()
    );

    // Basic validation: Ensure 'go' command is available in the system's PATH.
    // We check for its existence by attempting to run `go version`.
    // If it fails, it means Go is not installed or not accessible.
    if Command::new("go").arg("version").output().is_err() {
        log_error!(
            "[Go Installer] 'go' command not found. Please ensure Go is installed and in your PATH."
        );
        return None; // Cannot proceed without the Go executable.
    }

    // Determine the base package path for `go install`.
    // Prioritize `tool_entry.url` if it's provided, otherwise use `tool_entry.name`.
    // This allows users to specify a direct repository URL for `go install`.
    let base_package_path = tool_entry.url.as_ref().unwrap_or(&tool_entry.name);

    // Initialize command arguments for `go install`.
    let mut command_args = vec!["install"];

    // Combine the base package path with the version if provided.
    // Go modules typically use `@version` syntax (e.g., `module_path@version`).
    let package_path_with_version = if let Some(version) = &tool_entry.version {
        format!("{}@{}", base_package_path, version) // Format as `module_path_or_url@version`.
    } else {
        base_package_path.clone() // If no version specified, install the latest (implicitly `@latest`).
    };
    command_args.push(&package_path_with_version); // Add the constructed package path to arguments.

    // Add any additional options specified in the `ToolEntry`, e.g., build flags (`-ldflags`).
    if let Some(options) = &tool_entry.options {
        for opt in options {
            command_args.push(opt); // Add each option string directly to the arguments list.
        }
    }

    // Log the full command being executed for debugging and user visibility.
    // The command and its arguments are colored for better readability in the terminal.
    log_info!(
        "[Go Installer] Executing: {} {}",
        "go".cyan().bold(),
        command_args.join(" ").cyan()
    );

    // Execute the `go install` command and capture its output.
    // This is a blocking call that waits for the command to complete.
    let output: Output = match Command::new("go") // Specify the `go` executable.
        .args(&command_args) // Pass all constructed arguments.
        .output() // Execute and capture stdout, stderr, and exit status.
    {
        Ok(out) => out, // Command executed successfully (not necessarily *installed* successfully).
        Err(e) => {
            // Log an error if the command itself failed to spawn or execute (e.g., permissions issues).
            log_error!("[Go Installer] Failed to execute 'go install' command for '{}': {}", tool_entry.name.bold().red(), e);
            return None; // Indicate an installation failure.
        }
    };

    // Check if the command executed successfully (exit code 0).
    // A successful exit status (`status.success()`) means the Go command ran without error,
    // indicating a successful tool installation.
    if output.status.success() {
        log_info!(
            "[Go Installer] Successfully installed Go tool: {}",
            tool_entry.name.bold().green() // Green color for success.
        );
        // Log standard output if available, usually contains build progress or confirmation.
        if !output.stdout.is_empty() {
            log_debug!(
                "[Go Installer] Stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        // Log standard error if available. Go commands might print warnings to stderr even on success.
        if !output.stderr.is_empty() {
            log_warn!(
                "[Go Installer] Stderr (might contain warnings): {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Determine the installation path.
        // Go typically installs binaries to `$GOPATH/bin` or `$HOME/go/bin` if `GOPATH` is not set.
        // This logic attempts to construct that common default path.
        // A more robust solution might involve executing `go env GOBIN` to get the exact path.
        let install_path = if let Ok(mut home) = std::env::var("HOME") {
            home.push_str("/go/bin/"); // Common default for `go install` if `GOPATH` isn't customized.
            // Join the base path with the tool's name, assuming the binary is named after the tool.
            PathBuf::from(home)
                .join(&tool_entry.name) // Use `tool_entry.name` for the *binary filename*.
                .to_string_lossy()
                .into_owned()
        } else {
            // Fallback path if HOME environment variable is not found.
            // This is less accurate but provides a placeholder.
            "/usr/local/go/bin/".to_string()
        };

        // Return a `ToolState` struct to record the successful installation.
        // This `ToolState` will be serialized into `state.json` for persistent tracking.
        Some(ToolState {
            // Record the actual version used, or "latest" if not explicitly specified.
            version: tool_entry
                .version
                .clone()
                .unwrap_or_else(|| "latest".to_string()),
            // The determined installation path.
            install_path,
            // Mark as installed by this application.
            installed_by_devbox: true,
            // Specify the installation method.
            install_method: "go-install".to_string(),
            // Record if the tool was renamed by the user in `tools.yaml`.
            renamed_to: tool_entry.rename_to.clone(),
            // Define the package type for categorization in `state.json`.
            package_type: "go-module".to_string(),
            // `repo` and `tag` are not directly applicable for standard `go install`
            // from `tool_entry` as it typically resolves modules from Go proxies, not direct Git repos.
            // Therefore, these fields are set to `None` for clarity and clean `state.json`.
            repo: None,
            tag: None,
            // Pass the additional `options` that were used during the `go install` command.
            options: tool_entry.options.clone(),
            // Store the URL if it was used as the source for `go install`.
            url: tool_entry.url.clone(),
            executable_path_after_extract: None,
            additional_cmd_executed: tool_entry.additional_cmd.clone(),
        })
    } else {
        // Handle failed installation.
        // If the `go install` command exited with a non-zero status code, it indicates a failure.
        // Capture and log the standard error output for debugging purposes.
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error!(
            "[Go Installer] Failed to install Go tool '{}'. Exit code: {}. Error: {}",
            tool_entry.name.bold().red(), // Tool name, colored red.
            output.status.code().unwrap_or(-1), // Exit code (default to -1 if not available).
            stderr.red()                  // Standard error output, colored red.
        );
        // Also log stdout on failure, as it might contain useful context or partial build logs.
        if !output.stdout.is_empty() {
            log_debug!(
                "[Go Installer] Stdout (on failure): {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        None // Return None to indicate failure.
    }
}

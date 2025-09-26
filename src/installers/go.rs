// This module provides the installation logic for Go binaries and tools
// using the `go install` command. It's designed to integrate seamlessly
// with the `setup-devbox` application, allowing users to define Go tools
// in their `tools.yaml` configuration.

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
use crate::libs::tool_installer::execute_post_installation_hooks;
use crate::libs::utilities::assets::current_timestamp;
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
///     a module path (e.g., "[https://github.com/spf13/cobra](https://github.com/spf13/cobra)"), it will be used as the source.
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
        format!("{base_package_path}@{version}") // Format as `module_path_or_url@version`.
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
        let install_path = if let Some(path) = get_go_env_path("GOBIN", &tool_entry.name) {
            // 1. Get the path from 'go env GOBIN'
            log_debug!(
                "[Go Installer] Using path from 'go env GOBIN': {}",
                path.display()
            );
            path
        } else if let Some(go_path) = std::env::var_os("GOPATH") {
            // 2. Look for "GOPATH" environment variable (traditional check)
            log_debug!(
                "[Go Installer] Using GOPATH env var: {}",
                PathBuf::from(&go_path).display()
            );
            PathBuf::from(go_path).join("bin").join(&tool_entry.name)
        } else if let Some(home_path_os) = std::env::var_os("HOME") {
            // 3. Look for "$HOME" and build the path "$HOME/go/bin"
            log_debug!(
                "[Go Installer] Using default $HOME/go/bin path: {}",
                PathBuf::from(&home_path_os).display()
            );
            PathBuf::from(home_path_os)
                .join("go")
                .join("bin")
                .join(&tool_entry.name)
        } else {
            // 4. The default system path
            log_warn!(
                "[Go Installer] No environment variables found. Defaulting to /usr/local/bin/"
            );
            PathBuf::from("/usr/local/bin/").join(&tool_entry.name)
        };

        log_debug!(
            "[Go Installer] Determined installation path: {}",
            format!("{}", install_path.display()).cyan()
        );

        // Execute Post installation hooks (if specified)
        // After the main installation is complete, execute any additional commands specified
        // in the tool configuration. These commands are often used for post-installation setup,
        // such as copying configuration files, creating directories, or setting up symbolic links.
        // Optional - failure won't stop installation
        let executed_post_installation_hooks =
            execute_post_installation_hooks("[Go Installer]", tool_entry, &install_path);
        // If execution reaches this point, the installation was successful.
        log_info!(
            "[Go Installer] Installation of {} completed successfully at {}!",
            tool_entry.name.to_string().bold(),
            format!("{}", install_path.display()).green()
        );

        // Return ToolState for Tracking
        // Construct a `ToolState` object to record the details of this successful installation.
        // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
        // what tools are installed, where they are, and how they were installed. This is crucial
        // for future operations like uninstallation, updates, or syncing.
        Some(ToolState {
            // Record the actual version used, or "latest" if not explicitly specified.
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
            // In this module, it's always "go-install".
            install_method: "go-install".to_string(),
            // Records if the binary was renamed during installation, storing the new name.
            renamed_to: tool_entry.rename_to.clone(),
            // `repo` and `tag` are not directly applicable for standard `go install`
            // from `tool_entry` as it typically resolves modules from Go proxies, not direct Git repos.
            // Therefore, these fields are set to `None` for clarity and clean `state.json`.
            repo: None,
            tag: None,
            // The actual package type detected by the `file` command or inferred. This is for diagnostic
            // purposes, providing the most accurate type even if the installation logic
            // used a filename-based guess (e.g., "binary", "macos-pkg-installer").
            // For Go Installer it will always be 'go-module`
            package_type: "go-module".to_string(),
            // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
            // The additional `options` that were used during the `go install` command.
            options: tool_entry.options.clone(),
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
            executed_post_installation_hooks,
            configuration_manager: None,
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

// Helper function to safely get the path from 'go env <ENV_VAR>'
// The function now takes the environment variable name (e.g., "GOBIN", "GOPATH")
fn get_go_env_path(env_var: &str, tool_name: &str) -> Option<PathBuf> {
    // 1. Execute 'go env <ENV_VAR>'
    match Command::new("go").arg("env").arg(env_var).output() {
        Ok(output) if output.status.success() => {
            // 2. Capture and clean the output
            let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if output_str.is_empty() {
                // If the output is empty, the variable is likely not set.
                None
            } else {
                // 3. Construct the path
                let path = PathBuf::from(output_str);

                // Note: For variables like GOBIN or GOPATH, the output is typically a directory.
                // We join the tool_name to get the full binary path.
                Some(path.join(tool_name))
            }
        }
        _ => {
            // Log or handle the case where the command fails (e.g., 'go' not in PATH)
            None
        }
    }
}

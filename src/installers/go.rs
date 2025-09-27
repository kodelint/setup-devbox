//! # Go Installer Module
//!
//! This module provides a robust, production-grade installer for Go tools using the `go install` command.
//! It follows the same reliability standards as the official Go toolchain with comprehensive
//! error handling, verification mechanisms, and accurate path detection.
//!
//! ## Key Features
//!
//! - **Dual Source Support**: Handles both standard Go module installations and custom URL-based installations
//! - **Smart Binary Detection**: Automatically detects actual binary names from URLs when they differ from tool names
//! - **Comprehensive Validation**: Validates Go availability, installation success, and binary paths
//! - **Smart State Tracking**: Maintains accurate installation state with version tracking
//! - **Flexible Configuration**: Supports version specifications, custom Go options, and URL sources
//! - **Post-Installation Hooks**: Executes additional setup commands after successful installation
//! - **Environment Awareness**: Properly handles different Go environment variables and installation paths
//! - **Rename Support**: Fully supports the `rename_to` option for custom binary names
//!
//! ## Installation Workflow
//!
//! The installer follows a meticulous 8-step process:
//!
//! 1. **Source Determination** - Chooses between URL and module name sources
//! 2. **Command Preparation** - Constructs appropriate `go install` command with versioning
//! 3. **Command Execution** - Runs installation with comprehensive error handling
//! 4. **Installation Verification** - Confirms the tool was properly installed with smart binary name detection
//! 5. **Path Resolution** - Accurately determines binary installation path
//! 6. **Post-Installation Hooks** - Executes any additional setup commands
//! 7. **State Creation** - Creates comprehensive tool state for persistence
//!
//! ## Error Handling
//!
//! The module provides detailed error messages and logging at multiple levels:
//! - **Info**: High-level installation progress
//! - **Debug**: Detailed command construction and path resolution
//! - **Warn**: Non-fatal issues or warnings during installation
//! - **Error**: Installation failures with specific error codes and messages

use std::path::PathBuf;
use std::process::Command;

// Post-installation hook execution functionality.
use crate::libs::tool_installer::execute_post_installation_hooks;
// Internal module imports:
// `ToolEntry`: Represents a single tool's configuration from `tools.yaml`.
// `ToolState`: Represents the actual state of an installed tool for persistence in `state.json`.
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;
// Custom logging macros for structured output.
use crate::{log_debug, log_error, log_info, log_warn};
// External crate imports:
//   Library for adding color to terminal output for better readability.
use colored::Colorize;

/// Installs a Go tool using the `go install` command with comprehensive error handling.
///
/// This function provides a robust installer for Go tools that mirrors the quality and
/// reliability of the official Go toolchain. It includes validation, verification, and
/// accurate state tracking.
///
/// # Workflow:
/// 1. **Environment Validation**: Verifies `go` is installed and accessible
/// 2. **Source Determination**: Chooses between URL and module name sources
/// 3. **Command Preparation**: Constructs `go install` command with versioning
/// 4. **Command Execution**: Runs installation with comprehensive error handling
/// 5. **Installation Verification**: Confirms the tool was properly installed with smart binary name detection
/// 6. **Path Resolution**: Accurately determines the installation path
/// 7. **Post-installation Hooks**: Executes any additional setup commands
/// 8. **State Creation**: Creates comprehensive `ToolState` with all relevant metadata
///
/// # Arguments:
/// * `tool_entry`: A reference to the `ToolEntry` struct containing tool configuration
///   - `tool_entry.name`: **Required** - The Go module name or tool name
///   - `tool_entry.url`: Optional URL for custom repository installations
///   - `tool_entry.version`: Optional version specification using Go module syntax
///   - `tool_entry.options`: Optional list of go install options (-ldflags, etc.)
///   - `tool_entry.rename_to`: Optional custom binary name
///
/// # Returns:
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if installation was completely successful with accurate metadata
/// * `None` if any step of the installation process fails
///
/// ## Examples - YAML
///
/// ```yaml
/// ## `golangci-lint` - Fast linters runner for Go
/// ## https://golangci-lint.run/
/// - name: my-linter
///   source: go
///   version: v1.54.2
///   url: github.com/golangci/golangci-lint/cmd/golangci-lint
///   # Binary will be detected as 'golangci-lint' from URL, not 'my-linter'
///
/// ## `air` - Live reload for Go apps
/// ## https://github.com/cosmtrek/air
/// - name: air
///   source: go
///   version: latest
///   options:
///     - -ldflags=-s -w
///
/// ## Custom binary name
/// - name: gopls
///   source: go
///   url: golang.org/x/tools/gopls
///   rename_to: my-gopls
///   # Binary will be installed as 'my-gopls'
/// ```
///
/// ## Examples - Rust Code
///
/// ### Basic Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "golang.org/x/tools/gopls".to_string(),
///     version: None, // Install latest version
///     url: None,
///     options: None,
///     rename_to: None,
/// };
/// install(&tool_entry);
/// ```
///
/// ### Version-Specific Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "my-tool".to_string(),
///     version: Some("v1.54.2".to_string()),
///     url: Some("github.com/golangci/golangci-lint/cmd/golangci-lint".to_string()),
///     options: None,
///     rename_to: None,
/// };
/// install(&tool_entry);
/// ```
///
/// ### URL-Based Installation with Rename
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "custom-tool".to_string(),
///     version: Some("v0.1.0".to_string()),
///     url: Some("github.com/example/custom-tool".to_string()),
///     options: Some(vec!["-ldflags=-s -w".to_string()]),
///     rename_to: Some("my-custom-tool".to_string()),
/// };
/// install(&tool_entry);
/// ```
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[Go Installer] Attempting to install Go tool: {}",
        tool_entry.name.bold()
    );
    log_debug!("[Go Installer] ToolEntry details: {:#?}", tool_entry);

    // 1. Determine installation source - choose between URL and module name
    let installation_source = determine_installation_source(tool_entry);
    log_debug!(
        "[Go Installer] Installation source: {}",
        installation_source.cyan()
    );

    // 2. Prepare and execute go install command
    log_debug!(
        "[Go Installer] Prepare the command to install: {}",
        tool_entry.name.bold()
    );
    let command_args = prepare_go_install_command(tool_entry, &installation_source);
    if !execute_go_install_command(&command_args, tool_entry) {
        return None;
    }

    // 3. Verify the installation was successful - ensure the binary is actually available
    log_debug!(
        "[Go Installer] Verify if {} was actually installed",
        tool_entry.name.bold()
    );
    if !verify_go_installation(tool_entry) {
        return None;
    }

    // 4. Determine accurate installation path - where the binary was actually installed
    let binary_name = determine_actual_binary_name(tool_entry);
    let install_path = determine_go_installation_path(&binary_name);
    log_debug!(
        "[Go Installer] Determined installation path: {}",
        install_path.display().to_string().cyan()
    );

    // 5. Execute post-installation hooks - run any additional setup commands
    log_debug!(
        "[Go Installer] Executing post installation hooks, post installing {}",
        tool_entry.name.bold()
    );

    // 6. Execute post-installation hooks - run any additional setup commands
    let executed_post_installation_hooks =
        execute_post_installation_hooks("[Go Installer]", tool_entry, &install_path);

    log_info!(
        "[Go Installer] Successfully installed Go tool: {} (version: {}) as {}",
        tool_entry.name.bold().green(),
        tool_entry
            .version
            .as_ref()
            .unwrap_or(&"latest".to_string())
            .green(),
        binary_name.green()
    );

    // 7. Return comprehensive ToolState for tracking
    //
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed.
    Some(ToolState::new(
        tool_entry,
        &install_path,
        "go-install".to_string(),
        "go-module".to_string(),
        tool_entry
            .version
            .clone()
            .unwrap_or_else(|| "latest".to_string()),
        tool_entry.url.clone(),
        None,
        executed_post_installation_hooks,
    ))
}

/// Determines the installation source for the Go tool.
///
/// This function examines the tool configuration to determine whether to use
/// the URL or the name as the installation source, with URL taking precedence.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing source information
///
/// # Returns
/// The installation source string (either URL or name)
fn determine_installation_source(tool_entry: &ToolEntry) -> String {
    if let Some(url) = &tool_entry.url {
        url.clone()
    } else {
        tool_entry.name.clone()
    }
}

/// Prepares the go install command arguments based on installation source.
///
/// This function constructs the appropriate command-line arguments for `go install`
/// including version specifications and custom options.
///
/// # Arguments
/// * `tool_entry` - The tool configuration
/// * `installation_source` - The source (URL or name) to install from
///
/// # Returns
/// A vector of command-line arguments to pass to `go install`
///
/// # Examples
///
/// ```yaml
/// ## `golangci-lint` - Fast linters runner for Go
/// ## https://golangci-lint.run/
/// - name: my-linter
///   source: go
///   version: v1.54.2
///   url: github.com/golangci/golangci-lint/cmd/golangci-lint
///
/// ## `air` - Live reload for Go apps
/// ## https://github.com/cosmtrek/air
/// - name: air
///   source: go
///   version: latest
///   options:
///     - -ldflags=-s -w
/// ```
///
/// ## Go Installation
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "my-tool".to_string(),
///     version: Some("v1.54.2".to_string()),
///     url: Some("github.com/golangci/golangci-lint/cmd/golangci-lint".to_string()),
///     options: None,
///     rename_to: None,
/// };
///
/// let args = prepare_go_install_command(&tool_entry, "github.com/golangci/golangci-lint/cmd/golangci-lint");
/// // args: ["install", "github.com/golangci/golangci-lint/cmd/golangci-lint@v1.54.2"]
/// ```
///
/// ## Installation with Options
/// ```rust
/// let tool_entry = ToolEntry {
///     name: "air".to_string(),
///     version: None,
///     url: None,
///     options: Some(vec!["-ldflags=-s -w".to_string()]),
///     rename_to: None,
/// };
///
/// let args = prepare_go_install_command(&tool_entry, "air");
/// // args: ["install", "air", "-ldflags=-s -w"]
/// ```
fn prepare_go_install_command(tool_entry: &ToolEntry, installation_source: &str) -> Vec<String> {
    let mut command_args = Vec::new();
    command_args.push("install".to_string());

    // Construct package path with version if specified
    let package_path = if let Some(version) = &tool_entry.version {
        format!("{installation_source}@{version}")
    } else {
        installation_source.to_string()
    };
    command_args.push(package_path);

    // Add any additional options
    if let Some(options) = &tool_entry.options {
        log_debug!("[Go Installer] Adding custom options: {:#?}", options);
        for opt in options {
            command_args.push(opt.clone());
        }
    }

    log_debug!(
        "[Go Installer] Prepared command arguments: {} {}",
        "go".cyan().bold(),
        command_args.join(" ").cyan()
    );

    command_args
}

/// Executes the go install command with comprehensive error handling.
///
/// This function runs the actual `go install` command and provides detailed
/// logging and error reporting. It captures both stdout and stderr for debugging.
///
/// # Arguments
/// * `command_args` - The command-line arguments for `go install`
/// * `tool_entry` - The tool being installed (for logging purposes)
///
/// # Returns
/// `true` if installation was successful, `false` otherwise
///
/// # Error Handling
/// - Logs success with green highlighting
/// - Captures and logs stdout/stderr for debugging
/// - Provides specific error codes and messages for failures
/// - Handles both command execution failures and non-zero exit codes
fn execute_go_install_command(command_args: &[String], tool_entry: &ToolEntry) -> bool {
    log_info!(
        "[Go Installer] Executing: {} {}",
        "go".cyan().bold(),
        command_args.join(" ").cyan()
    );

    match Command::new("go").args(command_args).output() {
        Ok(output) if output.status.success() => {
            log_info!(
                "[Go Installer] Successfully installed tool: {}",
                tool_entry.name.bold().green()
            );

            // Log output for debugging
            if !output.stdout.is_empty() {
                log_debug!(
                    "[Go Installer] Stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            if !output.stderr.is_empty() {
                log_warn!(
                    "[Go Installer] Stderr (may contain warnings): {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            true
        }
        Ok(output) => {
            log_error!(
                "[Go Installer] Failed to install tool '{}'. Exit code: {}. Error: {}",
                tool_entry.name.bold().red(),
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).red()
            );

            if !output.stdout.is_empty() {
                log_debug!(
                    "[Go Installer] Stdout (on failure): {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            false
        }
        Err(e) => {
            log_error!(
                "[Go Installer] Failed to execute 'go install' for '{}': {}",
                tool_entry.name.bold().red(),
                e.to_string().red()
            );
            false
        }
    }
}

/// Verifies that a Go tool was successfully installed.
///
/// This function checks if the installed binary exists at the expected path,
/// taking into account the `rename_to` option and potential differences between
/// the tool name and the actual binary name from the URL/module path.
///
/// # Arguments
/// * `tool_entry` - The tool configuration containing rename information
///
/// # Returns
/// `true` if the tool is installed and executable, `false` otherwise
fn verify_go_installation(tool_entry: &ToolEntry) -> bool {
    // Determine the actual binary name using priority order:
    // 1. rename_to option (highest priority)
    // 2. Last part of URL (if provided and different from tool name)
    // 3. Tool name (lowest priority)
    let binary_name = determine_actual_binary_name(tool_entry);

    let install_path = determine_go_installation_path(&binary_name);

    if install_path.exists() {
        log_debug!(
            "[Go Installer] Verified installation: {} exists at {}",
            binary_name.bold(),
            install_path.display()
        );

        // Additional check: verify the binary is executable
        if is_executable(&install_path) {
            log_debug!(
                "[Go Installer] Binary is executable: {}",
                install_path.display()
            );
            true
        } else {
            log_warn!(
                "[Go Installer] Binary exists but may not be executable: {}",
                install_path.display()
            );
            true // Still consider it installed, but warn about permissions
        }
    } else {
        log_error!(
            "[Go Installer] Installation verification failed: {} not found at {}",
            binary_name.bold().red(),
            install_path.display()
        );

        // Try alternative names for better error reporting
        try_alternative_names(tool_entry, &install_path);
        false
    }
}

/// Determines the actual binary name by checking multiple sources in priority order.
///
/// Priority order:
/// 1. `rename_to` option (explicit user preference)
/// 2. Last part of URL (if provided and different from tool name)
/// 3. Tool name (fallback)
///
/// # Arguments
/// * `tool_entry` - The tool configuration
///
/// # Returns
/// The actual binary name to look for
fn determine_actual_binary_name(tool_entry: &ToolEntry) -> String {
    // Priority 1: rename_to option (explicit user preference)
    if let Some(rename_to) = &tool_entry.rename_to {
        log_debug!(
            "[Go Installer] Using rename_to option for binary name: {}",
            rename_to
        );
        return rename_to.clone();
    }

    // Priority 2: Extract binary name from URL (if provided)
    if let Some(url) = &tool_entry.url {
        if let Some(url_binary_name) = extract_binary_name_from_url(url) {
            // Only use URL binary name if it's different from the tool name
            if url_binary_name != tool_entry.name {
                log_debug!(
                    "[Go Installer] Using binary name from URL: {} (tool name: {})",
                    url_binary_name,
                    tool_entry.name
                );
                return url_binary_name;
            }
        }
    }

    // Priority 3: Use tool name as fallback
    log_debug!(
        "[Go Installer] Using tool name as binary name: {}",
        tool_entry.name
    );
    tool_entry.name.clone()
}

/// Extracts the likely binary name from a Go module URL.
///
/// For Go modules, the binary name is typically the last part of the path.
/// Examples:
/// - "github.com/golangci/golangci-lint/cmd/golangci-lint" → "golangci-lint"
/// - "golang.org/x/tools/gopls" → "gopls"
/// - "github.com/cosmtrek/air" → "air"
///
/// # Arguments
/// * `url` - The Go module URL
///
/// # Returns
/// The extracted binary name, or None if unable to determine
fn extract_binary_name_from_url(url: &str) -> Option<String> {
    let segments: Vec<&str> = url.split('/').collect();
    let last_segment = segments
        .iter()
        .rev()
        .find(|&&segment| !segment.is_empty())?;

    if last_segment == &"cmd" && segments.len() > 1 {
        segments.iter().rev().nth(1).map(|&s| s.to_string())
    } else {
        Some(last_segment.to_string())
    }
}

/// Tries alternative binary names and logs suggestions for better error reporting.
///
/// # Arguments
/// * `tool_entry` - The tool configuration
/// * `expected_path` - The path where we expected to find the binary
fn try_alternative_names(tool_entry: &ToolEntry, expected_path: &std::path::Path) {
    let alternatives = generate_alternative_names(tool_entry);

    for alt_name in alternatives {
        let alt_path = determine_go_installation_path(&alt_name);
        if alt_path.exists() {
            log_warn!(
                "[Go Installer] Found binary with different name: {} at {}",
                alt_name.bold().yellow(),
                alt_path.display()
            );
            log_info!(
                "[Go Installer] Consider using rename_to: '{}' in your configuration",
                alt_name
            );
            return;
        }
    }

    // If no alternatives found, check what's actually in the bin directory
    if let Some(bin_dir) = expected_path.parent() {
        if let Ok(entries) = std::fs::read_dir(bin_dir) {
            let installed_binaries: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    if let Ok(metadata) = entry.metadata() {
                        metadata.is_file() && is_executable(&entry.path())
                    } else {
                        false
                    }
                })
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect();

            if !installed_binaries.is_empty() {
                log_info!(
                    "[Go Installer] Installed binaries in {}: {}",
                    bin_dir.display(),
                    installed_binaries.join(", ").cyan()
                );
            }
        }
    }
}

/// Generates alternative binary names to try during verification.
///
/// # Arguments
/// * `tool_entry` - The tool configuration
///
/// # Returns
/// A vector of alternative binary names to try
fn generate_alternative_names(tool_entry: &ToolEntry) -> Vec<String> {
    let mut alternatives = Vec::new();

    // Alternative 1: Tool name (if we were using URL name before)
    if tool_entry.url.is_some() {
        alternatives.push(tool_entry.name.clone());
    }

    // Alternative 2: URL binary name (if we were using tool name before)
    if let Some(url) = &tool_entry.url {
        if let Some(url_name) = extract_binary_name_from_url(url) {
            if url_name != tool_entry.name {
                alternatives.push(url_name);
            }
        }
    }

    // Alternative 3: Common variations
    if tool_entry.name.starts_with("cmd-") {
        alternatives.push(tool_entry.name.trim_start_matches("cmd-").to_string());
    }

    alternatives
}

/// Determines the accurate installation path for go-installed binaries.
///
/// This function attempts to locate where go actually installed the binary
/// by checking environment variables and Go configuration in order of precedence.
///
/// # Arguments
/// * `binary_name` - The name of the installed binary
///
/// # Returns
/// A `PathBuf` containing the full path to the installed binary
///
/// # Path Resolution Order
/// 1. `GOBIN` environment variable (highest priority)
/// 2. `GOPATH` environment variable with `/bin` suffix
/// 3. `HOME` environment variable with default `~/go/bin` path
/// 4. System fallback to `/usr/local/bin` (lowest priority)
fn determine_go_installation_path(binary_name: &str) -> PathBuf {
    // Try environment variables in order of preference
    if let Some(path) = get_go_install_path(binary_name) {
        return path;
    }

    // Final fallback to system default PATH
    log_warn!("[Go Installer] Could not determine Go installation path, using system fallback");
    PathBuf::from("/usr/local/bin").join(binary_name)
}

/// Gets the installation path for a go-installed tool by checking
/// `GOBIN`, `GOPATH`, and a default `HOME` based path, in order.
///
/// This is a helper function for `determine_go_installation_path` that implements
/// the actual environment variable checking logic.
///
/// # Arguments
/// * `binary_name` - The name of the installed binary
///
/// # Returns
/// `Some(PathBuf)` if a valid path can be constructed, `None` otherwise
///
/// # Environment Variable Precedence
///
/// 1. **GOBIN**: Directly specifies the binary installation directory
/// 2. **GOPATH**: The Go workspace directory (usually `~/go`)
/// 3. **HOME**: Used to construct the default Go path (`~/go/bin`)
fn get_go_install_path(binary_name: &str) -> Option<PathBuf> {
    // 1. Check GOBIN (highest priority)
    if let Some(path) = get_go_env_path("GOBIN", binary_name) {
        log_debug!("[Go Installer] Using GOBIN path: {}", path.display());
        return Some(path);
    }

    // 2. Check GOPATH (medium priority)
    if let Some(go_path) = get_go_env_path("GOPATH", "") {
        let path = go_path.join("bin").join(binary_name);
        log_debug!("[Go Installer] Using GOPATH path: {}", path.display());
        return Some(path);
    }

    // 3. Check HOME (for the default ~/go/bin path) (lowest priority)
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join("go").join("bin").join(binary_name);
        log_debug!(
            "[Go Installer] Using HOME-based Go path: {}",
            path.display()
        );
        return Some(path);
    }

    // If none of the environment variables are set or paths are constructed, return None
    None
}

/// Helper function to safely get the path from 'go env <ENV_VAR>'
///
/// This function executes `go env` to get Go-specific environment variables
/// which is more reliable than reading system environment variables directly.
///
/// # Arguments
/// * `env_var` - The Go environment variable to query (e.g., "GOBIN", "GOPATH")
/// * `binary_name` - The name of the binary to append to the path
///
/// # Returns
/// `Some(PathBuf)` if the environment variable is set and valid, `None` otherwise
fn get_go_env_path(env_var: &str, binary_name: &str) -> Option<PathBuf> {
    match Command::new("go").arg("env").arg(env_var).output() {
        Ok(output) if output.status.success() => {
            let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if output_str.is_empty() {
                None
            } else {
                let mut path = PathBuf::from(output_str);
                if !binary_name.is_empty() {
                    path = path.join(binary_name);
                }
                Some(path)
            }
        }
        _ => None,
    }
}

/// Checks if a file is executable.
///
/// This function attempts to determine if a file has executable permissions
/// by checking the file's metadata on Unix-like systems.
///
/// # Arguments
/// * `path` - The path to the file to check
///
/// # Returns
/// `true` if the file appears to be executable, `false` otherwise
fn is_executable(path: &PathBuf) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, check for .exe extension or just existence
        path.extension().map_or(false, |ext| ext == "exe") || path.exists()
    }
}

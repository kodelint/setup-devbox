use std::process::Command;
// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_info, log_warn};
// The 'colored' crate helps us make our console output look pretty and readab
use crate::schemas::tools::InstallerError;
use colored::Colorize;

/// Checks if a given asset filename from a GitHub release (or similar source)
/// is likely compatible with the current operating system and architecture.
/// This is how `setup-devbox` intelligently selects the correct download asset
/// from a list of available files. It performs fuzzy matching using aliases.
///
/// # Arguments
/// * `filename`: The full filename (`&str`) of the asset (e.g., "mytool_1.0.0_macOS_arm64.tar.gz").
/// * `os`: The current operating system as a normalized string (e.g., "macos").
/// * `arch`: The current architecture as a normalized string (e.g., "arm64").
///
/// # Returns
/// * `bool`: `true` if the filename contains recognizable keywords for the platform's OS and architecture,
///           considering aliases and a special Rosetta 2 fallback for macOS ARM64. `false` otherwise.

pub fn asset_matches_platform(filename: &str, os: &str, arch: &str) -> bool {
    // Convert inputs to lowercase for case-insensitive comparison.
    let asset_name_lower = filename.to_lowercase();

    // Pre-Step: Normalize the input OS and Architecture first
    // This makes sure we are working with consistent, canonical names like "macOS" and "arm64",
    // regardless of whether the input was "Darwin" or "aarch64".
    let os_normalized = normalize_os(os);
    let arch_normalized = normalize_arch(arch);

    // 1. Check for OS match:
    // Iterate through all known aliases for the current OS. If any alias is found
    // as a substring within the asset filename, it's considered an OS match.
    let os_matches =
        os_aliases(&os_normalized).iter().any(|alias| asset_name_lower.contains(alias));

    // If no OS match, immediately return false. No need to check architecture.
    if !os_matches {
        log_debug!("[Utils] Asset '{}' does not match OS '{}'", filename.dimmed(), os);
        return false;
    }

    // 2. Handle universal macOS packages (.dmg, .pkg)
    // If the OS is macOS and the asset is a .dmg or .pkg file, we consider it a match
    // because these are often universal installers that don't specify an architecture.
    let is_macos_universal_package = os_normalized == "macos"
        && (asset_name_lower.ends_with(".dmg") || asset_name_lower.ends_with(".pkg"));

    if is_macos_universal_package {
        log_debug!(
            "[Utils] Asset '{}' matches as a potential universal macOS package (dmg/pkg).",
            filename.dimmed()
        );
        // We'll still run the exclusion check to be safe.
        // If it passes, we're done here and we return `true`.
        return !is_excluded_asset(&asset_name_lower);
    }
    // 3. Check for Architecture match:
    // Iterate through all known aliases for the current architecture. If any alias is found
    // as a substring within the asset filename, it's considered an architecture match.
    let arch_matches = arch_aliases(&arch_normalized)
        .iter()
        .any(|alias| asset_name_lower.contains(alias));

    // 4. Special consideration for macOS ARM64 (aarch64) with Rosetta 2 fallback:
    // By using our normalized strings here, we handle aliases like "darwin" and "aarch64" automatically.
    let rosetta_fallback = (os_normalized == "macos" && arch_normalized == "arm64")
        && asset_name_lower.contains("x86_64")
        && !(asset_name_lower.contains("arm64") || asset_name_lower.contains("aarch64"));

    // If neither a direct architecture match nor the Rosetta fallback condition is met, return false.
    if !(arch_matches || rosetta_fallback) {
        log_debug!(
            "[Utils] Asset '{}' does not match architecture '{}' (and no Rosetta fallback).",
            filename.dimmed(),
            arch
        );
        return false;
    }

    // 4. Optional: Exclude common source, debug, or checksum files.
    // These files are usually not the actual executable binaries we want to download.
    // This helps in picking the actual binary release.
    if is_excluded_asset(&asset_name_lower) {
        return false;
    }

    // If all checks pass, the asset is considered a match for the current platform.
    log_debug!(
        "[Utils] Asset '{}' matches platform (OS: {}, ARCH: {}) -> {}",
        filename.dimmed(),
        os.cyan(),
        arch.magenta(),
        "true".bold()
    );
    true
}

/// Checks if an asset filename should be excluded based on common keywords for non-binary files.
///
/// This function is a helper used to filter out files from a list of release assets
/// that are typically not the primary executable binary. This includes source code archives,
/// debug symbols, checksums, and signature files.
///
/// # Arguments
///
/// * `asset_name_lower`: A string slice (`&str`) of the asset's filename, which is expected to be
///   in lowercase for case-insensitive matching.
///
/// # Returns
///
/// * `bool`: Returns `true` if the filename contains keywords for exclusion (e.g., "src", "checksum", ".asc").
///           Returns `false` if the asset is considered a potential binary.
fn is_excluded_asset(asset_name_lower: &str) -> bool {
    // These files are usually not the actual executable binaries we want to download.
    // This helps in picking the actual binary release.
    let is_excluded = asset_name_lower.contains("src") ||
        asset_name_lower.contains("source") ||
        asset_name_lower.contains("debug") ||
        asset_name_lower.contains("checksum") ||
        asset_name_lower.contains("sha256") ||
        asset_name_lower.contains("tar.gz.sig") || // Common signature file for tar.gz
        asset_name_lower.ends_with(".asc"); // Common detached signature file extension

    if is_excluded {
        log_debug!(
            "[Utils] Asset '{}' excluded due to containing common non-binary keywords.",
            asset_name_lower.dimmed()
        );
    }
    is_excluded
}

/// Helper function: Provides a list of common alternative names (aliases) for a given CPU architecture.
/// This is used internally by `asset_matches_platform` to handle different naming conventions
/// for architectures in release asset filenames (e.g., "aarch64" vs "arm64").
///
/// # Arguments
/// * `arch`: A string slice representing a normalized architecture name (e.g., "arm64", "x86_64").
///
/// # Returns
/// * `Vec<String>`: A vector of strings containing the input architecture name and its known aliases.
fn arch_aliases(arch: &str) -> Vec<String> {
    match arch.to_lowercase().as_str() {
        "arm64" => vec!["arm64", "aarch64"] // Aliases for ARM 64-bit.
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        "x86_64" => vec!["x86_64", "amd64"] // Aliases for x86 64-bit.
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        other => vec![other.to_string()], // For unknown architecture, just return the input string.
    }
}

/// Helper function: Provides a list of common alternative names (aliases) for a given operating system.
/// This is used internally by `asset_matches_platform` to improve the flexibility of matching
/// GitHub release asset filenames, which might use various naming conventions for the same OS.
///
/// # Arguments
/// * `os`: A string slice representing a normalized OS name (e.g., "macos", "linux").
///
/// # Returns
/// * `Vec<String>`: A vector of strings containing the input OS name and its known aliases.
fn os_aliases(os: &str) -> Vec<String> {
    match os.to_lowercase().as_str() {
        "macos" => {
            vec!["macos", "darwin", "apple-darwin", "macosx", "pkg", "dmg"] // Aliases for macOS.
                .into_iter()
                .map(|s| s.to_string())
                .collect()
        },
        "linux" => vec!["linux"] // Aliases for Linux (currently just "linux" itself).
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        "windows" => vec!["windows", "win32", "win64"] // Aliases for Windows.
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        other => vec![other.to_string()], // For unknown OS, just return the input string.
    }
}

/// Detects the current machine's CPU architecture (e.g., "arm64", "x86_64").
/// This is vital for downloading the correct version of a binary from GitHub releases
/// or other sources that provide platform-specific builds.
///
/// # Returns
/// * `Option<String>`:
///   - `Some(String)` containing the detected architecture as a canonical string
///     (e.g., "arm64" for "aarch64", "x86_64" for "amd64").
///   - `None` if detection somehow fails, though `std::env::consts::ARCH` is generally reliable.
pub fn detect_architecture() -> Option<String> {
    // `std::env::consts::ARCH` provides the target architecture Rust was compiled for
    // (e.g., "aarch64", "x86_64"). This is highly reliable for the running binary.
    // We then pass it to `normalize_arch` to get a consistent string format.
    Some(normalize_arch(std::env::consts::ARCH).to_string())
}

/// Detects the current operating system (e.g., "macos", "linux", "windows").
/// Similar to architecture detection, this is crucial for finding the right software release
/// assets that are built for the specific OS.
///
/// # Returns
/// * `Option<String>`:
///   - `Some(String)` containing the detected OS as a canonical string
///     (e.g., "macOS" for "darwin", "windows" for "win32").
///   - `None` if detection somehow fails, though `std::env::consts::OS` is generally reliable.
pub fn detect_os() -> Option<String> {
    // `std::env::consts::OS` provides the target operating system Rust was compiled for
    // (e.g., "macOS", "linux", "windows").
    // We then pass it to `normalize_os` to get a consistent string format.
    Some(normalize_os(std::env::consts::OS).to_string())
}

/// Normalizes various input strings for operating systems into a consistent, lowercase format.
/// This helps `setup-devbox` deal with different ways OS names might appear in asset names
/// (e.g., "macOS", "Darwin", "OSX") or from system information, mapping them to a common set.
///
/// # Arguments
/// * `os`: An input string (`&str`) representing an OS (e.g., "macOS", "darwin", "Linux").
///
/// # Returns
/// * `String`: The normalized OS string (e.g., "macOS", "linux", "windows").
///             If the input is not a known alias, the lowercase version of the input is returned.
pub fn normalize_os(os: &str) -> String {
    // Convert the input OS string to lowercase for case-insensitive matching.
    match os.to_lowercase().as_str() {
        "macos" | "darwin" | "apple-darwin" => "macos".to_string(), // Map various macOS/Darwin names to "macos".
        "linux" => "linux".to_string(), // Linux is typically straightforward.
        "windows" | "win32" | "win64" => "windows".to_string(), // Map various Windows names to "windows".
        other => {
            // If we encounter an unknown OS variant, log a warning.
            // We return the lowercase version of the unknown string as-is,
            // hoping it might still match some asset names.
            log_warn!(
                "[Utils] Unknown OS variant '{}', using as-is. This might cause issues with asset matching.",
                other.purple()
            );
            other.to_string()
        },
    }
}

/// Normalizes various input strings for CPU architectures into a consistent, lowercase format.
/// This ensures `setup-devbox` can correctly match architectures (e.g., "aarch64" vs "arm64",
/// or "amd64" vs "x86_64") when parsing asset names from releases.
///
/// # Arguments
/// * `arch`: An input string (`&str`) representing an architecture (e.g., "AARCH64", "x86_64", "amd64").
///
/// # Returns
/// * `String`: The normalized architecture string (e.g., "arm64", "x86_64").
///             If the input is not a known alias, the lowercase version of the input is returned.
pub fn normalize_arch(arch: &str) -> String {
    // Convert the input architecture string to lowercase for case-insensitive matching.
    match arch.to_lowercase().as_str() {
        "aarch64" | "arm64" => "arm64".to_string(), // Map ARM 64-bit variants to "arm64".
        "amd64" | "x86_64" => "x86_64".to_string(), // Map AMD/Intel 64-bit variants to "x86_64".
        other => {
            // If we encounter an unknown architecture variant, log a warning.
            // We return the lowercase version of the unknown string as-is.
            log_warn!(
                "[Utils] Unknown ARCH variant '{}', using as-is. This might cause issues with asset matching.",
                other.purple()
            );
            other.to_string()
        },
    }
}

/// Checks if a given command (installer) exists in the system's PATH and is executable.
///
/// This function attempts to run the command with a harmless argument like "--version"
/// or just checks its status to determine if it's found. It's a robust way to verify
/// the presence of external tools like `brew`, `go`, `cargo`, etc.
///
/// # Arguments
/// * `command_name`: The name of the command to check (e.g., "brew", "go", "cargo").
///
/// # Returns
/// `Ok(())` if the command is found and appears to be executable.
/// `Err(InstallerError::MissingCommand)` if the command is not found in PATH.
pub fn check_installer_command_available(command_name: &str) -> Result<(), InstallerError> {
    log_debug!("[Utils] Checking for installer command: '{}'", command_name.cyan());

    // Attempt to run the command. We use `output()` first with `--version` because
    // many commands support it as a lightweight way to check existence and print version info.
    // If that fails, we fall back to just `status()` to see if the command can be invoked at all.
    let command_found = Command::new(command_name)
        .arg("--version")
        .output()
        .is_ok() // Did the command execute and return an output (even if error)?
        || Command::new(command_name)
        .status()
        .is_ok(); // Or did it just exit with a status code (meaning it was found)?

    if command_found {
        log_debug!("[Utils] Installer command '{}' found.", command_name.green());
        Ok(())
    } else {
        log_error!(
            "[Utils] Installer command '{}' not found in system PATH. Cannot proceed with installations requiring it.",
            command_name.red()
        );
        Err(InstallerError::MissingCommand(command_name.to_string()))
    }
}

/// Executes additional commands specified in the tool configuration after successful installation.
///
/// This function handles the execution of post-installation commands that may be required
/// for proper tool setup, such as copying configuration files, creating directories, or
/// setting up symbolic links.
///
/// # Arguments
/// * `commands`: A reference to a vector of command strings to execute
/// * `working_dir`: The directory where commands should be executed (typically the extraction directory)
/// * `tool_name`: The name of the tool (used for logging purposes)
///
/// # Returns
/// * `Result<Vec<String>, String>`:
///   - `Ok(Vec<String>)`: Successfully executed all commands, returns the list of executed commands
///   - `Err(String)`: An error occurred during command execution, contains error description
///
/// # Command Execution Context
/// Commands are executed with the following characteristics:
/// - Working directory is set to the specified `working_dir` (usually the extracted archive location)
/// - Commands have access to all environment variables (including $HOME, $USER, etc.)
/// - Commands are executed using the system shell (`/bin/sh` on Unix-like systems)
/// - Each command is executed independently and sequentially
/// - If any command fails, the entire operation fails and returns an error
///
/// # Security Considerations
/// - Commands are executed with the same permissions as the current user
/// - No sandboxing or privilege restriction is applied
/// - Input validation should be performed by the caller to prevent command injection
/// - Consider the security implications of executing user-provided commands
pub(crate) fn execute_additional_commands(
    installer_prefix: &str,
    commands: &[String],
    working_dir: &std::path::Path,
    tool_name: &str,
) -> Result<Vec<String>, String> {
    log_info!(
        "{} Executing {} additional command(s) for {}",
        installer_prefix,
        commands.len().to_string().yellow(),
        tool_name.bold()
    );

    let mut executed_commands = Vec::new();

    for (index, command) in commands.iter().enumerate() {
        log_debug!(
            "{} Executing command {}/{} for {}: {}",
            installer_prefix,
            (index + 1).to_string().cyan(),
            commands.len().to_string().cyan(),
            tool_name.bold(),
            command.dimmed()
        );

        // Execute the command using the system shell
        // We use `/bin/sh` for Unix-like systems as it's the most portable option
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command).current_dir(working_dir);

        // Execute the command and capture the result
        match cmd.output() {
            Ok(output) => {
                // Check if the command succeeded (exit status 0)
                if output.status.success() {
                    log_debug!(
                        "{} Command {}/{} executed successfully for {}",
                        installer_prefix,
                        (index + 1).to_string().green(),
                        commands.len().to_string().green(),
                        tool_name.bold()
                    );

                    // Log stdout if present (for debugging purposes)
                    if !output.stdout.is_empty() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        log_debug!(
                            "[GitHub] Command output for {}: {}",
                            tool_name.bold(),
                            stdout.trim().dimmed()
                        );
                    }

                    executed_commands.push(command.clone());
                } else {
                    // Command failed - log error details and return failure
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    log_error!(
                        "{} Command {}/{} failed for {} with exit code {}: {}",
                        installer_prefix,
                        (index + 1).to_string().red(),
                        commands.len().to_string().red(),
                        tool_name.red(),
                        output.status.code().unwrap_or(-1).to_string().red(),
                        command.red()
                    );

                    if !stderr.is_empty() {
                        log_error!(
                            "{} Command stderr for {}: {}",
                            installer_prefix,
                            tool_name.red(),
                            stderr.trim().red()
                        );
                    }

                    if !stdout.is_empty() {
                        log_debug!(
                            "{} Command stdout for {}: {}",
                            installer_prefix,
                            tool_name.dimmed(),
                            stdout.trim().dimmed()
                        );
                    }

                    return Err(format!(
                        "Command '{}' failed with exit code {}: {}",
                        command,
                        output.status.code().unwrap_or(-1),
                        stderr.trim()
                    ));
                }
            },
            Err(e) => {
                // Failed to execute the command (e.g., command not found, permission denied)
                log_error!(
                    "{} Failed to execute command {}/{} for {}: {} - Error: {}",
                    installer_prefix,
                    (index + 1).to_string().red(),
                    commands.len().to_string().red(),
                    tool_name.red(),
                    command.red(),
                    e.to_string().red()
                );

                return Err(format!("Failed to execute command '{}': {}", command, e));
            },
        }
    }

    log_info!(
        "{} Successfully executed all {} additional command(s) for {}",
        installer_prefix,
        executed_commands.len().to_string().green(),
        tool_name.bold()
    );

    Ok(executed_commands)
}

/// Version that considers "1", "yes", "y", "on" as true values (case-insensitive)
pub fn is_env_var_set(env_var_name: &str) -> bool {
    std::env::var(env_var_name)
        .map(|val| {
            let lower_val = val.to_lowercase();
            matches!(lower_val.as_str(), "true" | "1" | "yes" | "y" | "on" | "enabled")
        })
        .unwrap_or(false)
}

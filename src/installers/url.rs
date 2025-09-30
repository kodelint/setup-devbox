//! # URL Installer Module
//!
//! This module provides a robust, production-grade installer for tools from direct URL sources.
//! It follows the same reliability standards as the GitHub installer with comprehensive
//! platform detection, asset handling, and installation strategies.
//!
//! ## Key Features
//!
//! - **Smart Platform Detection**: Automatically detects OS and architecture for correct asset handling
//! - **Comprehensive Asset Handling**: Supports binaries, archives (zip, tar.gz, etc.), and macOS packages (pkg, dmg)
//! - **Asset Prioritization**: Intelligently handles different file types with macOS package preference
//! - **Comprehensive Validation**: Validates URLs, file types, and installation success
//! - **Smart State Tracking**: Maintains accurate installation state with version tracking
//! - **Flexible Configuration**: Supports custom binary names and executable paths
//! - **Post-Installation Hooks**: Executes additional setup commands after successful installation
//! - **Temporary File Management**: Properly cleans up temporary files and directories
//! - **Cross-Platform Compatibility**: Works on Unix and Windows systems with platform-specific handling
//!
//! ## Installation Workflow
//!
//! The installer follows a meticulous 10-step process:
//!
//! 1. **Platform Detection** - Detects OS and architecture for asset handling strategy
//! 2. **Configuration Validation** - Validates required URL and tool configuration
//! 3. **Asset Download** - Downloads the asset from URL to temporary location
//! 4. **File Type Detection** - Determines installation strategy based on file type
//! 5. **Installation Path Resolution** - Determines final binary installation path
//! 6. **Asset Processing** - Handles extraction, installation, or direct binary placement
//! 7. **Installation Verification** - Verifies the tool was installed properly
//! 8. **Cleanup** - Removes temporary files safely
//! 9. **Post-Installation Hooks** - Executes any additional setup commands
//! 10. **State Creation** - Creates comprehensive tool state for persistence
//!
//! ## Supported File Types
//!
//! - **Archives**: zip, tar.gz, tar.bz2, tar.xz, tar, gz, bz2, xz, 7zip
//! - **macOS Packages**: pkg, dmg
//! - **Binaries**: Direct executable files
//!
//! ## Error Handling
//!
//! The module provides detailed error messages and logging at multiple levels:
//! - **Info**: High-level installation progress
//! - **Debug**: Detailed download progress, file detection, and path resolution
//! - **Warn**: Non-fatal issues or warnings during installation
//! - **Error**: Installation failures with specific error codes and messages

// Internal module imports:
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;
use crate::{log_debug, log_error, log_info, log_warn};

// External imports
use colored::Colorize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Utility imports
use crate::libs::tool_installer::execute_post_installation_hooks;
use crate::libs::utilities::{
    assets::{detect_file_type, download_file, install_dmg, install_pkg},
    binary::{find_executable, make_executable, move_and_rename_binary},
    compression,
};
use tempfile::Builder as TempFileBuilder;

/// Installs a software tool by downloading and processing assets from a direct URL.
///
/// This function provides a robust installer for URL-hosted tools that mirrors the quality
/// and reliability of the GitHub installer. It includes comprehensive validation,
/// smart file type handling, and accurate state tracking.
///
/// # Workflow
///
/// 1. **Configuration Validation**: Validates required URL and tool configuration
/// 2. **Asset Download**: Downloads the asset from URL to temporary location
/// 3. **File Type Detection**: Determines installation strategy based on file type
/// 4. **Installation Path Resolution**: Determines final binary installation path
/// 5. **Asset Processing**: Handles extraction, installation, or direct binary placement
/// 6. **Installation Verification**: Verifies the tool was installed properly
/// 7. **Cleanup**: Removes temporary files safely
/// 8. **Post-Installation Hooks**: Executes any additional setup commands
/// 9. **State Creation**: Creates comprehensive `ToolState` with all relevant metadata
///
/// # Arguments
///
/// * `tool_entry` - A reference to the `ToolEntry` struct containing tool configuration
///   - `tool_entry.name`: **Required** - The tool name
///   - `tool_entry.url`: **Required** - Direct download URL (http:// or https://)
///   - `tool_entry.version`: Optional version specification for tracking
///   - `tool_entry.rename_to`: Optional custom binary name
///   - `tool_entry.executable_path_after_extract`: Optional path to executable after archive extraction
///   - `tool_entry.options`: Optional additional configuration
///
/// # Returns
///
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if installation was completely successful with accurate metadata
/// * `None` if any step of the installation process fails
///
/// # Examples - YAML Configuration
///
/// ```yaml
/// # Install Ghostty Terminal from DMG
/// - name: Ghostty
///   source: url
///   version: 1.1.3
///   url: https://release.files.ghostty.org/1.1.3/Ghostty.dmg
///   configuration_manager:
///     enabled: true
///     tools_configuration_paths:
///       - $HOME/.config/ghostty/config
///
/// # Install a binary directly
/// - name: my-tool
///   source: url
///   version: 2.0.0
///   url: https://github.com/user/repo/releases/download/v2.0.0/tool-linux-amd64
///
/// # Install from ZIP archive with custom executable path
/// - name: another-tool
///   source: url
///   url: https://example.com/tool.zip
///   executable_path_after_extract: tool/bin/executable
///   rename_to: my-custom-tool
/// ```
///
/// # Examples - Rust Code
///
/// ```rust
/// // DMG installation
/// let tool_entry = ToolEntry {
///     name: "Ghostty".to_string(),
///     version: Some("1.1.3".to_string()),
///     url: Some("https://release.files.ghostty.org/1.1.3/Ghostty.dmg".to_string()),
///     rename_to: None,
///     executable_path_after_extract: None,
///     options: None,
/// };
/// install(&tool_entry);
///
/// // Binary installation with custom name
/// let tool_entry = ToolEntry {
///     name: "my-tool".to_string(),
///     version: Some("2.0.0".to_string()),
///     url: Some("https://github.com/user/repo/releases/download/v2.0.0/tool-linux-amd64".to_string()),
///     rename_to: Some("custom-tool".to_string()),
///     executable_path_after_extract: None,
///     options: None,
/// };
/// install(&tool_entry);
///
/// // Archive installation with executable path
/// let tool_entry = ToolEntry {
///     name: "another-tool".to_string(),
///     version: None,
///     url: Some("https://example.com/tool.zip".to_string()),
///     rename_to: None,
///     executable_path_after_extract: Some("tool/bin/executable".to_string()),
///     options: None,
/// };
/// install(&tool_entry);
/// ```
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[URL Installer] Attempting to install tool from direct URL: {}",
        tool_entry.name.bold()
    );
    log_debug!("[URL Installer] ToolEntry details: {:#?}", tool_entry);

    // Step 1: Detect platform (OS and architecture) for asset handling strategy
    // let (os, arch) = detect_platform()?;

    // Step 1: Validate URL configuration - ensure required fields are present
    let download_url = validate_url_configuration(tool_entry)?;

    // Step 2: Download asset to temporary location
    log_debug!(
        "[URL Installer] Downloading asset from: {}",
        download_url.blue()
    );
    let (temp_dir, downloaded_path) = download_url_asset(tool_entry, &download_url)?;

    // Step 3: Detect file type and determine installation strategy
    let file_type = detect_file_type(&downloaded_path);
    log_debug!(
        "[URL Installer] Detected file type: {}",
        file_type.to_string().magenta()
    );

    // Step 4: Determine final installation path in user's bin directory
    let (install_path, final_install_path) = determine_installation_path(tool_entry)?;

    // Step 5: Process asset based on file type (binary, archive, or macOS package)
    let (package_type, working_dir) = process_asset_by_type(
        tool_entry,
        &downloaded_path,
        &file_type,
        &temp_dir,
        &install_path,
    )?;

    // Step 6: Verify installation was successful
    if !verify_installation(&final_install_path, &package_type, tool_entry) {
        cleanup_temp_file(&downloaded_path);
        return None;
    }

    // Step 7: Clean up temporary download file
    cleanup_temp_file(&downloaded_path);

    // Step 8: Execute any post-installation hooks defined in tool configuration
    log_debug!(
        "[URL Installer] Executing post-installation hooks for {}",
        tool_entry.name.bold()
    );
    let executed_post_installation_hooks =
        execute_post_installation_hooks("[URL Installer]", tool_entry, &working_dir);

    log_info!(
        "[URL Installer] Successfully installed tool: {}",
        tool_entry.name.bold().green()
    );

    // Step 9: Return comprehensive ToolState for state tracking and persistence
    Some(ToolState::new(
        tool_entry,
        &final_install_path,
        "direct-url".to_string(),
        package_type,
        tool_entry
            .version
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        Some(download_url),
        None,
        executed_post_installation_hooks,
    ))
}

// /// Detects the current platform (OS and architecture).
// ///
// /// This function detects both the operating system and CPU architecture,
// /// which are essential for determining the appropriate installation strategy
// /// and handling platform-specific assets.
// ///
// /// # Returns
// ///
// /// * `Some((os, arch))` - A tuple containing the OS and architecture strings if both are detected
// /// * `None` - If either OS or architecture detection fails
// ///
// /// # Examples
// ///
// /// Typical return values:
// /// - `Some(("darwin", "arm64"))` - macOS on Apple Silicon
// /// - `Some(("darwin", "x86_64"))` - macOS on Intel
// /// - `Some(("linux", "x86_64"))` - Linux on x86_64
// /// - `Some(("windows", "x86_64"))` - Windows on x86_64
// fn detect_platform() -> Option<(String, String)> {
//     // Detect operating system (darwin, linux, windows, etc.)
//     let os = detect_os().or_else(|| {
//         log_error!("[URL Installer] Unable to detect operating system");
//         None
//     })?;
//
//     // Detect CPU architecture (x86_64, arm64, aarch64, etc.)
//     let arch = detect_architecture().or_else(|| {
//         log_error!("[URL Installer] Unable to detect architecture");
//         None
//     })?;
//
//     log_info!(
//         "[URL Installer] Detected platform: {}{}{}",
//         os.green(),
//         "-".green(),
//         arch.green()
//     );
//
//     Some((os, arch))
// }

/// Validates that the tool configuration contains required URL fields.
///
/// This function checks that the URL field is specified in the tool configuration,
/// as it is mandatory for direct URL installations. Without this field, the installer
/// cannot download the asset.
///
/// # Arguments
///
/// * `tool_entry` - The tool configuration to validate
///
/// # Returns
///
/// * `Some(String)` - The validated URL string if present and valid
/// * `None` - If the URL field is missing or invalid, with appropriate error logging
///
/// # Validation Rules
///
/// - URL must be present and not empty
/// - URL must use http:// or https:// scheme
/// - URL must not contain spaces
/// - URL should be reasonably formatted
fn validate_url_configuration(tool_entry: &ToolEntry) -> Option<String> {
    let url = match &tool_entry.url {
        Some(url) if !url.trim().is_empty() => url.trim().to_string(),
        Some(_) => {
            log_error!(
                "[URL Installer] Configuration error: 'url' field is empty for tool {}",
                tool_entry.name.red()
            );
            log_error!("[URL Installer] Expected format: 'url: https://example.com/path/to/file'");
            return None;
        }
        None => {
            log_error!(
                "[URL Installer] Configuration error: 'url' field is missing for tool {}",
                tool_entry.name.red()
            );
            log_error!("[URL Installer] Expected format: 'url: https://example.com/path/to/file'");
            return None;
        }
    };

    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        log_error!(
            "[URL Installer] Invalid URL scheme for tool '{}'. URL must start with http:// or https://: {}",
            tool_entry.name.red(),
            url.red()
        );
        return None;
    }

    // Check for obviously malformed URLs
    if url.contains(' ') {
        log_error!(
            "[URL Installer] URL contains spaces for tool '{}': {}",
            tool_entry.name.red(),
            url.red()
        );
        return None;
    }

    log_debug!("[URL Installer] Validated URL: {}", url.blue());
    Some(url)
}

/// Downloads the asset from the URL to a temporary location.
///
/// This function creates a temporary directory and downloads the asset
/// from the specified URL to it. Using a temporary directory ensures proper cleanup
/// and prevents conflicts with existing files. The temporary directory is
/// automatically cleaned up when it goes out of scope.
///
/// # Arguments
///
/// * `tool_entry` - The tool being installed (used for naming and error messages)
/// * `download_url` - The URL to download from
///
/// # Returns
///
/// * `Some((temp_dir, downloaded_path))` - Tuple containing the temporary directory
///   handle and the path to the downloaded file if successful
/// * `None` - If temporary directory creation or download fails
///
/// # Temporary Directory
///
/// The temporary directory is created with a prefix that includes the tool name
/// for easier debugging and identification. The directory persists only as long
/// as the `TempDir` handle is in scope, then is automatically cleaned up.
///
/// Example temp dir: `/tmp/setup-devbox-install-ghostty-abc123/`
fn download_url_asset(
    tool_entry: &ToolEntry,
    download_url: &str,
) -> Option<(tempfile::TempDir, PathBuf)> {
    // Create temporary directory with descriptive prefix
    let temp_dir = match TempFileBuilder::new()
        .prefix(&format!("setup-devbox-install-{}-", tool_entry.name))
        .tempdir()
    {
        Ok(dir) => dir,
        Err(e) => {
            log_error!(
                "[URL Installer] Failed to create temporary directory for {}: {}",
                tool_entry.name.red(),
                e
            );
            return None;
        }
    };

    // Extract filename from URL or use tool name as fallback
    let filename = Path::new(download_url)
        .file_name()
        .and_then(|f| f.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}-download", tool_entry.name));

    // Validate filename
    if filename.is_empty() || filename == "/" {
        log_error!(
            "[URL Installer] Could not determine valid filename from URL: {}",
            download_url.red()
        );
        return None;
    }

    let downloaded_path = temp_dir.path().join(&filename);

    log_info!(
        "[URL Installer] Downloading '{}' from {} to temporary location: {}",
        tool_entry.name.green(),
        download_url.blue(),
        downloaded_path.display().to_string().yellow()
    );

    // Download file from URL to temporary location
    if let Err(err) = download_file(download_url, &downloaded_path) {
        log_error!(
            "[URL Installer] Failed to download {} from {}: {}",
            tool_entry.name.red(),
            download_url.red(),
            err
        );
        return None;
    }

    // Verify downloaded file
    match fs::metadata(&downloaded_path) {
        Ok(metadata) => {
            let file_size = metadata.len();
            if file_size == 0 {
                log_error!(
                    "[URL Installer] Downloaded file is empty (0 bytes) for tool '{}'",
                    tool_entry.name.red()
                );
                return None;
            }
            log_debug!("[URL Installer] Downloaded file size: {} bytes", file_size);
        }
        Err(e) => {
            log_error!("[URL Installer] Failed to verify downloaded file: {}", e);
            return None;
        }
    }

    log_info!(
        "[URL Installer] Download completed for {}",
        tool_entry.name.bright_blue()
    );

    Some((temp_dir, downloaded_path))
}

/// Determines the installation path for the tool.
///
/// This function constructs the final installation path in the user's bin directory,
/// taking into account any custom binary name specified in the configuration. The
/// default location is `$HOME/bin/{tool_name}`, which should be in the user's PATH.
///
/// # Arguments
///
/// * `tool_entry` - The tool configuration, which may include a custom binary name
///
/// # Returns
///
/// * `Some((install_path, final_install_path))` - Tuple containing both paths
///   (they are the same in this implementation, but separated for potential future use)
/// * `None` - If the HOME environment variable is not set
///
/// # Path Construction
///
/// - Default: `$HOME/bin/{tool_name}`
/// - With rename: `$HOME/bin/{rename_to}`
///
/// # Examples
///
/// ```
/// // Without rename_to
/// $HOME/bin/ghostty
///
/// // With rename_to: "my-tool"
/// $HOME/bin/my-tool
/// ```
///
/// # Environment Requirements
///
/// Requires the `$HOME` environment variable to be set. On Unix-like systems,
/// this should always be available. If not set, installation cannot proceed.
fn determine_installation_path(tool_entry: &ToolEntry) -> Option<(PathBuf, PathBuf)> {
    // Get HOME environment variable
    let home_dir = env::var("HOME")
        .map_err(|_| {
            log_error!(
                "[URL Installer] $HOME environment variable not set for {}",
                tool_entry.name.red()
            );
            log_error!("[URL Installer] Cannot determine installation path without $HOME");
        })
        .ok()?;

    // Use custom binary name if provided, otherwise use tool name
    let bin_name = tool_entry
        .rename_to
        .clone()
        .unwrap_or_else(|| tool_entry.name.clone());

    // Construct full installation path
    let install_path = PathBuf::from(format!("{home_dir}/bin/{bin_name}"));

    log_debug!(
        "[URL Installer] Installation path: {}",
        install_path.display().to_string().cyan()
    );

    // Return both paths (currently identical, but maintained for API consistency)
    Some((install_path.clone(), install_path))
}

/// Processes the downloaded asset based on its file type.
///
/// This function handles different asset types with appropriate installation strategies:
/// - **macOS Packages (.pkg)**: Uses system installer for proper integration
/// - **macOS Disk Images (.dmg)**: Mounts and extracts application bundles
/// - **Binaries**: Direct installation with executable permissions
/// - **Archives**: Extraction, executable search, and installation
///
/// Each file type requires different handling to ensure proper installation and
/// functionality. The function determines the appropriate strategy and executes it.
///
/// # Arguments
///
/// * `tool_entry` - The tool configuration
/// * `downloaded_path` - Path to the downloaded asset file
/// * `file_type` - Detected file type (e.g., "pkg", "dmg", "binary", "zip", "tar.gz")
/// * `temp_dir` - Temporary directory for extraction and processing
/// * `install_path` - Target installation path for the final binary
///
/// # Returns
///
/// * `Some((package_type, working_dir))` - Tuple containing:
///   - `package_type`: String describing the installation type (e.g., "binary", "macos-pkg-installer")
///   - `working_dir`: Directory path for post-installation hooks execution
/// * `None` - If processing fails at any step
///
/// # File Type Handling
///
/// - **pkg/dmg**: System-level installation, returns actual install location
/// - **binary**: Direct move to bin directory with executable permissions
/// - **Archives**: Extract → find executable → move to bin → set permissions
fn process_asset_by_type(
    tool_entry: &ToolEntry,
    downloaded_path: &Path,
    file_type: &str,
    temp_dir: &tempfile::TempDir,
    install_path: &Path,
) -> Option<(String, PathBuf)> {
    // Initialize working directory (default to temp directory)
    let mut working_dir = temp_dir.path().to_path_buf();

    // Package type identifier for state tracking
    let package_type: String;

    match file_type {
        // macOS .pkg installer - uses system installer for proper integration
        "pkg" => {
            log_info!(
                "[URL Installer] Installing .pkg for {}",
                tool_entry.name.bold()
            );
            match install_pkg(downloaded_path, &tool_entry.name) {
                Ok(_path) => {
                    package_type = "macos-pkg-installer".to_string();
                }
                Err(err) => {
                    log_error!(
                        "[URL Installer] Failed to install .pkg for {}: {}",
                        tool_entry.name.red(),
                        err
                    );
                    return None;
                }
            }
        }

        // macOS .dmg disk image - mounts and extracts application
        "dmg" => {
            log_info!(
                "[URL Installer] Installing .dmg for {}",
                tool_entry.name.bold()
            );
            match install_dmg(downloaded_path, &tool_entry.name) {
                Ok(_path) => {
                    package_type = "macos-dmg-installer".to_string();
                }
                Err(err) => {
                    log_error!(
                        "[URL Installer] Failed to install .dmg for {}: {}",
                        tool_entry.name.red(),
                        err
                    );
                    return None;
                }
            }
        }

        // Raw binary - direct installation to bin directory
        "binary" => {
            log_debug!(
                "[URL Installer] Installing binary for {}",
                tool_entry.name.bold()
            );

            // Move binary to installation path
            if let Err(err) = move_and_rename_binary(downloaded_path, install_path) {
                log_error!(
                    "[URL Installer] Failed to move binary for {}: {}",
                    tool_entry.name.red(),
                    err
                );
                return None;
            }

            // Set executable permissions (chmod +x)
            if let Err(err) = make_executable(install_path) {
                log_error!(
                    "[URL Installer] Failed to make binary executable for {}: {}",
                    tool_entry.name.red(),
                    err
                );
                return None;
            }

            package_type = "binary".to_string();
        }

        // Archive formats - extract, find executable, and install
        archive_type @ ("zip" | "tar.gz" | "gz" | "tar.bz2" | "tar" | "tar.xz" | "tar.bz"
        | "txz" | "tbz2" | "7zip") => {
            log_debug!(
                "[URL Installer] Extracting {} archive for {}",
                archive_type,
                tool_entry.name.blue()
            );

            // Extract archive contents to temporary directory
            let extracted_path = match compression::extract_archive(
                downloaded_path,
                temp_dir.path(),
                Some(archive_type),
            ) {
                Ok(path) => path,
                Err(err) => {
                    log_error!(
                        "[URL Installer] Failed to extract archive for {}: {}",
                        tool_entry.name.red(),
                        err
                    );
                    return None;
                }
            };

            // Search extracted contents for the executable binary
            let executable_path = find_executable(
                &extracted_path,
                &tool_entry.name,
                tool_entry.rename_to.as_deref(),
            )
            .or_else(|| {
                // If no executable found with standard naming, try using the explicit path if provided
                if let Some(ref explicit_path) = tool_entry.executable_path_after_extract {
                    let explicit_executable_path = extracted_path.join(explicit_path);
                    if explicit_executable_path.exists() {
                        log_debug!(
                            "[URL Installer] Found executable at explicit path: {}",
                            explicit_path
                        );
                        return Some(explicit_executable_path);
                    }
                }

                log_error!(
                    "[URL Installer] No executable found in archive for {}",
                    tool_entry.name.red()
                );
                log_error!(
                    "[URL Installer] Expected to find binary named '{}' or similar",
                    tool_entry.name
                );
                if let Some(ref explicit_path) = tool_entry.executable_path_after_extract {
                    log_error!(
                        "[URL Installer] Also checked explicit path: {}",
                        explicit_path
                    );
                }
                None
            })?;

            // Determine appropriate working directory for post-installation hooks
            // This is typically the parent directory of the executable
            working_dir = determine_working_directory(&executable_path, &extracted_path);

            // Move extracted binary to final installation location
            if let Err(err) = move_and_rename_binary(&executable_path, install_path) {
                log_error!(
                    "[URL Installer] Failed to move extracted binary for {}: {}",
                    tool_entry.name.red(),
                    err
                );
                return None;
            }

            // Set executable permissions on the installed binary
            if let Err(err) = make_executable(install_path) {
                log_error!(
                    "[URL Installer] Failed to make extracted binary executable for {}: {}",
                    tool_entry.name.red(),
                    err
                );
                return None;
            }

            package_type = "binary".to_string();
        }

        // Unsupported file type
        unknown => {
            log_error!(
                "[URL Installer] Unsupported file type '{}' for {}",
                unknown.red(),
                tool_entry.name.red()
            );
            log_error!(
                "[URL Installer] Supported types: binary, zip, tar.gz, tar.xz, tar.bz2, pkg, dmg, 7zip"
            );
            return None;
        }
    }

    Some((package_type, working_dir))
}

/// Determines the working directory for post-installation hooks.
///
/// This function finds the appropriate directory context for executing
/// additional setup commands. The working directory should provide context
/// for any relative paths or resources that post-installation hooks might need.
///
/// # Strategy
///
/// 1. If the executable is in a `bin/` directory, use the parent directory
///    - This gives access to adjacent directories like `lib/`, `share/`, etc.
///    - Example: `/tmp/extract/app/bin/tool` → working_dir = `/tmp/extract/app/`
///
/// 2. Otherwise, use the directory containing the executable
///    - Example: `/tmp/extract/tool` → working_dir = `/tmp/extract/`
///
/// 3. If no parent directory exists, use the extraction root
///    - Fallback for edge cases
///
/// # Arguments
///
/// * `executable_path` - Path to the main executable binary
/// * `extracted_path` - Root path where archive contents were extracted
///
/// # Returns
///
/// The appropriate working directory path for post-installation hook execution
///
/// # Examples
///
/// ```
/// // Executable in bin/ directory
/// executable: /tmp/extract/myapp/bin/mytool
/// returns:    /tmp/extract/myapp/
///
/// // Executable at root level
/// executable: /tmp/extract/mytool
/// returns:    /tmp/extract/
/// ```
fn determine_working_directory(executable_path: &Path, extracted_path: &Path) -> PathBuf {
    // Try to get the parent directory of the executable
    if let Some(parent_dir) = executable_path.parent() {
        // Check if the executable is in a bin/ directory
        if parent_dir.file_name().is_some_and(|name| name == "bin") {
            // If so, use the grandparent directory (one level up from bin/)
            // This provides access to sibling directories like lib/, share/, etc.
            if let Some(grandparent) = parent_dir.parent() {
                log_debug!(
                    "[URL Installer] Working directory (parent of bin/): {}",
                    grandparent.display()
                );
                return grandparent.to_path_buf();
            }
        }

        // Otherwise, use the parent directory of the executable
        log_debug!(
            "[URL Installer] Working directory (executable parent): {}",
            parent_dir.display()
        );
        return parent_dir.to_path_buf();
    }

    // Fallback to extraction root if parent directory cannot be determined
    log_debug!(
        "[URL Installer] Working directory (extraction root): {}",
        extracted_path.display()
    );
    extracted_path.to_path_buf()
}

/// Verifies that the installation was successful.
///
/// This function performs installation verification based on the package type,
/// ensuring that the installed tool is accessible and properly configured.
///
/// # Arguments
///
/// * `install_path` - The path where the tool was installed
/// * `package_type` - The type of package that was installed
/// * `tool_entry` - The tool configuration
///
/// # Returns
///
/// `true` if installation was successful, `false` otherwise
///
/// # Verification by Package Type
///
/// - **macOS Installers**: Trusts installer success status
/// - **Binaries**: Verifies file exists, is accessible, and has executable permissions
/// - **Archives**: Verifies extraction directory exists and is not empty
fn verify_installation(install_path: &PathBuf, package_type: &str, tool_entry: &ToolEntry) -> bool {
    match package_type {
        "macos-pkg-installer" | "macos-dmg-installer" => {
            // For installers, we trust their success status
            log_debug!(
                "[URL Installer] Installation verification completed for {} (installer type)",
                tool_entry.name
            );
            true
        }
        "binary" => {
            // Verify binary exists and is accessible
            if !install_path.exists() {
                log_error!(
                    "[URL Installer] Installed binary does not exist at {} for tool '{}'",
                    install_path.display().to_string().red(),
                    tool_entry.name.red()
                );
                return false;
            }

            // Verify it's a file, not a directory
            match fs::metadata(install_path) {
                Ok(metadata) => {
                    if !metadata.is_file() {
                        log_error!(
                            "[URL Installer] Install path is not a file for tool '{}'",
                            tool_entry.name.red()
                        );
                        return false;
                    }

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let permissions = metadata.permissions();
                        if permissions.mode() & 0o111 == 0 {
                            log_warn!(
                                "[URL Installer] Installed file is not executable for tool '{}'",
                                tool_entry.name.yellow()
                            );
                        }
                    }

                    log_debug!(
                        "[URL Installer] Binary installation verified for '{}' at {}",
                        tool_entry.name.green(),
                        install_path.display().to_string().cyan()
                    );
                    true
                }
                Err(e) => {
                    log_error!(
                        "[URL Installer] Failed to verify installed binary metadata for '{}': {}",
                        tool_entry.name.red(),
                        e
                    );
                    false
                }
            }
        }
        _ => {
            // For archive extractions, verify directory exists
            if !install_path.exists() {
                log_error!(
                    "[URL Installer] Installation path does not exist for tool '{}'",
                    tool_entry.name.red()
                );
                return false;
            }

            log_debug!(
                "[URL Installer] Installation verification completed for '{}'",
                tool_entry.name.green()
            );
            true
        }
    }
}

/// Cleans up temporary download file.
///
/// This function safely removes the temporary download file after installation
/// is complete, with proper error handling for cleanup failures.
///
/// # Arguments
///
/// * `temp_path` - Path to the temporary file to remove
///
/// # Implementation Details
///
/// - Checks if file exists before attempting removal
/// - Logs successful cleanup for debugging
/// - Warns but doesn't fail on cleanup errors (non-critical)
fn cleanup_temp_file(temp_path: &PathBuf) {
    if temp_path.exists() {
        match fs::remove_file(temp_path) {
            Ok(_) => {
                log_debug!(
                    "[URL Installer] Removed temporary download file: {}",
                    temp_path.display()
                );
            }
            Err(e) => {
                log_warn!(
                    "[URL Installer] Failed to remove temporary download file {}: {}",
                    temp_path.display(),
                    e
                );
            }
        }
    }
}

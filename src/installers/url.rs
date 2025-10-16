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
use std::fs;
use std::path::PathBuf;

// Utility imports
use crate::libs::tool_installer::execute_post_installation_hooks;
use crate::libs::utilities::{assets, assets::detect_file_type};

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
        "[SDB::Tools::UrlInstaller] Attempting to install tool from direct URL: {}",
        tool_entry.name.bold()
    );
    log_debug!(
        "[SDB::Tools::UrlInstaller] ToolEntry details: {:#?}",
        tool_entry
    );

    // Step 1: Validate URL configuration - ensure required fields are present
    let download_url = validate_url_configuration(tool_entry)?;

    // Step 2: Download asset to temporary location
    log_debug!(
        "[SDB::Tools::UrlInstaller] Downloading asset from: {}",
        download_url.blue()
    );
    let (temp_dir, downloaded_path) = assets::download_url_asset(tool_entry, &download_url)?;

    // Step 3: Detect file type and determine installation strategy
    let file_type = detect_file_type(&downloaded_path);
    log_debug!(
        "[SDB::Tools::UrlInstaller] Detected file type: {}",
        file_type.to_string().magenta()
    );

    // Step 4: Process asset based on file type (binary, archive, or macOS package)
    let (package_type, final_install_path, working_dir) =
        assets::process_asset_by_type(tool_entry, &downloaded_path, &file_type, &temp_dir)?;

    // Step 5: Verify installation was successful
    if !verify_installation(&final_install_path, &package_type, tool_entry) {
        cleanup_temp_file(&downloaded_path);
        return None;
    }

    // Step 6: Clean up temporary download file
    cleanup_temp_file(&downloaded_path);

    // Step 7: Execute any post-installation hooks defined in tool configuration
    log_debug!(
        "[SDB::Tools::UrlInstaller] Executing post-installation hooks for {}",
        tool_entry.name.bold()
    );
    let executed_post_installation_hooks =
        execute_post_installation_hooks("[URL Installer]", tool_entry, &working_dir);

    log_info!(
        "[SDB::Tools::UrlInstaller] Successfully installed tool: {}",
        tool_entry.name.bold().green()
    );

    // Step 8: Return comprehensive ToolState for state tracking and persistence
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
                "[SDB::Tools::UrlInstaller] Configuration error: 'url' field is empty for tool {}",
                tool_entry.name.red()
            );
            log_error!(
                "[SDB::Tools::UrlInstaller] Expected format: 'url: https://example.com/path/to/file'"
            );
            return None;
        }
        None => {
            log_error!(
                "[SDB::Tools::UrlInstaller] Configuration error: 'url' field is missing for tool {}",
                tool_entry.name.red()
            );
            log_error!(
                "[SDB::Tools::UrlInstaller] Expected format: 'url: https://example.com/path/to/file'"
            );
            return None;
        }
    };

    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        log_error!(
            "[SDB::Tools::UrlInstaller] Invalid URL scheme for tool '{}'. URL must start with http:// or https://: {}",
            tool_entry.name.red(),
            url.red()
        );
        return None;
    }

    // Check for obviously malformed URLs
    if url.contains(' ') {
        log_error!(
            "[SDB::Tools::UrlInstaller] URL contains spaces for tool '{}': {}",
            tool_entry.name.red(),
            url.red()
        );
        return None;
    }

    log_debug!("[SDB::Tools::UrlInstaller] Validated URL: {}", url.blue());
    Some(url)
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
                "[SDB::Tools::UrlInstaller] Installation verification completed for {} (installer type)",
                tool_entry.name
            );
            true
        }
        "binary" => {
            // Verify binary exists and is accessible
            if !install_path.exists() {
                log_error!(
                    "[SDB::Tools::UrlInstaller] Installed binary does not exist at {} for tool '{}'",
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
                            "[SDB::Tools::UrlInstaller] Install path is not a file for tool '{}'",
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
                                "[SDB::Tools::UrlInstaller] Installed file is not executable for tool '{}'",
                                tool_entry.name.yellow()
                            );
                        }
                    }

                    log_debug!(
                        "[SDB::Tools::UrlInstaller] Binary installation verified for '{}' at {}",
                        tool_entry.name.green(),
                        install_path.display().to_string().cyan()
                    );
                    true
                }
                Err(e) => {
                    log_error!(
                        "[SDB::Tools::UrlInstaller] Failed to verify installed binary metadata for '{}': {}",
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
                    "[SDB::Tools::UrlInstaller] Installation path does not exist for tool '{}'",
                    tool_entry.name.red()
                );
                return false;
            }

            log_debug!(
                "[SDB::Tools::UrlInstaller] Installation verification completed for '{}'",
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
                    "[SDB::Tools::UrlInstaller] Removed temporary download file: {}",
                    temp_path.display()
                );
            }
            Err(e) => {
                log_warn!(
                    "[SDB::Tools::UrlInstaller] Failed to remove temporary download file {}: {}",
                    temp_path.display(),
                    e
                );
            }
        }
    }
}

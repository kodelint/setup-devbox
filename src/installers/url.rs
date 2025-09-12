// Internal module imports:
// These `use` statements bring necessary types and functions into scope for this module.

// `ToolEntry`: Represents a single tool's configuration as defined in your `tools.yaml` file.
//              It's a struct that contains all possible configuration fields for a tool,
//              such as name, version, source, URL, repository, etc.
// `ToolState`: Represents the actual state of an *installed* tool. This struct is used to
//              persist information about installed tools in the application's `state.json` file.
//              It helps `setup-devbox` track what's installed, its version, and where it's located.
use crate::schema::{ToolEntry, ToolState};
// Custom logging macros:
// These macros (`log_debug!`, `log_error!`, `log_info!`, `log_warn!`) provide a
// consistent and structured way to output messages at different severity levels.
// They help in debugging, providing user feedback, and indicating critical issues.
use crate::{log_debug, log_error, log_info, log_warn};
// Removed ToolEntryFull as ToolEntry is a struct
// `Colorize` trait from the `colored` crate:
// This trait extends string types, allowing them to be easily colored for improved
// readability in terminal output. For example, `my_string.bold().red()`
use colored::Colorize;
// `dirs` crate:
// Used to find common user directories, such as the home directory, which is essential
// for determining where `setup-devbox` should store its data and installed tools.
use dirs::home_dir;
// For file system operations: creating directories, reading files, etc.
// `std::fs` provides functions for interacting with the file system.
use std::{fs, io};
// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::{Path, PathBuf};

// External crate imports:
// `tempfile`: For creating temporary files and directories. Useful for downloads
//             to avoid polluting the main file system until installation is complete.
use tempfile::tempdir;

// Internal utility module imports:
// `download_file`: A custom utility function for downloading files, likely wrapping `ureq`.
// `detect_file_type_from_filename`: A utility to guess file types based on name/extension.
// `extract_archive`: A utility function to decompress and extract various archive formats.
use crate::libs::utilities::assets::{
    current_timestamp, detect_file_type, download_file, install_dmg, install_pkg,
};
use crate::libs::utilities::compression::extract_archive;

/// Installs a tool from a direct URL source.
///
/// This function handles the entire lifecycle of installing a tool provided via a direct URL:
/// 1. Determines the appropriate installation directory within the `setup-devbox` structure.
/// 2. Downloads the tool's asset from the specified URL to a temporary location.
/// 3. Detects the file type of the downloaded asset (e.g., zip, tar.gz, binary, pkg, dmg).
/// 4. Based on the file type, it extracts the archive, copies the binary, or executes the installer.
/// 5. Cleans up temporary download files.
/// 6. Persists the tool's installation state (`ToolState`) for `setup-devbox` to track.
///
/// # Arguments
/// * `tool_entry`: A `ToolEntry` struct containing the configuration for the tool to be installed,
///                 including its name, URL, and any specific installation options.
///
/// # Returns
/// * `Option<ToolState>`:
///   - `Some(ToolState)` if the installation was successful, containing the details of the installed tool.
///   - `None` if the installation failed at any step, with detailed errors logged.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[URL Installer] Attempting to install tool from direct URL: {}",
        tool_entry.name.green()
    );

    // 1. Validate tool entry has a URL
    let download_url_str = match &tool_entry.url {
        Some(url) => url.clone(),
        None => {
            log_error!(
                "[URL Installer] Tool '{}' is configured for direct URL installation but no URL was provided.",
                tool_entry.name.red()
            );
            return None;
        }
    };

    // 2. Determine installation directories
    // Get the user's home directory to construct the base path for devbox tools.
    let home_dir_path = home_dir()
        .ok_or_else(|| {
            log_error!("[URL Installer] Could not determine home directory.");
            io::Error::new(io::ErrorKind::NotFound, "Home directory not found")
        })
        .ok()?; // Propagate error if home_dir is not found

    // Construct the base directory for all setup-devbox tools (~/.setup-devbox/tools/).
    let devbox_tools_dir = home_dir_path.join(".setup-devbox").join("tools");
    // Construct the specific installation directory for this tool (~/.setup-devbox/tools/<tool_name>/).
    let tool_install_dir = devbox_tools_dir.join(&tool_entry.name);

    // Create the tool-specific installation directory if it doesn't already exist.
    // `fs::create_dir_all` creates all necessary parent directories if they don't exist.
    if let Err(e) = fs::create_dir_all(&tool_install_dir) {
        log_error!(
            "[URL Installer] Failed to create tool installation directory {}: {}",
            tool_install_dir.display().to_string().red(),
            e
        );
        return None;
    }

    // 3. Download the asset
    // Create a temporary directory for the downloaded file.
    let temp_dir = match tempdir() {
        Ok(dir) => dir,
        Err(e) => {
            log_error!(
                "[URL Installer] Failed to create temporary directory for download: {}",
                e
            );
            return None;
        }
    };
    // Construct the full path for the temporary downloaded file.
    let filename = Path::new(&download_url_str)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("downloaded_file");
    let temp_download_path = temp_dir.path().join(filename);

    log_info!(
        "[URL Installer] Downloading '{}' from {} to temporary location: {}",
        tool_entry.name.green(),
        download_url_str.blue(),
        temp_download_path.display().to_string().yellow()
    );
    if let Err(e) = download_file(&download_url_str, &temp_download_path) {
        log_error!(
            "[URL Installer] Failed to download '{}' from {}: {}",
            tool_entry.name.red(),
            download_url_str.red(),
            e.to_string().red()
        );
        return None;
    }
    log_info!(
        "[URL Installer] Download completed for: {}",
        tool_entry.name.green()
    );

    // 4. Determine file type and perform installation
    let final_install_path_for_state: PathBuf;
    let package_type: String;
    let executable_path_after_extract_for_state: Option<String>;

    // Use the more robust detect_file_type from assets.rs
    let detected_file_type = detect_file_type(&temp_download_path);
    log_debug!(
        "[URL Installer] Detected file type for {} is: {}",
        temp_download_path.display(),
        detected_file_type.yellow()
    );

    // This match block dispatches to specific installation logic based on the detected file type.
    // Each arm handles a different type of asset (archives, macOS packages, direct binaries)
    // to ensure proper extraction, copying, or execution.
    match detected_file_type.as_str() {
        // Handles various archive formats.
        "zip" | "tar.gz" | "tar.bz2" | "tar.xz" | "tar" | "gz" | "bz2" | "xz" | "7zip" => {
            log_info!(
                "[URL Installer] Extracting archive from URL: {}",
                download_url_str.cyan()
            );
            // Call the `extract_archive` utility function to decompress and extract the contents
            // of the downloaded archive into the tool's designated installation directory.
            match extract_archive(
                &temp_download_path,
                &tool_install_dir,
                Some(&detected_file_type),
            ) {
                Ok(path) => {
                    // `path` here refers to the directory where the archive was extracted (e.g., ~/.setup-devbox/tools/zed/extracted/).
                    final_install_path_for_state = path.clone();
                    // Set the package type to "binary" as the extracted contents will typically contain the binary.
                    package_type = "binary".to_string();
                    // If the tool entry specifies a path to the executable within the extracted archive,
                    // we clone it for persistence in `ToolState`.
                    executable_path_after_extract_for_state =
                        tool_entry.executable_path_after_extract.clone();
                }
                Err(e) => {
                    log_error!(
                        "[URL Installer] Failed to extract archive from {}: {}",
                        download_url_str.red(),
                        e.to_string().red()
                    );
                    return None;
                }
            }
        }
        "pkg" => {
            // macOS-specific .pkg installer handling.
            log_info!(
                "[URL Installer] Detected .pkg installer for {}. Initiating macOS package installation...",
                tool_entry.name.green()
            );
            // Call `install_pkg` to execute the macOS package installer. This function handles
            // the complexities of running `.pkg` files, which often install to system-wide locations
            // (e.g., `/Applications` or `/usr/local/bin`).
            match install_pkg(&temp_download_path, &tool_entry.name) {
                Ok(path) => {
                    // The `path` returned by `install_pkg` is the actual location where the application/tool was installed.
                    final_install_path_for_state = path;
                    // Mark the package type as a macOS PKG installer.
                    package_type = "macos-pkg-installer".to_string();
                    // For .pkg installs, the `install_path` typically points directly to the installed application
                    // or binary, so `executable_path_after_extract` is not applicable.
                    executable_path_after_extract_for_state = None;
                }
                Err(e) => {
                    log_error!(
                        "[URL] Failed to install .pkg for {}: {}",
                        tool_entry.name.red(),
                        e.to_string().red()
                    );
                    return None;
                }
            }
        }
        "dmg" => {
            // macOS-specific .dmg installer handling.
            log_info!(
                "[URL Installer] Detected .dmg installer for {}. Initiating macOS disk image installation...",
                tool_entry.name.green()
            );
            // Call `install_dmg` to handle mounting the disk image and copying its contents.
            // This function specifically deals with macOS disk images, which are common distribution formats.
            match install_dmg(&temp_download_path, &tool_entry.name) {
                Ok(path) => {
                    // The `path` here refers to the location where the application from the DMG was copied (e.g., `/Applications/AppName.app`).
                    final_install_path_for_state = path;
                    // Mark the package type as a macOS DMG installer.
                    package_type = "macos-dmg-installer".to_string();
                    // Similar to .pkg, the `install_path` directly points to the installed app, so this is `None`.
                    executable_path_after_extract_for_state = None;
                }
                Err(e) => {
                    log_error!(
                        "[URL Installer] Failed to install .dmg for {}: {}",
                        tool_entry.name.red(),
                        e.to_string().red()
                    );
                    return None;
                }
            }
        }
        "binary" => {
            // Handles cases where the downloaded file is a standalone executable binary.
            log_info!(
                "[URL Installer] Detected direct binary for {}. Copying to install directory...",
                tool_entry.name.green()
            );
            // Construct the target path for the binary within the tool's installation directory.
            let target_path = tool_install_dir.join(&tool_entry.name);
            // Copy the downloaded binary directly to its final installation path.
            match fs::copy(&temp_download_path, &target_path) {
                Ok(_) => {
                    log_info!(
                        "[URL Installer] Binary copied to: {}",
                        target_path.display().to_string().green()
                    );
                    // Ensure executable permissions on Unix-like systems.
                    // This `cfg(unix)` attribute ensures this code only compiles on Unix-like OSes
                    // where file permissions are relevant for executables.
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let metadata = match fs::metadata(&target_path) {
                            Ok(md) => md,
                            Err(e) => {
                                log_error!(
                                    "[URL Installer] Failed to get metadata for binary {}: {}",
                                    target_path.display().to_string().red(),
                                    e.to_string().red()
                                );
                                return None;
                            }
                        };
                        let mut permissions = metadata.permissions();
                        // Set permissions to 0o755 (rwx for owner, rx for group and others).
                        permissions.set_mode(0o755);
                        if let Err(e) = fs::set_permissions(&target_path, permissions) {
                            log_error!(
                                "[URL Installer] Failed to set permissions for binary {}: {}",
                                target_path.display().to_string().red(),
                                e.to_string().red()
                            );
                            return None;
                        }
                        log_debug!(
                            "[URL Installer] Executable permissions set for: {}",
                            target_path.display()
                        );
                    }
                    // The final install path is the path to the copied binary itself.
                    final_install_path_for_state = target_path;
                    // Mark the package type as a direct binary.
                    package_type = "binary".to_string();
                    // No extraction occurred, so this field is `None`.
                    executable_path_after_extract_for_state = None;
                }
                Err(e) => {
                    log_error!(
                        "[URL Installer] Failed to copy binary for {}: {}",
                        tool_entry.name.red(),
                        e.to_string().red()
                    );
                    return None;
                }
            }
        }
        _ => {
            // Fallback for any unsupported or unrecognized file types.
            log_error!(
                "[URL Installer] Unsupported file type detected for {}: '{}'.",
                tool_entry.name.red(),
                detected_file_type.red()
            );
            return None;
        }
    };

    // 5. Clean up the temporary downloaded file
    if let Err(e) = fs::remove_file(&temp_download_path) {
        log_warn!(
            "[URL Installer] Failed to remove temporary download file {}: {}",
            temp_download_path.display(),
            e
        );
    }

    // 6. Return ToolState for Tracking
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed. This is crucial
    // for future operations like uninstallation, updates, or syncing.
    Some(ToolState {
        // The version field for tracking. Defaults to "latest" if not explicitly set in `tools.yaml`.
        version: tool_entry
            .version
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        // The canonical path where the tool's executable was installed. This is the path
        // that will be recorded in the `state.json` file.
        install_path: final_install_path_for_state.to_string_lossy().into_owned(),
        // Flag indicating that this tool was installed by `setup-devbox`. This helps distinguish
        // between tools managed by our system and those installed manually.
        installed_by_devbox: true,
        // The method of installation, useful for future diagnostics or differing update logic.
        // In this module, it's always "direct-url".
        install_method: "direct-url".to_string(),
        // Records if the binary was renamed during installation, storing the new name.
        renamed_to: tool_entry.rename_to.clone(),
        // For direct URL installs, `repo` and `tag` are typically not applicable, so they are `None`.
        repo: None,
        tag: None,
        // The actual package type detected by the `file` command or inferred. This is for diagnostic
        // purposes, providing the most accurate type even if the installation logic
        // used a filename-based guess (e.g., "binary", "macos-pkg-installer").
        // Record the detected or assigned package type (e.g., "zip-archive", "binary", "macos-pkg-installer").
        package_type: package_type.to_string(),
        // Store the original download URL in the state for potential re-downloads or verification.
        url: Some(download_url_str.clone()),
        // Clone any additional options provided in the configuration.
        options: tool_entry.options.clone(),
        // Record the timestamp when the tool was installed or updated
        last_updated: Some(current_timestamp()),
        // This field is currently `None` but could be used to store the path to an executable
        // *within* an extracted archive if `install_path` points to the archive's root.
        executable_path_after_extract: executable_path_after_extract_for_state,
        // Record any additional commands that were executed during installation.
        // This is useful for tracking what was done and potentially for cleanup during uninstall.
        additional_cmd_executed: tool_entry.additional_cmd.clone(),
    })
}

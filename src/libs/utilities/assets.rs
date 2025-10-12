use crate::libs::utilities::binary::{find_executable, make_executable, move_and_rename_binary};
use crate::libs::utilities::compression;
use crate::libs::utilities::osx_pkg::{install_dmg, install_pkg};
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::tools::ToolEntry;
// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_info, log_warn};
// The 'colored' crate helps us make our console output look pretty and readab
use colored::Colorize;
// For creating and interacting with files.
// `std::fs::{File, OpenOptions}` allows for fine-grained control over file opening and creation.
use std::fs::File;
// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::{Path, PathBuf};
// To run external commands (like 'file' or 'sudo installer').
// `std::process::Command` allows the application to spawn and control external processes.
use std::process::Command;
use std::str;
use std::{fs, io};
use tempfile::Builder as TempFileBuilder;

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
pub fn download_url_asset(
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

/// Downloads a file from a given URL and saves it to a specified destination on the local file system.
/// This is crucial for fetching tools and resources from the internet (e.g., GitHub releases).
///
/// # Arguments
/// * `url`: The URL (as a string slice) of the file to download (e.g. [https://example.com/file.zip](https://example.com/file.zip)).
/// * `dest`: The local file system path (`&Path`) where the downloaded file should be saved.
///   This should be a full file path, including the desired filename.
///
/// # Returns
/// * `io::Result<()>`:
///   - `Ok(())` if the download was successful and the file was saved.
///   - An `io::Error` if anything went wrong during the HTTP request, file creation, or data copying.
pub fn download_file(url: &str, dest: &Path) -> io::Result<()> {
    // Log a debug message indicating the start of the download, coloring the URL for clarity.
    log_debug!("[Utils] Starting download from URL: {}", url.blue());

    // Execute the HTTP GET request using the `ureq` library.
    // `ureq::get(url).call()` sends the request and waits for a response.
    let response = match ureq::get(url).call() {
        Ok(res) => res, // If the request was successful, `res` contains the HTTP response.
        Err(e) => {
            // If the HTTP request itself failed (e.g., network error, invalid URL, DNS resolution failure).
            log_error!("[Utils] HTTP request failed for {}: {}", url.red(), e);
            // Convert the `ureq` error into a standard `io::Error` for consistent error handling
            // across the application. `std::io::Error::other` is a generic error kind.
            return Err(io::Error::other(format!("HTTP error: {e}")));
        }
    };

    // Open the destination file for writing.
    // `File::create(dest)` will create a new file if `dest` does not exist,
    // or truncate (empty) an existing file at `dest` if it does.
    // The `?` operator propagates any `io::Error` that occurs during file creation.
    let mut file = File::create(dest)?;

    // Get a reader for the response body (the actual data being downloaded from the network).
    let mut reader = response.into_reader();
    // Copy all data from the network `reader` directly into our local `file`.
    // This is an efficient way to stream data from the network to disk.
    // The `?` operator propagates any `io::Error` that occurs during the copy process (read/write errors).
    std::io::copy(&mut reader, &mut file)?;

    // Log a debug message upon successful download, coloring the destination path.
    log_debug!(
        "[Utils] File downloaded successfully to {}",
        dest.to_string_lossy().green()
    );
    Ok(()) // Indicate success by returning `Ok(())`.
}

/// Detects the file type of given path.
///
/// This function first attempts to guess the file type based on its extension (fast and common).
/// If the extension doesn't provide a clear, actionable type, it falls back to using the
/// `file` command for a deeper inspection of the file's magic bytes.
///
/// The returned string is a simplified, actionable type (e.g., "zip", "tar.gz", "pkg", "dmg", "binary").
/// This single function replaces both `detect_file_type`.
///
/// # Arguments
/// * `path`: A reference to the `Path` of the file whose type needs to be detected.
///
/// # Returns
/// * `String`: A string representing the detected file type.
pub fn detect_file_type(path: &Path) -> String {
    // 1. Initial quick check based on full filename and compound extensions first
    if let Some(file_name_str) = path.file_name().and_then(|s| s.to_str()) {
        let lower_file_name = file_name_str.to_lowercase();

        // Check for compound extensions (e.g., .tar.gz, .tar.xz) first
        if lower_file_name.ends_with(".tar.gz") {
            return "tar.gz".to_string();
        } else if lower_file_name.ends_with(".tar.xz") || lower_file_name.ends_with(".txz") {
            return "tar.xz".to_string();
        } else if lower_file_name.ends_with(".tar.bz2")
            || lower_file_name.ends_with(".tbz")
            || lower_file_name.ends_with(".tbz2")
        {
            return "tar.bz2".to_string();
        }
        // Then check for common single extensions. The order here is important
        // to ensure compound extensions are caught first.
        else if lower_file_name.ends_with(".zip") {
            return "zip".to_string();
        } else if lower_file_name.ends_with(".tar") {
            return "tar".to_string();
        } else if lower_file_name.ends_with(".gz") {
            return "gz".to_string();
        } else if lower_file_name.ends_with(".bz2") {
            return "bz2".to_string();
        } else if lower_file_name.ends_with(".xz") {
            return "xz".to_string();
        } else if lower_file_name.ends_with(".7z") {
            return "7zip".to_string();
        } else if lower_file_name.ends_with(".pkg") {
            return "pkg".to_string(); // macOS Package Installer
        } else if lower_file_name.ends_with(".dmg") {
            return "dmg".to_string(); // macOS Disk Image
        }
    }

    // 2. Fallback to `file` command for deeper inspection (more accurate for binaries, etc.)
    let output = match Command::new("file")
        .arg("--mime-type")
        .arg("--brief")
        .arg(path)
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            log_warn!(
                "[Utils] Failed to execute 'file' command for type detection: {}. Falling back to 'binary'.",
                e
            );
            return "binary".to_string(); // Default to binary if 'file' command fails
        }
    };

    let mime_type = String::from_utf8_lossy(&output.stdout).trim().to_string();
    log_debug!("[Utils] 'file' command detected MIME type: {}", mime_type);

    match mime_type.as_str() {
        "application/zip" => "zip".to_string(),
        "application/x-tar" => "tar".to_string(),
        "application/gzip" => "gz".to_string(),
        "application/x-bzip2" => "bz2".to_string(),
        "application/x-xz" => "xz".to_string(),
        // Specific handling for macOS installers based on MIME type, but confirm extension as a fallback
        "application/x-xar"
            if path
                .extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("pkg")) =>
        {
            "pkg".to_string()
        }
        "application/x-apple-diskimage"
            if path
                .extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("dmg")) =>
        {
            "dmg".to_string()
        }
        // Generic binary or unknown
        _ => "binary".to_string(), // Default fallback
    }
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
pub fn process_asset_by_type(
    tool_entry: &ToolEntry,
    downloaded_path: &Path,
    file_type: &str,
    temp_dir: &tempfile::TempDir,
) -> Option<(String, PathBuf, PathBuf)> {
    // Initialize working directory (default to temp directory)
    let mut working_dir = temp_dir.path().to_path_buf();

    // Package type identifier for state tracking
    let package_type: String;
    let final_install_path: PathBuf;

    match file_type {
        // macOS .pkg installer - uses system installer for proper integration
        "pkg" => {
            log_info!(
                "[GitHub Installer] Installing .pkg for {}",
                tool_entry.name.bold()
            );
            match install_pkg(downloaded_path, &tool_entry.name, &tool_entry.rename_to) {
                Ok(path) => {
                    package_type = "macos-pkg-installer".to_string();
                    final_install_path = path;
                }
                Err(err) => {
                    log_error!(
                        "[GitHub Installer] Failed to install .pkg for {}: {}",
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
                "[GitHub Installer] Installing .dmg for {}",
                tool_entry.name.bold()
            );
            match install_dmg(downloaded_path, &tool_entry.name, &tool_entry.rename_to) {
                Ok(path) => {
                    package_type = "macos-dmg-installer".to_string();
                    final_install_path = path;
                }
                Err(err) => {
                    log_error!(
                        "[GitHub Installer] Failed to install .dmg for {}: {}",
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
                "[GitHub Installer] Installing binary for {}",
                tool_entry.name.bold()
            );
            final_install_path = PathResolver::get_user_home_dir()?;
            // Move binary to installation path
            if let Err(err) = move_and_rename_binary(downloaded_path, &final_install_path) {
                log_error!(
                    "[GitHub Installer] Failed to move binary for {}: {}",
                    tool_entry.name.red(),
                    err
                );
                return None;
            }

            // Set executable permissions (chmod +x)
            if let Err(err) = make_executable(&final_install_path) {
                log_error!(
                    "[GitHub Installer] Failed to make binary executable for {}: {}",
                    tool_entry.name.red(),
                    err
                );
                return None;
            }

            package_type = "binary".to_string();
        }

        // Archive formats - extract, find executable, and install
        archive_type @ ("zip" | "tar.gz" | "gz" | "tar.bz2" | "tar" | "tar.xz" | "tar.bz"
        | "txz" | "tbz2") => {
            log_debug!(
                "[GitHub Installer] Extracting {} archive for {}",
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
                        "[GitHub Installer] Failed to extract archive for {}: {}",
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
                log_error!(
                    "[GitHub Installer] No executable found in archive for {}",
                    tool_entry.name.red()
                );
                log_error!(
                    "[GitHub Installer] Expected to find binary named '{}' or similar",
                    tool_entry.name
                );
                None
            })?;

            // Determine appropriate working directory for post-installation hooks
            // This is typically the parent directory of the executable
            working_dir =
                PathResolver::determine_working_directory(&executable_path, &extracted_path);

            final_install_path = PathResolver::get_user_home_dir()?;

            // Move extracted binary to final installation location
            if let Err(err) = move_and_rename_binary(&executable_path, &final_install_path) {
                log_error!(
                    "[GitHub Installer] Failed to move extracted binary for {}: {}",
                    tool_entry.name.red(),
                    err
                );
                return None;
            }

            // Set executable permissions on the installed binary
            if let Err(err) = make_executable(&final_install_path) {
                log_error!(
                    "[GitHub Installer] Failed to make extracted binary executable for {}: {}",
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
                "[GitHub Installer] Unsupported file type '{}' for {}",
                unknown.red(),
                tool_entry.name.red()
            );
            log_error!(
                "[GitHub Installer] Supported types: binary, zip, tar.gz, tar.xz, tar.bz2, pkg, dmg"
            );
            return None;
        }
    }

    Some((package_type, final_install_path, working_dir))
}

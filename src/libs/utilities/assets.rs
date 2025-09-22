// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::{Path, PathBuf};
// The 'colored' crate helps us make our console output look pretty and readab
use colored::Colorize;
// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_info, log_warn};
// For creating and interacting with files.
// `std::fs::{File, OpenOptions}` allows for fine-grained control over file opening and creation.
use std::fs::File;
// To run external commands (like 'file' or 'sudo installer').
// `std::process::Command` allows the application to spawn and control external processes.
use std::process::{Command, Stdio};
// `std::io` contains core input/output functionalities and error types.
use chrono::{DateTime, Duration, Utc};
use std::str;
use std::str::FromStr;
use std::{fs, io};
// Needed for `String::from_utf8_lossy`

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
            return Err(std::io::Error::other(format!("HTTP error: {e}")));
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
            if path.extension().map_or(false, |ext| {
                ext.to_string_lossy().eq_ignore_ascii_case("pkg")
            }) =>
        {
            "pkg".to_string()
        }
        "application/x-apple-diskimage"
            if path.extension().map_or(false, |ext| {
                ext.to_string_lossy().eq_ignore_ascii_case("dmg")
            }) =>
        {
            "dmg".to_string()
        }
        // Generic binary or unknown
        _ => "binary".to_string(), // Default fallback
    }
}

// install_pkg function (Updated to return PathBuf for the installed app)
/// Installs a software from a .pkg file on macOS.
/// This is a dummy implementation; your actual function needs to:
/// 1. Execute the `installer` command with the .pkg file.
/// 2. Determine and return the actual installation path (e.g., /Applications/AppName.app).
///
/// # Arguments
/// * `pkg_path`: The path to the .pkg file.
/// * `tool_name`: The name of the tool, used to guess the installation path.
///
/// # Returns
/// * `io::Result<PathBuf>`: `Ok(PathBuf)` if the PKG was installed successfully,
///   `Err(io::Error)` otherwise.
#[cfg(target_os = "macos")]
pub fn install_pkg(pkg_path: &Path, tool_name: &str) -> io::Result<PathBuf> {
    log_info!(
        "[macOS Installer] Initiating .pkg installation for: {}",
        pkg_path.display().to_string().bold()
    );
    log_info!("[macOS Installer] Executing .pkg installer (may require admin privileges)...");

    let installer_output = Command::new("sudo")
        .arg("installer")
        .arg("-pkg")
        .arg(pkg_path)
        .arg("-target")
        .arg("/") // Install to the root volume
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !installer_output.status.success() {
        let stderr = String::from_utf8_lossy(&installer_output.stderr);
        log_error!("[macOS Installer] Failed to install .pkg: {}", stderr.red());
        return Err(std::io::Error::other(format!(
            "Failed to install .pkg: {stderr}"
        )));
    }

    // Determine the actual installation path using a more generic heuristic
    // This part is inherently heuristic for generic PKG files.
    // For ultimate precision, a feature allowing users to specify `install_path` in tools.yaml
    // would be ideal, or a more complex PKG manifest parser.
    // Otherwise, we rely on common macOS installation patterns.
    let mut inferred_install_path = None;

    // 1. Check for application bundles in /Applications (common for GUI apps)
    let app_path = PathBuf::from(format!("/Applications/{tool_name}.app"));
    if app_path.exists() {
        log_debug!(
            "[macOS Installer] Found application bundle at: {}",
            app_path.display()
        );
        inferred_install_path = Some(app_path);
    }

    // 2. If not an app bundle, check common CLI tool root directories (e.g., /usr/local/go)
    if inferred_install_path.is_none() {
        let cli_root_path = PathBuf::from(format!("/usr/local/{tool_name}"));
        if cli_root_path.exists() && cli_root_path.is_dir() {
            log_debug!(
                "[macOS Installer] Found CLI tool root directory at: {}",
                cli_root_path.display()
            );
            inferred_install_path = Some(cli_root_path);
        } else {
            // As a fallback, check if a binary directly exists in /usr/local/bin
            let cli_bin_path = PathBuf::from(format!("/usr/local/bin/{tool_name}"));
            if cli_bin_path.exists() {
                log_debug!(
                    "[macOS Installer] Found CLI binary at: {}",
                    cli_bin_path.display()
                );
                inferred_install_path = Some(cli_bin_path);
            }
        }
    }

    // 3. Fallback if no specific path was found, or if the tool name doesn't lead to a direct match.
    // This is the least specific guess.
    let final_path = inferred_install_path.unwrap_or_else(|| {
        log_warn!(
            "[macOS Installer] Unable to precisely determine install path for '{}' PKG. \
             Returning a generic fallback path. For critical tools, consider manually verifying \
             the installation path or adding an explicit 'install_path' if that feature becomes available.",
            tool_name.green()
        );
        // Defaulting to /usr/local/bin/<tool_name> as a very common CLI install location.
        PathBuf::from(format!("/usr/local/bin/{tool_name}"))
    });

    log_info!(
        "[macOS Installer] PKG for {} installed successfully. Inferred install path: {}",
        tool_name.green(),
        final_path.display().to_string().green()
    );
    Ok(final_path)
}

#[cfg(not(target_os = "macos"))]
pub fn install_pkg(_pkg_path: &Path, _tool_name: &str) -> io::Result<PathBuf> {
    log_warn!(
        "[macOS Installer] .pkg installation is only supported on macOS. Skipping for this platform."
    );
    Err(io::Error::new(
        io::ErrorKind::Other,
        ".pkg installation is only supported on macOS.",
    ))
}

// install_dmg function (With corrected return type logic to PathBuf)
/// Installs a software from a .dmg (Disk Image) file on macOS.
///
/// This function attempts to:
/// 1. Mount the .dmg file.
/// 2. Search for either a .pkg installer or a .app bundle within the mounted volume,
///    prioritizing .pkg if both are present.
/// 3. If a .pkg is found, it calls `install_pkg` to install it.
/// 4. If a .app is found, it's copied to the `/Applications` directory.
/// 5. Unmount the .dmg file, **reliably**, regardless of installation success or failure.
///
/// # Arguments
/// * `dmg_path`: The path to the .dmg file.
/// * `app_name`: The expected name of the application (e.g., "Zed") to correctly
///   find and copy the `.app` bundle (e.g., "Zed.app").
///
/// # Returns
/// * `io::Result<PathBuf>`: `Ok(PathBuf)` if the DMG was processed successfully,
///   containing the final installation path; `Err(io::Error)` otherwise.
#[cfg(target_os = "macos")]
pub fn install_dmg(dmg_path: &Path, app_name: &str) -> io::Result<PathBuf> {
    log_info!(
        "[macOS Installer] Initiating .dmg installation for: {}",
        dmg_path.display().to_string().bold()
    );

    if !dmg_path.exists() || !dmg_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "DMG file does not exist or is not a file: {}",
                dmg_path.display()
            ),
        ));
    }

    let mounted_path: Option<PathBuf>;

    log_debug!("[macOS Installer] Mounting DMG: {}", dmg_path.display());
    let hdiutil_output = Command::new("sudo")
        .arg("hdiutil")
        .arg("attach")
        .arg("-nobrowse")
        .arg("-plist")
        .arg("-readonly")
        .arg("-noverify")
        .arg(dmg_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !hdiutil_output.status.success() {
        let stderr = String::from_utf8_lossy(&hdiutil_output.stderr);
        log_error!("[macOS Installer] Failed to mount DMG: {}", stderr.red());
        return Err(std::io::Error::other(format!(
            "[macOS Installer] Failed to mount DMG: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&hdiutil_output.stdout);
    if let Some(path_str) = extract_mounted_path_from_hdiutil_plist(&stdout) {
        let path = PathBuf::from(path_str);
        if path.exists() && path.is_dir() {
            log_info!(
                "[macOS Installer] DMG mounted successfully at: {}",
                path.display().to_string().green()
            );
            mounted_path = Some(path);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "hdiutil reported successful mount, but path does not exist or is not a directory: {}",
                    path.display()
                ),
            ));
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse mounted path from hdiutil output for {}",
                dmg_path.display()
            ),
        ));
    }

    let mounted_volume_path = mounted_path.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "DMG was not mounted or mounted path could not be determined.",
        )
    })?;

    //  Perform Installation and ensure unmount happens
    let install_result: io::Result<PathBuf> = (|| {
        // Changed closure return type to PathBuf
        let mut pkg_found: Option<PathBuf> = None;
        let mut app_found: Option<PathBuf> = None;

        for entry in fs::read_dir(&mounted_volume_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "pkg") {
                pkg_found = Some(path);
                break;
            } else if path.extension().map_or(false, |ext| ext == "app") {
                app_found = Some(path);
            }
        }

        if let Some(pkg_path) = pkg_found {
            log_info!(
                "[macOS Installer] Found .pkg installer: {}",
                pkg_path.display().to_string().bold()
            );
            log_info!(
                "[macOS Installer] Executing .pkg installer (may require admin privileges)..."
            );
            // Call install_pkg and return its result (which is PathBuf)
            install_pkg(&pkg_path, app_name)
        } else if let Some(app_path) = app_found {
            log_info!(
                "[macOS Installer] Found .app bundle: {}",
                app_path.display().to_string().bold()
            );
            let target_app_path = PathBuf::from("/Applications").join(format!("{}.app", app_name));

            if target_app_path.exists() {
                log_info!(
                    "[macOS Installer] Removing existing app at: {}",
                    target_app_path.display().to_string().yellow()
                );
                // --- FIX: Use sudo rm -rf for permission issues ---
                let rm_output = Command::new("sudo")
                    .arg("rm")
                    .arg("-rf") // Force recursively delete
                    .arg(&target_app_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()?;

                if !rm_output.status.success() {
                    let stderr = String::from_utf8_lossy(&rm_output.stderr);
                    log_error!(
                        "[macOS Installer] Failed to remove existing app {}: {}",
                        target_app_path.display(),
                        stderr.red()
                    );
                    return Err(std::io::Error::other(format!(
                        "Failed to remove existing app {}: {stderr}",
                        target_app_path.display()
                    )));
                }
                log_info!("[macOS Installer] Existing app removed successfully.");
            }

            log_debug!(
                "[macOS Installer] Copying .app to: {}",
                target_app_path.display()
            );
            let cp_output = Command::new("sudo")
                .arg("cp")
                .arg("-R")
                .arg(&app_path)
                .arg(&PathBuf::from("/Applications"))
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()?;

            if !cp_output.status.success() {
                let stderr = String::from_utf8_lossy(&cp_output.stderr);
                log_error!(
                    "[macOS Installer] Failed to copy .app to /Applications: {}",
                    stderr.red()
                );
                return Err(std::io::Error::other(format!(
                    "Failed to copy .app: {stderr}"
                )));
            }
            log_info!(
                "[macOS Installer] .app copied successfully to {}",
                target_app_path.display().to_string().green()
            );
            Ok(target_app_path) // Return the path for .app
        } else {
            log_warn!(
                "[macOS Installer] No .pkg or .app found in DMG: {}. Manual intervention may be required.",
                mounted_volume_path.display()
            );
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "No installable .app or .pkg found in DMG: {}",
                    mounted_volume_path.display()
                ),
            ))
        }
    })();

    // Unmount the DMG (always attempt, regardless of install_result)
    match unmount_dmg(&mounted_volume_path) {
        Ok(_) => log_debug!("[macOS Installer] DMG unmounted successfully."),
        Err(e) => {
            log_error!(
                "[macOS Installer] Failed to unmount DMG {}: {}",
                mounted_volume_path.display(),
                e.to_string().red()
            );
            if install_result.is_ok() {
                return Err(e);
            }
        }
    }

    log_info!(
        "[macOS Installer] .dmg installation process completed for: {}",
        dmg_path.display().to_string().green()
    );
    install_result // Return the result of the installation process (which includes the PathBuf)
}

#[cfg(not(target_os = "macos"))]
pub fn install_dmg(_dmg_path: &Path, _app_name: &str) -> io::Result<PathBuf> {
    log_warn!(
        "[macOS Installer] .dmg installation is only supported on macOS. Skipping for this platform."
    );
    Err(io::Error::new(
        io::ErrorKind::Other,
        ".dmg installation is only supported on macOS.",
    ))
}

/// Helper function to unmount a DMG.
///
/// # Arguments
/// * `mount_path`: The path where the DMG is mounted.
///
/// # Returns
/// * `io::Result<()>`: `Ok(())` if the DMG was unmounted successfully,
///   `Err(io::Error)` otherwise.
fn unmount_dmg(mount_path: &Path) -> io::Result<()> {
    log_debug!(
        "[macOS Installer] Attempting to unmount DMG from: {}",
        mount_path.display()
    );
    let detach_output = Command::new("sudo")
        .arg("hdiutil")
        .arg("detach")
        .arg("-force") // Force detach in case of busy errors
        .arg(mount_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !detach_output.status.success() {
        let stderr = String::from_utf8_lossy(&detach_output.stderr);
        return Err(std::io::Error::other(format!(
            "Failed to unmount DMG {}: {}",
            mount_path.display(),
            stderr
        )));
    }
    log_debug!("[macOS Installer] DMG unmounted successfully.");
    Ok(())
}

/// Helper to extract the mounted path from hdiutil's XML (plist) output.
///
/// This function parses the XML output from `hdiutil attach -plist` to find the
/// `<string>` value associated with the `<key>mount-point</key>`.
///
/// # Arguments
/// * `plist_output`: The `&str` containing the XML (plist) output from `hdiutil attach -plist`.
///
/// # Returns
/// * `Option<String>`: The mounted path as a `String` if found, otherwise `None`.
fn extract_mounted_path_from_hdiutil_plist(plist_output: &str) -> Option<String> {
    // A simple line-by-line search for the mount-point key and its subsequent string value.
    // For more complex plist structures, using a dedicated plist parser crate would be ideal.
    let mut lines = plist_output.lines().map(|s| s.trim());
    while let Some(line) = lines.next() {
        if line == "<key>mount-point</key>" {
            if let Some(path_line) = lines.next() {
                // The mount path is typically enclosed in <string> tags
                if path_line.starts_with("<string>") && path_line.ends_with("</string>") {
                    return Some(path_line[8..path_line.len() - 9].to_string());
                }
            }
        }
    }
    None
}

/// Returns the current timestamp in RFC 3339 format (ISO 8601).
///
/// This function provides a standardized, human-readable timestamp string
/// that includes timezone information. The RFC 3339 format is ideal for
/// serialization and storage as it's both machine-parsable and human-readable.
///
/// # Returns
/// A `String` containing the current UTC timestamp in RFC 3339 format.
/// Example: "2023-12-07T10:30:45.123456789+00:00"
///
/// # Examples
/// ```
/// let timestamp = current_timestamp();
/// println!("Current time: {}", timestamp); // e.g., "2023-12-07T10:30:45.123456789+00:00"
/// ```
pub fn current_timestamp() -> String {
    use chrono::Utc;
    // Get the current UTC datetime and format it according to RFC 3339
    // This includes fractional seconds and timezone offset
    Utc::now().to_rfc3339()
}

/// Parses a human-readable duration string into a Chrono `Duration` object.
///
/// This function converts natural language duration specifications into
/// a precise time duration that can be used for time calculations and comparisons.
///
/// # Arguments
/// * `duration_str` - A string slice containing the duration specification.
///   Expected format: "`<amount>` `<unit>`" (e.g., "7 days", "1 hour", "30 minutes")
///
/// # Returns
/// * `Some(Duration)` - If parsing was successful
/// * `None` - If the input format is invalid or contains unsupported units
///
/// # Supported Units
/// - "day" or "days" (converted to `Duration::days`)
/// - "hour" or "hours" (converted to `Duration::hours`)
/// - "minute" or "minutes" (converted to `Duration::minutes`)
///
/// # Examples
/// ```
/// let duration = parse_duration("7 days");
/// assert!(duration.is_some());
///
/// let invalid = parse_duration("soon");
/// assert!(invalid.is_none());
/// ```
pub fn parse_duration(duration_str: &str) -> Option<Duration> {
    // Split the input string by whitespace to separate amount from unit
    let parts: Vec<&str> = duration_str.split_whitespace().collect();

    // Validate the expected format: exactly two parts (amount and unit)
    if parts.len() != 2 {
        return None; // Invalid format - wrong number of components
    }

    // Parse the numeric amount from the first part
    let amount = i64::from_str(parts[0]).ok()?; // Returns None if parsing fails

    // Normalize the unit to lowercase for case-insensitive matching
    let unit = parts[1].to_lowercase();

    // Match the unit string to appropriate Duration constructor
    match unit.as_str() {
        "day" | "days" => Some(Duration::days(amount)),
        "hour" | "hours" => Some(Duration::hours(amount)),
        "minute" | "minutes" => Some(Duration::minutes(amount)),
        _ => None, // Unsupported time unit
    }
}

/// Determines if a given RFC 3339 timestamp is older than a specified duration.
///
/// This function is essential for implementing update policies where tools
/// with version "latest" should only be updated after a certain time has elapsed
/// since their last installation/update.
///
/// # Arguments
/// * `timestamp` - An RFC 3339 formatted timestamp string to check
/// * `duration` - Reference to a Chrono `Duration` representing the threshold
///
/// # Returns
/// * `true` - If the timestamp is older than the specified duration OR
///   if the timestamp cannot be parsed (error-safe default)
/// * `false` - If the timestamp is newer than or equal to the duration threshold
///
/// # Error Handling
/// If the timestamp cannot be parsed (invalid format), the function returns `true`
/// as a safety measure, assuming the tool should be updated to establish a proper
/// timestamp record.
///
/// # Examples
/// ```
/// let old_timestamp = "2023-01-01T00:00:00Z";
/// let new_timestamp = current_timestamp();
/// let one_day = Duration::days(1);
///
/// assert!(is_timestamp_older_than(old_timestamp, &one_day));
/// assert!(!is_timestamp_older_than(new_timestamp, &one_day));
/// ```
pub fn is_timestamp_older_than(timestamp: &str, duration: &Duration) -> bool {
    // Attempt to parse the RFC 3339 timestamp string into a DateTime object
    if let Ok(parsed_time) = DateTime::parse_from_rfc3339(timestamp) {
        // Convert the parsed time to UTC timezone for consistent comparison
        let time_utc = parsed_time.with_timezone(&Utc);
        let now = Utc::now();

        // Calculate the time elapsed since the timestamp and compare with threshold
        now - time_utc > *duration
    } else {
        // If timestamp parsing fails, adopt a conservative approach:
        // assume the timestamp is old and requires update.
        // This ensures tools with corrupted or missing timestamp data get updated
        // to establish a proper timestamp record.
        true
    }
}

/// Converts an RFC 3339 timestamp into a human-readable relative time string.
///
/// This function provides user-friendly time descriptions like "2 days ago"
/// or "3 hours ago" which are more intuitive for users than raw timestamps.
///
/// # Arguments
/// * `timestamp` - An RFC 3339 formatted timestamp string
///
/// # Returns
/// * `Some(String)` - Human-readable relative time description if parsing succeeds
/// * `None` - If the timestamp cannot be parsed
///
/// # Time Ranges
/// - More than 1 day: "X days ago"
/// - More than 1 hour: "X hours ago"
/// - More than 1 minute: "X minutes ago"
/// - Less than 1 minute: "just now"
///
/// # Examples
/// ```
/// let recent_time = current_timestamp();
/// println!("{}", time_since(&recent_time).unwrap()); // "just now" or "2 minutes ago"
///
/// let old_time = "2023-01-01T00:00:00Z";
/// println!("{}", time_since(old_time).unwrap()); // "340 days ago"
/// ```
pub fn time_since(timestamp: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| {
            // Calculate the duration between now and the provided timestamp
            let duration = Utc::now().signed_duration_since(dt.with_timezone(&Utc));

            // Select the most appropriate time unit based on the duration magnitude
            if duration.num_days() > 0 {
                format!("{} days ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{} hours ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{} minutes ago", duration.num_minutes())
            } else {
                // For durations less than a minute, use "just now"
                "just now".to_string()
            }
        })
        .ok() // Convert Result to Option, discarding any parsing errors
}

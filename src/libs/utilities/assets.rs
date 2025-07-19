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
use std::process::Command;
// `std::io` contains core input/output functionalities and error types.
use std::{fs, io};

/// Downloads a file from a given URL and saves it to a specified destination on the local file system.
/// This is crucial for fetching tools and resources from the internet (e.g., GitHub releases).
///
/// # Arguments
/// * `url`: The URL (as a string slice) of the file to download (e.g., "https://example.com/file.zip").
/// * `dest`: The local file system path (`&Path`) where the downloaded file should be saved.
///           This should be a full file path, including the desired filename.
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
            // across the application. `io::ErrorKind::Other` is a generic error kind.
            return Err(io::Error::new(io::ErrorKind::Other, format!("HTTP error: {}", e)));
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
    log_debug!("[Utils] File downloaded successfully to {}", dest.to_string_lossy().green());
    Ok(()) // Indicate success by returning `Ok(())`.
}

/// Attempts to detect the type of given file by executing the native `file` command (on Unix-like systems).
/// This is super useful for figuring out if a downloaded file is a zip, tar.gz, a raw binary, etc.,
/// which then guides how we should extract or handle it. This method provides more accurate detection
/// than just filename extensions because it inspects the file's content/magic bytes.
///
/// # Arguments
/// * `path`: The path (`&Path`) to the file whose type we want to detect.
///
/// # Returns
/// * `String`: A normalized string representing the detected file type (e.g., "zip", "tar.gz", "binary", "unknown").
///             It will never return an empty string.
/// Determines the type of file based on its extension and, more robustly,
/// by using the `file` command on Unix-like systems.
/// This helps in deciding how to process the file (e.g., extract, move, or skip).
pub fn detect_file_type(path: &Path) -> String {
    log_debug!("[Utils] Entering detect_file_type for: {}", path.display().to_string().yellow());

    let file_name_str = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_lowercase();

    log_debug!("[Utils] (Debug) file_name_str: '{}'", file_name_str);
    log_debug!("[Utils] (Debug) extension: '{}'", extension);

    // 1. Check by common *compound* extensions first (most specific match)
    if file_name_str.ends_with(".tar.xz") {
        log_debug!("[Utils] (Debug) Matched .tar.xz via file_name_str.ends_with. Returning 'tar.xz'.");
        return "tar.xz".to_string();
    }
    if file_name_str.ends_with(".txz") { // txz is a common alias for tar.xz
        log_debug!("[Utils] (Debug) Matched .txz via file_name_str.ends_with. Returning 'tar.xz'.");
        return "tar.xz".to_string();
    }
    if file_name_str.ends_with(".tar.gz") {
        log_debug!("[Utils] (Debug) Matched .tar.gz via file_name_str.ends_with. Returning 'tar.gz'.");
        return "tar.gz".to_string();
    }
    if file_name_str.ends_with(".tgz") { // tgz is a common alias for tar.gz
        log_debug!("[Utils] (Debug) Matched .tgz via file_name_str.ends_with. Returning 'tar.gz'.");
        return "tar.gz".to_string();
    }
    if file_name_str.ends_with(".tar.bz2") || file_name_str.ends_with(".tbz2") || file_name_str.ends_with(".tar.bz") {
        log_debug!("[Utils] (Debug) Matched .tar.bz2/.tbz2/.tar.bz via file_name_str.ends_with. Returning 'tar.bz2'.");
        return "tar.bz2".to_string();
    }
    if file_name_str.ends_with(".7z") {
        log_debug!("[Utils] (Debug) Matched .7z via file_name_str.ends_with. Returning '7zip'.");
        return "7zip".to_string();
    }

    // 2. Then check by single common extensions.
    log_debug!("[Utils] (Debug) Proceeding to check single extension match for: '{}'", extension);
    match extension.as_str() {
        "zip" => { log_debug!("[Utils] (Debug) Matched 'zip' extension. Returning 'zip'."); return "zip".to_string(); },
        "tar" => { log_debug!("[Utils] (Debug) Matched 'tar' extension. Returning 'tar'."); return "tar".to_string(); },
        "gz" => { log_debug!("[Utils] (Debug) Matched 'gz' extension. Returning 'gzip'."); return "gzip".to_string(); },
        "bz2" => { log_debug!("[Utils] (Debug) Matched 'bz2' extension. Returning 'bzip2'."); return "bzip2".to_string(); },
        "xz" => { log_debug!("[Utils] (Debug) Matched 'xz' extension. Returning 'xz'."); return "xz".to_string(); },
        "pkg" => { log_debug!("[Utils] (Debug) Matched 'pkg' extension. Returning 'macos-pkg-installer'."); return "macos-pkg-installer".to_string(); },
        "dmg" => { log_debug!("[Utils] (Debug) Matched 'dmg' extension. Returning 'macos-dmg-installer'."); return "macos-dmg-installer".to_string(); },
        _ => { log_debug!("[Utils] (Debug) No single extension matched. Falling through."); }
    }

    // 3. Use `file --brief` command for more accurate detection on Unix-like systems
    #[cfg(unix)]
    {
        log_debug!("[Utils] (Debug) Attempting 'file --brief' command for deeper analysis.");
        if let Ok(output) = Command::new("file").arg("--brief").arg(path).output() {
            if output.status.success() {
                let file_type_output = String::from_utf8_lossy(&output.stdout).to_lowercase();
                log_debug!("[Utils] 'file' command raw output for {:?}: '{}'", path.display(), file_type_output.trim());

                if file_type_output.contains("zip archive") { log_debug!("[Utils] (Debug) File command matched zip archive. Returning 'zip'."); return "zip".to_string(); }
                if file_type_output.contains("gzip compressed data") { log_debug!("[Utils] (Debug) File command matched gzip. Returning 'gzip'."); return "gzip".to_string(); }
                if file_type_output.contains("bzip2 compressed data") { log_debug!("[Utils] (Debug) File command matched bzip2. Returning 'bzip2'."); return "bzip2".to_string(); }
                if file_type_output.contains("xz compressed data") { log_debug!("[Utils] (Debug) File command matched xz. Returning 'xz'."); return "xz".to_string(); }
                if file_type_output.contains("posix tar archive") || file_type_output.contains("tar archive") { log_debug!("[Utils] (Debug) File command matched tar. Returning 'tar'."); return "tar".to_string(); }
                if file_type_output.contains("7-zip archive") { log_debug!("[Utils] (Debug) File command matched 7zip. Returning '7zip'."); return "7zip".to_string(); }
                if file_type_output.contains("xar archive") { log_debug!("[Utils] (Debug) File command matched xar archive. Returning 'macos-pkg-installer'."); return "macos-pkg-installer".to_string(); }
                if file_type_output.contains("apple binary property list") || file_type_output.contains("disk image") || file_type_output.contains("apple hfs") { log_debug!("[Utils] (Debug) File command matched dmg. Returning 'macos-dmg-installer'."); return "macos-dmg-installer".to_string(); }
                if file_type_output.contains("mach-o") && file_type_output.contains("executable") { log_debug!("[Utils] (Debug) File command matched Mach-O executable. Returning 'binary'."); return "binary".to_string(); }

                log_warn!("[Utils] Unrecognized file type by 'file' command for {:?}: '{}'. Treating as unknown.", path.display(), file_type_output.trim());
                return "unknown".to_string();
            } else {
                log_warn!("[Utils] 'file' command failed for {:?}: {}. Falling back to filename heuristics.", path.display(), String::from_utf8_lossy(&output.stderr).trim());
            }
        } else {
            log_warn!("[Utils] Could not execute 'file' command for {:?}. Falling back to filename heuristics.", path.display());
        }
    }

    // 4. Fallback to filename heuristics (less reliable, used if previous methods fail)
    log_debug!("[Utils] (Debug) Falling back to filename heuristics for direct binary detection.");
    if file_name_str.contains("linux") || file_name_str.contains("windows") ||
        file_name_str.contains("darwin") || file_name_str.contains("macos") ||
        file_name_str.contains("amd64") || file_name_str.contains("aarch64") ||
        file_name_str.contains("x86_64") || file_name_str.contains("arm64") {
        log_debug!("[Utils] Filename '{}' suggests a direct binary (no standard archive/installer extension). Returning 'binary'.", file_name_str);
        return "binary".to_string();
    }


    // If all else fails
    log_warn!("[Utils] Could not determine file type for {:?} based on any method. Defaulting to 'unknown'.", path.display());
    "unknown".to_string()
}

/// Detects file type based purely on its filename extension(s) or common naming patterns.
/// This is useful when the exact file type is known from its source (e.g., GitHub asset name)
/// and a full `file` command inspection might be overkill or less precise for specific cases.
/// This method is faster but less reliable than `detect_file_type` as it relies on naming conventions.
///
/// # Arguments
/// * `filename`: The full filename (`&str`) to analyze (e.g., "mytool-v1.0.0-linux-x64.tar.gz").
///
/// # Returns
/// * `String`: A normalized string representing the detected file type (e.g., "zip", "tar.gz", "binary", "unknown").
pub fn detect_file_type_from_filename(filename: &str) -> String {
    // Convert the filename to lowercase for case-insensitive matching.
    let filename_lower = filename.to_lowercase();
    log_debug!("[Utils] Detecting file type from filename: {}", filename_lower.yellow());

    // Prioritize more specific/longer extensions first to avoid false positives.
    // For example, ".tar.gz" should be matched before just ".gz".
    if filename_lower.ends_with(".tar.gz") || filename_lower.ends_with(".tgz") {
        "tar.gz".to_string()
    } else if filename_lower.ends_with(".tar.bz2") || filename_lower.ends_with(".tbz") {
        "tar.bz2".to_string()
    } else if filename_lower.ends_with(".tar.xz") || filename_lower.ends_with(".txz") {
        "tar.xz".to_string()
    } else if filename_lower.ends_with(".zip") {
        "zip".to_string()
    } else if filename_lower.ends_with(".tar") {
        "tar".to_string()
    } else if filename_lower.ends_with(".gz") { // .gz should come after .tar.gz to ensure .tar.gz is matched first
        "gz".to_string()
    } else if filename_lower.ends_with(".deb") { // Debian package format
        "deb".to_string()
    } else if filename_lower.ends_with(".rpm") { // Red Hat package manager format
        "rpm".to_string()
    } else if filename_lower.ends_with(".dmg") { // macOS Disk Image
        "dmg".to_string()
    } else if filename_lower.ends_with(".pkg") { // macOS package type
        "pkg".to_string()
    }
    // Added logic for direct binaries without explicit extensions, but containing platform info.
    // This heuristic tries to identify raw executables often named without extensions
    // but containing OS/architecture hints.
    else if filename_lower.contains("macos") || filename_lower.contains("linux") || filename_lower.contains("windows") ||
        filename_lower.contains("darwin") || filename_lower.contains("amd64") || filename_lower.contains("x86_64") ||
        filename_lower.contains("arm64") || filename_lower.contains("aarch64") ||
        // Consider common binary naming conventions without explicit OS/Arch if it's not an archive.
        // This checks if the filename has no extension (e.g., "kubectl" instead of "kubectl.exe")
        // AND contains common binary/CLI-related keywords.
        (!filename_lower.contains('.') && (filename_lower.contains("bin") || filename_lower.contains("cli"))) {
        log_debug!("[Utils] Filename '{}' suggests a direct binary (no standard archive extension, but contains platform/binary hints).", filename_lower.cyan());
        "binary".to_string()
    }
    else {
        // If no known extension or pattern matches, return "unknown".
        log_warn!("[Utils] Unrecognized file type based on filename for '{}'. Returning 'unknown'.", filename_lower.purple());
        "unknown".to_string()
    }
}

/// Installs a macOS `.pkg` installer file using `sudo installer`.
/// This function is specifically designed for macOS as `.pkg` files are native macOS installers.
/// It requires `sudo` privileges to run, meaning the user will be prompted for their password.
///
/// # Arguments
/// * `path`: The path (`&Path`) to the `.pkg` file that needs to be installed.
///
/// # Returns
/// * `io::Result<()>`:
///   - `Ok(())` if the installation command was executed successfully and returned a success status.
///   - An `io::Error` if the command fails to execute or returns a non-zero exit status (indicating failure).
#[cfg(target_os = "macos")] // This function only compiles when the target operating system is macOS.
pub fn install_pkg(path: &Path) -> io::Result<()> {
    log_info!("[Utils] Installing .pkg file: {:?}", path.to_string_lossy().bold());

    // Execute the `sudo installer -pkg <path> -target /` command.
    // - `Command::new("sudo")`: Invokes the `sudo` command, which prompts for user password
    //                          and then executes the following command with root privileges.
    // - `.arg("installer")`: The macOS built-in command-line tool for installing packages.
    // - `.arg("-pkg").arg(path)`: Specifies the package file to install.
    // - `.arg("-target").arg("/")`: Specifies the installation target. `/` typically means
    //                               the root of the boot volume, making it a system-wide installation.
    // - `.status()`: Executes the command and returns its exit status (`ExitStatus`).
    // - `?`: Propagates any `io::Error` that occurs if the `sudo` command itself cannot be spawned.
    let status = Command::new("sudo")
        .arg("installer")
        .arg("-pkg")
        .arg(path)
        .arg("-target")
        .arg("/")
        .status()?;

    // Check if the command exited successfully (i.e., with an exit code of 0).
    if !status.success() {
        // If the command failed, log an error message and return an `io::Error`.
        log_error!("[Utils] .pkg installer failed with status: {:?}", status);
        return Err(io::Error::new(io::ErrorKind::Other, "Installer failed"));
    }

    log_info!("[Utils] .pkg file installed successfully!");
    Ok(()) // Indicate success.
}

/// Installs a software from a .dmg (Disk Image) file on macOS.
///
/// This function attempts to:
/// 1. Mount the .dmg file.
/// 2. Search for either a .app bundle or a .pkg installer within the mounted volume.
/// 3. If a .app is found, it's copied to the /Applications directory (requires sudo).
/// 4. If a .pkg is found, it calls `install_pkg` to install it (requires sudo).
/// 5. Unmount the .dmg file, regardless of installation success or failure.
///
/// # Arguments
/// * `dmg_path`: The path to the .dmg file.
///
/// # Returns
/// * `io::Result<()>`: `Ok(())` if the DMG was processed successfully,
///   `Err(io::Error)` otherwise.
pub fn install_dmg(dmg_path: &Path) -> io::Result<()> {
    log_info!("[macOS Installer] Initiating .dmg installation for: {:?}", dmg_path.to_string_lossy().bold());

    // Mount the DMG
    log_debug!("[macOS Installer] Mounting DMG: {:?}", dmg_path.display());
    let mount_output = Command::new("sudo") // `sudo` might be needed for some DMGs or in certain environments
        .arg("hdiutil")
        .arg("attach")
        .arg("-nobrowse") // Don't open a Finder window
        .arg("-readonly") // Mount read-only to be safe
        .arg("-noverify") // Skip verification (can speed up, but less safe)
        .arg("-noautofsck") // Skip filesystem check
        .arg(dmg_path)
        .output()?;

    if !mount_output.status.success() {
        let stderr = str::from_utf8(&mount_output.stderr).unwrap_or("Failed to read stderr");
        log_error!("[macOS Installer] Failed to mount DMG: {}", stderr.red());
        return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to mount DMG: {}", stderr)));
    }

    let stdout = str::from_utf8(&mount_output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8 in mount output: {}", e)))?;

    // Parse mount path from hdiutil output. Output typically looks like:
    // /dev/diskXsY             /Volumes/VolumeName
    // We're looking for the last path, which is the mount point.
    let mount_path_str = stdout.lines()
        .last() // Get the last line
        .and_then(|line| line.split('\t').last()) // Split by tab, get last part
        .map(|s| s.trim()) // Trim whitespace
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Could not parse DMG mount path from hdiutil output"))?;

    let mount_path = PathBuf::from(mount_path_str);
    log_info!("[macOS Installer] DMG mounted successfully at: {}", mount_path.display().to_string().green());

    // Ensure the DMG is unmounted when the function exits (success or error)
    // Using a scopeguard or similar deferred execution would be more robust,
    // but for this example, we'll ensure it's called before any returns.
    let _unmount_result = unmount_dmg(&mount_path); // Call unmount at the end

    // Search for Contents (.app or .pkg)
    let mut pkg_found = None;
    let mut app_found = None;

    if mount_path.exists() && mount_path.is_dir() {
        for entry in fs::read_dir(&mount_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "pkg") {
                pkg_found = Some(path);
                break; // Prioritize .pkg installers
            } else if path.extension().map_or(false, |ext| ext == "app") {
                app_found = Some(path);
                // Don't break yet, in case there's a .pkg after an .app
            }
        }
    } else {
        log_error!("[macOS Installer] Mounted path does not exist or is not a directory: {:?}", mount_path.display());
        return Err(io::Error::new(io::ErrorKind::NotFound, "Mounted DMG path not found"));
    }

    // Install Contents
    if let Some(pkg_path) = pkg_found {
        log_info!("[macOS Installer] Found .pkg installer: {:?}", pkg_path.display());
        // Call the existing install_pkg function
        match install_pkg(&pkg_path) {
            Ok(_) => {
                log_info!("[macOS Installer] .pkg installed successfully from DMG.");
                unmount_dmg(&mount_path)?; // Explicit unmount on success
                return Ok(());
            },
            Err(e) => {
                log_error!("[macOS Installer] Failed to install .pkg from DMG: {}", e);
                unmount_dmg(&mount_path)?; // Explicit unmount on failure
                return Err(e);
            }
        }
    } else if let Some(app_path) = app_found {
        log_info!("[macS Installer] Found .app bundle: {:?}", app_path.display());
        let target_app_path = PathBuf::from("/Applications").join(app_path.file_name().unwrap());

        log_debug!("[macOS Installer] Copying .app to: {:?}", target_app_path.display());
        let cp_status = Command::new("sudo")
            .arg("cp")
            .arg("-R") // Recursive copy for directories (like .app bundles)
            .arg(&app_path)
            .arg(&target_app_path)
            .status()?;

        if !cp_status.success() {
            log_error!("[macOS Installer] Failed to copy .app to /Applications. Status: {:?}", cp_status);
            unmount_dmg(&mount_path)?; // Explicit unmount on failure
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to copy .app"));
        }
        log_info!("[macOS Installer] .app copied successfully to {}", target_app_path.display().to_string().green());
    } else {
        log_warn!("[macOS Installer] No .pkg or .app found in DMG: {:?}. Manual intervention may be required.", mount_path.display());
        unmount_dmg(&mount_path)?; // Explicit unmount, nothing to install but cleanup is needed
        return Err(io::Error::new(io::ErrorKind::NotFound, "No installable .app or .pkg found in DMG"));
    }

    // Unmount the DMG (handled by explicit calls or the deferred function call)
    unmount_dmg(&mount_path)?; // Final unmount in case previous paths didn't return early

    log_info!("[macOS Installer] .dmg installation process completed for: {}", dmg_path.display().to_string().green());
    Ok(())
}

/// Helper function to unmount a DMG.
fn unmount_dmg(mount_path: &Path) -> io::Result<()> {
    log_debug!("[macOS Installer] Unmounting DMG from: {:?}", mount_path.display());
    let detach_status = Command::new("sudo")
        .arg("hdiutil")
        .arg("detach")
        .arg("-force") // Force detach in case of busy errors
        .arg(mount_path)
        .status()?;

    if !detach_status.success() {
        log_error!("[macOS Installer] Failed to unmount DMG: {:?}", detach_status);
        // This is a cleanup step, so don't necessarily return an error
        // that halts the main installation flow if the installation itself succeeded.
        // However, for strict error handling, you might want to return an error here.
        // For this example, we will return an error to signal the unmount issue.
        return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to unmount DMG: {:?}", detach_status)));
    }
    log_debug!("[macOS Installer] DMG unmounted successfully.");
    Ok(())
}

// Provide a dummy implementation for `install_pkg` on non-macOS systems to avoid compilation errors.
// This ensures the code compiles on all platforms, even if the functionality isn't available.
#[cfg(not(target_os = "macos"))]
pub fn install_pkg(_path: &Path) -> io::Result<()> {
    log_warn!("[Utils] .pkg installation is only supported on macOS. Skipping for this platform.");
    // Return an error indicating this operation is not supported on the current platform.
    Err(io::Error::new(io::ErrorKind::Other, ".pkg installation is only supported on macOS."))
}

// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::Path;
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
use std::io;

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
    log_debug!("[Utils] File downloaded successfully to {:?}", dest.to_string_lossy().green());
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
pub fn detect_file_type(path: &Path) -> String {
    // Log a debug message indicating which file's type is being detected.
    log_debug!("[Utils] Detecting file type for: {:?}", path.to_string_lossy().yellow());

    // Execute the `file --brief <path>` command.
    // - `Command::new("file")`: Creates a new command instance for the `file` utility.
    // - `.arg("--brief")`: Adds the `--brief` argument, which tells `file` to output
    //                      only the file type description, without the filename prefix.
    // - `.arg(path)`: Adds the path of the file to be inspected.
    // - `.output()`: Executes the command and waits for it to complete, capturing its
    //                stdout, stderr, and exit status.
    // - `.expect(...)`: If the command itself fails to execute (e.g., `file` not found
    //                   in PATH), this will panic with the provided message.
    let output = Command::new("file")
        .arg("--brief")
        .arg(path)
        .output()
        .expect("Failed to execute 'file' command. Is it installed and in your PATH?");

    // Convert the command's standard output (which is `Vec<u8>`) into a `String`.
    // `String::from_utf8_lossy()` converts bytes to a string, replacing invalid UTF-8 sequences.
    // `.to_lowercase()` converts the entire string to lowercase for case-insensitive matching.
    let out = String::from_utf8_lossy(&output.stdout).to_lowercase();
    log_debug!("[Utils] 'file' command raw output for {:?}: '{}'", path, out.blue()); // Added for debugging

    // Now, we check the output string for various keywords to determine the file type.
    // The order of `if-else if` statements is important to prioritize more specific matches.
    if out.contains("zip") {
        "zip".into() // If it contains "zip", it's a zip archive.
    } else if out.contains("gzip compressed") && out.contains("tar") {
        "tar.gz".into() // If it's gzipped AND a tar archive, it's a tar.gz.
    } else if out.contains("gzip compressed") {
        "gz".into() // If just gzipped (not necessarily tarred), it's a plain .gz.
    } else if out.contains("bzip2 compressed") && out.contains("tar") {
        "tar.bz2".into() // Bzip2 + tar = tar.bz2.
    } else if out.contains("bzip2") {
        "bz2".into() // Just bzip2 compressed data.
    } else if out.contains("xar archive") && out.contains(".pkg") {
        "pkg".into() // Specifically for macOS installer packages (XAR archives with .pkg extension).
    } else if out.contains("executable") || out.contains("binary") || out.contains("mach-o") || out.contains("elf") {
        // Broaden detection for executables.
        // - "executable" or "binary" are general indicators.
        // - "mach-o" is a specific executable format for macOS/iOS.
        // - "elf" is a specific executable format for Linux/Unix.
        "binary".into() // It's a standalone executable file.
    } else if out.contains("tar archive") {
        "tar".into() // A plain tar archive (uncompressed).
    } else {
        // If none of the above keywords are found, we don't know the type.
        log_warn!("[Utils] Unrecognized file type for {:?}: '{}'. Treating as unknown.", path, out.purple());
        "unknown".into() // Return "unknown".
    }
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

// Provide a dummy implementation for `install_pkg` on non-macOS systems to avoid compilation errors.
// This ensures the code compiles on all platforms, even if the functionality isn't available.
#[cfg(not(target_os = "macos"))]
pub fn install_pkg(_path: &Path) -> io::Result<()> {
    log_warn!("[Utils] .pkg installation is only supported on macOS. Skipping for this platform.");
    // Return an error indicating this operation is not supported on the current platform.
    Err(io::Error::new(io::ErrorKind::Other, ".pkg installation is only supported on macOS."))
}

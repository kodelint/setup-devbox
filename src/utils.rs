// src/utils.rs
// This file is a collection of handy helper functions that our 'devbox' application
// uses throughout its various commands. Think of it as a toolbox with general-purpose
// utilities for things like path manipulation, file operations, downloading, and system detection.

// Bring in our custom logging macros for debug, error, info, and warning messages.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to our terminal output (makes logs easier to read!).
use colored::Colorize;
// For decompressing gzipped archives (like .tar.gz files).
use flate2::read::GzDecoder;
// To get environment variables, like the temporary directory or home directory.
use std::env;
// For file system operations: creating directories, reading files, etc.
use std::fs;
// For creating and interacting with files.
use std::fs::File;
// Core I/O functionalities and error handling.
use std::io::{self};
// Types for working with file paths in a robust way.
use std::path::{Path, PathBuf};
// To run external commands (like 'file' or 'sudo installer').
use std::process::Command;
// For extracting tar archives.
use tar::Archive;
// For extracting zip archives.
use zip::ZipArchive;

// This line is conditional: it's only compiled when targeting Unix-like systems (macOS, Linux).
// It's used to set file permissions, specifically making files executable.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// A super useful function to resolve paths that start with a tilde `~`.
/// On Unix-like systems, `~` is a shortcut for the user's home directory.
/// This function expands that `~` into the full, absolute path, like `/Users/yourusername/`.
///
/// # Arguments
/// * `path`: A string slice representing the path, which might start with `~`.
///
/// # Returns
/// * `PathBuf`: The fully resolved path, or the original path if `~` wasn't present or home directory couldn't be found.
pub fn expand_tilde(path: &str) -> PathBuf {
    // Check if the path actually starts with a tilde.
    if path.starts_with("~") {
        // Try to get the user's home directory. `dirs::home_dir()` is from an external crate
        // that reliably finds the home directory across different OSes.
        if let Some(home) = dirs::home_dir() {
            // If we found the home directory, replace the first occurrence of '~' with the home path.
            return PathBuf::from(path.replacen("~", &home.to_string_lossy(), 1));
        }
    }
    // If no tilde, or home directory wasn't found, just return the path as is.
    PathBuf::from(path)
}

/// Downloads a file from a given URL and saves it to a specified destination on the local file system.
/// This is crucial for fetching tools and resources from the internet (e.g., GitHub releases).
///
/// # Arguments
/// * `url`: The URL of the file to download.
/// * `dest`: The local file system path where the downloaded file should be saved.
///
/// # Returns
/// * `io::Result<()>`: `Ok(())` if the download was successful, or an `io::Error` if anything went wrong.
pub fn download_file(url: &str, dest: &Path) -> io::Result<()> {
    log_debug!("[Utils] Starting download from URL: {}", url.blue());

    // Execute the HTTP GET request using the `ureq` library.
    let response = match ureq::get(url).call() {
        Ok(res) => res, // If the request was successful, we get a response.
        Err(e) => {
            // If the HTTP request itself failed (e.g., network error, invalid URL).
            log_error!("[Utils] HTTP request failed for {}: {}", url.red(), e);
            // Convert the ureq error into a standard `io::Error` for consistent error handling.
            return Err(io::Error::new(io::ErrorKind::Other, format!("HTTP error: {}", e)));
        }
    };

    // Open the destination file for writing. This will create the file if it doesn't exist, or truncate it if it does.
    let mut file = File::create(dest)?; // The `?` here handles potential file creation errors.

    // Get a reader for the response body (the actual data being downloaded).
    let mut reader = response.into_reader();
    // Copy all data from the network reader directly into our local file.
    std::io::copy(&mut reader, &mut file)?; // The `?` here handles potential read/write errors.

    log_debug!("[Utils] File downloaded successfully to {:?}", dest.to_string_lossy().green());
    Ok(()) // Indicate success!
}

/// Attempts to detect the type of a given file by executing the native `file` command (on Unix-like systems).
/// This is super useful for figuring out if a downloaded file is a zip, tar.gz, a raw binary, etc.,
/// which then guides how we should extract or handle it.
///
/// # Arguments
/// * `path`: The path to the file whose type we want to detect.
///
/// # Returns
/// * `String`: A normalized string representing the detected file type (e.g., "zip", "tar.gz", "binary", "unknown").
pub fn detect_file_type(path: &Path) -> String {
    log_debug!("[Utils] Detecting file type for: {:?}", path.to_string_lossy().yellow());

    // Execute the `file --brief <path>` command.
    // `--brief` makes the output concise, just the file type description.
    let output = Command::new("file")
        .arg("--brief")
        .arg(path)
        .output()
        .expect("Failed to execute 'file' command. Is it installed and in your PATH?");

    // Convert the command's output (raw bytes) into a lowercase string for easier matching.
    let out = String::from_utf8_lossy(&output.stdout).to_lowercase();

    // Now, we check the output string for various keywords to determine the file type.
    if out.contains("zip") {
        "zip".into() // If it contains "zip", it's a zip archive.
    } else if out.contains("gzip compressed") && out.contains("tar") {
        "tar.gz".into() // If it's gzipped AND a tar archive, it's a tar.gz.
    } else if out.contains("gzip compressed") {
        "gz".into() // If just gzipped (not necessarily tarred), it's a plain .gz.
    } else if out.contains("bzip2 compressed") && out.contains("tar") {
        "tar.bz2".into() // Bzip2 + tar = tar.bz2.
    } else if out.contains("bzip2") {
        "bz2".into() // Just bzip2.
    } else if out.contains("xar archive") && out.contains(".pkg") {
        "pkg".into() // Specifically for macOS installer packages.
    } else if out.contains("executable") || out.contains("binary") {
        "binary".into() // It's a standalone executable file.
    } else if out.contains("tar archive") {
        "tar".into() // A plain tar archive (uncompressed).
    } else {
        log_warn!("[Utils] Unrecognized file type for {:?}: '{}'. Treating as unknown.", path, out.purple());
        "unknown".into() // If none of the above, we don't know what it is.
    }
}

/// Extracts the contents of a compressed archive (zip, tar.gz, etc.) into a new subdirectory
/// within the specified destination path. This is a core utility for unpacking downloaded tools.
///
/// # Arguments
/// * `src`: The path to the compressed archive file.
/// * `dest`: The directory where the *extracted* content should be placed (inside a new "extracted" subdir).
///
/// # Returns
/// * `io::Result<PathBuf>`: `Ok(PathBuf)` with the path to the newly created "extracted" directory,
///                         or an `io::Error` if extraction fails or the archive type is unsupported.
pub fn extract_archive(src: &Path, dest: &Path) -> io::Result<PathBuf> {
    log_debug!("[Utils] Extracting archive {:?} into {:?}", src.to_string_lossy().blue(), dest.to_string_lossy().cyan());

    // First, determine the type of the archive using our helper function.
    let file_type = detect_file_type(src);
    // Create a specific subdirectory named "extracted" inside the `dest` directory.
    // All contents will be unpacked here to keep things tidy.
    let extracted_path = dest.join("extracted");
    // Ensure this directory (and any parent directories) exist before unpacking.
    fs::create_dir_all(&extracted_path)?; // `?` propagates any directory creation errors.

    // Now, we use a `match` statement to call the correct extraction logic based on the `file_type`.
    match file_type.as_str() {
        "zip" => {
            let file = File::open(src)?; // Open the zip file.
            let mut archive = ZipArchive::new(file)?; // Create a zip archive reader.
            archive.extract(&extracted_path)?; // Extract all contents to the specified path.
            log_debug!("[Utils] Zip archive extracted successfully.");
        }
        "tar.gz" | "gz" => {
            let tar_gz = File::open(src)?; // Open the gzipped tar file.
            let decompressor = GzDecoder::new(tar_gz); // Create a Gzip decompressor.
            let mut archive = Archive::new(decompressor); // Create a tar archive reader from the decompressed stream.
            archive.unpack(&extracted_path)?; // Unpack the tar archive.
            log_debug!("[Utils] Tar.gz archive extracted successfully.");
        }
        "tar.bz2" => {
            let tar_bz2 = File::open(src)?; // Open the bzip2 compressed tar file.
            // Note: `bzip2::read::BzDecoder` requires the `bzip2` feature in your Cargo.toml for `flate2` or a separate `bzip2` crate.
            // Assuming `bzip2` is available and used correctly.
            let decompressor = bzip2::read::BzDecoder::new(tar_bz2); // Create a Bzip2 decompressor.
            let mut archive = Archive::new(decompressor); // Create a tar archive reader.
            archive.unpack(&extracted_path)?; // Unpack the tar archive.
            log_debug!("[Utils] Tar.bz2 archive extracted successfully.");
        }
        "tar" => {
            let tar = File::open(src)?; // Open the uncompressed tar file.
            let mut archive = Archive::new(tar); // Create a tar archive reader directly.
            archive.unpack(&extracted_path)?; // Unpack the tar archive.
            log_debug!("[Utils] Tar archive extracted successfully.");
        }
        _ => {
            // If the file type is not supported for extraction, return an error.
            log_error!("[Utils] Unsupported archive type '{}' for extraction: {:?}", file_type.red(), src);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported archive type: {}", file_type),
            ));
        }
    }

    log_debug!("[Utils] ✨ Archive contents available at: {:?}", extracted_path.to_string_lossy().green());
    Ok(extracted_path) // Return the path to the directory where contents were extracted.
}

/// Recursively searches a given directory for the first executable file it finds.
/// This is essential after extracting an archive, as the actual binary we need
/// might be nested deep within subdirectories.
///
/// # Arguments
/// * `dir`: The directory to start searching from.
///
/// # Returns
/// * `Option<PathBuf>`: `Some(PathBuf)` if an executable is found, `None` otherwise.
pub fn find_executable(dir: &Path) -> Option<PathBuf> {
    log_debug!("[Utils] 🔎 Searching for an executable in: {:?}", dir.to_string_lossy().yellow());

    // Read the contents of the directory. `ok()?` handles potential errors and turns them into `None`.
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?; // Get the directory entry, handling errors.
        let path = entry.path(); // Get the full path of the entry.

        if path.is_dir() {
            // If the entry is a directory, recursively call `find_executable` on it.
            if let Some(found) = find_executable(&path) {
                return Some(found); // If an executable is found in a subdirectory, return it immediately.
            }
        } else {
            // If the entry is a file, check if it's executable.
            let meta = fs::metadata(&path).ok()?; // Get file metadata (permissions).
            // On Unix-like systems, check if the executable bit (0o111) is set in the file permissions.
            // `PermissionsExt` is available via `#[cfg(unix)]`.
            #[cfg(unix)]
            if meta.permissions().mode() & 0o111 != 0 {
                log_debug!("[Utils] Found executable: {:?}", path.to_string_lossy().green());
                return Some(path); // Found an executable! Return its path.
            }
            // On Windows, there isn't a direct "executable bit" in the same way.
            // For Windows, you might check for `.exe` extension or just assume if it's a file
            // in an expected binary location. For simplicity, this example primarily focuses on Unix.
            // You might add `#[cfg(windows)]` logic here.
        }
    }

    log_warn!("[Utils] No executable found within {:?}", dir.to_string_lossy().purple());
    None // No executable found after checking all entries.
}

/// Installs a macOS `.pkg` installer file using `sudo installer`.
/// This is typically used for system-wide software installations on macOS.
///
/// # Arguments
/// * `path`: The path to the `.pkg` file.
///
/// # Returns
/// * `io::Result<()>`: `Ok(())` if installation was successful, or an `io::Error` if it failed.
pub fn install_pkg(path: &Path) -> io::Result<()> {
    log_info!("[Utils] Installing .pkg file: {:?}", path.to_string_lossy().bold());

    // Execute the `sudo installer -pkg <path> -target /` command.
    // `sudo` requires user password. `installer` is the macOS package installer tool.
    // `-pkg <path>` specifies the package file. `-target /` specifies the root as the installation target.
    let status = Command::new("sudo")
        .arg("installer")
        .arg("-pkg")
        .arg(path)
        .arg("-target")
        .arg("/")
        .status()?; // `?` propagates any command execution errors.

    // Check if the command exited successfully.
    if !status.success() {
        log_error!("[Utils] .pkg installer failed with status: {:?}", status);
        return Err(io::Error::new(io::ErrorKind::Other, "Installer failed"));
    }

    log_info!("[Utils] .pkg file installed successfully!");
    Ok(())
}

/// Moves a file (typically a binary) from a source path to a destination path,
/// and can also rename it in the process. It ensures that the destination's parent
/// directories exist before attempting the move.
///
/// # Arguments
/// * `from`: The source path of the file.
/// * `to`: The destination path, including the new filename if renaming.
///
/// # Returns
/// * `io::Result<()>`: `Ok(())` on success, or `io::Error` if move/rename fails.
pub fn move_and_rename_binary(from: &Path, to: &Path) -> io::Result<()> {
    log_debug!("[Utils] Moving binary from {:?} to {:?}", from.to_string_lossy().yellow(), to.to_string_lossy().cyan());

    // If the destination path has parent directories (e.g., `/usr/local/bin/`),
    // ensure those directories exist before trying to move the file into them.
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?; // Create all necessary parent directories.
    }

    // Perform the copy operation. Note: `fs::copy` is used here,
    // which copies the file. For a true 'move' (renaming within the same filesystem
    // or copying then deleting), `fs::rename` or `fs::copy` followed by `fs::remove_file`
    // would be more explicit. This current implementation copies.
    fs::copy(from, to)?;
    // A potential improvement here for a true move would be:
    // fs::rename(from, to)?; // This attempts a rename first (fast if same filesystem)
    // if it fails due to different filesystems, then fall back to copy+delete.
    // For simplicity, `fs::copy` is used which implies a copy.
    log_info!("[Utils] Binary moved/copied to {:?}", to.to_string_lossy().green());
    Ok(())
}

/// Makes a given file executable. On Unix-like systems, this is equivalent to `chmod +x file`.
/// This is crucial for downloaded binaries to be runnable.
///
/// # Arguments
/// * `path`: The path to the file to make executable.
///
/// # Returns
/// * `io::Result<()>`: `Ok(())` on success, or `io::Error` if permissions cannot be set.
pub fn make_executable(path: &Path) -> io::Result<()> {
    log_debug!("[Utils] Making {:?} executable", path.to_string_lossy().yellow());

    // Get the current permissions of the file.
    let mut perms = fs::metadata(path)?.permissions();

    // This block only compiles on Unix-like operating systems.
    #[cfg(unix)]
    {
        // Set the file's permissions to 0o755 (rwxr-xr-x). This grants read, write,
        // and execute permissions to the owner, and read/execute to group and others.
        perms.set_mode(0o755);
    }
    // For Windows, executable permissions are often implicit for `.exe` files,
    // or managed differently, so no specific `set_mode` might be needed here.
    // If you need Windows-specific executable setting, you'd add `#[cfg(windows)]` logic.

    // Apply the modified permissions back to the file.
    fs::set_permissions(path, perms)?;
    log_debug!("[Utils] File {:?} is now executable.", path.to_string_lossy().green());
    Ok(())
}

/// Returns a path to a dedicated temporary directory for 'devbox' operations.
/// This ensures that temporary files are kept separate and can be easily cleaned up.
///
/// # Returns
/// * `PathBuf`: The path to the 'devbox' specific temporary directory.
pub fn get_temp_dir() -> PathBuf {
    // Start with the system's standard temporary directory.
    let path = env::temp_dir().join("setup-devbox");
    // Attempt to create this directory (and any necessary parent directories).
    // We use `let _ =` to ignore the `Result` here because it's okay if it already exists.
    let _ = fs::create_dir_all(&path);
    // Return the full path to our specific temporary directory.
    path
}

/// Detects the current machine's CPU architecture (e.g., "arm64", "x86_64").
/// This is vital for downloading the correct version of a binary from GitHub releases.
///
/// # Returns
/// * `Option<String>`: The detected architecture as a canonical string, or `None` if detection fails.
pub fn detect_architecture() -> Option<String> {
    // `std::env::consts::ARCH` gives us the architecture Rust was compiled for (e.g., "aarch64", "x86_64").
    // We then normalize it to a more common form.
    Some(normalize_arch(std::env::consts::ARCH).to_string())
}

/// Detects the current operating system (e.g., "macos", "linux", "windows").
/// Similar to architecture detection, this is crucial for finding the right software release.
///
/// # Returns
/// * `Option<String>`: The detected OS as a canonical string, or `None` if detection fails.
pub fn detect_os() -> Option<String> {
    // `std::env::consts::OS` gives us the OS Rust was compiled for (e.g., "macos", "linux").
    // We then normalize it.
    Some(normalize_os(std::env::consts::OS).to_string())
}

/// Generates a canonical filename for an asset based on a tool name, OS, and architecture.
/// This helps in constructing expected download file names or matching against GitHub asset names.
///
/// # Arguments
/// * `tool`: The name of the tool (e.g., "gh").
/// * `os`: The operating system (e.g., "macos").
/// * `arch`: The architecture (e.g., "arm64").
///
/// # Returns
/// * `String`: The generated filename string (e.g., "gh-macos-arm64").
pub fn generate_asset_filename(tool: &str, os: &str, arch: &str) -> String {
    // First, normalize the OS and architecture strings to ensure consistency.
    let os = normalize_os(os);
    let arch = normalize_arch(arch);
    // Format them into a standard `tool-os-arch` string.
    let fname = format!("{}-{}-{}", tool, os, arch);
    log_debug!("[Utils] Generated asset filename: {}", fname.green());
    fname
}

/// Normalizes various input strings for operating systems into a consistent, lowercase format.
/// This helps 'devbox' deal with different ways OS names might appear in asset names or system info.
///
/// # Arguments
/// * `os`: An input string representing an OS (e.g., "MacOS", "darwin", "Linux").
///
/// # Returns
/// * `String`: The normalized OS string (e.g., "macos", "linux", "windows").
pub fn normalize_os(os: &str) -> String {
    match os.to_lowercase().as_str() {
        "macos" | "darwin" | "apple-darwin" => "macos".to_string(), // macOS variants map to "macos"
        "linux" => "linux".to_string(), // Linux is straightforward
        "windows" | "win32" | "win64" => "windows".to_string(), // Windows variants map to "windows"
        other => {
            // If we encounter an unknown OS, log a warning and use it as-is.
            log_warn!("[Utils] Unknown OS variant '{}', using as-is. This might cause issues with asset matching.", other.purple());
            other.to_string()
        }
    }
}

/// Normalizes various input strings for CPU architectures into a consistent, lowercase format.
/// This ensures 'devbox' can correctly match architectures (e.g., "aarch64" vs "arm64").
///
/// # Arguments
/// * `arch`: An input string representing an architecture (e.g., "AARCH64", "x86_64", "amd64").
///
/// # Returns
/// * `String`: The normalized architecture string (e.g., "arm64", "x86_64").
pub fn normalize_arch(arch: &str) -> String {
    match arch.to_lowercase().as_str() {
        "aarch64" | "arm64" => "arm64".to_string(), // ARM 64-bit variants map to "arm64"
        "amd64" | "x86_64" => "x86_64".to_string(), // AMD 64-bit variants map to "x86_64"
        other => {
            // If unknown, log a warning and use it as-is.
            log_warn!("[Utils] Unknown ARCH variant '{}', using as-is. This might cause issues with asset matching.", other.purple());
            other.to_string()
        }
    }
}

// These functions (`os_aliases` and `arch_aliases`) are used internally by `asset_matches_platform`.
// They provide a list of common alternative names for a given OS or architecture,
// helping us correctly identify relevant files from GitHub releases, even if they use
// slightly different naming conventions.

fn os_aliases(os: &str) -> Vec<String> {
    match os.to_lowercase().as_str() {
        "macos" => vec!["macos", "darwin", "apple-darwin", "macosx"] // Removed duplicate "macos"
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        "linux" => vec!["linux"].into_iter().map(|s| s.to_string()).collect(),
        "windows" => vec!["windows", "win32", "win64"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        other => vec![other.to_string()], // If it's something else, just use it as is.
    }
}

fn arch_aliases(arch: &str) -> Vec<String> {
    match arch.to_lowercase().as_str() {
        "arm64" => vec!["arm64", "aarch64"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        "x86_64" => vec!["x86_64", "amd64"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        other => vec![other.to_string()], // If it's something else, just use it as is.
    }
}

/// Checks if a given asset filename from a GitHub release (or similar source)
/// is likely compatible with the current operating system and architecture.
/// This is how 'devbox' smartly picks the correct download for your machine.
///
/// # Arguments
/// * `filename`: The full filename of the asset (e.g., "mytool_1.0.0_macOS_arm64.tar.gz").
/// * `os`: The current operating system (e.g., "macos").
/// * `arch`: The current architecture (e.g., "arm64").
///
/// # Returns
/// * `bool`: `true` if the filename contains recognizable OS and architecture keywords for the platform, `false` otherwise.
pub(crate) fn asset_matches_platform(filename: &str, os: &str, arch: &str) -> bool {
    // Convert the filename to lowercase for case-insensitive matching.
    let fname = filename.to_lowercase();

    // Get all known aliases for the current OS and architecture.
    let os_aliases = os_aliases(os);
    let arch_aliases = arch_aliases(arch);

    // Special case for macOS: Sometimes macOS binaries might be universal or packaged differently,
    // so we specifically include all common macOS architectures in its alias check.
    // This provides more flexibility when matching macOS-specific assets.
    let macos_arch_aliases = vec!["arm64", "aarch64", "x86_64", "amd64"];

    // Check if the filename contains any of the current OS's aliases.
    let os_match = os_aliases.iter().any(|o| fname.contains(o));
    // Check if the filename contains any of the current architecture's aliases.
    // If it's macOS, we use the broader `macos_arch_aliases` for the architecture check.
    let arch_match = if os == "macos" {
        macos_arch_aliases.iter().any(|a| fname.contains(a))
    } else {
        arch_aliases.iter().any(|a| fname.contains(a))
    };

    // An asset matches our platform only if *both* the OS and architecture components match.
    // This helps ensure we download the truly correct binary.
    let matches = os_match && arch_match;
    log_debug!("[Utils] Asset '{}' matches platform (OS: {}, ARCH: {}) -> {}", filename.dimmed(), os.cyan(), arch.magenta(), matches.to_string().bold());
    matches
}
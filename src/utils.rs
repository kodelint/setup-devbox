// This file is a collection of handy helper functions that our `setup-devbox` application
// uses throughout its various commands. Think of it as a toolbox with general-purpose
// utilities for things like path manipulation, file operations, downloading, and system detection.
// We bring in all the essential data structures (schemas) we defined earlier.
// These structs tell us how our configuration files and our internal state file should be shaped.
use crate::schema::DevBoxState;
// Bring in our custom logging macros for debug, error, info, and warning messages.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to our terminal output (makes logs easier to read!).
use colored::Colorize;
// For decompressing gzipped archives (like .tar.gz files).
use flate2::read::GzDecoder;
// To get environment variables, like the temporary directory or home directory.
use std::{env, io};
// For file system operations: creating directories, reading files, etc.
use std::fs;
// For creating and interacting with files.
use std::fs::{File, OpenOptions};
// Core I/O functionalities and error handling.
use std::io::{BufRead, BufReader, Write};
// Types for working with file paths in a robust way.
use std::path::{Path, PathBuf};
// To run external commands (like 'file' or 'sudo installer').
use std::process::Command;
// For extracting tar archives.
use tar::Archive;
// For extracting zip archives.
use zip::ZipArchive;
// For making HTTP requests (downloading files).
use ureq; // Added missing import for ureq

// This line is conditional: it's only compiled when targeting Unix-like systems (macOS, Linux).
// It's used to set file permissions, specifically making files executable.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// For bzip2 decompression. This import is conditional on the "bzip2" feature.
// Ensure your Cargo.toml has `bzip2 = { version = "...", optional = true }` and `features = ["bzip2"]` when used.
#[cfg(feature = "bzip2")]
use bzip2::read::BzDecoder;

// For recursive directory walking in `find_executable`.
// Ensure your Cargo.toml has `walkdir = "..."`.
use walkdir;

// For saving and loading DevBox state (JSON serialization).
// Ensure your Cargo.toml has `serde = { version = "...", features = ["derive"] }` and `serde_json = "..."`.
use serde::{Deserialize, Serialize};
use serde_json;

/// Represents a single installed tool within the DevBox state.
/// This includes its name and the path where it was installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledTool {
    pub name: String,
    pub path: PathBuf,
    // Add more details about the tool here if necessary, e.g., version, source_url, etc.
}


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

/// Attempts to detect the type of given file by executing the native `file` command (on Unix-like systems).
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
    log_debug!("[Utils] 'file' command raw output for {:?}: '{}'", path, out.blue()); // Added for debugging

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
    } else if out.contains("executable") || out.contains("binary") || out.contains("mach-o") || out.contains("elf") {
        // Broaden detection for executables (added mach-o and elf for better Unix detection)
        "binary".into() // It's a standalone executable file.
    } else if out.contains("tar archive") {
        "tar".into() // A plain tar archive (uncompressed).
    } else {
        log_warn!("[Utils] Unrecognized file type for {:?}: '{}'. Treating as unknown.", path, out.purple());
        "unknown".into() // If none of the above, we don't know what it is.
    }
}

/// Detects file type based purely on its filename extension(s) or common naming patterns.
/// This is useful when the exact file type is known from its source (e.g., GitHub asset name)
/// and a full `file` command inspection might be overkill or less precise for specific cases.
pub fn detect_file_type_from_filename(filename: &str) -> String {
    let filename_lower = filename.to_lowercase();
    log_debug!("[Utils] Detecting file type from filename: {}", filename_lower.yellow());

    // Prioritize more specific/longer extensions first
    if filename_lower.ends_with(".tar.gz") || filename_lower.ends_with(".tgz") {
        "tar.gz".to_string()
    } else if filename_lower.ends_with(".tar.bz2") || filename_lower.ends_with(".tbz") {
        "tar.bz2".to_string()
    } else if filename_lower.ends_with(".zip") {
        "zip".to_string()
    } else if filename_lower.ends_with(".tar") {
        "tar".to_string()
    } else if filename_lower.ends_with(".gz") { // .gz should come after .tar.gz
        "gz".to_string()
    } else if filename_lower.ends_with(".deb") {
        "deb".to_string()
    } else if filename_lower.ends_with(".rpm") {
        "rpm".to_string()
    } else if filename_lower.ends_with(".dmg") {
        "dmg".to_string()
    }
    // Added logic for direct binaries without explicit extensions, but containing platform info
    else if filename_lower.contains("macos") || filename_lower.contains("linux") || filename_lower.contains("windows") ||
        filename_lower.contains("darwin") || filename_lower.contains("amd64") || filename_lower.contains("x86_64") ||
        filename_lower.contains("arm64") || filename_lower.contains("aarch64") ||
        // Consider common binary naming conventions without explicit OS/Arch if it's not an archive
        (!filename_lower.contains('.') && (filename_lower.contains("bin") || filename_lower.contains("cli"))) {
        log_debug!("[Utils] Filename '{}' suggests a direct binary (no standard archive extension, but contains platform/binary hints).", filename_lower.cyan());
        "binary".to_string()
    }
    else {
        log_warn!("[Utils] Unrecognized file type based on filename for '{}'. Returning 'unknown'.", filename_lower.purple());
        "unknown".to_string()
    }
}


/// Extracts the contents of a compressed archive (zip, tar.gz, etc.) into a new subdirectory
/// within the specified destination path. This is a core utility for unpacking downloaded tools.
///
/// # Arguments
/// * `src`: The path to the compressed archive file.
/// * `dest`: The directory where the *extracted* content should be placed (inside a new "extracted" sub-dir).
/// * `known_file_type`: An optional string slice that, if provided, tells the function
///   the exact type of the archive, bypassing internal detection. Useful when the
///   caller already knows the type (e.g., from a GitHub asset name).
///
/// # Returns
/// * `io::Result<PathBuf>`: `Ok(PathBuf)` with the path to the newly created "extracted" directory,
///                         or an `io::Error` if extraction fails or the archive type is unsupported.
pub fn extract_archive(src: &Path, dest: &Path, known_file_type: Option<&str>) -> io::Result<PathBuf> {
    log_debug!("[Utils] Extracting archive {:?} into {:?}", src.to_string_lossy().blue(), dest.to_string_lossy().cyan());

    // Use the known_file_type if provided, otherwise detect.
    let file_type = if let Some(ft) = known_file_type {
        log_debug!("[Utils] Using known file type from argument: {}", ft.green());
        ft.to_string()
    } else {
        // Fallback to auto-detection using 'file' command
        log_debug!("[Utils] No known file type provided. Auto-detecting using 'file' command...");
        detect_file_type(src)
    };

    // Create a specific subdirectory named "extracted" inside the `dest` directory.
    let extracted_path = dest.join("extracted");
    fs::create_dir_all(&extracted_path)?;

    match file_type.as_str() {
        "zip" => {
            let file = File::open(src)?;
            let mut archive = ZipArchive::new(file)?;
            archive.extract(&extracted_path)?;
            log_debug!("[Utils] Zip archive extracted successfully.");
        }
        "tar.gz" => { // Be specific about tar.gz here
            let tar_gz = File::open(src)?;
            let decompressor = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(decompressor);
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar.gz archive extracted successfully.");
        }
        "gz" => { // Handle pure .gz files (not tarred)
            log_info!("[Utils] Decompressing plain GZ file. Contents will be the original file without tar extraction.");
            let gz_file = File::open(src)?;
            let mut decompressor = GzDecoder::new(gz_file);
            let output_file_path = extracted_path.join(src.file_stem().unwrap_or_default()); // Remove .gz extension
            let mut output_file = File::create(&output_file_path)?;
            io::copy(&mut decompressor, &mut output_file)?;
            log_debug!("[Utils] GZ file decompressed successfully to {:?}", output_file_path.display());
        }
        #[cfg(feature = "bzip2")] // Conditional compilation for bzip2
        "tar.bz2" => {
            let tar_bz2 = File::open(src)?;
            let decompressor = BzDecoder::new(tar_bz2); // Use BzDecoder
            let mut archive = Archive::new(decompressor);
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar.bz2 archive extracted successfully.");
        }
        "tar" => {
            let tar = File::open(src)?;
            let mut archive = Archive::new(tar);
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar archive extracted successfully.");
        }
        "binary" => { // For standalone binaries like a .exe or uncompressed Mac binary
            log_info!("[Utils] Copying detected 'binary' directly to extraction path.");
            let file_name = src.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Source path has no filename")
            })?;
            fs::copy(src, extracted_path.join(file_name))?;
            log_debug!("[Utils] Binary copied successfully to {:?}", extracted_path.join(file_name).display());
        }
        "pkg" => { // For macOS .pkg files, copy them as they are installers, not archives to unpack
            log_info!("[Utils] Detected .pkg installer. Copying directly to extraction path for installation.");
            let file_name = src.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Source path has no filename")
            })?;
            fs::copy(src, extracted_path.join(file_name))?;
            log_debug!("[Utils] .pkg file copied successfully to {:?}", extracted_path.join(file_name).display());
        }
        _ => {
            log_error!("[Utils] Unsupported archive type '{}' for extraction: {:?}", file_type.red(), src);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported archive type: {}", file_type),
            ));
        }
    }

    log_debug!("[Utils] ✨ Archive contents available at: {:?}", extracted_path.to_string_lossy().green());
    Ok(extracted_path)
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

    // Common executable names or keywords to look for (added to improve detection).
    let common_executables = [
        "bin", "cli", "app", "main", "daemon", // Generic names
        "go", "node", "python", "ruby", // Common runtime names if the binary is a wrapper
    ];

    // Use `walkdir` to recursively iterate through the directory.
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok()) // Filter out any errors during directory traversal
    {
        let path = entry.path();
        let file_name_str = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();

        if path.is_file() {
            // Check for common executable extensions first (especially for Windows)
            if file_name_str.ends_with(".exe") || file_name_str.ends_with(".sh") || file_name_str.ends_with(".bat") {
                log_debug!("[Utils] Found potential executable (by extension): {:?}", path.display());
                return Some(path.to_path_buf());
            }

            // Check if the filename itself contains common executable indicators
            if common_executables.iter().any(|&name| file_name_str.contains(name)) ||
                // If there's no extension, consider it a potential binary on Unix-like systems
                (cfg!(unix) && path.extension().is_none() && file_name_str.len() > 0) {
                log_debug!("[Utils] Found potential executable (by name heuristic): {:?}", path.display());
                // On Unix, perform an additional check for execute permission
                #[cfg(unix)]
                {
                    if let Ok(metadata) = fs::metadata(path) {
                        if (metadata.permissions().mode() & 0o111) != 0 { // Check if any execute bit is set (user, group, or other)
                            log_debug!("[Utils] Found executable (name and permissions): {:?}", path.display());
                            return Some(path.to_path_buf());
                        } else {
                            log_debug!("[Utils] Skipping {:?}: Not executable by permissions.", path.display());
                        }
                    }
                }
                #[cfg(not(unix))] // On non-Unix (e.g., Windows), if it matches name/no extension, assume it's executable
                {
                    log_debug!("[Utils] Found potential executable (by name/no extension, non-Unix): {:?}", path.display());
                    return Some(path.to_path_buf());
                }
            } else {
                log_debug!("[Utils] Skipping non-executable candidate: {:?} (no common name match or executable permissions)", path.display());
            }
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
#[cfg(target_os = "macos")] // This function only compiles on macOS
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

// Provide a dummy implementation for `install_pkg` on non-macOS systems to avoid compilation errors.
#[cfg(not(target_os = "macos"))]
pub fn install_pkg(_path: &Path) -> io::Result<()> {
    log_warn!("[Utils] .pkg installation is only supported on macOS. Skipping for this platform.");
    // Return an error indicating this operation is not supported
    Err(io::Error::new(io::ErrorKind::Other, ".pkg installation is only supported on macOS."))
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

    // Perform the move operation. `fs::rename` is generally preferred for performance
    // and atomicity if the source and destination are on the same filesystem.
    // If they are on different filesystems, `rename` will fail, and you'd typically
    // fall back to copy + remove.
    match fs::rename(from, to) {
        Ok(_) => {
            log_debug!("[Utils] Binary moved/renamed to {}", to.to_string_lossy().green());
            Ok(())
        },
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            // Fallback for different filesystems: copy and then remove original
            log_warn!("[Utils] Cross-device link detected, falling back to copy and remove for {:?} to {:?}: {}", from.display(), to.display(), e);
            fs::copy(from, to)?;
            fs::remove_file(from)?;
            log_info!("[Utils] Binary copied and old removed successfully to {:?}", to.to_string_lossy().green());
            Ok(())
        },
        Err(e) => {
            log_error!("[Utils] Failed to move binary from {:?} to {:?}: {}", from.display(), to.display(), e);
            Err(e)
        }
    }
}

/// Makes a given file executable. On Unix-like systems, this is equivalent to `chmod +x file`.
/// This is crucial for downloaded binaries to be runnable.
///
/// # Arguments
/// * `path`: The path to the file to make executable.
///
/// # Returns
/// * `io::Result<()>`: `Ok(())` on success, or `io::Error` if permissions cannot be set.
#[cfg(unix)] // This function only compiles on Unix-like systems
pub fn make_executable(path: &Path) -> io::Result<()> {
    log_debug!("[Utils] Making {:?} executable", path.to_string_lossy().yellow());

    // Get the current permissions of the file.
    let mut perms = fs::metadata(path)?.permissions();

    // Set the file's permissions to 0o755 (rwxr-xr-x). This grants read, write,
    // and execute permissions to the owner, and read/execute to group and others.
    perms.set_mode(0o755);

    // Apply the modified permissions back to the file.
    fs::set_permissions(path, perms)?;
    log_debug!("[Utils] File {:?} is now executable.", path.to_string_lossy().green());
    Ok(())
}

// Provide a dummy implementation for `make_executable` on non-Unix systems to avoid compilation errors.
#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> io::Result<()> {
    log_debug!("[Utils] `make_executable` is a no-op on this non-Unix platform (permissions handled differently).");
    Ok(()) // On Windows, executable permissions are often implicit for `.exe` files.
}

/// Returns a path to a dedicated temporary directory for `setup-devbox` operations.
/// This ensures that temporary files are kept separate and can be easily cleaned up.
///
/// # Returns
/// * `PathBuf`: The path to the `setup-devbox` specific temporary directory.
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
/// This helps `setup-devbox` deal with different ways OS names might appear in asset names or system info.
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
/// This ensures `setup-devbox` can correctly match architectures (e.g., "aarch64" vs "arm64").
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
        "macos" => vec!["macos", "darwin", "apple-darwin", "macosx"]
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
/// This is how `setup-devbox` smartly picks the correct download for your machine.
///
/// # Arguments
/// * `filename`: The full filename of the asset (e.g., "mytool_1.0.0_macOS_arm64.tar.gz").
/// * `os`: The current operating system (e.g., "macos").
/// * `arch`: The current architecture (e.g., "arm64").
///
/// # Returns
/// * `bool`: `true` if the filename contains recognizable OS and architecture keywords for the platform, `false` otherwise.
pub fn asset_matches_platform(filename: &str, os: &str, arch: &str) -> bool {
    let asset_name_lower = filename.to_lowercase();
    let os_lower = os.to_lowercase();
    let arch_lower = arch.to_lowercase();

    // Check for OS match first using aliases
    let os_matches = os_aliases(&os_lower)
        .iter()
        .any(|alias| asset_name_lower.contains(alias));

    if !os_matches {
        log_debug!("[Utils] Asset '{}' does not match OS '{}'", filename.dimmed(), os);
        return false;
    }

    // Check for Architecture match using aliases
    let arch_matches = arch_aliases(&arch_lower)
        .iter()
        .any(|alias| asset_name_lower.contains(alias));

    // Special consideration for macOS ARM64 (aarch64):
    // If the target is macOS ARM64 and the asset contains "x86_64" (and not an explicit ARM64/aarch64 variant),
    // it might be runnable via Rosetta 2. This is a common fallback strategy for universal binaries or older releases.
    let rosetta_fallback = (os_lower == "macos" && arch_lower == "arm64") &&
        asset_name_lower.contains("x86_64") &&
        !(asset_name_lower.contains("arm64") || asset_name_lower.contains("aarch64"));

    if !(arch_matches || rosetta_fallback) {
        log_debug!("[Utils] Asset '{}' does not match architecture '{}' (and no Rosetta fallback).", filename.dimmed(), arch);
        return false;
    }

    // Optional: Exclude common source/debug/checksum files if not explicitly desired.
    // This helps in picking the actual binary release.
    if asset_name_lower.contains("src") ||
        asset_name_lower.contains("source") ||
        asset_name_lower.contains("debug") ||
        asset_name_lower.contains("checksum") ||
        asset_name_lower.contains("sha256") ||
        asset_name_lower.contains("tar.gz.sig") ||
        asset_name_lower.ends_with(".asc") { // Common signature file extension
        log_debug!("[Utils] Asset '{}' excluded due to containing common non-binary keywords.", filename.dimmed());
        return false;
    }

    log_debug!("[Utils] Asset '{}' matches platform (OS: {}, ARCH: {}) -> {}", filename.dimmed(), os.cyan(), arch.magenta(), "true".bold());
    true
}

/// Helper function: Determines the appropriate shell RC file path for a given shell name.
///
/// This function currently supports `.zshrc` for Zsh and `.bashrc` for Bash.
/// It constructs the full path by joining the user's home directory with the
/// specific RC file name.
///
/// # Arguments
/// * `shell`: A string slice representing the name of the shell (e.g., "zsh", "bash").
///
/// # Returns
/// * `Option<PathBuf>`:
///   - `Some(PathBuf)` containing the full path to the RC file if the shell is supported
///     and the home directory can be determined.
///   - `None` if the shell is not supported or the home directory cannot be found.
pub fn get_rc_file(shell: &str) -> Option<PathBuf> {
    log_debug!("[ShellRC:get_rc_file] Attempting to find RC file for shell: '{}'", shell.bold());

    // Use `dirs::home_dir()` from the `dirs` crate to reliably get the current user's home directory.
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            log_warn!("[ShellRC:get_rc_file] Could not determine the user's home directory. Cannot find RC file.");
            return None; // Cannot proceed without the home directory.
        }
    };
    log_debug!("[ShellRC:get_rc_file] User's home directory detected: {:?}", home_dir.display());

    // Match the lowercase version of the shell name to determine the correct RC file.
    let rc_file_name = match shell.to_lowercase().as_str() {
        "zsh" => ".zshrc",
        "bash" => ".bashrc",
        _ => {
            // If the shell name doesn't match a supported type, log a warning and return None.
            log_warn!(
                "[ShellRC:get_rc_file] Unsupported shell type '{}'. Currently only 'zsh' and 'bash' are explicitly mapped to RC files.",
                shell.red()
            );
            return None;
        }
    };
    log_debug!("[ShellRC:get_rc_file] RC file name determined: {}", rc_file_name.cyan());

    // Construct the full path to the RC file by joining the home directory and the file name.
    let rc_path = home_dir.join(rc_file_name);
    log_debug!("[ShellRC:get_rc_file] Full RC file path: {:?}", rc_path.display());

    Some(rc_path) // Return the constructed path wrapped in `Some`.
}

/// Helper function: Reads all non-empty, non-comment lines from a given RC file.
///
/// This function is designed to read the existing content of an RC file efficiently
/// for later comparison. It handles cases where the file might not exist or be unreadable.
///
/// # Arguments
/// * `rc_path`: A reference to a `Path` indicating the RC file to read.
///
/// # Returns
/// * `Vec<String>`: A vector containing each line read from the file as a `String`.
///                  Returns an empty vector if the file doesn't exist or an error occurs during reading.
pub fn read_rc_lines(rc_path: &Path) -> Vec<String> {
    log_debug!("[ShellRC:read_rc_lines] Attempting to read lines from RC file: {:?}", rc_path.display().to_string().dimmed());

    // First, check if the file actually exists. If not, there are no lines to read.
    if !rc_path.exists() {
        log_debug!("[ShellRC:read_rc_lines] RC file {:?} does not exist. Returning an empty list of lines.", rc_path.display().to_string().yellow());
        return vec![];
    }

    // Attempt to open the file for reading.
    match fs::File::open(rc_path) {
        Ok(file) => {
            log_debug!("[ShellRC:read_rc_lines] RC file {:?} opened successfully for reading.", rc_path.display());
            // Create a buffered reader for efficient line-by-line reading.
            BufReader::new(file)
                .lines() // Get an iterator over lines.
                .filter_map(Result::ok) // Filter out any lines that resulted in an I/O error.
                // It's good practice to also filter out empty lines or lines that are just comments,
                // as these typically don't represent active configurations to compare against.
                .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .collect() // Collect all valid lines into a `Vec<String>`.
        }
        Err(err) => {
            // If the file cannot be opened (e.g., permission issues), log a warning.
            log_warn!(
                "[ShellRC:read_rc_lines] Could not read RC file {:?}: {}. Returning an empty list of lines.",
                rc_path.display().to_string().red(),
                err.to_string().red()
            );
            vec![] // Return an empty vector on error.
        }
    }
}

/// Helper function: Appends new lines to the end of the specified RC file.
///
/// This function opens the RC file in append mode. If the file doesn't exist,
/// it will be created. It also adds a comment header to denote lines added by `setup-devbox`.
///
/// # Arguments
/// * `rc_path`: A reference to a `Path` indicating the RC file to append to.
/// * `lines`: A `Vec<String>` containing the new lines to be written.
///
/// # Returns
/// * `std::io::Result<()>`: `Ok(())` on successful write, or an `Err` if an I/O error occurs.
pub fn append_to_rc_file(rc_path: &Path, lines: Vec<String>) -> std::io::Result<()> {
    log_debug!("[ShellRC:append_to_rc_file] Preparing to append {} new lines to RC file: {:?}", lines.len().to_string().bold(), rc_path.display().to_string().yellow());

    // Open the file with specific options:
    // - `create(true)`: If the file doesn't exist, create it.
    // - `append(true)`: Open the file in append mode, so new writes go to the end.
    let mut file = OpenOptions::new()
        .create(true) // Create the file if it doesn't exist.
        .append(true) // Open in append mode.
        .open(rc_path)?; // Attempt to open the file. The `?` operator will propagate any errors.

    log_debug!("[ShellRC:append_to_rc_file] RC file {:?} opened in append mode.", rc_path.display());

    // Add a clear comment header before appending new configurations.
    // This makes it easy for users to identify entries added by 'setup-devbox'.
    writeln!(file, "\n# Added by setup-devbox")?;
    log_debug!("[ShellRC:append_to_rc_file] Added 'Added by setup-devbox' header.");

    // Write each new line to the file, followed by a newline character.
    for (index, line) in lines.iter().enumerate() {
        writeln!(file, "{}", line)?;
        log_debug!("[ShellRC:append_to_rc_file] Appended line {}: '{}'", (index + 1).to_string().dimmed(), line.dimmed());
    }

    log_debug!("[ShellRC:append_to_rc_file] All new lines successfully written to {:?}", rc_path.display());
    Ok(()) // Indicate success.
}

/// This function checks if a multi-line string `needle_lines` exists
/// sequentially within a vector of `haystack_lines`.
pub fn contains_multiline_block(haystack_lines: &[String], needle_lines: &[String]) -> bool {
    if needle_lines.is_empty() {
        return true; // An empty block is always "contained"
    }
    if haystack_lines.len() < needle_lines.len() {
        return false; // Haystack is too short to contain the needle
    }

    // Iterate through the haystack, looking for the start of the needle
    for i in 0..=(haystack_lines.len() - needle_lines.len()) {
        let mut match_found = true;
        // Check if the sequence of lines from `haystack_lines[i]` matches `needle_lines`
        for j in 0..needle_lines.len() {
            // Trim both lines for robust comparison, and then check if the haystack line
            // *starts with* the trimmed needle line. Using `starts_with` is better than `==`
            // because there might be slight leading/trailing spaces or comments in the RC file
            // that `read_rc_lines` might not perfectly normalize, but the core content should match.
            // Also, consider `contains` if individual lines might have other text, but `starts_with`
            // is generally more precise for configuration blocks.
            if !haystack_lines[i + j].trim().starts_with(needle_lines[j].trim()) {
                match_found = false;
                break;
            }
        }
        if match_found {
            return true; // Found the entire multi-line block
        }
    }
    false // Block not found
}

/// Saves the current `DevBoxState` to the specified `state.json` file.
///
/// This function serializes the state into a pretty-printed JSON format
/// and writes it to the disk. It also handles creating the parent directories
/// if they do not exist.
///
/// # Arguments
/// * `state`: A reference to the `DevBoxState` struct to be saved.
/// * `state_path`: A reference to a `PathBuf` indicating where the state file should be saved.
///
/// # Returns
/// * `bool`: `true` if the state was saved successfully, `false` otherwise.
pub fn save_devbox_state(state: &DevBoxState, state_path: &PathBuf) -> bool {
    log_debug!("[StateSave] Attempting to save DevBoxState to: {:?}", state_path.display());

    // Ensure the parent directory for the state file exists.
    if let Some(parent_dir) = state_path.parent() {
        if !parent_dir.exists() {
            log_info!("[StateSave] Parent directory {:?} does not exist. Creating it now.", parent_dir.display());
            if let Err(e) = fs::create_dir_all(parent_dir) {
                log_error!(
                    "[StateSave] Failed to create directory for state file at {:?}: {}. Cannot save state.",
                    parent_dir.display().to_string().red(),
                    e
                );
                return false; // Critical failure, cannot save.
            }
        }
    }

    // Try to serialize the `DevBoxState` into a pretty-printed JSON string.
    match serde_json::to_string_pretty(state) {
        Ok(serialized_state) => {
            // Attempt to write the serialized JSON content to the state file.
            match fs::write(state_path, serialized_state) {
                Ok(_) => {
                    log_info!("[StateSave] DevBox state saved successfully to {}", state_path.display().to_string().green());
                    log_debug!("[StateSave] State content written to disk.");
                    true // Success!
                },
                Err(err) => {
                    log_error!(
                        "[StateSave] Failed to write updated state file to {:?}: {}. Your `setup-devbox` memory might not be saved correctly.",
                        state_path.display().to_string().red(),
                        err
                    );
                    false // Failed to write.
                }
            }
        },
        Err(err) => {
            // If serialization itself fails, it's an internal error (e.g., schema mismatch).
            log_error!("[StateSave] Failed to serialize DevBox state for saving: {}. This is an internal application error.", err);
            false // Failed to serialize.
        }
    }
}
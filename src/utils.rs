// This file is a collection of handy helper functions that our `setup-devbox` application
// uses throughout its various commands. Think of it as a toolbox with general-purpose
// utilities for things like path manipulation, file operations, downloading, and system detection.

// Bring in our custom logging macros.
// These macros (`log_debug`, `log_error`, `log_info`, `log_warn`) provide
// a standardized way to output messages to the console with different severity levels,
// making it easier to track the application's flow and diagnose issues.
use crate::{log_debug, log_error, log_info, log_warn};
// For adding color to our terminal output.
// The `colored` crate allows us to make log messages and other terminal output more readable
// by applying colors (e.g., `.blue()`, `.green()`, `.red()`).
use colored::Colorize;
// For decompressing gzipped archives (like .tar.gz files).
// `flate2` is a widely used Rust library for handling various compression formats,
// and `GzDecoder` specifically deals with gzip decompression.
use flate2::read::GzDecoder;
// To get environment variables, like the temporary directory or home directory.
// `std::env` provides functions to interact with the process's environment.
// `std::io` contains core input/output functionalities and error types.
use std::{env, io};
// For file system operations: creating directories, reading files, etc.
// `std::fs` provides functions for interacting with the file system.
use std::fs;
// For creating and interacting with files.
// `std::fs::{File, OpenOptions}` allows for fine-grained control over file opening and creation.
use std::fs::File;
// Types for working with file paths in a robust way.
// `std::path::{Path, PathBuf}` are essential for handling file system paths.
// `Path` is a borrowed, immutable view of a path, while `PathBuf` owns the path data.
use std::path::{Path, PathBuf};
// To run external commands (like 'file' or 'sudo installer').
// `std::process::Command` allows the application to spawn and control external processes.
use std::process::Command;
// For extracting tar archives.
// The `tar` crate provides functionality to read and write tar archives.
use tar::Archive;
// For extracting zip archives.
// The `zip` crate provides functionality to read and write zip archives.
use zip::ZipArchive;
// For making HTTP requests (downloading files).
// `ureq` is a simple and expressive HTTP client for Rust, used here for downloading files from URLs.
use ureq;

// This line is conditional: it's only compiled when targeting Unix-like systems (macOS, Linux).
// It's used to set file permissions, specifically making files executable, which is a Unix-specific concept.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
// For bzip2 decompression.
use bzip2::read::BzDecoder;

// For recursive directory walking in `find_executable`.
// The `walkdir` crate provides an efficient way to traverse directory trees.
// Ensure your Cargo.toml has `walkdir = "..."`.
use walkdir;

/// A super useful function to resolve paths that start with a tilde `~`.
/// On Unix-like systems, `~` is a shortcut for the user's home directory.
/// This function expands that `~` into the full, absolute path, like `/Users/yourusername/`.
/// This is crucial for user-friendly path inputs.
///
/// # Arguments
/// * `path`: A string slice (`&str`) representing the path, which might start with `~`.
///
/// # Returns
/// * `PathBuf`: The fully resolved path if `~` was present and the home directory
///              could be determined. Otherwise, it returns the original path unchanged.
pub fn expand_tilde(path: &str) -> PathBuf {
    // Check if the input path string actually begins with a tilde character.
    if path.starts_with("~") {
        // Attempt to retrieve the current user's home directory.
        // `dirs::home_dir()` is a cross-platform way to get this path.
        if let Some(home) = dirs::home_dir() {
            // If the home directory was successfully found:
            // 1. Convert the home directory `PathBuf` into a string slice (`to_string_lossy()`)
            //    which safely handles non-UTF8 characters by replacing them.
            // 2. Use `replacen` to replace only the *first* occurrence of `~` with the home path.
            //    This ensures paths like `~/Documents/~/file.txt` are handled correctly.
            return PathBuf::from(path.replacen("~", &home.to_string_lossy(), 1));
        }
    }
    // If the path does not start with `~`, or if `dirs::home_dir()` failed to find
    // the home directory, simply convert the original input path string into a `PathBuf`
    // and return it as is.
    PathBuf::from(path)
}

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


/// Extracts the contents of a compressed archive (zip, tar.gz, etc.) into a new subdirectory
/// within the specified destination path. This is a core utility for unpacking downloaded tools.
/// The extracted contents will be placed in a new directory named "extracted" inside `dest`.
///
/// # Arguments
/// * `src`: The path (`&Path`) to the compressed archive file that needs to be extracted.
/// * `dest`: The parent directory (`&Path`) where the *extracted* content should be placed.
///           A new subdirectory named "extracted" will be created inside this `dest` path.
/// * `known_file_type`: An `Option<&str>`. If `Some(type_str)` is provided, it tells the function
///   the exact type of the archive (e.g., "zip", "tar.gz"), bypassing internal detection.
///   This is useful when the caller already knows the type (e.g., from a GitHub asset name),
///   which can be faster or more accurate than re-detecting. If `None`, `detect_file_type` is used.
///
/// # Returns
/// * `io::Result<PathBuf>`:
///   - `Ok(PathBuf)` with the path to the newly created "extracted" directory if extraction was successful.
///   - An `io::Error` if extraction fails, the archive type is unsupported, or any I/O operation fails.
pub fn extract_archive(src: &Path, dest: &Path, known_file_type: Option<&str>) -> io::Result<PathBuf> {
    log_debug!("[Utils] Extracting archive {:?} into {:?}", src.to_string_lossy().blue(), dest.to_string_lossy().cyan());

    // Determine the file type to guide the extraction process.
    // If `known_file_type` is provided (i.e., `Some(ft)`), use that.
    // Otherwise, fall back to `detect_file_type` which uses the `file` command.
    let file_type = if let Some(ft) = known_file_type {
        log_debug!("[Utils] Using known file type from argument: {}", ft.green());
        ft.to_string()
    } else {
        log_debug!("[Utils] No known file type provided. Auto-detecting using 'file' command...");
        detect_file_type(src)
    };

    // Create a specific subdirectory named "extracted" inside the `dest` directory.
    // This keeps extracted contents organized and prevents clutter in the main temporary directory.
    // `fs::create_dir_all` creates all necessary parent directories if they don't exist.
    // The `?` operator propagates any I/O error (e.g., permission denied) from directory creation.
    let extracted_path = dest.join("extracted");
    fs::create_dir_all(&extracted_path)?;

    // Use a `match` statement to handle different archive types.
    match file_type.as_str() {
        "zip" => {
            // Open the source zip file.
            let file = File::open(src)?;
            // Create a new `ZipArchive` reader from the opened file.
            let mut archive = ZipArchive::new(file)?;
            // Extract all contents of the zip archive into the `extracted_path`.
            archive.extract(&extracted_path)?;
            log_debug!("[Utils] Zip archive extracted successfully.");
        }
        "tar.gz" => { // Handle specific `tar.gz` files.
            // Open the gzipped tar file.
            let tar_gz = File::open(src)?;
            // Create a `GzDecoder` to decompress the gzip stream.
            let decompressor = GzDecoder::new(tar_gz);
            // Create a `tar::Archive` reader from the decompressed stream.
            let mut archive = Archive::new(decompressor);
            // Unpack all contents of the tar archive into the `extracted_path`.
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar.gz archive extracted successfully.");
        }
        "gz" => { // Handle pure `.gz` files (not tarred, typically a single compressed file).
            log_info!("[Utils] Decompressing plain GZ file. Contents will be the original file without tar extraction.");
            let gz_file = File::open(src)?;
            let mut decompressor = GzDecoder::new(gz_file);
            // Determine the output file path by removing the ".gz" extension from the source filename.
            let output_file_path = extracted_path.join(src.file_stem().unwrap_or_default());
            let mut output_file = File::create(&output_file_path)?;
            // Copy the decompressed data from the `GzDecoder` to the new output file.
            io::copy(&mut decompressor, &mut output_file)?;
            log_debug!("[Utils] GZ file decompressed successfully to {:?}", output_file_path.display());
        }
        "tar.bz2" => {
            // Open the bzipped tar file.
            let tar_bz2 = File::open(src)?;
            // Create a `BzDecoder` to decompress the bzip2 stream.
            let decompressor = BzDecoder::new(tar_bz2);
            // Create a `tar::Archive` reader from the decompressed stream.
            let mut archive = Archive::new(decompressor);
            // Unpack all contents of the tar archive into the `extracted_path`.
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar.bz2 archive extracted successfully.");
        }
        "tar" => { // Handle plain `.tar` archives (uncompressed).
            // Open the tar file.
            let tar = File::open(src)?;
            // Create a `tar::Archive` reader directly from the file.
            let mut archive = Archive::new(tar);
            // Unpack all contents of the tar archive into the `extracted_path`.
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar archive extracted successfully.");
        }
        "binary" => { // For standalone binaries like a .exe or uncompressed Mac binary.
            // In this case, "extraction" means simply copying the binary to the `extracted_path`.
            log_info!("[Utils] Copying detected 'binary' directly to extraction path.");
            // Get the filename part from the source path.
            let file_name = src.file_name().ok_or_else(|| {
                // If the source path doesn't have a filename, return an error.
                io::Error::new(io::ErrorKind::InvalidInput, "Source path has no filename")
            })?;
            // Copy the source file to the `extracted_path` maintaining its original filename.
            fs::copy(src, extracted_path.join(file_name))?;
            log_debug!("[Utils] Binary copied successfully to {:?}", extracted_path.join(file_name).display());
        }
        "pkg" => { // For macOS `.pkg` files, which are installers, not archives to unpack in the traditional sense.
            // We copy them to the extracted path so they are available for installation later.
            log_info!("[Utils] Detected .pkg installer. Copying directly to extraction path for installation.");
            let file_name = src.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Source path has no filename")
            })?;
            // Copy the `.pkg` file to the `extracted_path`.
            fs::copy(src, extracted_path.join(file_name))?;
            log_debug!("[Utils] .pkg file copied successfully to {:?}", extracted_path.join(file_name).display());
        }
        _ => {
            // If the `file_type` string does not match any of the supported types.
            log_error!("[Utils] Unsupported archive type '{}' for extraction: {:?}", file_type.red(), src);
            // Return an `io::Error` indicating that the archive type is not supported.
            return Err(io::Error::new(
                io::ErrorKind::InvalidData, // `InvalidData` is suitable for unsupported file formats.
                format!("Unsupported archive type: {}", file_type),
            ));
        }
    }

    // Log a success message with the path to the extracted contents.
    log_debug!("[Utils] ✨ Archive contents available at: {:?}", extracted_path.to_string_lossy().green());
    Ok(extracted_path) // Return the path to the directory where contents were extracted.
}

/// Recursively searches a given directory for the first executable file it finds.
/// This is essential after extracting an archive, as the actual binary we need
/// might be nested deep within subdirectories or have a non-standard name.
///
/// # Arguments
/// * `dir`: The directory (`&Path`) to start searching from. This directory and its
///          subdirectories will be traversed.
///
/// # Returns
/// * `Option<PathBuf>`:
///   - `Some(PathBuf)` containing the full path to the first executable file found.
///   - `None` if no executable file is found within the specified directory tree.
pub fn find_executable(dir: &Path) -> Option<PathBuf> {
    log_debug!("[Utils] 🔎 Searching for an executable in: {:?}", dir.to_string_lossy().yellow());

    // Define a list of common executable names or keywords to look for.
    // This helps improve detection for binaries that might not have standard extensions
    // or are named generically.
    let common_executables = [
        "bin", "cli", "app", "main", "daemon", // Generic names
        "go", "node", "python", "ruby", // Common runtime names if the binary is a wrapper
    ];

    // Use `walkdir::WalkDir::new(dir)` to create an iterator that recursively
    // traverses the directory `dir` and its subdirectories.
    for entry in walkdir::WalkDir::new(dir)
        .into_iter() // Convert `WalkDir` into an iterator.
        .filter_map(|e| e.ok()) // Filter out any `Err` entries (e.g., permission denied)
    // and unwrap `Ok` entries into `DirEntry`.
    {
        let path = entry.path(); // Get the `Path` for the current directory entry.
        // Get the filename part of the path and convert it to lowercase for case-insensitive checks.
        let file_name_str = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();

        // Check if the current entry is a file (not a directory).
        if path.is_file() {
            // 1. Check for common executable extensions first, especially relevant for Windows.
            if file_name_str.ends_with(".exe") || file_name_str.ends_with(".sh") || file_name_str.ends_with(".bat") {
                log_debug!("[Utils] Found potential executable (by extension): {:?}", path.display());
                return Some(path.to_path_buf()); // Return the path if an executable extension is found.
            }

            // 2. Check if the filename itself contains common executable indicators or patterns.
            // This includes the `common_executables` array and a heuristic for Unix-like systems:
            // if there's no extension and the filename is not empty, it might be an executable.
            if common_executables.iter().any(|&name| file_name_str.contains(name)) ||
                // The `cfg!(unix)` attribute ensures this block only compiles on Unix-like systems.
                (cfg!(unix) && path.extension().is_none() && !file_name_str.is_empty()) {
                log_debug!("[Utils] Found potential executable (by name heuristic): {:?}", path.display());

                // On Unix-like systems, perform an additional check for execute permissions.
                #[cfg(unix)]
                {
                    if let Ok(metadata) = fs::metadata(path) {
                        // Check if any execute bit is set (user, group, or other).
                        // `metadata.permissions().mode()` gets the Unix file mode bits.
                        // `& 0o111` performs a bitwise AND with `0o111` (octal for execute permissions for all).
                        if (metadata.permissions().mode() & 0o111) != 0 {
                            log_debug!("[Utils] Found executable (name and permissions): {:?}", path.display());
                            return Some(path.to_path_buf()); // Return the path if it's executable.
                        } else {
                            log_debug!("[Utils] Skipping {:?}: Not executable by permissions.", path.display());
                        }
                    }
                }
                // On non-Unix systems (e.g., Windows), if it matches name/no extension, assume it's executable.
                // Windows handles executability differently (e.g., via `.exe` extension, not permissions bits).
                #[cfg(not(unix))]
                {
                    log_debug!("[Utils] Found potential executable (by name/no extension, non-Unix): {:?}", path.display());
                    return Some(path.to_path_buf());
                }
            } else {
                log_debug!("[Utils] Skipping non-executable candidate: {:?} (no common name match or executable permissions)", path.display());
            }
        }
    }
    // If the loop completes without finding any executable, log a warning and return `None`.
    log_warn!("[Utils] No executable found within {:?}", dir.to_string_lossy().purple());
    None
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


/// Moves a file (typically a binary) from a source path to a destination path,
/// and can also rename it in the process by providing a new filename in the `to` path.
/// It ensures that the destination's parent directories exist before attempting the move.
///
/// # Arguments
/// * `from`: The source path (`&Path`) of the file to be moved.
/// * `to`: The destination path (`&Path`), including the new desired filename if renaming.
///
/// # Returns
/// * `io::Result<()>`:
///   - `Ok(())` on successful move/rename.
///   - `io::Error` if the operation fails (e.g., source file not found, permission issues,
///     or failure during fallback copy/remove).
pub fn move_and_rename_binary(from: &Path, to: &Path) -> io::Result<()> {
    log_debug!("[Utils] Moving binary from {:?} to {:?}", from.to_string_lossy().yellow(), to.to_string_lossy().cyan());

    // If the destination path has parent directories (e.g., `/usr/local/bin/`),
    // ensure those directories exist before trying to move the file into them.
    // `to.parent()` returns `Some(Path)` if `to` has a parent, `None` if `to` is a root or single component.
    if let Some(parent) = to.parent() {
        // `fs::create_dir_all` creates all necessary parent directories recursively.
        // The `?` propagates any `io::Error` from directory creation.
        fs::create_dir_all(parent)?;
    }

    // Perform the move operation using `fs::rename`.
    // `fs::rename` is generally preferred because it is:
    // 1. Atomic: The operation either fully succeeds or fully fails, preventing corrupted states.
    // 2. Performant: It's often a simple metadata update on the same filesystem.
    // However, it fails if source and destination are on different filesystems (cross-device link).
    match fs::rename(from, to) {
        Ok(_) => {
            log_debug!("[Utils] Binary moved/renamed to {}", to.to_string_lossy().green());
            Ok(()) // Success case.
        },
        // Handle the specific error case where `fs::rename` fails due to `CrossesDevices`.
        // This means the source and destination paths are on different file systems.
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            log_warn!("[Utils] Cross-device link detected, falling back to copy and remove for {:?} to {:?}: {}", from.display(), to.display(), e);
            // Fallback strategy: copy the file to the new location.
            fs::copy(from, to)?;
            // Then, remove the original file. This is not atomic.
            fs::remove_file(from)?;
            log_info!("[Utils] Binary copied and old removed successfully to {:?}", to.to_string_lossy().green());
            Ok(()) // Success after fallback.
        },
        // Handle any other `io::Error` that `fs::rename` might return.
        Err(e) => {
            log_error!("[Utils] Failed to move binary from {:?} to {:?}: {}", from.display(), to.display(), e);
            Err(e) // Propagate the original error.
        }
    }
}

/// Makes a given file executable. On Unix-like systems, this is equivalent to `chmod +x file`.
/// This is crucial for downloaded binaries to be runnable, as files downloaded from the internet
/// often do not preserve executable permissions.
///
/// # Arguments
/// * `path`: The path (`&Path`) to the file to make executable.
///
/// # Returns
/// * `io::Result<()>`:
///   - `Ok(())` on success (permissions set or no-op on non-Unix).
///   - `io::Error` if permissions cannot be read or set (only applicable on Unix).
#[cfg(unix)] // This function only compiles on Unix-like operating systems (Linux, macOS).
pub fn make_executable(path: &Path) -> io::Result<()> {
    log_debug!("[Utils] Making {:?} executable", path.to_string_lossy().yellow());

    // Get the current metadata (including permissions) of the file.
    // The `?` operator propagates any `io::Error` if metadata cannot be retrieved.
    let mut perms = fs::metadata(path)?.permissions();

    // Set the file's permissions.
    // `0o755` is an octal representation of file permissions:
    // - Owner: Read (4) + Write (2) + Execute (1) = 7
    // - Group: Read (4) + Execute (1) = 5
    // - Others: Read (4) + Execute (1) = 5
    // This grants full control to the owner, and read/execute to group and others.
    perms.set_mode(0o755);

    // Apply the modified permissions back to the file.
    // The `?` operator propagates any `io::Error` if permissions cannot be set.
    fs::set_permissions(path, perms)?;
    log_debug!("[Utils] File {:?} is now executable.", path.to_string_lossy().green());
    Ok(()) // Indicate success.
}

// Provide a dummy implementation for `make_executable` on non-Unix systems to avoid compilation errors.
// On Windows, executable permissions are often implicit for `.exe` files and not controlled by mode bits.
#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> io::Result<()> {
    log_debug!("[Utils] `make_executable` is a no-op on this non-Unix platform (permissions handled differently).");
    Ok(()) // Return success, as no action is needed or possible on these platforms.
}

/// Returns a path to a dedicated temporary directory for `setup-devbox` operations.
/// This ensures that temporary files are kept separate from other system temporary files
/// and can be easily identified and cleaned up. The directory is created if it doesn't exist.
///
/// # Returns
/// * `PathBuf`: The absolute path to the `setup-devbox` specific temporary directory.
pub fn get_temp_dir() -> PathBuf {
    // Start with the system's standard temporary directory (e.g., `/tmp` on Unix, `%TEMP%` on Windows).
    let path = env::temp_dir()
        // Append a subdirectory specific to our application (`setup-devbox`).
        .join("setup-devbox");
    // Attempt to create this directory (and any necessary parent directories).
    // `let _ =` is used to ignore the `Result` returned by `fs::create_dir_all`.
    // It's acceptable if the directory already exists, and if creation fails due to
    // other reasons (e.g., permissions), we proceed with the path anyway, and subsequent
    // file operations will fail, which is the correct behavior.
    let _ = fs::create_dir_all(&path);
    // Return the full path to our specific temporary directory.
    path
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
        "linux" => "linux".to_string(),                             // Linux is typically straightforward.
        "windows" | "win32" | "win64" => "windows".to_string(),     // Map various Windows names to "windows".
        other => {
            // If we encounter an unknown OS variant, log a warning.
            // We return the lowercase version of the unknown string as-is,
            // hoping it might still match some asset names.
            log_warn!("[Utils] Unknown OS variant '{}', using as-is. This might cause issues with asset matching.", other.purple());
            other.to_string()
        }
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
            log_warn!("[Utils] Unknown ARCH variant '{}', using as-is. This might cause issues with asset matching.", other.purple());
            other.to_string()
        }
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
        "macos" => vec!["macos", "darwin", "apple-darwin", "macosx"] // Aliases for macOS.
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
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
    let os_lower = os.to_lowercase();
    let arch_lower = arch.to_lowercase();

    // 1. Check for OS match:
    // Iterate through all known aliases for the current OS. If any alias is found
    // as a substring within the asset filename, it's considered an OS match.
    let os_matches = os_aliases(&os_lower)
        .iter()
        .any(|alias| asset_name_lower.contains(alias));

    // If no OS match, immediately return false. No need to check architecture.
    if !os_matches {
        log_debug!("[Utils] Asset '{}' does not match OS '{}'", filename.dimmed(), os);
        return false;
    }

    // 2. Check for Architecture match:
    // Iterate through all known aliases for the current architecture. If any alias is found
    // as a substring within the asset filename, it's considered an architecture match.
    let arch_matches = arch_aliases(&arch_lower)
        .iter()
        .any(|alias| asset_name_lower.contains(alias));

    // 3. Special consideration for macOS ARM64 (aarch64) with Rosetta 2 fallback:
    // If the target is macOS ARM64, and the asset filename contains "x86_64" (Intel architecture)
    // but *does not* contain "arm64" or "aarch64" (explicit ARM64),
    // it's considered a potential match because macOS can run x86_64 binaries via Rosetta 2 emulation.
    let rosetta_fallback = (os_lower == "macos" && arch_lower == "arm64") &&
        asset_name_lower.contains("x86_64") &&
        !(asset_name_lower.contains("arm64") || asset_name_lower.contains("aarch64"));

    // If neither a direct architecture match nor the Rosetta fallback condition is met, return false.
    if !(arch_matches || rosetta_fallback) {
        log_debug!("[Utils] Asset '{}' does not match architecture '{}' (and no Rosetta fallback).", filename.dimmed(), arch);
        return false;
    }

    // 4. Optional: Exclude common source, debug, or checksum files.
    // These files are usually not the actual executable binaries we want to download.
    // This helps in picking the actual binary release.
    if asset_name_lower.contains("src") ||
        asset_name_lower.contains("source") ||
        asset_name_lower.contains("debug") ||
        asset_name_lower.contains("checksum") ||
        asset_name_lower.contains("sha256") ||
        asset_name_lower.contains("tar.gz.sig") || // Common signature file for tar.gz
        asset_name_lower.ends_with(".asc") {      // Common detached signature file extension
        log_debug!("[Utils] Asset '{}' excluded due to containing common non-binary keywords.", filename.dimmed());
        return false;
    }

    // If all checks pass, the asset is considered a match for the current platform.
    log_debug!("[Utils] Asset '{}' matches platform (OS: {}, ARCH: {}) -> {}", filename.dimmed(), os.cyan(), arch.magenta(), "true".bold());
    true
}

/// Returns the canonical path to the DevBox directory, typically `~/.setup-devbox`.
/// This is the base directory where `state.json` and the `config` folder reside.
/// If the home directory cannot be determined, it will log an error and
/// fall back to the current directory, which might lead to unexpected behavior.
///
/// # Returns
/// * `PathBuf`: The path to the `.setup-devbox` directory.
pub fn get_devbox_dir() -> PathBuf {
    // Attempt to get the user's home directory.
    if let Some(home_dir) = dirs::home_dir() {
        // Corrected: Use ".setup-devbox" instead of ".devbox"
        let setup_devbox_dir = home_dir.join(".setup-devbox");
        log_debug!("[Utils] DevBox directory resolved to: {}", setup_devbox_dir.display().to_string().cyan());
        setup_devbox_dir
    } else {
        // If the home directory cannot be determined, log an error and use the current directory as a fallback.
        log_error!("[Utils] Could not determine home directory. Falling back to current directory for .setup-devbox path.");
        // Get the current working directory.
        let current_dir = env::current_dir().unwrap_or_else(|e| {
            // If even current directory can't be found, panic as it's a critical error.
            panic!("Failed to get current directory and home directory: {}", e);
        });
        // Corrected: Use ".setup-devbox" for the fallback path
        let fallback_dir = current_dir.join(".setup-devbox");
        log_warn!("[Utils] Fallback DevBox directory: {}", fallback_dir.display().to_string().yellow());
        fallback_dir
    }
}
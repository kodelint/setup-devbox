// The 'colored' crate helps us make our console output look pretty and readab
use colored::Colorize;
// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_info, log_warn};
// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::{Path, PathBuf};
// For file system operations: creating directories, reading files, etc.
// `std::fs` provides functions for interacting with the file system.
use std::fs;
// This line is conditional: it's only compiled when targeting Unix-like systems (macOS, Linux).
// It's used to set file permissions, specifically making files executable, which is a Unix-specific concept.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
// To get environment variables, like the temporary directory or home directory.
// `std::env` provides functions to interact with the process's environment.
// `std::io` contains core input/output functionalities and error types.
use std::io;

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

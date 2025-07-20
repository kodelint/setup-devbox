// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_info, log_warn};
// The 'colored' crate helps us make our console output look pretty and readab
use colored::Colorize;
// A powerful library for parsing executable file formats (ELF, Mach-O, PE)
use goblin::Object;
// For file system operations: creating directories, reading files, etc.
// `std::fs` provides functions for interacting with the file system.
use std::fs;
// To get environment variables, like the temporary directory or home directory.
// `std::env` provides functions to interact with the process's environment.
// `std::io` contains core input/output functionalities and error types.
use std::io;
// This line is conditional: it's only compiled when targeting Unix-like systems (macOS, Linux).
// It's used to set file permissions, specifically making files executable, which is a Unix-specific concept.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::{Path, PathBuf};
// For recursively traversing directory trees
use walkdir::WalkDir;

/// Recursively searches a given directory for the most likely executable file.
/// This function is crucial for post-extraction processes, as downloaded binaries
/// might be nested deep within archive subdirectories or have non-standard names
/// (e.g., `helix` tool installed as `hx` binary).
///
/// It employs a multi-stage heuristic for robust executable identification:
/// 1. **Early Exit for Non-Binaries:** Skips files with common non-executable extensions (e.g., `.md`, `.txt`).
/// 2. **Header-based Detection (`goblin`):** Attempts to parse file headers for known executable formats
///    like ELF (Linux), Mach-O (macOS). If a binary is detected, it verifies/sets executable permissions.
/// 3. **Shebang Detection:** Checks for `#!` at the start of the file, indicating a script (e.g., Bash, Python).
/// 4. **Fallback Name Match:** If no executable format is confirmed, it performs a fallback check:
///    if the file's name *exactly* matches the expected tool name (or its renamed version),
///    it attempts to set executable permissions and includes it as a candidate.
/// 5. **Candidate Prioritization:** Collects all potential executables and sorts them,
///    prioritizing an exact filename match with the `target_name_lower` (the expected binary name,
///    considering renames) and then by file size (larger files are often the main binary).
///
/// # Arguments
/// * `dir`: The `&Path` to the directory where the search should begin. The function
///          will traverse this directory and all its subdirectories.
/// * `tool_name`: The original, user-defined name of the tool (e.g., "helix").
///                Used as a fallback for name matching and logging.
/// * `rename_to`: An `Option<&str>` specifying an alternative name for the executable
///                if it's different from `tool_name` (e.g., "hx" for "helix"). This is
///                the primary name targeted during the search and sorting.
///
/// # Returns
/// * `Option<PathBuf>`:
///   - `Some(PathBuf)` containing the full path to the most probable executable file found.
///   - `None` if no suitable executable file is identified within the specified directory tree.


pub fn find_executable(dir: &Path, tool_name: &str, rename_to: Option<&str>) -> Option<PathBuf> {
    let tool_name_lower = tool_name.to_lowercase();
    let target_name_lower = rename_to.map_or(tool_name_lower.clone(), |s| s.to_lowercase());
    let mut candidates: Vec<(PathBuf, u64)> = Vec::new();

    // Read entries in the extracted directory (non-recursive)
    let entries: Vec<_> = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .collect();

    if entries.len() == 1 {
        let sole_path = entries[0].path();
        if sole_path.is_file() {
            log_debug!("Single file found, inspecting as potential binary: {}", sole_path.display());

            match fs::read(&sole_path) {
                Ok(data) => {
                    log_debug!("Read {} bytes from file for header inspection.", data.len());
                    match Object::parse(&data) {
                        Ok(obj) => {
                            match obj {
                                Object::Elf(_) | Object::Mach(_) => {
                                    log_debug!("Detected native binary (ELF/Mach-O) in single file: {}", sole_path.display());
                                    #[cfg(unix)]
                                    {
                                        if !is_executable(&sole_path) {
                                            if let Ok(metadata) = fs::metadata(&sole_path) {
                                                let mut perms = metadata.permissions();
                                                perms.set_mode(perms.mode() | 0o755);
                                                let _ = fs::set_permissions(&sole_path, perms);
                                                log_debug!("Updated permissions for single file binary: {}", sole_path.display());
                                            }
                                        }
                                    }
                                    return Some(sole_path);
                                }
                                _ => {
                                    if data.starts_with(b"#!") {
                                        log_debug!("Detected shebang script in single file: {}", sole_path.display());
                                        #[cfg(unix)]
                                        {
                                            if !is_executable(&sole_path) {
                                                if let Ok(metadata) = fs::metadata(&sole_path) {
                                                    let mut perms = metadata.permissions();
                                                    perms.set_mode(perms.mode() | 0o755);
                                                    let _ = fs::set_permissions(&sole_path, perms);
                                                    log_debug!("Updated permissions for single file script: {}", sole_path.display());
                                                }
                                            }
                                        }
                                        return Some(sole_path);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            log_debug!("Goblin failed to parse the file: {}", err);
                        }
                    }
                }
                Err(err) => {
                    log_warn!("Failed to read file {} for parsing: {}", sole_path.display(), err);
                }
            }
        }
    }

    // Walk recursively for multiple files
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if file_name.ends_with(".md")
            || file_name.ends_with(".txt")
            || file_name.ends_with(".json")
            || file_name.ends_with(".1")
            || file_name.ends_with(".ps1")
            || file_name.ends_with(".fish")
            || file_name.ends_with(".zsh")
            || file_name.ends_with(".bash")
            || file_name.ends_with(".log")
            || file_name.ends_with(".yaml") || file_name.ends_with(".yml")
            || file_name.contains("license")
            || file_name.contains("readme")
        {
            log_debug!("Skipping known non-executable file by extension/name: {}", file_name);
            continue;
        }

        let mut add_candidate = false;

        if let Ok(data) = fs::read(path) {
            log_debug!("Read {} bytes from file for header inspection: {}", data.len(), path.display());
            if let Ok(obj) = Object::parse(&data) {
                match obj {
                    Object::Elf(_) | Object::Mach(_) => {
                        if is_executable(path) {
                            log_debug!("Found executable binary (ELF/Mach-O): {}", path.display());
                            add_candidate = true;
                        } else {
                            #[cfg(unix)]
                            if let Ok(metadata) = fs::metadata(path) {
                                let mut perms = metadata.permissions();
                                perms.set_mode(perms.mode() | 0o755);
                                if fs::set_permissions(path, perms).is_ok() {
                                    log_debug!("Set executable bit and added ELF/Mach-O binary: {}", path.display());
                                    add_candidate = true;
                                } else {
                                    log_warn!("Failed to set executable permissions for {}.", path.display());
                                }
                            }
                            #[cfg(not(unix))]
                            {
                                log_debug!("Found executable binary (ELF/Mach-O) on non-Unix: {}", path.display());
                                add_candidate = true;
                            }
                        }
                    }
                    _ => {
                        if data.starts_with(b"#!") {
                            if is_executable(path) {
                                log_debug!("Found executable script (shebang): {}", path.display());
                                add_candidate = true;
                            } else {
                                #[cfg(unix)]
                                if let Ok(metadata) = fs::metadata(path) {
                                    let mut perms = metadata.permissions();
                                    perms.set_mode(perms.mode() | 0o755);
                                    if fs::set_permissions(path, perms).is_ok() {
                                        log_debug!("Set executable bit and added script: {}", path.display());
                                        add_candidate = true;
                                    } else {
                                        log_warn!("Failed to set executable permissions for script {}.", path.display());
                                    }
                                }
                                #[cfg(not(unix))]
                                {
                                    log_debug!("Found executable script (shebang) on non-Unix: {}", path.display());
                                    add_candidate = true;
                                }
                            }
                        }
                    }
                }
            } else {
                log_debug!("Goblin failed to parse the file: {}", path.display());
            }
        } else {
            log_warn!("Failed to read file {} for parsing", path.display());
        }

        if !add_candidate && file_name == target_name_lower {
            log_debug!("Fallback: Forcing executable candidate for {} (exact name match).", path.display());
            #[cfg(unix)]
            if let Ok(metadata) = fs::metadata(path) {
                let mut perms = metadata.permissions();
                perms.set_mode(perms.mode() | 0o755);
                if fs::set_permissions(path, perms).is_ok() {
                    if is_executable(path) {
                        add_candidate = true;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                add_candidate = true;
            }
        }

        if add_candidate {
            let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            log_debug!("Adding candidate: {} ({} bytes)", path.display(), size);
            candidates.push((path.to_path_buf(), size));
        }
    }

    candidates.sort_by(|(a_path, a_size), (b_path, b_size)| {
        let a = a_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let b = b_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();

        if a == target_name_lower && b != target_name_lower {
            std::cmp::Ordering::Less
        } else if a != target_name_lower && b == target_name_lower {
            std::cmp::Ordering::Greater
        } else {
            b_size.cmp(a_size)
        }
    });

    candidates.into_iter().map(|(p, _)| p).next()
}

/// Helper function to check if a file has executable permissions.
///
/// # Arguments
/// * `path`: The `&Path` to the file to check.
///
/// # Returns
/// * `bool`: `true` if the file exists and has any executable bit set (on Unix-like systems).
///           On Windows, this check is less relevant as executability is primarily determined
///           by file extension (`.exe`, `.bat`, etc.) rather than permission bits.
///           Returns `false` if metadata cannot be retrieved or it's not executable.
fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        // If metadata is retrieved successfully, check the permissions.
        // `mode() & 0o111` checks if any of the execute bits (user, group, other) are set.
        .map(|m| m.permissions().mode() & 0o111 != 0)
        // If `fs::metadata` fails (e.g., file not found, permission denied), default to `false`.
        .unwrap_or(false)
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
    log_debug!("[Utils] Moving binary from {} to {}", from.to_string_lossy().yellow(), to.to_string_lossy().cyan());

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
    log_debug!("[Utils] Making {} executable", path.to_string_lossy().yellow());
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
    log_debug!("[Utils] File {} is now executable.", path.to_string_lossy().green());
    Ok(()) // Indicate success.
}

// Provide a dummy implementation for `make_executable` on non-Unix systems to avoid compilation errors.
// On Windows, executable permissions are often implicit for `.exe` files and not controlled by mode bits.
#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> io::Result<()> {
    log_debug!("[Utils] `make_executable` is a no-op on this non-Unix platform (permissions handled differently).");
    Ok(()) // Return success, as no action is needed or possible on these platforms.
}

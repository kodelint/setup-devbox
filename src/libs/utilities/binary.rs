use crate::schemas::tools::ToolEntry;
// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_warn};
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
use crate::schemas::path_resolver::PathResolver;
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
///   will traverse this directory and all its subdirectories.
/// * `tool_name`: The original, user-defined name of the tool (e.g., "helix").
///   Used as a fallback for name matching and logging.
/// * `rename_to`: An `Option<&str>` specifying an alternative name for the executable
///   if it's different from `tool_name` (e.g., "hx" for "helix"). This is
///   the primary name targeted during the search and sorting.
///
/// # Returns
/// * `Option<PathBuf>`:
///   - `Some(PathBuf)` containing the full path to the most probable executable file found.
///   - `None` if no suitable executable file is identified within the specified directory tree.
pub fn find_executable(
    dir: &Path,
    tool_name: &str,
    rename_to: Option<&str>,
    tool_source: String,
) -> Option<PathBuf> {
    // Convert tool name and target (renamed) name to lowercase for case-insensitive comparisons.
    let tool_name_lower = tool_name.to_lowercase();
    let target_name_lower = rename_to.map_or(tool_name_lower.clone(), |s| s.to_lowercase());
    // Vector to store potential executable candidates, along with their file sizes for sorting.
    let mut candidates: Vec<(PathBuf, u64)> = Vec::new();

    // Special handling for directories containing a single entry.
    // This optimization attempts to quickly identify the executable if it's the only file/directory.
    let entries: Vec<_> = fs::read_dir(dir)
        .ok()? // Return None if the directory cannot be read.
        .filter_map(Result::ok) // Filter out any entries that caused an error.
        .collect();

    // If there's exactly one entry in the directory:
    if entries.len() == 1 {
        let sole_path = entries[0].path();
        // If that single entry is a file:
        if sole_path.is_file() {
            log_debug!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Single file found, inspecting as potential binary: {}",
                sole_path.display()
            );

            // Attempt to read the file's content for header inspection.
            match fs::read(&sole_path) {
                Ok(data) => {
                    log_debug!(
                        "[SDB::Tools::{tool_source}::BinaryInstaller] Read {} bytes from file for header inspection.",
                        data.len()
                    );
                    // Use `goblin` to parse the file's header and detect known executable formats.
                    match Object::parse(&data) {
                        Ok(obj) => {
                            match obj {
                                // If it's an ELF (Linux) or Mach-O (macOS) executable:
                                Object::Elf(_) | Object::Mach(_) => {
                                    log_debug!(
                                        "[SDB::Tools::{tool_source}::BinaryInstaller] Detected native binary (ELF/Mach-O) in single file: {}",
                                        sole_path.display()
                                    );
                                    // On Unix-like systems, ensure it's executable.
                                    #[cfg(unix)]
                                    {
                                        if !is_executable(&sole_path) {
                                            if let Ok(metadata) = fs::metadata(&sole_path) {
                                                let mut perms = metadata.permissions();
                                                // Add executable permissions (0o755) to existing ones.
                                                perms.set_mode(perms.mode() | 0o755);
                                                let _ = fs::set_permissions(&sole_path, perms);
                                                log_debug!(
                                                    "[SDB::Tools::{tool_source}::BinaryInstaller] Updated permissions for single file binary: {}",
                                                    sole_path.display()
                                                );
                                            }
                                        }
                                    }
                                    // If it's a confirmed native binary, return it immediately as the most likely candidate.
                                    return Some(sole_path);
                                }
                                // For other object types (e.g., PE for Windows, or unknown):
                                _ => {
                                    // Check if the file starts with `#!` (shebang), indicating a script.
                                    if data.starts_with(b"#!") {
                                        log_debug!(
                                            "[SDB::Tools::{tool_source}::BinaryInstaller] Detected shebang script in single file: {}",
                                            sole_path.display()
                                        );
                                        // On Unix-like systems, ensure it's executable.
                                        #[cfg(unix)]
                                        {
                                            if !is_executable(&sole_path) {
                                                if let Ok(metadata) = fs::metadata(&sole_path) {
                                                    let mut perms = metadata.permissions();
                                                    perms.set_mode(perms.mode() | 0o755);
                                                    let _ = fs::set_permissions(&sole_path, perms);
                                                    log_debug!(
                                                        "[SDB::Tools::{tool_source}::BinaryInstaller] Updated permissions for single file script: {}",
                                                        sole_path.display()
                                                    );
                                                }
                                            }
                                        }
                                        // If it's a script, return it immediately.
                                        return Some(sole_path);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            log_debug!(
                                "[SDB::Tools::{tool_source}::BinaryInstaller] Goblin failed to parse the file: {}",
                                err
                            );
                        }
                    }
                }
                Err(err) => {
                    log_warn!(
                        "[SDB::Tools::{tool_source}::BinaryInstaller] Failed to read file {} for parsing: {}",
                        sole_path.display(),
                        err
                    );
                }
            }
        }
    }

    // If the single-file optimization didn't yield a result, or there are multiple files,
    // walk the directory recursively to find all potential executables.
    for entry in WalkDir::new(dir)
        .into_iter() // Convert to an iterator.
        .filter_map(Result::ok) // Filter out any errors during directory traversal.
        .filter(|e| e.path().is_file())
    // Only process actual files, skip directories.
    {
        let path = entry.path();
        // Get the lowercase filename for comparison.
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Skip files with common non-executable extensions or names.
        // This is an optimization to avoid processing irrelevant files.
        if file_name.ends_with(".md")
            || file_name.ends_with(".txt")
            || file_name.ends_with(".json")
            || file_name.ends_with(".1") // Man pages often have `.1` extension
            || file_name.ends_with(".ps1") // PowerShell scripts
            || file_name.ends_with(".fish")
            || file_name.ends_with(".zsh")
            || file_name.ends_with(".bash")
            || file_name.ends_with(".log")
            || file_name.ends_with(".yaml") || file_name.ends_with(".yml")
            || file_name.contains("license")
            || file_name.contains("readme")
        {
            log_debug!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Skipping known non-executable file by extension/name: {}",
                file_name
            );
            continue; // Move to the next file.
        }

        let mut add_candidate = false; // Flag to determine if the current file should be a candidate.

        // Attempt to read file data for `goblin` and shebang checks.
        if let Ok(data) = fs::read(path) {
            log_debug!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Read {} bytes from file for header inspection: {}",
                data.len(),
                path.display()
            );
            if let Ok(obj) = Object::parse(&data) {
                match obj {
                    // If it's an ELF or Mach-O executable:
                    Object::Elf(_) | Object::Mach(_) => {
                        if is_executable(path) {
                            log_debug!(
                                "[SDB::Tools::{tool_source}::BinaryInstaller] Found executable binary (ELF/Mach-O): {}",
                                path.display()
                            );
                            add_candidate = true;
                        } else {
                            // On Unix, try to set executable permissions if not already set.
                            #[cfg(unix)]
                            if let Ok(metadata) = fs::metadata(path) {
                                let mut perms = metadata.permissions();
                                perms.set_mode(perms.mode() | 0o755); // Add executable bits
                                if fs::set_permissions(path, perms).is_ok() {
                                    log_debug!(
                                        "[SDB::Tools::{tool_source}::BinaryInstaller] Set executable bit and added ELF/Mach-O binary: {}",
                                        path.display()
                                    );
                                    add_candidate = true;
                                } else {
                                    log_warn!(
                                        "[SDB::Tools::{tool_source}::BinaryInstaller] Failed to set executable permissions for {}.",
                                        path.display()
                                    );
                                }
                            }
                            // On non-Unix, assume it's executable if recognized as ELF/Mach-O (e.g., Windows Subsystem for Linux scenario)
                            #[cfg(not(unix))]
                            {
                                log_debug!(
                                    "[SDB::Tools::{tool_source}::BinaryInstaller] Found executable binary (ELF/Mach-O) on non-Unix: {}",
                                    path.display()
                                );
                                add_candidate = true;
                            }
                        }
                    }
                    _ => {
                        // If not a native binary, check for shebang.
                        if data.starts_with(b"#!") {
                            if is_executable(path) {
                                log_debug!(
                                    "[SDB::Tools::{tool_source}::BinaryInstaller] Found executable script (shebang): {}",
                                    path.display()
                                );
                                add_candidate = true;
                            } else {
                                // On Unix, try to set executable permissions for scripts.
                                #[cfg(unix)]
                                if let Ok(metadata) = fs::metadata(path) {
                                    let mut perms = metadata.permissions();
                                    perms.set_mode(perms.mode() | 0o755);
                                    if fs::set_permissions(path, perms).is_ok() {
                                        log_debug!(
                                            "[SDB::Tools::{tool_source}::BinaryInstaller] Set executable bit and added script: {}",
                                            path.display()
                                        );
                                        add_candidate = true;
                                    } else {
                                        log_warn!(
                                            "[SDB::Tools::{tool_source}::BinaryInstaller] Failed to set executable permissions for script {}.",
                                            path.display()
                                        );
                                    }
                                }
                                // On non-Unix, assume executable.
                                #[cfg(not(unix))]
                                {
                                    log_debug!(
                                        "[SDB::Tools::{tool_source}::BinaryInstaller] Found executable script (shebang) on non-Unix: {}",
                                        path.display()
                                    );
                                    add_candidate = true;
                                }
                            }
                        }
                    }
                }
            } else {
                log_debug!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Goblin failed to parse the file: {}",
                    path.display()
                );
            }
        } else {
            log_warn!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Failed to read file {} for parsing",
                path.display()
            );
        }

        // Fallback: If no executable format was confirmed, check for an exact filename match.
        // This is important for tools that might not have standard executable headers or shebangs
        // but are still meant to be executed (e.g., custom scripts without shebangs, or certain Windows executables).
        if !add_candidate && file_name == target_name_lower {
            log_debug!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Fallback: Forcing executable candidate for {} (exact name match).",
                path.display()
            );
            // On Unix, attempt to set executable permissions for this fallback candidate.
            #[cfg(unix)]
            if let Ok(metadata) = fs::metadata(path) {
                let mut perms = metadata.permissions();
                perms.set_mode(perms.mode() | 0o755);
                if fs::set_permissions(path, perms).is_ok() {
                    // Only add as candidate if permissions were successfully set and it's now executable.
                    if is_executable(path) {
                        add_candidate = true;
                    }
                }
            }
            // On non-Unix, simply add it as a candidate based on name match.
            #[cfg(not(unix))]
            {
                add_candidate = true;
            }
        }

        // If the file is deemed a candidate, add its path and size to the `candidates` vector.
        if add_candidate {
            let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            log_debug!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Adding candidate: {} ({} bytes)",
                path.display(),
                size
            );
            candidates.push((path.to_path_buf(), size));
        }
    }

    // Sort the collected candidates to prioritize the most likely executable.
    candidates.sort_by(|(a_path, a_size), (b_path, b_size)| {
        let a = a_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let b = b_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        // Primary sort criterion: exact match with the `target_name_lower`.
        // A file whose name exactly matches the expected (renamed) binary name is highly prioritized.
        if a == target_name_lower && b != target_name_lower {
            std::cmp::Ordering::Less // 'a' comes before 'b'
        } else if a != target_name_lower && b == target_name_lower {
            std::cmp::Ordering::Greater // 'b' comes before 'a'
        } else {
            // Secondary sort criterion (if neither or both match the target name):
            // Sort by file size in descending order (larger files first), as main executables are often the largest.
            b_size.cmp(a_size)
        }
    });

    // Return the path of the highest-priority candidate.
    // `into_iter().map(|(p, _)| p).next()` takes the first element (highest priority)
    // and discards the size, returning only the PathBuf.
    candidates.into_iter().map(|(p, _)| p).next()
}

/// Helper function to check if a file has executable permissions.
///
/// # Arguments
/// * `path`: The `&Path` to the file to check.
///
/// # Returns
/// * `bool`: `true` if the file exists and has any executable bit set (on Unix-like systems).
///   On Windows, this check is less relevant as executability is primarily determined
///   by file extension (`.exe`, `.bat`, etc.) rather than permission bits.
///   Returns `false` if metadata cannot be retrieved or it's not executable.
fn is_executable(path: &Path) -> bool {
    // Get file metadata, specifically permissions.
    fs::metadata(path)
        // If metadata is retrieved successfully, check the permissions.
        // `mode()` returns the file type and permissions as an integer.
        // `0o111` is an octal mask representing the executable bits for owner, group, and others.
        // `& 0o111 != 0` checks if *any* of these executable bits are set.
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
/// * `tool_entry`: The tool entry containing optional rename configuration.
///
/// # Returns
/// * `io::Result<()>`:
///   - `Ok(())` on successful move/rename.
///   - `io::Error` if the operation fails (e.g., source file not found, permission issues,
///     or failure during fallback copy/remove).
pub fn move_and_rename_binary(
    from: &Path,
    to: &Path,
    tool_entry: &ToolEntry,
    tool_source: String,
) -> io::Result<()> {
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Starting move_and_rename_binary operation"
    );
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Source path: {}",
        from.to_string_lossy().yellow()
    );
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Initial destination path: {}",
        to.to_string_lossy().cyan()
    );
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Tool entry rename_to: {:?}",
        tool_entry.rename_to
    );

    // Check if source file exists
    if !from.exists() {
        log_error!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] Source file does not exist: {}",
            from.to_string_lossy().red()
        );
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Source file not found: {}",
                from.display()
            ),
        ));
    }
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Source file exists, proceeding with operation"
    );

    // Determine the final destination path based on rename_to
    let final_destination = if let Some(ref new_name) = tool_entry.rename_to {
        log_debug!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] rename_to is set to: {}",
            new_name.green()
        );

        // Check if 'to' is a directory
        if to.is_dir() {
            log_debug!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Destination is a directory, appending new filename: {}",
                new_name
            );
            to.join(new_name)
        } else {
            log_debug!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Destination appears to be a file path, using parent directory and appending new filename"
            );
            // If 'to' has a parent, join new_name to parent; otherwise use 'to' as is
            if let Some(parent) = to.parent() {
                parent.join(new_name)
            } else {
                PathBuf::from(new_name)
            }
        }
    } else {
        log_debug!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] rename_to is not set, will copy without renaming"
        );

        // If no rename, use the original filename
        if to.is_dir() {
            log_debug!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] Destination is a directory, appending original filename"
            );
            let file_name = from.file_name().ok_or_else(|| {
                log_error!("[SDB::Tools::{tool_source}::BinaryInstaller] Source path has no filename component");
                io::Error::new(io::ErrorKind::InvalidInput, "[SDB::Tools::{tool_source}::BinaryInstaller] Source path has no filename")
            })?;
            to.join(file_name)
        } else {
            log_debug!("[SDB::Tools::{tool_source}::BinaryInstaller] Using destination path as-is");
            to.to_path_buf()
        }
    };

    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Final destination path: {}",
        final_destination.to_string_lossy().cyan()
    );

    // Check if destination already exists
    if final_destination.exists() {
        log_warn!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] Destination file already exists, will overwrite: {}",
            final_destination.to_string_lossy().yellow()
        );
    }

    // Create parent directories if needed
    if let Some(parent) = final_destination.parent() {
        log_debug!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] Ensuring parent directories exist: {}",
            parent.to_string_lossy()
        );
        fs::create_dir_all(parent)?;
        log_debug!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] Parent directories verified/created successfully"
        );
    } else {
        log_debug!("[SDB::Tools::{tool_source}::BinaryInstaller] No parent directories to create");
    }

    // Decide operation based on rename_to
    if tool_entry.rename_to.is_none() {
        log_debug!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] Performing simple copy operation (no rename)"
        );
        log_debug!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] Copying from {} to {}",
            from.to_string_lossy().yellow(),
            final_destination.to_string_lossy().cyan()
        );

        fs::copy(from, &final_destination)?;

        log_debug!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] Binary copied successfully to {}",
            final_destination.to_string_lossy().green()
        );
        Ok(())
    } else {
        log_debug!("[SDB::Tools::{tool_source}::BinaryInstaller] Performing rename/move operation");
        log_debug!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] Attempting fs::rename from {} to {}",
            from.to_string_lossy().yellow(),
            final_destination.to_string_lossy().cyan()
        );

        // Perform the move operation using `fs::rename`.
        match fs::rename(from, &final_destination) {
            Ok(_) => {
                log_debug!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Binary renamed/moved successfully to {}",
                    final_destination.to_string_lossy().green()
                );
                Ok(())
            }
            // Handle the specific error case where `fs::rename` fails due to `CrossesDevices`.
            Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
                log_warn!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Cross-device link detected (error: {}), falling back to copy and remove",
                    e
                );
                log_debug!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Fallback: copying from {} to {}",
                    from.to_string_lossy().yellow(),
                    final_destination.to_string_lossy().cyan()
                );

                fs::copy(from, &final_destination)?;
                log_debug!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Fallback: copy completed successfully"
                );
                log_debug!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Fallback: removing original file at {}",
                    from.to_string_lossy()
                );

                fs::remove_file(from)?;
                log_debug!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Fallback: original file removed from {}",
                    from.to_string_lossy()
                );
                log_debug!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Binary copied and original removed successfully. Final location: {}",
                    final_destination.to_string_lossy().green()
                );
                Ok(())
            }
            Err(e) => {
                log_error!(
                    "[SDB::Tools::{tool_source}::BinaryInstaller] Failed to rename binary from {} to {}: {} (error kind: {:?})",
                    from.to_string_lossy(),
                    final_destination.to_string_lossy(),
                    e,
                    e.kind()
                );
                Err(e)
            }
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
#[cfg(unix)]
pub fn make_executable(path: &Path, tool_entry: &ToolEntry, tool_source: String) -> io::Result<()> {
    log_debug!("[SDB::Tools::{tool_source}::BinaryInstaller] Starting make_executable operation");

    // Get the final file path using the helper function
    let file_path = PathResolver::get_final_file_path(path, tool_entry);

    // Check if the file exists before attempting to modify permissions
    if !file_path.exists() {
        log_error!(
            "[SDB::Tools::{tool_source}::BinaryInstaller] File does not exist: {}",
            file_path.to_string_lossy().red()
        );
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "[SDB::Tools::{tool_source}::BinaryInstaller] File not found: {}",
                file_path.display()
            ),
        ));
    }
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] File exists, proceeding with permission change"
    );

    // Get the current metadata (including permissions) of the file.
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Reading current file metadata for {}",
        file_path.to_string_lossy()
    );
    let metadata = fs::metadata(&file_path)?;
    let mut perms = metadata.permissions();

    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Current permissions mode: {:o}",
        perms.mode()
    );

    // Set the file's permissions to 0o755 (rwxr-xr-x)
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Setting permissions to 0o755 (rwxr-xr-x)"
    );
    perms.set_mode(0o755);

    // Apply the modified permissions back to the file.
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] Applying new permissions to {}",
        file_path.to_string_lossy()
    );
    fs::set_permissions(&file_path, perms)?;

    // Verify the permissions were set correctly
    let updated_perms = fs::metadata(&file_path)?.permissions();
    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] New permissions mode: {:o}",
        updated_perms.mode()
    );

    log_debug!(
        "[SDB::Tools::{tool_source}::BinaryInstaller] File {} is now executable.",
        file_path.to_string_lossy().green()
    );
    Ok(())
}

// Provide a dummy implementation for `make_executable` on non-Unix systems to avoid compilation errors.
// On Windows, executable permissions are often implicit for `.exe` files and not controlled by mode bits.
#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> io::Result<()> {
    log_debug!(
        "[Utils] `make_executable` is a no-op on this non-Unix platform (permissions handled differently)."
    );
    Ok(()) // Return success, as no action is needed or possible on these platforms.
}

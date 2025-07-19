// This module orchestrates the application of shell-specific configurations,
// including raw shell commands, environment variables, and aliases. It acts
// as an intermediary, delegating the actual file modification logic to the
// `shellrc` installer module. The functions here focus on preparing the data,
// logging the process, and managing the overall flow of shell configuration
// application.

use std::fs; // Standard library module for file system operations.
use std::fs::OpenOptions; // Provides options for opening files (e.g., append mode).
use std::path::{Path, PathBuf}; // Provides `Path` and `PathBuf` for working with file paths.
use colored::Colorize; // Imports the `Colorize` trait for adding color to console output.
use crate::{log_debug, log_info, log_warn}; // Custom logging macros for various log levels.
use crate::installers::shellrc; // Imports the `shellrc` module, which contains the core logic for modifying shell RC files.
use crate::schema::ShellConfig; // Imports `ShellConfig` which defines the structure for shell configurations.
use std::io::{BufRead, BufReader, Write}; // Imports for buffered reading, reading lines, and writing to files.


/// Applies shell configurations (raw commands, environment variables, aliases).
///
/// This function serves as the entry point for applying all shell-related configurations.
/// It takes a `ShellConfig` struct, logs its contents (for debugging), and then
/// delegates the actual modification of shell RC files to the `shellrc::apply_shellrc`
/// function. This separation of concerns keeps this module focused on orchestration
/// while `shellrc` handles the low-level file system interactions.
///
/// # Arguments
/// * `shell_cfg`: A `ShellConfig` struct containing all defined shell configurations,
///                including `shellrc` (raw commands/env vars) and `aliases`.
pub fn apply_shell_configs(shell_cfg: ShellConfig) {
    log_info!("[Shell Config] Applying Shell Configurations..."); // Informative log that the shell config process has started.
    log_debug!("Entering apply_shell_configs() function."); // Debug log for function entry.

    // Attempt to pretty-print the `shellrc` part of the config for detailed debug logging.
    match serde_json::to_string_pretty(&shell_cfg.shellrc) {
        Ok(pretty_shellrc) => {
            log_debug!("[Shell Config] Calling shellrc::apply_shellrc with shell config:\n{}", pretty_shellrc);
        },
        Err(e) => {
            // Log a warning if pretty-printing fails, but still proceed with debug log of raw struct.
            log_warn!("[Shell Config] Failed to pretty-print shell config for debug log: {}", e);
            log_debug!("[Shell Config] Calling shellrc::apply_shellrc with shell config: {:#?}", shell_cfg.shellrc);
        }
    }

    // Attempt to pretty-print the `aliases` part of the config for detailed debug logging.
    match serde_json::to_string_pretty(&shell_cfg.aliases) {
        Ok(pretty_aliases) => {
            log_debug!("[Shell Config] And aliases:\n{}", pretty_aliases);
        },
        Err(e) => {
            // Log a warning if pretty-printing fails, but still proceed with debug log of raw struct.
            log_warn!("[Shell Config] Failed to pretty-print aliases for debug log: {}", e);
            log_debug!("[Shell Config] And aliases: {:#?}", shell_cfg.aliases);
        }
    }

    // Delegate the actual application of shell configurations and aliases to the `shellrc` installer.
    shellrc::apply_shellrc(&shell_cfg.shellrc, &shell_cfg.aliases);

    log_debug!("[Shell Config] Shell configuration application phase completed."); // Debug log for completion.
    eprintln!(); // Print a newline for better console formatting.
    log_debug!("Exiting apply_shell_configs() function."); // Debug log for function exit.
}

/// This function checks if a multi-line string `needle_lines` exists
/// sequentially within a vector of `haystack_lines`. It performs a robust comparison
/// by trimming whitespace and checking if the haystack lines *start with* the needle lines.
/// This is useful for verifying if a block of configuration has already been added to an RC file.
///
/// This helps prevent duplicate entries in `.zshrc` or `.bashrc` files if the `devbox`
/// setup is run multiple times.
///
/// # Arguments
/// * `haystack_lines`: A slice (`&[String]`) representing the lines to search within
///                     (e.g., lines read from an RC file).
/// * `needle_lines`: A slice (`&[String]`) representing the sequence of lines to find
///                   (e.g., the configuration block to check for).
///
/// # Returns
/// * `bool`: `true` if the entire `needle_lines` block is found sequentially within `haystack_lines`,
///           `false` otherwise. Returns `true` if `needle_lines` is empty.
pub fn contains_multiline_block(haystack_lines: &[String], needle_lines: &[String]) -> bool {
    // Edge case: An empty block is always considered "contained".
    if needle_lines.is_empty() {
        return true;
    }
    // Optimization: If the haystack is shorter than the needle, it cannot contain it.
    if haystack_lines.len() < needle_lines.len() {
        return false;
    }

    // Iterate through the `haystack_lines`, starting from index `i`.
    // The loop runs up to `haystack_lines.len() - needle_lines.len()` to ensure there are
    // enough remaining lines in the haystack to potentially contain the entire needle.
    for i in 0..=(haystack_lines.len() - needle_lines.len()) {
        let mut match_found = true; // Assume a match until proven otherwise for the current `i`.

        // Check if the sequence of lines starting from `haystack_lines[i]` matches `needle_lines`.
        for j in 0..needle_lines.len() {
            // Trim leading/trailing whitespace from both the haystack line and the needle line
            // for more robust comparison, as RC files might have inconsistent formatting.
            //
            // Using `starts_with` is better than `==` because:
            // 1. There might be slight leading/trailing spaces or comments on the same line
            //    in the RC file that `read_rc_lines` might not perfectly normalize.
            // 2. A line in the haystack might contain the needle line as a prefix, but then
            //    have additional content (e.g., "export PATH=... # My comment").
            if !haystack_lines[i + j].trim().starts_with(needle_lines[j].trim()) {
                match_found = false; // If any line doesn't match, this sequence is not a match.
                break;               // Stop checking this sequence and move to the next `i`.
            }
        }
        if match_found {
            return true; // If the inner loop completed, it means the entire multi-line block was found.
        }
    }
    false // If the outer loop completes, the block was not found anywhere in the haystack.
}

/// Helper function: Appends new lines to the end of the specified RC file.
///
/// This function opens the RC file in append mode. If the file doesn't exist,
/// it will be created. It also adds a clear comment header to denote lines added by `setup-devbox`,
/// making it easy for users to identify and manage these additions.
///
/// # Arguments
/// * `rc_path`: A reference to a `Path` indicating the RC file to append to.
/// * `lines`: A `Vec<String>` containing the new lines to be written to the file.
///
/// # Returns
/// * `std::io::Result<()>`:
///   - `Ok(())` on successful write of all lines.
///   - An `Err` if any I/O error occurs during file opening or writing.
pub fn append_to_rc_file(rc_path: &Path, lines: Vec<String>) -> std::io::Result<()> {
    log_debug!("[ShellRC:append_to_rc_file] Preparing to append {} new lines to RC file: {}", lines.len().to_string().bold(), rc_path.display().to_string().yellow());

    // Open the file with specific options using `OpenOptions::new()`:
    // - `create(true)`: If the file does not exist, create it.
    // - `append(true)`: Open the file in append mode, so new writes are added to the end of the file.
    let mut file = OpenOptions::new()
        .create(true) // Create the file if it doesn't exist.
        .append(true) // Open in append mode.
        .open(rc_path)?; // Attempt to open the file. The `?` operator will propagate any `io::Error`.

    log_debug!("[ShellRC:append_to_rc_file] RC file {} opened in append mode.", rc_path.display());

    // Add a clear comment header before appending new configurations.
    // This makes it easy for users to identify entries added by 'setup-devbox' in their RC file.
    // `writeln!` writes the string followed by a newline.
    writeln!(file, "\n# Added by setup-devbox")?;
    log_debug!("[ShellRC:append_to_rc_file] Added 'Added by setup-devbox' header.");

    // Write each new line from the `lines` vector to the file.
    // Each line is written followed by a newline character to ensure they are on separate lines.
    for (index, line) in lines.iter().enumerate() {
        writeln!(file, "{}", line)?;
        log_debug!("[ShellRC:append_to_rc_file] Appended line {}: '{}'", (index + 1).to_string().dimmed(), line.dimmed());
    }

    log_debug!("[ShellRC:append_to_rc_file] All new lines successfully written to {:?}", rc_path.display());
    Ok(()) // Indicate successful completion of the append operation.
}

/// Helper function: Determines the appropriate shell RC file path for a given shell name.
///
/// This function currently supports `.zshrc` for Zsh and `.bashrc` for Bash.
/// It constructs the full path by joining the user's home directory with the
/// specific RC file name. This is crucial for modifying shell configurations.
/// It relies on the `dirs` crate to get the user's home directory reliably across OS.
///
/// # Arguments
/// * `shell`: A string slice (`&str`) representing the name of the shell (e.g., "zsh", "bash").
///
/// # Returns
/// * `Option<PathBuf>`:
///   - `Some(PathBuf)` containing the full absolute path to the RC file if the shell is supported
///     and the user's home directory can be determined.
///   - `None` if the shell is not supported (not "zsh" or "bash") or if the home directory
///     cannot be found (e.g., `dirs::home_dir()` returns None).
pub fn get_rc_file(shell: &str) -> Option<PathBuf> {
    log_debug!("[ShellRC:get_rc_file] Attempting to find RC file for shell: '{}'", shell.bold());

    // Use `dirs::home_dir()` from the `dirs` crate to reliably get the current user's home directory
    // across different operating systems.
    let home_dir = match dirs::home_dir() {
        Some(path) => path, // If found, assign it to `home_dir`.
        None => {
            // If the home directory cannot be determined, log a warning and return `None`.
            log_warn!("[ShellRC:get_rc_file] Could not determine the user's home directory. Cannot find RC file.");
            return None; // Cannot proceed without the home directory.
        }
    };
    log_debug!("[ShellRC:get_rc_file] User's home directory detected: {:?}", home_dir.display());

    // Match the lowercase version of the shell name to determine the correct RC file name.
    let rc_file_name = match shell.to_lowercase().as_str() {
        "zsh" => ".zshrc", // For Zsh, the configuration file is typically `.zshrc` in the home directory.
        "bash" => ".bashrc", // For Bash, it's typically `.bashrc`.
        _ => {
            // If the provided `shell` name doesn't match any supported type, log a warning
            // and return `None`, as we don't know which RC file to use.
            log_warn!(
                "[ShellRC:get_rc_file] Unsupported shell type '{}'. Currently only 'zsh' and 'bash' are explicitly mapped to RC files.",
                shell.red()
            );
            return None;
        }
    };
    log_debug!("[ShellRC:get_rc_file] RC file name determined: {}", rc_file_name.cyan());

    // Construct the full absolute path to the RC file by joining the home directory and the RC file name.
    let rc_path = home_dir.join(rc_file_name);
    log_debug!("[ShellRC:get_rc_file] Full RC file path: {:?}", rc_path.display());

    Some(rc_path) // Return the constructed path wrapped in `Some` to indicate success.
}

/// Helper function: Reads all non-empty, non-comment lines from a given RC file.
///
/// This function is designed to read the existing content of an RC file efficiently
/// for later comparison (e.g., to check if certain configurations already exist) or
/// for other processing. It gracefully handles cases where the file might not exist
/// or be unreadable, returning an empty vector instead of panicking.
///
/// # Arguments
/// * `rc_path`: A reference to a `Path` indicating the RC file to read.
///
/// # Returns
/// * `Vec<String>`: A vector containing each relevant line read from the file as a `String`.
///                  - Relevant lines are those that are not empty after trimming whitespace
///                    and do not start with '#'.
///                  - Returns an empty vector (`Vec::new()`) if the file doesn't exist,
///                    or if an error occurs during reading (e.g., permission issues).
pub fn read_rc_lines(rc_path: &Path) -> Vec<String> {
    log_debug!("[ShellRC:read_rc_lines] Attempting to read lines from RC file: {}", rc_path.display().to_string().dimmed());
    // First, check if the file actually exists. If not, there are no lines to read,
    // and we can return an empty vector immediately without attempting to open it.
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
                .lines() // Get an iterator over lines. Each item is a `Result<String, io::Error>`.
                .filter_map(Result::ok) // Filter out any lines that resulted in an I/O error (`Err` variants)
                // and unwrap `Ok` variants to get the `String`.
                // Filter out empty lines or lines that are just comments (starting with '#').
                // `.trim()` removes leading/trailing whitespace.
                .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .collect() // Collect all valid, non-empty, non-comment lines into a `Vec<String>`.
        }
        Err(err) => {
            // If the file cannot be opened (e.g., permission issues), log a warning.
            log_warn!(
                "[ShellRC:read_rc_lines] Could not read RC file {:?}: {}. Returning an empty list of lines.",
                rc_path.display().to_string().red(),
                err.to_string().red()
            );
            vec![] // Return an empty vector on error, signifying no lines could be read.
        }
    }
}
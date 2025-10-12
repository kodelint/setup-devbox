use crate::{log_info, log_warn};
use colored::Colorize;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Sources the RC file by executing a shell command to load the file into the current shell session
/// This makes the changes available immediately without requiring a new shell session
///
/// # Arguments
/// * `shell_type` - The type of shell (e.g., "zsh", "bash")
/// * `rc_path` - Path to the RC file to source
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error with details
///
/// # Process
/// 1. Constructs the source command appropriate for the shell
/// 2. Executes the command using the specified shell
/// 3. Checks if the command succeeded
/// 4. Returns appropriate result based on execution outcome
pub fn source_rc_file(shell_type: &str, rc_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Build the source command - this tells the shell to load the RC file
    let source_command = format!("source {}", rc_path.display());

    // Execute the source command using the specified shell
    // The "-c" flag tells the shell to execute the following command string
    let output = std::process::Command::new(shell_type)
        .arg("-c")
        .arg(&source_command)
        .output()?;

    if output.status.success() {
        log_info!(
            "[Shell Config] Successfully sourced {} file",
            shell_type.bold()
        );
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to source RC file: {error_msg}").into())
    }
}

/// Determines the appropriate shell RC file path based on the shell type
/// This function handles the different RC file locations for different shells
///
/// # Arguments
/// * `shell` - The shell type (e.g., "zsh", "bash")
///
/// # Returns
/// * `Option<PathBuf>` - Path to the RC file if shell is supported and home directory exists
///
/// # Supported Shells
/// - "zsh" -> ~/.zshrc
/// - "bash" -> ~/.bashrc
/// - Others -> None (unsupported)
pub fn get_rc_file(shell: &str) -> Option<PathBuf> {
    // Get the user's home directory - returns None if home directory cannot be determined
    let home_dir = dirs::home_dir()?;

    let rc_file_name = match shell.to_lowercase().as_str() {
        "zsh" => ".zshrc",
        "bash" => ".bashrc",
        _ => {
            log_warn!(
                "[Shell Config] Unsupported shell '{}'. Only 'zsh' and 'bash' are supported.",
                shell.red()
            );
            return None;
        }
    };

    Some(home_dir.join(rc_file_name))
}

/// Removes the RC file from the filesystem
/// This is used during full regeneration when we want to start with a clean file
///
/// # Arguments
/// * `rc_path` - Path to the RC file to remove
///
/// # Returns
/// * `Result<(), std::io::Error>` - Success or IO error
///
/// # Safety
/// - Only removes the file if it exists to avoid unnecessary errors
/// - Logs the removal operation for visibility
pub fn remove_rc_file(rc_path: &Path) -> Result<(), std::io::Error> {
    if rc_path.exists() {
        fs::remove_file(rc_path)?;
        log_info!(
            "[Shell Config] Removed the '{}' file",
            rc_path.display().to_string().green()
        );
    }
    Ok(())
}

/// Writes all lines back to the RC file with proper formatting
/// This function handles the final write operation after all processing is complete
///
/// # Arguments
/// * `rc_path` - Path to the RC file to write
/// * `lines` - Slice of strings representing the file content lines
///
/// # Returns
/// * `std::io::Result<()>` - Success or IO error
///
/// # Formatting
/// - Joins all lines with newline characters
/// - Ensures the file ends with a trailing newline for proper shell parsing
/// - Handles empty content gracefully
pub fn write_rc_file(rc_path: &Path, lines: &[String]) -> std::io::Result<()> {
    // Join all lines with newline characters to form the file content
    let content = lines.join("\n");

    // Ensure the file ends with a trailing newline for proper shell parsing
    // Empty files don't need a trailing newline
    let final_content = if content.is_empty() {
        content
    } else {
        format!("{content}\n")
    };

    // Write the final content to the file, overwriting any existing content
    fs::write(rc_path, final_content)
}

/// Reads RC file lines while preserving comments, empty lines, and original formatting
/// This function is careful to maintain the exact content of the original file
///
/// # Arguments
/// * `rc_path` - Path to the RC file to read
///
/// # Returns
/// * `Vec<String>` - Vector of lines from the file, or empty vector if file doesn't exist or can't be read
///
/// # Error Handling
/// - Returns empty vector if file doesn't exist (treated as new file)
/// - Returns empty vector and logs warning if file exists but can't be read
/// - Preserves all original content including comments and empty lines
pub fn read_rc_file(rc_path: &Path) -> Vec<String> {
    // Check if the file exists - if not, return empty vector (treat as new file)
    if !rc_path.exists() {
        return vec![];
    }

    match fs::File::open(rc_path) {
        Ok(file) => BufReader::new(file).lines().map_while(Result::ok).collect(),
        Err(err) => {
            log_warn!(
                "[Shell Config] Could not read RC file {}: {}. Using empty file.",
                rc_path.display().to_string().red(),
                err.to_string().red()
            );
            vec![]
        }
    }
}

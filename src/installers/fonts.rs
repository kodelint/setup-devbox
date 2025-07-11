// This module is exclusively responsible for installing fonts from GitHub releases
// specifically on macOS operating systems. It encapsulates the entire workflow
// from validating font configuration to downloading, extracting, and finally
// copying font files into the user's system font directory.

// Import custom logging macros for different verbosity levels (debug, error, info, warn).
// These macros are defined elsewhere in the crate and allow for consistent logging.
use crate::{log_debug, log_error, log_info, log_warn};
// Import `colored` crate for adding color to terminal output, which enhances
// the readability of log messages for the user.
use colored::Colorize;
// Standard library module for interacting with the operating system's environment variables.
use std::env;
// Standard library module for file system operations, such as creating directories,
// reading directory contents, and copying files.
use std::fs;
// Standard library module for working with file paths in an OS-agnostic manner.
use std::path::PathBuf;

// Import the `FontEntry` schema, which defines the structure for how fonts are
// configured in the `fonts.yaml` file by the user.
use crate::schema::FontEntry;
// Import the `FontState` schema, which defines the structure for how the state
// of installed fonts is recorded in our application's state file.
use crate::schema::FontState;
// Import general utility functions that perform common tasks like downloading files
// and extracting archives, which are reused across various installer modules.
use crate::utils;

/// Installs a font from a GitHub release as specified by the `FontEntry`.
///
/// This function performs the following steps:
/// 1. Validates that the font's source is "GitHub".
/// 2. Extracts necessary repository and tag information from the `FontEntry`.
/// 3. Constructs the precise download URL for the font archive.
/// 4. Downloads the font archive to a temporary directory.
/// 5. Extracts the contents of the archive.
/// 6. Identifies the standard macOS font installation directory (~/Library/Fonts).
/// 7. Iterates through the extracted files, identifying and copying actual font files (.ttf, .otf).
/// 8. Cleans up temporary files and directories.
/// 9. Returns a `FontState` object upon successful installation, providing details
///    about the installed font, or `None` if any critical step fails.
///
/// # Arguments
/// * `font`: A reference to a `FontEntry` struct that holds all the configuration
///           details for the font to be installed (e.g., name, repo, tag).
///
/// # Returns
/// * `Option<FontState>`:
///   - `Some(FontState)` if the installation process was successful and at least one
///     valid font file was copied to the system. The `FontState` contains details
///     like the font's name, version, download URL, and a list of installed files.
///   - `None` if the installation failed at any point, or if no font files were
///     found/copied from the downloaded archive.
pub fn install(font: &FontEntry) -> Option<FontState> {
    log_info!("Initiating font installation for: {}", font.name.bold());
    log_debug!("[Font] Detailed font entry configuration received: {:?}", font);

    // 1. Validate Font Source
    // The current implementation is designed only for fonts sourced from GitHub.
    // If the 'source' field in the FontEntry is not "github", we log an error
    // and terminate the installation attempt for this font.
    if font.source != "github" {
        log_error!(
            "[Font] Installation failed for '{}': Unsupported font source '{}'. Only 'github' is currently supported.",
            font.name.red(),
            font.source.red()
        );
        return None;
    }
    log_debug!("[Font] Font source 'github' is validated and supported, proceeding with installation workflow.");

    // 2. Validate Required GitHub Fields (Repository and Tag)
    // For GitHub-sourced fonts, both the 'repo' (repository owner/name) and 'tag'
    // (release tag, e.g., 'v1.0.0') fields are absolutely essential to locate the release.

    // Attempt to extract the repository string from the FontEntry.
    let repo = match &font.repo {
        Some(r) => r, // If present, use the reference to the repository string.
        None => {
            // If 'repo' is missing, log an error and return None.
            log_error!(
                "[Font] Installation failed for '{}': Missing 'repo' field in font configuration. This is required for GitHub sources.",
                font.name.red()
            );
            return None;
        }
    };
    log_debug!("[Font] GitHub repository specified for font '{}': {}", font.name, repo.blue());

    // Attempt to extract the release tag string from the FontEntry.
    let tag = match &font.tag {
        Some(t) => t, // If present, use the reference to the tag string.
        None => {
            // If 'tag' is missing, log an error and return None.
            log_error!(
                "[Font] Installation failed for '{}': Missing 'tag' field in font configuration. This is required for GitHub sources.",
                font.name.red()
            );
            return None;
        }
    };
    log_debug!("[Font] GitHub release tag specified for font '{}': {}", font.name, tag.blue());

    // 3. Construct Download URL
    // GitHub release assets often follow a common naming convention,
    // typically including the repository, release tag, and an asset filename.
    // We construct the expected asset filename by replacing spaces in the font name
    // (as filenames usually don't contain spaces) and appending the tag and ".zip".
    let asset_name = format!("{}.zip", font.name.replace(' ', "")); // Most fonts are distributed as ZIPs
    // Combine the repository, tag, and asset name into a full GitHub release download URL.
    let url = format!("https://github.com/{}/releases/download/{}/{}", repo, tag, asset_name);
    log_info!("[Font] Attempting to download font archive for '{}' from URL: {}", font.name.bold(), url.blue());
    log_debug!("[Font] Expected GitHub asset filename: {}", asset_name.cyan());

    // --- 4. Download the Font Archive ---
    // First, obtain a path to a temporary directory where we can safely download
    // and extract files without interfering with other system locations.
    let temp_dir = utils::get_temp_dir();
    // Construct the full path where the downloaded ZIP archive will be saved within the temporary directory.
    let archive_path = temp_dir.join(&asset_name);
    log_debug!("[Font] Temporary download path for font archive: {:?}", archive_path.display().to_string().yellow());

    // Use the `utils::download_file` function to perform the actual download.
    // This function handles the network request and saving the file.
    if let Err(err) = utils::download_file(&url, &archive_path) {
        log_error!(
            "[Font] Download failed for font '{}' from {}: {}. Please check URL or network connection.",
            font.name.red(),
            url.red(),
            err
        );
        // Attempt to clean up any partially downloaded archive file to free space.
        let _ = fs::remove_file(&archive_path);
        // Return None as the download failed, indicating installation cannot proceed.
        return None;
    }
    log_info!("[Font] Font archive downloaded successfully to {}", archive_path.display().to_string().green());

    // 5. Extract the Font Archive
    // Detect the file type based on the filename, which is more reliable for archives
    // from known sources like GitHub releases.
    let file_type_from_name = utils::detect_file_type_from_filename(&archive_path.to_string_lossy());
    log_debug!("[Font] Detected downloaded file type (from filename): {}", file_type_from_name.magenta());


    // Extract the contents of the downloaded ZIP archive into another specific
    // subdirectory within the temporary directory. This keeps extracted files organized.
    let extracted_dir = match utils::extract_archive(&archive_path, &temp_dir, Some(&file_type_from_name)) { // <--- PASS KNOWN TYPE HERE
        Ok(path) => path, // If extraction is successful, `path` is the directory where contents were placed.
        Err(err) => {
            // If extraction fails, log the error.
            log_error!(
                "[Font] Failed to extract archive {:?} for font '{}': {}. Archive might be corrupted or in an unsupported format.",
                archive_path.display().to_string().red(),
                font.name.red(),
                err
            );
            // Attempt to clean up the original downloaded archive and the entire temporary directory.
            let _ = fs::remove_file(&archive_path);
            let _ = fs::remove_dir_all(&temp_dir);
            return None; // Return None as extraction failed.
        }
    };
    log_info!("[Font] Font archive extracted to: {}", extracted_dir.display().to_string().green());
    // After successful extraction, the original downloaded archive is no longer needed.
    // Attempt to remove it to free up disk space.
    let _ = fs::remove_file(&archive_path);

    // 6. Determine macOS Fonts Directory
    // On macOS, user-specific fonts are conventionally installed in the
    // `~/Library/Fonts` directory. We need to derive this path reliably.

    // Use `dirs::home_dir()` from the `dirs` crate (a cross-platform way to get common directories)
    // to get the user's home directory.
    let fonts_dir = dirs::home_dir()
        // If `dirs::home_dir()` succeeds, append "Library/Fonts" to it.
        .map(|path| path.join("Library/Fonts"))
        // If `dirs::home_dir()` fails (unlikely on macOS), fallback to checking the `HOME`
        // environment variable, which is a common convention.
        .or_else(|| {
            env::var("HOME")
                .ok() // Convert `Result` to `Option` for chaining.
                .map(|home| PathBuf::from(home).join("Library/Fonts"))
        });

    let fonts_dir = match fonts_dir {
        Some(path) => path, // If a valid fonts directory path was determined, use it.
        None => {
            // If no fonts directory could be determined by any method, log an error.
            log_error!(
                "[Font] Could not determine the user's Home directory or standard Fonts directory. Cannot install font '{}'.",
                font.name.red()
            );
            let _ = fs::remove_dir_all(&temp_dir); // Clean up the temporary directory.
            return None; // Return None as the target path is unknown.
        }
    };

    // Ensure that the target fonts directory actually exists on the filesystem.
    // If it doesn't, create all necessary parent directories.
    if let Err(err) = fs::create_dir_all(&fonts_dir) {
        log_error!(
            "[Font] Failed to create target fonts directory {:?}: {}. This directory is required for installation.",
            fonts_dir.display().to_string().red(),
            err
        );
        let _ = fs::remove_dir_all(&temp_dir); // Clean up the temporary directory.
        return None; // Return None as we cannot install without the target directory.
    }
    log_debug!("[Font] Target fonts directory for installation set to: {:?}", fonts_dir.display().to_string().blue());

    // 7. Copy Font Files to Fonts Directory
    // Initialize a vector to store the paths of all successfully installed font files.
    // This list will be part of the `FontState` if the installation is successful.
    let mut installed_font_files: Vec<String> = Vec::new();

    // Attempt to read the contents of the extracted directory.
    let read_dir_result = fs::read_dir(&extracted_dir);
    if read_dir_result.is_err() {
        // If reading the directory fails, it means we can't find any font files.
        log_warn!(
            "[Font] Failed to read extracted directory {:?}. It might be empty or corrupted. Error: {}",
            extracted_dir.display().to_string().yellow(),
            read_dir_result.unwrap_err() // Safely unwrap the error for logging.
        );
        let _ = fs::remove_dir_all(&temp_dir); // Clean up the temporary directory.
        // Return None because no font files could be processed.
        return None;
    }

    // Iterate through each entry (file or subdirectory) found within the extracted archive's directory.
    for entry in read_dir_result.unwrap() {
        // Safely unwrap each directory entry. If an entry is invalid, skip it with a warning.
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log_warn!(
                    "[Font] Skipping invalid directory entry encountered while processing extracted fonts: {}",
                    e
                );
                continue; // Move to the next entry.
            }
        };

        let path = entry.path(); // Get the full path of the current entry.
        log_debug!("[Font] Examining extracted path: {:?}", path.display().to_string().dimmed());

        // Check if the current path points to a regular file. We are only interested in files.
        if path.is_file() {
            // Attempt to get the file extension.
            if let Some(ext) = path.extension() {
                // Convert the extension to a lowercase string for case-insensitive comparison.
                let ext_str = ext.display().to_string().to_lowercase();

                // Check if the file extension is a common font file type (.ttf or .otf).
                if ext_str == "ttf" || ext_str == "otf" {
                    // If it's a font file, extract its filename.
                    let file_name = match path.file_name() {
                        Some(name) => name,
                        None => {
                            log_warn!("[Font] Skipping font file with no valid filename detected: {:?}", path.display().to_string().yellow());
                            continue; // Skip this file if its name cannot be determined.
                        }
                    };
                    // Construct the full target path in the user's Fonts directory.
                    let target_path = fonts_dir.join(file_name);
                    log_debug!("[Font] Attempting to copy font file from {:?} to {:?}", path.display().to_string().yellow(), target_path.display().to_string().yellow());

                    // Perform the file copy operation.
                    if let Err(err) = fs::copy(&path, &target_path) {
                        // If copying fails for a specific font file, log a warning but continue
                        // processing other files, as one failed copy shouldn't halt the whole process.
                        log_warn!(
                            "[Font] Failed to copy font file {:?} to {:?}: {}",
                            path.display().to_string().yellow(),
                            target_path.display().to_string().yellow(),
                            err
                        );
                    } else {
                        // If the copy is successful, log a debug message and add the target path
                        // to our list of successfully installed font files.
                        log_debug!("[Font] Successfully copied font file: {:?}", target_path.display().to_string().green());
                        installed_font_files.push(target_path.display().to_string().to_string());
                    }
                } else {
                    log_debug!("[Font] Skipping non-font file based on extension: {:?} (extension: {})", path.display().to_string().dimmed(), ext_str.dimmed());
                }
            } else {
                log_debug!("[Font] Skipping file with no detected extension: {:?}", path.display().to_string().dimmed());
            }
        } else if path.is_dir() {
            log_debug!("[Font] Skipping directory entry: {:?}", path.display().to_string().dimmed());
            // This implementation does not recursively search subdirectories for fonts.
            // All font files are expected to be directly within the extracted root.
        }
    }

    // 8. Final Status and Cleanup
    // Provide a summary of the font installation attempt.
    if !installed_font_files.is_empty() {
        log_info!(
            "[Font] Installation of font '{}' completed. Successfully copied {} font files.",
            font.name.green(),
            installed_font_files.len().to_string().cyan()
        );
    } else {
        // If no font files were found or successfully copied, issue a warning.
        log_warn!(
            "[Font] No .ttf or .otf font files were found or successfully copied from the archive for '{}'. \
             The font might not be correctly installed or the archive format is unexpected.",
            font.name.yellow()
        );
    }

    // Clean up the temporary directory and all its contents (downloaded archive, extracted files).
    // Errors during cleanup are generally not critical to the main installation outcome.
    if let Err(err) = fs::remove_dir_all(&temp_dir) {
        log_warn!("[Font] Failed to clean up temporary directory {:?}: {}", temp_dir.display().to_string().yellow(), err);
    } else {
        log_debug!("[Font] Temporary directory {:?} successfully cleaned up.", temp_dir.display());
    }

    // 9. Return FontState for Tracking (Conditional)
    // A `FontState` object is returned only if at least one font file was successfully copied.
    // This ensures that the state file accurately reflects truly installed fonts.
    if !installed_font_files.is_empty() {
        log_debug!("[Font] Constructing FontState for '{}'.", font.name.bold());
        Some(FontState {
            name: font.name.clone(),
            // Prioritize the `version` field from `FontEntry`. If not present, fall back to the `tag`.
            // If neither is present (should be caught by earlier validation for 'tag'), use "unknown".
            version: font.version.clone().unwrap_or_else(|| font.tag.clone().unwrap_or_else(|| "unknown".to_string())),
            url, // Store the exact URL from which the font archive was downloaded.
            files: installed_font_files, // Store the list of absolute paths to the installed font files.
        })
    } else {
        // If no font files were successfully installed (even if the archive was downloaded/extracted),
        // return `None` to indicate that the font is not considered "installed" by devbox's tracking.
        log_debug!("[Font] No font files were installed for '{}', so no FontState will be recorded.", font.name.bold());
        None
    }
}
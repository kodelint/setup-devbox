// This module is exclusively responsible for installing fonts, primarily from GitHub releases.
// It encapsulates the entire workflow from validating font configuration to downloading,
// extracting, and finally copying font files into the user's system font directory.
//
// The primary goal is to provide a robust and automated mechanism for `setup-devbox`
// to manage font installations, particularly for developer-centric fonts
// often distributed via GitHub.

// Standard library imports:
use std::env; // For interacting with environment variables, like $HOME.
use std::fs; // For file system operations (creating directories, copying files, reading directories).
use std::io;
use std::path::{Path, PathBuf}; // For ergonomic and platform-agnostic path manipulation. // For core input/output functionalities and error types.

// External crate imports:
use colored::Colorize; // Used for adding color to terminal output, enhancing readability of logs.

// Internal module imports:
// Custom logging macros for different verbosity levels. These provide consistent
// and colored output throughout the application, aiding in debugging and user feedback.
use crate::{log_debug, log_error, log_info, log_warn};

// Internal module imports:
// `ToolEntry`: Represents a single tool's configuration as defined in your `tools.yaml` file.
//              It's a struct that contains all possible configuration fields for a tool,
//              such as name, version, source, URL, repository, etc.
// `ToolState`: Represents the actual state of an *installed* tool. This struct is used to
//              persist information about installed tools in the application's `state.json` file.
//              It helps `setup-devbox` track what's installed, its version, and where it's located.
use crate::schemas::fonts::FontEntry;
use crate::schemas::state_file::FontState;
// Utility functions from other parts of the crate:
use crate::libs::utilities::assets::download_file; // For downloading files from URLs.
use crate::libs::utilities::compression::extract_archive;
use crate::schemas::path_resolver::PathResolver;

/// Helper struct to hold validated font entry details, reducing redundancy.
#[allow(dead_code)]
struct ValidatedFontDetails {
    repo: String,
    tag: String,
    asset_name: String,
    url: String,
}

/// Helper function to validate a font entry and derive download details.
///
/// Ensures that the font entry specifies a GitHub source and has the necessary
/// `repo` and `tag` fields. Constructs the expected GitHub release asset URL.
///
/// # Arguments
/// * `font`: A reference to the `FontEntry` struct.
///
/// # Returns
/// An `Option<ValidatedFontDetails>`:
/// * `Some(ValidatedFontDetails)` on successful validation and URL construction.
/// * `None` if validation fails (e.g., missing required fields, unsupported source),
///   with an error message logged.
fn validate_font_entry(font: &FontEntry) -> Option<ValidatedFontDetails> {
    log_debug!("[Font Validation] Validating font entry: {}", font.name);

    if font.source.to_lowercase() != "github" {
        log_error!(
            "[Font Validation] Tool '{}' has unsupported font source: '{}'. Only 'github' is supported for now. Skipping.",
            font.name.bold().red(),
            font.source
        );
        return None;
    }

    let Some(repo) = &font.repo else {
        log_error!(
            "[Font Validation] Font '{}' with 'github' source is missing 'repo' field. Skipping.",
            font.name.bold().red()
        );
        return None;
    };

    let Some(tag) = &font.tag else {
        log_error!(
            "[Font Validation] Font '{}' with 'github' source is missing 'tag' field. Skipping.",
            font.name.bold().red()
        );
        return None;
    };

    // Construct the expected asset name. This is a heuristic based on common font naming conventions.
    // Example: For Fira Code v6.2, asset_name might be "FiraCode.zip".
    // This might need refinement if font releases use inconsistent naming.
    let asset_name = format!("{}.zip", font.name.replace(' ', "")); // Remove spaces for URL-friendly name

    // Construct the GitHub release download URL.
    // Example: https://github.com/ryanoasis/nerd-fonts/releases/download/v3.0.2/FiraCode.zip
    let url = format!("https://github.com/{repo}/releases/download/{tag}/{asset_name}");

    log_debug!("[Font Validation] Constructed download URL: {}", url.cyan());

    Some(ValidatedFontDetails {
        repo: repo.clone(),
        tag: tag.clone(),
        asset_name,
        url,
    })
}

// /// Determines the correct font installation directory for the current operating system.
// ///
// /// For macOS, this is `~/Library/Fonts`. This function also ensures the directory exists,
// /// creating it if necessary.
// ///
// /// # Returns
// /// A `Result` containing the `PathBuf` to the installation directory on success.
// /// Returns `Err(io::Error)` if the home directory cannot be found or directory creation fails,
// /// or if the operating system is not supported.
// #[cfg(target_os = "macos")]
// pub fn get_font_installation_dir() -> io::Result<PathBuf> {
//     log_debug!("[Font Paths] Attempting to get macOS font installation directory.");
//     let Some(home_dir) = dirs::home_dir() else {
//         log_error!(
//             "[Font Paths] Could not determine home directory. Cannot proceed with font installation."
//         );
//         return Err(io::Error::new(
//             io::ErrorKind::NotFound,
//             "Home directory not found",
//         ));
//     };
//
//     let font_dir = home_dir.join("Library").join("Fonts");
//
//     // Ensure the directory exists.
//     fs::create_dir_all(&font_dir).map_err(|e| {
//         log_error!(
//             "[Font Paths] Failed to create font installation directory '{}': {}",
//             font_dir.display(),
//             e.to_string().red()
//         );
//         e // Propagate the io::Error
//     })?;
//
//     log_debug!(
//         "[Font Paths] macOS font installation directory: {}",
//         font_dir.display()
//     );
//     Ok(font_dir)
// }

// Placeholder for other operating systems.
// This will return an error if compiled on a non-macOS system.
#[cfg(not(target_os = "macos"))]
fn get_font_installation_dir() -> io::Result<PathBuf> {
    log_error!(
        "[Font Paths] Font installation is not supported on this operating system. Currently only macOS is supported."
    );
    Err(io::Error::new(
        io::ErrorKind::Other,
        "Unsupported operating system for font installation",
    ))
}

/// Downloads the font archive from the given URL to a temporary directory.
///
/// # Arguments
/// * `font_name`: The name of the font (for logging).
/// * `url`: The URL to download the font archive from.
/// * `temp_dir`: The directory where the archive should be saved temporarily.
/// * `filename`: The expected filename of the downloaded archive.
///
/// # Returns
/// A `Result` containing the `PathBuf` to the downloaded archive on success.
/// Returns `Err(Box<dyn std::error::Error + Send + Sync>)` if download fails,
/// wrapping the specific error from `download_file`.
fn download_font_archive(
    font_name: &str,
    url: &str,
    temp_dir: &Path,
    filename: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let temp_download_path = temp_dir.join(filename);

    log_info!(
        "[Font Download] Downloading '{}' from {} to {}...",
        font_name.bold(),
        url.cyan(),
        temp_download_path.display()
    );

    // Use the centralized `download_file` utility.
    download_file(url, &temp_download_path)?; // `?` propagates the Box<dyn Error>

    log_debug!(
        "[Font Download] Download complete for '{}'.",
        font_name.bold()
    );
    Ok(temp_download_path)
}

/// Extracts the downloaded font archive to a temporary directory.
///
/// # Arguments
/// * `font_name`: The name of the font (for logging).
/// * `archive_path`: The path to the downloaded font archive.
/// * `extract_to_dir`: The directory where the archive contents should be extracted.
///
/// # Returns
/// A `Result` containing the `PathBuf` to the directory where contents were extracted
/// (e.g., `temp_dir/extracted_contents`) on success, or an `io::Error` on failure.
fn extract_font_archive(
    font_name: &str,
    archive_path: &Path,
    extract_to_dir: &Path,
) -> io::Result<PathBuf> {
    log_info!(
        "[Font Extraction] Extracting archive for '{}' from {}...",
        font_name.bold(),
        archive_path.display()
    );

    // `extract_archive` is designed to return the path to the extracted contents.
    // This handles various archive types (zip, tar.gz, etc.)
    let extracted_path = extract_archive(archive_path, extract_to_dir, None).map_err(|e| {
        log_error!(
            "[Font Extraction] Failed to extract archive for '{}' from '{}': {}",
            font_name.red(),
            archive_path.display().to_string().red(),
            e.to_string().red()
        );
        e // Propagate the io::Error
    })?;

    log_debug!(
        "[Font Extraction] Archive extracted to: {}",
        extracted_path.display()
    );

    // Clean up the original downloaded archive file after successful extraction.
    if archive_path.is_file() {
        if let Err(e) = fs::remove_file(archive_path) {
            log_warn!(
                "[Font Cleanup] Failed to remove temporary font archive '{}': {}",
                archive_path.display().to_string().yellow(),
                e.to_string().yellow()
            );
        } else {
            log_debug!(
                "[Font Cleanup] Removed temporary font archive '{}'.",
                archive_path.display()
            );
        }
    }

    Ok(extracted_path)
}

/// Copies actual font files (.ttf, .otf) from the extracted directory to the
/// system's font installation directory, applying filters if specified.
///
/// # Arguments
/// * `font_name`: The name of the font (for logging).
/// * `extracted_dir`: The directory where the font archive was extracted.
/// * `install_dir`: The final system font installation directory (e.g., `~/Library/Fonts`).
/// * `install_only`: An optional `Vec<String>` containing substrings to filter font files.
///
/// # Returns
/// A `Result` containing a `Vec<String>` of successfully installed font file names on success,
/// or an `io::Error` on file copy failures.
fn copy_non_hidden_font_files(
    font_name: &str,
    extracted_dir: &Path,
    install_dir: &Path,
    install_only: &Option<Vec<String>>,
) -> io::Result<Vec<String>> {
    let _ = font_name;
    log_debug!(
        "[Font Copy] Copying font files from '{}' to '{}'.",
        extracted_dir.display(),
        install_dir.display()
    );

    let mut installed_font_files: Vec<String> = Vec::new();

    // Recursively read directories to find font files (handles nested archives).
    // This is an improvement to ensure all fonts within subdirectories are found.
    for entry_result in walkdir::WalkDir::new(extracted_dir) {
        let entry = entry_result?;
        let path = entry.path();
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Skip hidden files/directories (starting with '.') and non-files.
        if filename.starts_with('.') || (!path.is_file()) {
            log_debug!(
                "[Font Copy] Skipping hidden or non-file entry: {}",
                filename.blue()
            );
            continue;
        }

        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let is_font_file = matches!(extension.to_lowercase().as_str(), "ttf" | "otf");

        if !is_font_file {
            log_debug!(
                "[Font Copy] Skipping non-font file (unsupported extension '{}'): {}",
                extension.blue(),
                filename.blue()
            );
            continue;
        }

        // Apply `install_only` filter if specified.
        if let Some(filters) = install_only {
            let lower_filename = filename.to_lowercase(); // Convert filename to lowercase once

            if !filters
                .iter()
                .any(|filter| lower_filename.contains(&filter.to_lowercase()))
            {
                // Compare against lowercase filter
                log_debug!(
                    "[Font Copy] Skipping font file '{}' as it does not match 'install_only' filters ({:#?}).",
                    filename.blue(),
                    filters
                );
                continue;
            }
        }

        let destination_path = install_dir.join(filename);
        log_info!(
            "[Font Copy] Copying font '{}' to '{}'.",
            filename.bold(),
            destination_path.display()
        );

        fs::copy(&path, &destination_path).inspect_err(|e| {
            log_error!(
                "[Font Copy] Failed to copy font file '{}' to '{}': {}",
                path.display().to_string().red(),
                destination_path.display().to_string().red(),
                e.to_string().red()
            );
        })?;

        installed_font_files.push(filename.to_string());
        log_debug!("[Font Copy] Successfully copied: {}", filename.green());
    }

    Ok(installed_font_files)
}

/// Helper to derive the version string for `FontState`.
/// Prefers `version` from `FontEntry`, then `tag`, then "unknown".
fn determine_font_version(font: &FontEntry) -> String {
    font.version
        .clone()
        .unwrap_or_else(|| font.tag.clone().unwrap_or_else(|| "unknown".to_string()))
}

/// Installs a font based on the provided `FontEntry` configuration.
///
/// This is the main public function for this module. It orchestrates the entire
/// font installation process: validation, directory setup, download, extraction,
/// file copying, and state recording.
///
/// # Arguments
/// * `font`: A reference to the `FontEntry` struct containing the font's configuration.
///
/// # Returns
/// An `Option<FontState>`:
/// * `Some(FontState)` if at least one font file was successfully installed and recorded.
/// * `None` if the font installation process failed for any reason (e.g., validation error,
///   download failure, no font files found/copied). Error details are logged internally.
pub fn install(font: &FontEntry) -> Option<FontState> {
    log_info!(
        "[Font Installer] Starting installation for font: {}",
        font.name.bold()
    );

    // 1. Validate the font entry and get download details.
    let font_details = validate_font_entry(font)?; // Returns None if validation fails

    // 2. Determine installation paths (temporary and final).
    let font_install_dir = match PathResolver::get_font_installation_dir() {
        Ok(dir) => dir,
        Err(_) => return None, // Already logged error, just abort
    };

    let temp_dir =
        env::temp_dir().join(format!("setup-devbox-font-{}", font.name.replace(' ', "-")));
    let temp_dir_clone_for_cleanup = temp_dir.clone(); // Clone for deferred cleanup

    // Ensure the temporary directory exists.
    if let Err(e) = fs::create_dir_all(&temp_dir) {
        log_error!(
            "[Font Installer] Failed to create temporary directory '{}': {}",
            temp_dir.display().to_string().red(),
            e.to_string().red()
        );
        cleanup_temp_dir(&temp_dir_clone_for_cleanup);
        return None;
    }

    // 3. Download the font archive.
    let downloaded_archive_path = match download_font_archive(
        &font.name,
        &font_details.url,
        &temp_dir,
        &font_details.asset_name,
    ) {
        Ok(path) => path,
        Err(e) => {
            log_error!(
                "[Font Installer] Failed to download font archive for '{}': {}",
                font.name.bold().red(),
                e.to_string().red()
            );
            cleanup_temp_dir(&temp_dir_clone_for_cleanup);
            return None;
        }
    };

    // 4. Extract the font archive.
    let extracted_contents_dir = match extract_font_archive(
        &font.name,
        &downloaded_archive_path,
        &temp_dir, // Extract into the temp directory
    ) {
        Ok(path) => path,
        Err(e) => {
            log_error!(
                "[Font Installer] Failed to extract font archive for '{}': {}",
                font.name.bold().red(),
                e.to_string().red()
            );
            cleanup_temp_dir(&temp_dir_clone_for_cleanup);
            return None;
        }
    };

    // 5. Copy font files from extracted contents to the final installation directory.
    let installed_font_files = match copy_non_hidden_font_files(
        &font.name,
        &extracted_contents_dir,
        &font_install_dir,
        &font.install_only,
    ) {
        Ok(files) => files,
        Err(e) => {
            log_error!(
                "[Font Installer] Failed to copy font files for '{}': {}",
                font.name.bold().red(),
                e.to_string().red()
            );
            cleanup_temp_dir(&temp_dir_clone_for_cleanup);
            return None;
        }
    };

    // 6. Clean up the main temporary directory.
    cleanup_temp_dir(&temp_dir_clone_for_cleanup);

    // 7. Return FontState for Tracking (Conditional):
    // Only record the font's state if at least one file was successfully installed.
    if !installed_font_files.is_empty() {
        log_debug!(
            "[Font Installer] Constructing FontState for '{}'.",
            font.name.bold()
        );
        Some(FontState {
            name: font.name.clone(),
            version: determine_font_version(font),
            url: font_details.url,
            install_method: font.source.clone(),
            repo: font.repo.clone(),
            tag: font.tag.clone(),
            files: installed_font_files, // List of all successfully installed font files.
            install_only: font.install_only.clone(),
        })
    } else {
        // If no files were installed, return `None` to indicate no state should be recorded,
        // but the process itself wasn't a critical failure. Log a warning.
        log_warn!(
            "[Font Installer] No .ttf or .otf font files were found or successfully copied from the archive for '{}'. \\\
             The font might not be correctly installed, the archive format is unexpected, or no files matched the 'install_only' filter.",
            font.name.yellow()
        );
        None
    }
}

/// Helper function to clean up a temporary directory.
///
/// This is called regardless of the success or failure of the main installation,
/// ensuring that temporary files are removed.
///
/// # Arguments
/// * `temp_dir`: The path to the temporary directory to remove.
fn cleanup_temp_dir(temp_dir: &Path) {
    if temp_dir.exists() {
        if let Err(e) = fs::remove_dir_all(temp_dir) {
            log_warn!(
                "[Font Cleanup] Failed to remove temporary directory '{}': {}",
                temp_dir.display().to_string().yellow(),
                e.to_string().yellow()
            );
        } else {
            log_debug!(
                "[Font Cleanup] Removed temporary directory '{}'.",
                temp_dir.display()
            );
        }
    }
}

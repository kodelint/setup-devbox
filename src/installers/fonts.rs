// This module is exclusively responsible for installing fonts from GitHub releases,
// with a specific focus on macOS operating systems. It encapsulates the entire workflow
// from validating font configuration to downloading, extracting, and finally
// copying font files into the user's system font directory.
//
// The primary goal is to provide a robust and automated mechanism for `devbox`
// to manage font installations, particularly for developer-centric fonts
// often distributed via GitHub.


// Standard library imports:
//
// For interacting with environment variables, like $HOME.
use std::env;
// For ergonomic and platform-agnostic path manipulation.
use std::path::PathBuf;
// For file system operations (creating directories, copying files, reading directories).
use std::fs;

// External crate imports:
// Used for adding color to terminal output, enhancing readability of logs.
use colored::Colorize;

// Internal module imports:
// Custom logging macros for different verbosity levels. These provide consistent
// and colored output throughout the application, aiding in debugging and user feedback.
use crate::{log_debug, log_error, log_info, log_warn};

// `FontEntry`: Defines the structure for how fonts are configured in our `fonts.yaml` file.
// It contains metadata like font name, GitHub repository, release tag, and optional filters.
use crate::schema::FontEntry;

// `FontState`: Defines the structure for how the state of successfully installed fonts
// is recorded in our application's internal state file (`state.json`). This allows `devbox`
// to track installed fonts, their versions, and paths.
use crate::schema::FontState;

// Provides various utility functions from libs/utilities:
// `download_file`: Handles downloading a file from a URL to a local path.
// `extract_archive`: Decompresses and extracts various archive formats (zip, tar.gz, etc.).
// `detect_os`, `detect_architecture`: Determine the current operating system and CPU architecture.
// `get_temp_dir`: Provides a temporary directory for downloads and extractions.
// `detect_file_type`, `detect_file_type_from_filename`: Utilities to infer file types.
use crate::libs::utilities::{
    assets::{
        detect_file_type_from_filename,
        download_file
    }
    ,
    compression::extract_archive,
    path_helpers::get_temp_dir
};

/// Helper struct to encapsulate validated font entry details.
/// This improves function signature readability and ensures all necessary
/// data is present before proceeding with download and installation.
struct ValidatedFontDetails<'a> {
    repo: &'a str,       // Reference to the GitHub repository string.
    tag: &'a str,        // Reference to the GitHub release tag string.
    asset_name: String,  // Constructed name of the expected font asset (e.g., "FiraCode.zip").
    url: String,         // Fully constructed download URL for the font asset.
}

/// Validates the `FontEntry` configuration.
///
/// This function performs initial checks to ensure the font's source is supported (`github`)
/// and that essential fields like `repo` and `tag` are present for GitHub-based installations.
/// It also constructs the expected GitHub release asset URL.
///
/// # Arguments
/// * `font`: A reference to the `FontEntry` struct containing the font's configuration.
///
/// # Returns
/// * `Result<ValidatedFontDetails, String>`:
///   - `Ok(ValidatedFontDetails)` if all validations pass. Contains the extracted `repo`, `tag`,
///     the derived `asset_name`, and the full `url` for download.
///   - `Err(String)` if any validation fails, with a descriptive error message indicating the problem.
fn validate_font_entry(font: &FontEntry) -> Result<ValidatedFontDetails, String> {
    log_debug!("[Font:Validation] Starting validation for font: {}", font.name.bold());

    // Validate Font Source: Currently, only "github" is supported for font installations.
    // This check ensures we don't attempt to install fonts from unsupported sources.
    if font.source != "github" {
        return Err(format!(
            "Installation failed for '{}': Unsupported font source '{}'. Only 'github' is currently supported.",
            font.name.red(),
            font.source.red()
        ));
    }
    log_debug!("[Font:Validation] Font source 'github' is validated.");

    // Validate Required GitHub Fields: `repo` and `tag` are mandatory for constructing
    // the GitHub release download URL. The `ok_or_else` combinator provides a custom
    // error message if the field is `None`.
    let repo = font.repo.as_ref().ok_or_else(|| {
        format!(
            "Installation failed for '{}': Missing 'repo' field in font configuration. This is required for GitHub sources.",
            font.name.red()
        )
    })?;
    log_debug!("[Font:Validation] GitHub repository specified: {}", repo.blue());

    let tag = font.tag.as_ref().ok_or_else(|| {
        format!(
            "Installation failed for '{}': Missing 'tag' field in font configuration. This is required for GitHub sources.",
            font.name.red()
        )
    })?;
    log_debug!("[Font:Validation] GitHub release tag specified: {}", tag.blue());

    // Construct Download URL: GitHub font releases are typically distributed as `.zip` files
    // named after the font (with spaces removed). This assumes a common pattern for font asset names.
    let asset_name = format!("{}.zip", font.name.replace(' ', ""));
    let url = format!("https://github.com/{}/releases/download/{}/{}", repo, tag, asset_name);
    log_debug!("[Font:Validation] Constructed download URL: {}", url.blue());

    // If all validations pass, return the encapsulated details.
    Ok(ValidatedFontDetails {
        repo,
        tag,
        asset_name,
        url,
    })
}

/// Downloads the font archive to a temporary directory.
///
/// This function utilizes the shared `libutils::download_file` to fetch the font archive.
/// It also handles potential cleanup of partially downloaded files in case of errors.
///
/// # Arguments
/// * `font_name`: The name of the font being downloaded, used for logging.
/// * `url`: The full URL to the font archive on GitHub.
/// * `asset_name`: The expected filename of the downloaded asset, used to construct the local path.
///
/// # Returns
/// * `Result<PathBuf, String>`:
///   - `Ok(PathBuf)` if the download is successful, containing the absolute path to the downloaded archive.
///   - `Err(String)` if the download fails (e.g., network error, invalid URL), with an error message.
fn download_font_archive(font_name: &str, url: &str, asset_name: &str) -> Result<PathBuf, String> {
    log_info!("[Font:Download] Attempting to download font archive for '{}' from URL: {}", font_name.bold(), url.blue());

    // Obtain a path to a temporary directory for downloads.
    let temp_dir = get_temp_dir();
    // Construct the full path where the downloaded archive will be saved.
    let archive_path = temp_dir.join(asset_name);
    log_debug!("[Font:Download] Temporary download path for font archive: {:?}", archive_path.display().to_string().yellow());

    // Perform the file download.
    if let Err(err) = download_file(url, &archive_path) {
        // If download fails, attempt to remove any partially downloaded file to prevent stale data.
        let _ = fs::remove_file(&archive_path);
        return Err(format!(
            "Download failed for font '{}' from {}: {}. Please check URL or network connection.",
            font_name.red(),
            url.red(),
            err
        ));
    }
    log_info!("[Font:Download] Font archive downloaded successfully to {}", archive_path.display().to_string().green());
    Ok(archive_path)
}

/// Extracts the contents of the font archive.
///
/// This function uses the `libutils::extract_archive` helper to decompress the downloaded file.
/// It passes the file type hint derived from the filename, which helps `extract_archive`
/// choose the correct decompression method. It also cleans up the original archive after extraction.
///
/// # Arguments
/// * `font_name`: The name of the font, for logging purposes.
/// * `archive_path`: The `PathBuf` to the downloaded font archive.
/// * `temp_dir`: The base temporary directory where extraction should occur.
///
/// # Returns
/// * `Result<PathBuf, String>`:
///   - `Ok(PathBuf)` if extraction is successful, containing the path to the directory
///     where the font files were extracted.
///   - `Err(String)` if extraction fails (e.g., corrupted archive, unsupported format),
///     with an error message.
fn extract_font_archive(font_name: &str, archive_path: &PathBuf, temp_dir: &PathBuf) -> Result<PathBuf, String> {
    log_debug!("[Font:Extract] Detecting file type from filename for extraction.");
    // Determine the file type using filename extension. This is more reliable for archives
    // than content-based detection (`file` command) when needing to pick an extractor.
    let file_type_from_name = detect_file_type_from_filename(&archive_path.to_string_lossy());
    log_debug!("[Font:Extract] Detected downloaded file type (from filename): {}", file_type_from_name.magenta());

    // Perform the archive extraction. The `Some(&file_type_from_name)` hint ensures
    // `extract_archive` knows what kind of archive it's dealing with.
    let extracted_dir = match extract_archive(archive_path, temp_dir, Some(&file_type_from_name)) {
        Ok(path) => path,
        Err(err) => {
            // Clean up the original archive file if extraction fails.
            let _ = fs::remove_file(archive_path);
            return Err(format!(
                "Failed to extract archive {:?} for font '{}': {}. Archive might be corrupted or in an unsupported format.",
                archive_path.display().to_string().red(),
                font_name.red(),
                err
            ));
        }
    };
    log_info!("[Font:Extract] Font archive extracted to: {}", extracted_dir.display().to_string().green());
    // Once extracted, the original archive is no longer needed; clean it up.
    let _ = fs::remove_file(archive_path);
    Ok(extracted_dir)
}

/// Determines and creates the macOS Fonts directory.
///
/// This function attempts to find the user's standard Fonts directory (`~/Library/Fonts`)
/// and ensures it exists. This is critical for macOS-specific font installations.
///
/// # Arguments
/// * `font_name`: The name of the font, used in error messages.
///
/// # Returns
/// * `Result<PathBuf, String>`:
///   - `Ok(PathBuf)` if the fonts directory is successfully determined and created (if needed).
///   - `Err(String)` if the user's home directory cannot be found or if the fonts directory
///     cannot be created due to permissions or other file system issues.
fn get_font_installation_dir(font_name: &str) -> Result<PathBuf, String> {
    // Attempt to get the home directory using `dirs::home_dir` (from the `dirs` crate, implicitly used)
    // or by falling back to the `HOME` environment variable.
    let fonts_dir = dirs::home_dir()
        .map(|path| path.join("Library/Fonts")) // Append "Library/Fonts" to the home path.
        .or_else(|| {
            // Fallback: if `dirs::home_dir` fails, try `std::env::var("HOME")`.
            env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join("Library/Fonts"))
        })
        .ok_or_else(|| {
            // If neither method works, we cannot proceed.
            format!(
                "Could not determine the user's Home directory or standard Fonts directory. Cannot install font '{}'.",
                font_name.red()
            )
        })?;

    // Ensure the target fonts directory exists, creating it and any necessary parent directories.
    if let Err(err) = fs::create_dir_all(&fonts_dir) {
        return Err(format!(
            "Failed to create target fonts directory {:?}: {}. This directory is required for installation.",
            fonts_dir.display().to_string().red(),
            err
        ));
    }
    log_debug!("[Font:TargetDir] Target fonts directory for installation set to: {:?}", fonts_dir.display().to_string().blue());
    Ok(fonts_dir)
}

/// Copies font files from the extracted directory to the target font directory.
///
/// This function iterates through the extracted contents, identifies actual font files
/// (.ttf and .otf), and copies them to the system font directory. It also applies
/// the optional `install_only` filter, allowing users to specify which specific
/// font variants (e.g., "Retina", "Light") they want to install from a larger release.
///
/// # Arguments
/// * `font_name`: The name of the font, for logging.
/// * `extracted_dir`: The `PathBuf` to the directory where the font archive was extracted.
/// * `fonts_dir`: The `PathBuf` to the target system fonts directory.
/// * `install_only_keywords`: An `Option<Vec<String>>` containing keywords. If `Some`,
///                            only font filenames containing any of these keywords (case-insensitive)
///                            will be copied. If `None` or empty `Vec`, all valid font files are copied.
///
/// # Returns
/// * `Result<Vec<String>, String>`:
///   - `Ok(Vec<String>)` if the files are successfully copied. The vector contains the
///     absolute paths of all font files that were successfully installed.
///   - `Err(String)` if reading the extracted directory fails, or other I/O errors prevent listing files.
fn copy_font_files(
    font_name: &str,
    extracted_dir: &PathBuf,
    fonts_dir: &PathBuf,
    install_only_keywords: &Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut installed_font_files: Vec<String> = Vec::new();

    // Read the contents of the extracted directory.
    let read_dir_result = fs::read_dir(extracted_dir).map_err(|e| {
        format!(
            "Failed to read extracted directory {:?}. It might be empty or corrupted. Error: {}",
            extracted_dir.display().to_string().yellow(),
            e
        )
    })?;

    // Pre-process `install_only_keywords` by converting them to lowercase for case-insensitive matching.
    let filter_keywords: Option<Vec<String>> = install_only_keywords.as_ref().map(|keywords| {
        keywords.iter().map(|k| k.to_lowercase()).collect()
    });

    // Log the application of the filter for user feedback.
    if let Some(keywords) = &filter_keywords {
        if keywords.is_empty() {
            log_info!("[Font:Copy] 'install_only' field is empty for '{}'. All detected font files will be considered for installation.", font_name.blue());
        } else {
            log_info!("[Font:Copy] Applying 'install_only' filter for '{}' with keywords: {:?}", font_name.blue(), keywords);
        }
    } else {
        log_info!("[Font:Copy] No 'install_only' filter specified for '{}'. All detected font files will be considered for installation.", font_name.blue());
    }

    // Iterate through each entry in the extracted directory.
    for entry in read_dir_result {
        let entry = entry.map_err(|e| format!("Skipping invalid directory entry: {}", e))?;
        let path = entry.path();
        log_debug!("[Font:Copy] Examining extracted path: {:?}", path.display().to_string().dimmed());

        // Process only files. Directories are skipped as fonts are direct files.
        if path.is_file() {
            // Check file extension to ensure it's a font file (.ttf or .otf).
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();

                if ext_str == "ttf" || ext_str == "otf" {
                    // Extract filename for filtering and target path construction.
                    let file_name = path.file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| format!("Skipping font file with no valid filename detected: {:?}", path.display()))?
                        .to_string();

                    let file_name_lower = file_name.to_lowercase();

                    // Apply the `install_only` filter.
                    // If keywords are provided, `should_install` is true if any keyword is found in the filename.
                    // If no keywords or an empty list is provided, all font files are considered for installation.
                    let should_install = if let Some(keywords) = &filter_keywords {
                        keywords.is_empty() || keywords.iter().any(|keyword| file_name_lower.contains(keyword))
                    } else {
                        true // No filter specified, so install all.
                    };

                    if should_install {
                        let target_path = fonts_dir.join(&file_name);
                        log_debug!("[Font:Copy] Attempting to copy font file from {:?} to {:?}", path.display().to_string().yellow(), target_path.display().to_string().yellow());

                        // Attempt to copy the font file. Log warnings for failures but continue processing other files.
                        if let Err(err) = fs::copy(&path, &target_path) {
                            log_warn!(
                                "[Font:Copy] Failed to copy font file {:?} to {:?}: {}",
                                path.display().to_string().yellow(),
                                target_path.display().to_string().yellow(),
                                err
                            );
                        } else {
                            log_debug!("[Font:Copy] Successfully copied font file: {:?}", target_path.display().to_string().green());
                            installed_font_files.push(target_path.display().to_string());
                        }
                    } else {
                        log_debug!("[Font:Copy] Skipping font file '{}' (does not match 'install_only' filter).", file_name.dimmed());
                    }
                } else {
                    log_debug!("[Font:Copy] Skipping non-font file based on extension: {:?} (extension: {})", path.display().to_string().dimmed(), ext_str.dimmed());
                }
            } else {
                log_debug!("[Font:Copy] Skipping file with no detected extension: {:?}", path.display().to_string().dimmed());
            }
        } else if path.is_dir() {
            log_debug!("[Font:Copy] Skipping directory entry: {:?}", path.display().to_string().dimmed());
        }
    }
    Ok(installed_font_files)
}

/// Cleans up the temporary directory used during installation.
///
/// This function is called at the end of the installation process, regardless of success
/// or failure, to remove all temporary files and directories.
///
/// # Arguments
/// * `temp_dir`: The `PathBuf` to the temporary directory to be removed.
fn cleanup_temp_dir(temp_dir: &PathBuf) {
    if let Err(err) = fs::remove_dir_all(temp_dir) {
        log_warn!("[Font:Cleanup] Failed to clean up temporary directory {:?}: {}", temp_dir.display().to_string().yellow(), err);
    } else {
        log_debug!("[Font:Cleanup] Temporary directory {:?} successfully cleaned up.", temp_dir.display());
    }
}

/// Installs a font from a GitHub release as specified by the `FontEntry`.
///
/// This is the main public function of the `fonts` module. It orchestrates the entire
/// font installation workflow by calling the various private helper functions in sequence.
/// It handles error propagation and ensures proper cleanup.
///
/// # Arguments
/// * `font`: A reference to a `FontEntry` struct that holds all the configuration
///           details for the font to be installed (e.g., name, repo, tag, install_only).
///
/// # Returns
/// * `Option<FontState>`:
///   - `Some(FontState)` if the installation process was successful and at least one
///     valid font file was copied to the system. The `FontState` contains details
///     for tracking the installed font.
///   - `None` if the installation failed at any point (e.g., validation, download, extraction,
///     copying), or if no font files were found or successfully copied from the archive.
pub fn install(font: &FontEntry) -> Option<FontState> {
    log_info!("Initiating font installation for: {}", font.name.bold());
    log_debug!("[Font] Detailed font entry configuration received: {:?}", font);

    // 1. Validate Font Entry: Ensure the configuration is correct and complete for GitHub source.
    let validated_details = match validate_font_entry(font) {
        Ok(details) => details,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            return None;
        }
    };
    // Destructure the validated details for easier access.
    let url = validated_details.url;
    let asset_name = validated_details.asset_name;
    let repo = validated_details.repo;
    let tag = validated_details.tag;

    // Get a dedicated temporary directory for this installation process.
    // A clone is made to ensure it can be cleaned up even if `temp_dir` is moved or consumed.
    let temp_dir = get_temp_dir();
    let temp_dir_clone_for_cleanup = temp_dir.clone();

    // Use a `defer!`-like pattern or manual call for cleanup. For Rust, `ScopeGuard` or a similar
    // RAII approach is ideal for guaranteed cleanup, but for directness, a manual call at exit points works.
    // In this implementation, `cleanup_temp_dir` is called explicitly before any `return None` and at the end.

    // 2. Download the Font Archive: Fetch the .zip file from the constructed URL.
    let archive_path = match download_font_archive(&font.name, &url, &asset_name) {
        Ok(path) => path,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            cleanup_temp_dir(&temp_dir_clone_for_cleanup); // Ensure cleanup on download failure.
            return None;
        }
    };

    // 3. Extract the Font Archive: Decompress the downloaded .zip file.
    let extracted_dir = match extract_font_archive(&font.name, &archive_path, &temp_dir) {
        Ok(path) => path,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            cleanup_temp_dir(&temp_dir_clone_for_cleanup); // Ensure cleanup on extraction failure.
            return None;
        }
    };

    // 4. Determine macOS Fonts Directory: Find or create `~/Library/Fonts`.
    let fonts_dir = match get_font_installation_dir(&font.name) {
        Ok(path) => path,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            cleanup_temp_dir(&temp_dir_clone_for_cleanup); // Ensure cleanup on target directory failure.
            return None;
        }
    };

    // 5. Copy Font Files to Fonts Directory (with `install_only` filter):
    // This is where font files are moved to their final system location.
    let installed_font_files = match copy_font_files(&font.name, &extracted_dir, &fonts_dir, &font.install_only) {
        Ok(files) => files,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            cleanup_temp_dir(&temp_dir_clone_for_cleanup); // Ensure cleanup on copy failure.
            return None;
        }
    };

    // 6. Final Status and Cleanup:
    // Provide feedback to the user regarding the outcome of the installation.
    if !installed_font_files.is_empty() {
        log_info!(
            "[Font] Installation of font '{}' completed. Successfully copied {} font files.",
            font.name.green(),
            installed_font_files.len().to_string().cyan()
        );
    } else {
        // If no font files were actually installed, warn the user.
        log_warn!(
            "[Font] No .ttf or .otf font files were found or successfully copied from the archive for '{}'. \
             The font might not be correctly installed, the archive format is unexpected, or no files matched the 'install_only' filter.",
            font.name.yellow()
        );
    }

    // Always clean up the temporary directory regardless of installation success or failure.
    cleanup_temp_dir(&temp_dir_clone_for_cleanup);

    // 7. Return FontState for Tracking (Conditional):
    // Only record the font's state if at least one file was successfully installed.
    if !installed_font_files.is_empty() {
        log_debug!("[Font] Constructing FontState for '{}'.", font.name.bold());
        Some(FontState {
            name: font.name.clone(),
            // Derive version from `font.version` or `font.tag`, or default to "unknown".
            version: font.version.clone().unwrap_or_else(|| font.tag.clone().unwrap_or_else(|| "unknown".to_string())),
            url,
            repo: font.repo.clone(),
            tag: font.tag.clone(),
            files: installed_font_files, // List of all successfully installed font files.
        })
    } else {
        // If no files were installed, return `None` to indicate no state should be recorded.
        log_debug!("[Font] No font files were installed for '{}', so no FontState will be recorded.", font.name.bold());
        None
    }
}
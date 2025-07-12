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

/// Helper struct to return validated font entry details.
struct ValidatedFontDetails<'a> {
    repo: &'a str,
    tag: &'a str,
    asset_name: String,
    url: String,
}

/// Validates the font entry's source and required GitHub fields.
///
/// # Arguments
/// * `font`: A reference to the `FontEntry` struct.
///
/// # Returns
/// * `Result<ValidatedFontDetails, String>`:
///   - `Ok(ValidatedFontDetails)` if validation passes, containing extracted repo, tag, asset name, and URL.
///   - `Err(String)` if validation fails, with an error message.
fn validate_font_entry(font: &FontEntry) -> Result<ValidatedFontDetails, String> {
    log_debug!("[Font:Validation] Starting validation for font: {}", font.name.bold());

    // Validate Font Source
    if font.source != "github" {
        return Err(format!(
            "Installation failed for '{}': Unsupported font source '{}'. Only 'github' is currently supported.",
            font.name.red(),
            font.source.red()
        ));
    }
    log_debug!("[Font:Validation] Font source 'github' is validated.");

    // Validate Required GitHub Fields (Repository and Tag)
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

    // Construct Download URL
    let asset_name = format!("{}.zip", font.name.replace(' ', ""));
    let url = format!("https://github.com/{}/releases/download/{}/{}", repo, tag, asset_name);
    log_debug!("[Font:Validation] Constructed download URL: {}", url.blue());

    Ok(ValidatedFontDetails {
        repo,
        tag,
        asset_name,
        url,
    })
}

/// Downloads the font archive to a temporary directory.
///
/// # Arguments
/// * `font_name`: The name of the font.
/// * `url`: The URL to download the archive from.
/// * `asset_name`: The expected filename of the downloaded asset.
///
/// # Returns
/// * `Result<PathBuf, String>`:
///   - `Ok(PathBuf)` if download is successful, containing the path to the downloaded archive.
///   - `Err(String)` if download fails.
fn download_font_archive(font_name: &str, url: &str, asset_name: &str) -> Result<PathBuf, String> {
    log_info!("[Font:Download] Attempting to download font archive for '{}' from URL: {}", font_name.bold(), url.blue());

    let temp_dir = utils::get_temp_dir();
    let archive_path = temp_dir.join(asset_name);
    log_debug!("[Font:Download] Temporary download path for font archive: {:?}", archive_path.display().to_string().yellow());

    if let Err(err) = utils::download_file(url, &archive_path) {
        let _ = fs::remove_file(&archive_path); // Clean up partial download
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
/// # Arguments
/// * `font_name`: The name of the font.
/// * `archive_path`: The path to the downloaded archive.
/// * `temp_dir`: The base temporary directory.
///
/// # Returns
/// * `Result<PathBuf, String>`:
///   - `Ok(PathBuf)` if extraction is successful, containing the path to the extracted directory.
///   - `Err(String)` if extraction fails.
fn extract_font_archive(font_name: &str, archive_path: &PathBuf, temp_dir: &PathBuf) -> Result<PathBuf, String> {
    log_debug!("[Font:Extract] Detecting file type from filename for extraction.");
    let file_type_from_name = utils::detect_file_type_from_filename(&archive_path.to_string_lossy());
    log_debug!("[Font:Extract] Detected downloaded file type (from filename): {}", file_type_from_name.magenta());

    let extracted_dir = match utils::extract_archive(archive_path, temp_dir, Some(&file_type_from_name)) {
        Ok(path) => path,
        Err(err) => {
            let _ = fs::remove_file(archive_path); // Clean up the original archive
            return Err(format!(
                "Failed to extract archive {:?} for font '{}': {}. Archive might be corrupted or in an unsupported format.",
                archive_path.display().to_string().red(),
                font_name.red(),
                err
            ));
        }
    };
    log_info!("[Font:Extract] Font archive extracted to: {}", extracted_dir.display().to_string().green());
    let _ = fs::remove_file(archive_path); // Clean up the original downloaded archive
    Ok(extracted_dir)
}

/// Determines and creates the macOS Fonts directory.
///
/// # Returns
/// * `Result<PathBuf, String>`:
///   - `Ok(PathBuf)` if the fonts directory is successfully determined and created.
///   - `Err(String)` if the directory cannot be determined or created.
fn get_font_installation_dir(font_name: &str) -> Result<PathBuf, String> {
    let fonts_dir = dirs::home_dir()
        .map(|path| path.join("Library/Fonts"))
        .or_else(|| {
            env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join("Library/Fonts"))
        })
        .ok_or_else(|| {
            format!(
                "Could not determine the user's Home directory or standard Fonts directory. Cannot install font '{}'.",
                font_name.red()
            )
        })?;

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

/// Copies font files from the extracted directory to the target font directory,
/// applying the `install_only` filter.
///
/// # Arguments
/// * `font_name`: The name of the font.
/// * `extracted_dir`: The path to the directory where font files were extracted.
/// * `fonts_dir`: The target system fonts directory.
/// * `install_only_keywords`: Optional list of keywords for filtering filenames.
///
/// # Returns
/// * `Result<Vec<String>, String>`:
///   - `Ok(Vec<String>)` if files are successfully copied, containing their paths.
///   - `Err(String)` if reading the extracted directory fails.
fn copy_font_files(
    font_name: &str,
    extracted_dir: &PathBuf,
    fonts_dir: &PathBuf,
    install_only_keywords: &Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut installed_font_files: Vec<String> = Vec::new();

    let read_dir_result = fs::read_dir(extracted_dir).map_err(|e| {
        format!(
            "Failed to read extracted directory {:?}. It might be empty or corrupted. Error: {}",
            extracted_dir.display().to_string().yellow(),
            e
        )
    })?;

    // Prepare filter keywords if present
    let filter_keywords: Option<Vec<String>> = install_only_keywords.as_ref().map(|keywords| {
        keywords.iter().map(|k| k.to_lowercase()).collect()
    });

    if let Some(keywords) = &filter_keywords {
        if keywords.is_empty() {
            log_info!("[Font:Copy] 'install_only' field is empty for '{}'. All detected font files will be considered for installation.", font_name.blue());
        } else {
            log_info!("[Font:Copy] Applying 'install_only' filter for '{}' with keywords: {:?}", font_name.blue(), keywords);
        }
    } else {
        log_info!("[Font:Copy] No 'install_only' filter specified for '{}'. All detected font files will be considered for installation.", font_name.blue());
    }

    for entry in read_dir_result {
        let entry = entry.map_err(|e| format!("Skipping invalid directory entry: {}", e))?;
        let path = entry.path();
        log_debug!("[Font:Copy] Examining extracted path: {:?}", path.display().to_string().dimmed());

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();

                if ext_str == "ttf" || ext_str == "otf" {
                    let file_name = path.file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| format!("Skipping font file with no valid filename detected: {:?}", path.display()))?
                        .to_string();

                    let file_name_lower = file_name.to_lowercase();

                    let should_install = if let Some(keywords) = &filter_keywords {
                        keywords.is_empty() || keywords.iter().any(|keyword| file_name_lower.contains(keyword))
                    } else {
                        true
                    };

                    if should_install {
                        let target_path = fonts_dir.join(&file_name);
                        log_debug!("[Font:Copy] Attempting to copy font file from {:?} to {:?}", path.display().to_string().yellow(), target_path.display().to_string().yellow());

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

/// Cleans up the temporary directory.
///
/// # Arguments
/// * `temp_dir`: The path to the temporary directory to remove.
fn cleanup_temp_dir(temp_dir: &PathBuf) {
    if let Err(err) = fs::remove_dir_all(temp_dir) {
        log_warn!("[Font:Cleanup] Failed to clean up temporary directory {:?}: {}", temp_dir.display().to_string().yellow(), err);
    } else {
        log_debug!("[Font:Cleanup] Temporary directory {:?} successfully cleaned up.", temp_dir.display());
    }
}

/// Installs a font from a GitHub release as specified by the `FontEntry`.
///
/// This function orchestrates the entire font installation workflow by calling
/// several private helper functions.
///
/// # Arguments
/// * `font`: A reference to a `FontEntry` struct that holds all the configuration
///           details for the font to be installed (e.g., name, repo, tag).
///
/// # Returns
/// * `Option<FontState>`:
///   - `Some(FontState)` if the installation process was successful and at least one
///     valid font file was copied to the system.
///   - `None` if the installation failed at any point, or if no font files were
///     found/copied from the downloaded archive.
pub fn install(font: &FontEntry) -> Option<FontState> {
    log_info!("Initiating font installation for: {}", font.name.bold());
    log_debug!("[Font] Detailed font entry configuration received: {:?}", font);

    // 1. Validate Font Entry
    let validated_details = match validate_font_entry(font) {
        Ok(details) => details,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            return None;
        }
    };
    let url = validated_details.url;
    let asset_name = validated_details.asset_name;
    let repo = validated_details.repo;
    let tag = validated_details.tag;

    // Get a dedicated temporary directory for this installation process
    let temp_dir = utils::get_temp_dir();
    let temp_dir_clone_for_cleanup = temp_dir.clone(); // Clone for eventual cleanup in defer

    // Use `defer!` if available, otherwise ensure cleanup with `_drop` or similar patterns.
    // For simplicity, we'll manually call cleanup_temp_dir at the end or on error.

    // 2. Download the Font Archive
    let archive_path = match download_font_archive(&font.name, &url, &asset_name) {
        Ok(path) => path,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            cleanup_temp_dir(&temp_dir_clone_for_cleanup);
            return None;
        }
    };

    // 3. Extract the Font Archive
    let extracted_dir = match extract_font_archive(&font.name, &archive_path, &temp_dir) {
        Ok(path) => path,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            cleanup_temp_dir(&temp_dir_clone_for_cleanup);
            return None;
        }
    };

    // 4. Determine macOS Fonts Directory
    let fonts_dir = match get_font_installation_dir(&font.name) {
        Ok(path) => path,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            cleanup_temp_dir(&temp_dir_clone_for_cleanup);
            return None;
        }
    };

    // 5. Copy Font Files to Fonts Directory (with install_only filter)
    let installed_font_files = match copy_font_files(&font.name, &extracted_dir, &fonts_dir, &font.install_only) {
        Ok(files) => files,
        Err(e) => {
            log_error!("[Font] {}. Aborting installation.", e);
            cleanup_temp_dir(&temp_dir_clone_for_cleanup);
            return None;
        }
    };

    // 6. Final Status and Cleanup
    if !installed_font_files.is_empty() {
        log_info!(
            "[Font] Installation of font '{}' completed. Successfully copied {} font files.",
            font.name.green(),
            installed_font_files.len().to_string().cyan()
        );
    } else {
        log_warn!(
            "[Font] No .ttf or .otf font files were found or successfully copied from the archive for '{}'. \
             The font might not be correctly installed, the archive format is unexpected, or no files matched the 'install_only' filter.",
            font.name.yellow()
        );
    }

    cleanup_temp_dir(&temp_dir_clone_for_cleanup);

    // 7. Return FontState for Tracking (Conditional)
    if !installed_font_files.is_empty() {
        log_debug!("[Font] Constructing FontState for '{}'.", font.name.bold());
        Some(FontState {
            name: font.name.clone(),
            version: font.version.clone().unwrap_or_else(|| font.tag.clone().unwrap_or_else(|| "unknown".to_string())),
            url,
            repo: font.repo.clone(),
            tag: font.tag.clone(),
            files: installed_font_files,
        })
    } else {
        log_debug!("[Font] No font files were installed for '{}', so no FontState will be recorded.", font.name.bold());
        None
    }
}
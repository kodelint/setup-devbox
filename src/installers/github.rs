// This module orchestrates the installation of software tools that are distributed as GitHub
// releases. It encapsulates the complete lifecycle from fetching release metadata to placing
// the final executable, handling platform-specific asset selection, downloads, and various
// extraction/installation routines.
//
// Our primary goal here is to provide a robust and extensible mechanism for installing tools
// where the source of truth for distribution is a GitHub repository's "Releases" section.

// Standard library imports:
use std::env;
// For interacting with environment variables, specifically to find the HOME directory.
use std::path::PathBuf;
// For ergonomic and platform-agnostic path manipulation.

// External crate imports:
// Used for adding color to terminal output, improving log readability.
use colored::Colorize;
// For deserializing JSON responses from the GitHub API into Rust structs.
use ureq;

// Internal module imports:
// `Release`: Defines the structure for deserializing GitHub Release API responses.
// `ToolEntry`: Represents a single tool's configuration as defined in our `tools.yaml`,
//              providing necessary details like repository name, tag, and desired tool name.
// `ToolState`: Represents the state of an installed tool, which we persist in `state.json`
//              to track installed tools, their versions, and paths.
use crate::schema::{Release, ToolEntry, ToolState};
// Provides various utility functions from libs/utilities:
// `download_file`: Handles downloading a file from a URL to a local path.
// `extract_archive`: Decompresses and extracts various archive formats (zip, tar.gz, etc.).
// `detect_os`, `detect_architecture`: Determine the current operating system and CPU architecture.
// `get_temp_dir`: Provides a temporary directory for downloads and extractions.
// `install_pkg`: Specific utility for installing macOS .pkg files.
// `move_and_rename_binary`: Moves a file and optionally renames it.
// `make_executable`: Sets executable permissions on a file.
// `find_executable`: Recursively searches for an executable within a directory.
// `detect_file_type`, `detect_file_type_from_filename`: Utilities to infer file types.
use crate::libs::utilities::{
    assets::{
        detect_file_type,
        detect_file_type_from_filename, 
        download_file, 
        install_pkg
    },
    binary::{
        find_executable, 
        make_executable, 
        move_and_rename_binary
    },
    compression::extract_archive,
    path_helpers::get_temp_dir, 
    platform::{
        detect_os,
        detect_architecture,
        asset_matches_platform
}};

// Custom logging macros. These are used throughout the module to provide informative output
// during the installation process, aiding in debugging and user feedback.
use crate::{log_debug, log_error, log_info};

/// Installs a software tool by fetching its release asset from GitHub.
///
/// This is the core function responsible for the GitHub-based installation flow. It orchestrates
/// several steps: platform detection, configuration validation, GitHub API interaction, asset
/// selection, download, extraction (if applicable), and final placement of the executable.
///
/// # Arguments
/// * `tool`: A reference to a `ToolEntry` struct. This `ToolEntry` contains all the
///           metadata read from the `tools.yaml` configuration file that specifies
///           how to install this particular tool from GitHub (e.g., `repo`, `tag`,
///           `name`, `rename_to`).
///
/// # Returns
/// * `Option<ToolState>`:
///   - `Some(ToolState)`: Indicates a successful installation. The contained `ToolState`
///     struct provides details like the installed version, the absolute path to the binary,
///     and the installation method, which are then persisted in our internal `state.json`.
///   - `None`: Signifies that the installation failed at some step. Detailed error logging
///     is performed before returning `None` to provide context for the failure.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    // Start the installation process with a debug log, clearly indicating which tool is being processed.
    log_debug!("[GitHub] Initiating installation process for tool: {:?}", tool_entry.name.to_string().bold());

    // 1. Detect Current Operating System and Architecture
    // The first critical step is to determine the host's platform. GitHub releases typically
    // provide different binaries for different OS/architecture combinations (e.g., `linux-amd64`,
    // `darwin-arm64`). Without this information, we cannot select the correct asset.
    let os = match detect_os() {
        Some(os) => os,
        None => {
            // If OS detection fails, log an error and abort. This is a fundamental requirement.
            log_error!("[GitHub] Unable to detect the current operating system. Aborting installation for {}.", tool_entry.name.to_string().red());
            return None;
        }
    };

    let arch = match detect_architecture() {
        Some(arch) => arch,
        None => {
            // Similarly, if architecture detection fails, log an error and abort.
            log_error!("[GitHub] Unable to detect the current machine architecture. Aborting installation for {}.", tool_entry.name.to_string().red());
            return None;
        }
    };

    log_info!("[GitHub] Detected platform for {}: {}{}{}", tool_entry.name.bold(), os.green(),"-".green(), arch.green());

    // 2. Validate Tool Configuration for GitHub Source
    // For a GitHub release installation, both `tag` (the release version) and `repo` (the GitHub
    // repository slug, e.g., "owner/repo_name") are absolutely essential. Without them, we can't
    // even form the API request to fetch release information.
    let tag = match tool_entry.tag.as_ref() {
        Some(t) => t,
        None => {
            log_error!("[GitHub] Configuration error: 'tag' field is missing for tool {}. Cannot download from GitHub.", tool_entry.name.to_string().red());
            return None;
        }
    };

    let repo = match tool_entry.repo.as_ref() {
        Some(r) => r,
        None => {
            log_error!("[GitHub] Configuration error: 'repo' field is missing for tool {}. Cannot download from GitHub.", tool_entry.name.to_string().red());
            return None;
        }
    };

    // 3. Fetch GitHub Release Information via API
    // Construct the GitHub API endpoint URL for the specific repository and release tag.
    // Example: https://api.github.com/repos/cli/cli/releases/tags/v2.5.0
    let api_url = format!("https://api.github.com/repos/{}/releases/tags/{}", repo, tag);
    log_debug!("[GitHub] Fetching release information from GitHub API: {}", api_url.blue());

    // Execute the HTTP GET request to the GitHub API.
    // It's crucial to set a `User-Agent` header as GitHub's API guidelines recommend it.
    // Without it, requests might be rate-limited or blocked.
    let response = match ureq::get(&api_url)
        .set("User-Agent", "setup-devbox") // Identify our application, preventing potential issues with GitHub's API.
        .call()
    {
        Ok(resp) => resp, // Successfully received an HTTP response.
        Err(e) => {
            // Log any network or ureq-specific error during the API call.
            log_error!("[GitHub] Failed to fetch GitHub release for {} ({}): {}", tool_entry.name.to_string().red(), repo.to_string().red(), e);
            return None;
        }
    };

    // Beyond network errors, we must check the HTTP status code. A 4xx or 5xx status
    // indicates an API-level error, such as a non-existent repository or tag (404 Not Found).
    if response.status() >= 400 {
        log_error!("[GitHub] GitHub API returned an error status (HTTP {}) for {} release {}. Check if the repo and tag are correct.", response.status(), repo.to_string().red(), tag.to_string().red());
        return None;
    }

    // Parse the successful JSON response body into our `Release` struct.
    // This deserialization uses `serde_json` and requires the `Release` struct
    // to correctly mirror the expected GitHub API JSON structure.
    let release: Release = match response.into_json() {
        Ok(json) => json,
        Err(err) => {
            // Log if JSON parsing fails, indicating an unexpected API response format.
            log_error!("[GitHub] Failed to parse GitHub release JSON for {}: {}", tool_entry.name.to_string().red(), err);
            return None;
        }
    };

    // 4. Find the Correct Asset for the Current Platform
    // A GitHub release can have multiple assets (downloadable files). We need to identify
    // the specific asset that is compatible with the detected OS and architecture.
    // The `asset_matches_platform` utility function contains the logic for this, typically
    // by pattern matching on asset filenames (e.g., checking for "linux-amd64" in the name).
    let asset_opt = release.assets.iter()
        .find(|asset| asset_matches_platform(&asset.name, &os, &arch));

    let asset = match asset_opt {
        Some(a) => {
            log_debug!("[GitHub] Found matching asset: {}", a.name.bold());
            a
        },
        None => {
            // If no matching asset is found, this is a critical failure. Provide informative
            // error messages, including a list of available assets to help with debugging
            // or adjusting the `tools.yaml` configuration.
            let available_assets = release.assets.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
            log_error!(
                "[GitHub] No suitable release asset found for platform {}-{} in repo {} tag {}. \
                 Please check the release assets on GitHub. Available assets: {:?}",
                os.to_string().red(), arch.to_string().red(), repo.to_string().red(), tag.to_string().red(), available_assets.join(", ").to_string().yellow()
            );
            return None;
        }
    };

    // Once the correct asset is identified, extract its download URL.
    let download_url = &asset.browser_download_url;
    log_debug!("[GitHub] Download URL for selected asset: {}", download_url.dimmed());

    // 5. Download the Asset
    // We download the asset to a temporary directory to avoid cluttering the user's system
    // and to facilitate extraction and cleanup.
    let temp_dir = get_temp_dir();
    let filename = &asset.name; // Use the original filename from the asset.
    let archive_path = temp_dir.join(filename); // Construct the full path for the downloaded file.

    log_debug!("[GitHub] Downloading {} to temporary location: {}", tool_entry.name.to_string().bold(), archive_path.display().to_string().cyan());
    if let Err(err) = download_file(download_url, &archive_path) {
        // Log specific download errors (e.g., network issues during download).
        log_error!("[GitHub] Failed to download tool {} from {}: {}", tool_entry.name.to_string().red(), download_url.to_string().red(), err);
        return None;
    }
    log_info!("[GitHub] Download completed for {}.", tool_entry.name.to_string().bright_blue());

    // 6. Detect Downloaded File Type
    // We need to know the file type (e.g., "zip", "tar.gz", "binary", "pkg") to determine
    // the appropriate installation strategy (extraction, direct move, package installation).
    // `detect_file_type_from_filename` is preferred here for archives because the filename
    // usually clearly indicates the compression format, which is more reliable than
    // `file` command output for archives.
    let file_type_from_name = detect_file_type_from_filename(&archive_path.to_string_lossy());
    log_debug!("[GitHub] Detected downloaded file type (from filename): {}", file_type_from_name.to_string().magenta());

    // `actual_file_type_for_state` uses the `file` command, which provides a deeper inspection
    // of the file's magic bytes. While `file_type_from_name` is used for deciding *how* to extract,
    // this `actual_file_type_for_state` is more accurate for recording the precise file type
    // in the `ToolState`, which can be valuable for diagnostics or future enhancements.
    let actual_file_type_for_state = detect_file_type(&archive_path);
    log_debug!("[GitHub] Detected downloaded file type (from `file` command for state): {}", actual_file_type_for_state.to_string().magenta());


    // 7. Determine Final Installation Path
    // Tools are typically installed into a `bin/` directory within the user's home directory
    // (e.g., `~/.local/bin/` or `~/bin/`). We need to retrieve the `$HOME` environment variable.
    let home_dir = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => {
            // The `$HOME` variable is fundamental for user-specific installations.
            log_error!("[GitHub] The $HOME environment variable is not set. Cannot determine installation path for {}.", tool_entry.name.to_string().red());
            return None;
        }
    };

    // The final executable name can be explicitly specified in `tools.yaml` via `rename_to`.
    // If not specified, we default to the tool's original `name`.
    let bin_name = tool_entry.rename_to.clone().unwrap_or_else(|| tool_entry.name.clone());
    // Construct the full absolute path where the tool's executable will be placed.
    let install_path = PathBuf::from(format!("{}/bin/{}", home_dir, bin_name));
    log_debug!("[GitHub] Target installation path for {}: {}", tool_entry.name.to_string().bright_blue(), install_path.display().to_string().cyan());

    // 8. Install Based on File Type
    // This `match` statement serves as the primary dispatcher for installation logic,
    // branching based on the detected `file_type_from_name`. Each branch handles a
    // specific type of asset:
    match file_type_from_name.as_str() {
        "pkg" => {
            // Handles macOS installer packages (.pkg). This often involves calling system utilities.
            log_info!("[GitHub] Installing .pkg file for {}.", tool_entry.name.to_string().bold());
            if let Err(err) = install_pkg(&archive_path) {
                log_error!("[GitHub] Failed to install .pkg for {}: {}", tool_entry.name.to_string().red(), err);
                return None;
            }
        }
        "binary" => {
            // Handles direct executable files (e.g., a single `.exe` or uncompressed binary).
            // These files don't need extraction; they just need to be moved and made executable.
            log_debug!("[GitHub] Moving standalone binary for {}.", tool_entry.name.to_string().bold());
            if let Err(err) = move_and_rename_binary(&archive_path, &install_path) {
                log_error!("[GitHub] Failed to move binary for {}: {}", tool_entry.name.to_string().red(), err);
                return None;
            }
            log_debug!("[GitHub] Making binary executable for {}.", tool_entry.name.to_string().bold());
            if let Err(err) = make_executable(&install_path) {
                log_error!("[GitHub] Failed to make binary executable for {}: {}", tool_entry.name.to_string().red(), err);
                return None;
            }
        }
        // Handles common archive formats. For these, extraction is required, followed by
        // finding the actual executable within the extracted contents.
        "zip" | "tar.gz" | "gz" | "tar.bz2" | "tar" => {
            log_debug!("[GitHub] Extracting archive for {}.", tool_entry.name.to_string().blue());
            // Extract the downloaded archive into a *new* temporary subdirectory.
            // We pass `Some(&file_type_from_name)` as a hint to `extract_archive` to ensure
            // it uses the correct decompression logic, overriding potential `file` command ambiguities.
            let extracted_path = match extract_archive(&archive_path, &temp_dir, Some(&file_type_from_name)) {
                Ok(path) => path,
                Err(err) => {
                    log_error!("[GitHub] Failed to extract archive for {}: {}", tool_entry.name.to_string().red(), err);
                    return None;
                }
            };
            log_debug!("[GitHub] Searching for executable in extracted contents for {}.", tool_entry.name.to_string().blue());
            // Many archives contain nested directories. `find_executable` recursively searches
            // the extracted contents to locate the actual binary we need to install.
            let executable_path = match find_executable(&extracted_path) {
                Some(path) => path,
                None => {
                    log_error!("[GitHub] No executable found in the extracted archive for {}. Manual intervention may be required.", tool_entry.name.to_string().red());
                    return None;
                }
            };

            log_debug!("[GitHub] Moving and renaming executable for {}.", tool_entry.name.to_string().blue());
            // Move the located executable to its final destination and apply any `rename_to` rule.
            if let Err(err) = move_and_rename_binary(&executable_path, &install_path) {
                log_error!("[GitHub] Failed to move extracted binary for {}: {}", tool_entry.name.to_string().red(), err);
                return None;
            }
            log_debug!("[GitHub] Making extracted binary executable for {}.", tool_entry.name.to_string().blue());
            // Ensure the final binary has executable permissions set.
            if let Err(err) = make_executable(&install_path) {
                log_error!("[GitHub] Failed to make extracted binary executable for {}: {}", tool_entry.name.to_string().red(), err);
                return None;
            }
        }
        unknown => {
            // Catch-all for unsupported or unrecognized file types.
            log_error!("[GitHub] Unsupported or unknown file type '{}' for tool {}. Cannot install.", unknown.to_string().red(), tool_entry.name.to_string().red());
            return None;
        }
    }

    // If execution reaches this point, the installation was successful.
    log_info!("[GitHub] Installation of {} completed successfully at {}!", tool_entry.name.to_string().bold(), install_path.display().to_string().green());

    // 9. Return ToolState for Tracking
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed. This is crucial
    // for future operations like uninstallation, updates, or syncing.
    Some(ToolState {
        // The version field for tracking. Defaults to "latest" if not explicitly set in `tools.yaml`.
        version: tool_entry.version.clone().unwrap_or_else(|| "latest".to_string()),
        // The canonical path where the tool's executable was installed.
        install_path: install_path.display().to_string(),
        // Flag indicating that this tool was installed by `devbox`.
        installed_by_devbox: true,
        // The method of installation, useful for future diagnostics or differing update logic.
        install_method: "github".to_string(),
        // Records if the binary was renamed during installation.
        renamed_to: tool_entry.rename_to.clone(),
        // Persist the GitHub repository slug, important for future sync/update checks.
        repo: tool_entry.repo.clone(),
        // Persist the GitHub tag (release version), important for future sync/update checks.
        tag: tool_entry.tag.clone(),
        // The actual package type detected by the `file` command. This is for diagnostic
        // purposes, providing the most accurate type even if the installation logic
        // used a filename-based guess.
        package_type: actual_file_type_for_state,
        // Placeholder for future options that might be stored with the tool's state.
        options: tool_entry.options.clone(),
        // For direct URL installations: The original URL from which the tool was downloaded.
        url: tool_entry.url.clone(),
        executable_path_after_extract: None,
    })
}
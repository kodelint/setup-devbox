// This module is responsible for installing tools that are distributed as GitHub releases.
// It handles fetching release information, selecting the correct asset for the current platform,
// downloading, extracting, and placing the executable in the appropriate location.

// Importing necessary data structures (schemas) for GitHub releases and tool configurations.
// `Release` models the JSON response from the GitHub API.
// `ToolEntry` is how the tool is defined in our `tools.yaml` config.
// `ToolState` is what we record in our internal `state.json` after a successful installation.
use crate::schema::{Release, ToolEntry, ToolState};
// Importing general utility functions from our `utils` module, such as file downloading,
// archive extraction, and path manipulation.
use crate::utils;
// Specifically importing the `asset_matches_platform` utility, which helps us pick the
// right download file from a GitHub release's assets based on the current OS and architecture.
use crate::utils::asset_matches_platform;
// Custom logging macros for different levels of detail (debug, error, info, warn).
use crate::{log_debug, log_error, log_info};
// For colored terminal output, making logs more readable.
use colored::Colorize;

// Standard library imports for environment variables and path handling.
use std::env;
// To access environment variables like $HOME.
use std::path::PathBuf;
// For building and manipulating file paths.

/// Installs a software tool by downloading its release asset from GitHub.
/// This function encapsulates the entire logic for a GitHub-based installation.
///
/// # Arguments
/// * `tool`: A reference to a `ToolEntry` struct, which contains all the necessary
///           information about the tool to be installed from GitHub (e.g., repo, tag, name).
///
/// # Returns
/// * `Option<ToolState>`:
///   - `Some(ToolState)` if the tool was successfully installed, containing details
///     about its installation path and version for `devbox`'s internal state tracking.
///   - `None` if the installation failed at any step (e.g., network error, no matching asset).
pub fn install(tool: &ToolEntry) -> Option<ToolState> {
    log_debug!("[GitHub] Initiating installation process for tool: {:?}", tool.name.to_string().bold());

    // 1. Detect Current Operating System and Architecture
    // We need to know the platform to download the correct binary.
    let os = match utils::detect_os() {
        Some(os) => os,
        None => {
            log_error!("[GitHub] Unable to detect the current operating system. Aborting installation for {}.", tool.name.to_string().red());
            return None; // Cannot proceed without OS information.
        }
    };

    let arch = match utils::detect_architecture() {
        Some(arch) => arch,
        None => {
            log_error!("[GitHub] Unable to detect the current machine architecture. Aborting installation for {}.", tool.name.to_string().red());
            return None; // Cannot proceed without architecture information.
        }
    };

    log_info!("[GitHub] Detected platform for {}: {}{}{}", tool.name.bold(), os.green(),"-".green(), arch.green());

    // 2. Validate Tool Configuration for GitHub Source
    // For GitHub installations, `tag` and `repo` fields are mandatory.
    let tag = match tool.tag.as_ref() {
        Some(t) => t,
        None => {
            log_error!("[GitHub] Configuration error: 'tag' field is missing for tool {}. Cannot download from GitHub.", tool.name.to_string().red());
            return None;
        }
    };

    let repo = match tool.repo.as_ref() {
        Some(r) => r,
        None => {
            log_error!("[GitHub] Configuration error: 'repo' field is missing for tool {}. Cannot download from GitHub.", tool.name.to_string().red());
            return None;
        }
    };

    // 3. Fetch GitHub Release Information via API
    // Construct the GitHub API URL for the specific repository and release tag.
    let api_url = format!("https://api.github.com/repos/{}/releases/tags/{}", repo, tag);
    log_debug!("[GitHub] Fetching release information from GitHub API: {}", api_url.blue());

    // Make an HTTP GET request to the GitHub API.
    // We set a `User-Agent` header, which is good practice for API requests.
    let response = match ureq::get(&api_url)
        .set("User-Agent", "setup-devbox") // Identify our application to GitHub.
        .call()
    {
        Ok(resp) => resp, // Successfully received a response.
        Err(e) => {
            log_error!("[GitHub] Failed to fetch GitHub release for {} ({}): {}", tool.name.to_string().red(), repo.to_string().red(), e);
            return None; // Network or HTTP error.
        }
    };

    // Explicitly check the HTTP status code. GitHub API returns 404 for non-existent releases/tags.
    if response.status() >= 400 {
        log_error!("[GitHub] GitHub API returned an error status (HTTP {}) for {} release {}. Check if the repo and tag are correct.", response.status(), repo.to_string().red(), tag.to_string().red());
        return None;
    }

    // Parse the JSON response body into our `Release` struct using `serde_json`.
    let release: Release = match response.into_json() {
        Ok(json) => json,
        Err(err) => {
            log_error!("[GitHub] Failed to parse GitHub release JSON for {}: {}", tool.name.to_string().red(), err);
            return None;
        }
    };

    // 4. Find the Correct Asset for the Current Platform
    // Iterate through all assets attached to the release and find one that matches our
    // detected OS and architecture using the `asset_matches_platform` utility.
    let asset_opt = release.assets.iter()
        .find(|asset| asset_matches_platform(&asset.name, &os, &arch));

    let asset = match asset_opt {
        Some(a) => {
            log_debug!("[GitHub] Found matching asset: {}", a.name.bold());
            a
        },
        None => {
            // If no matching asset is found, provide detailed error message including available assets.
            let available_assets = release.assets.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
            log_error!(
                "[GitHub] No suitable release asset found for platform {}-{} in repo {} tag {}. \
                 Please check the release assets on GitHub. Available assets: {:?}",
                os.to_string().red(), arch.to_string().red(), repo.to_string().red(), tag.to_string().red(), available_assets.join(", ").to_string().yellow()
            );
            return None;
        }
    };

    let download_url = &asset.browser_download_url;
    log_debug!("[GitHub] Download URL for selected asset: {}", download_url.dimmed());

    // 5. Download the Asset
    // Get a temporary directory path where we can download and extract files without cluttering.
    let temp_dir = utils::get_temp_dir();
    let filename = &asset.name;
    // Construct the full path where the downloaded archive will be saved.
    let archive_path = temp_dir.join(filename);

    log_debug!("[GitHub] Downloading {} to temporary location: {}", tool.name.to_string().bold(), archive_path.display().to_string().cyan());
    if let Err(err) = utils::download_file(download_url, &archive_path) {
        log_error!("[GitHub] Failed to download tool {} from {}: {}", tool.name.to_string().red(), download_url.to_string().red(), err);
        return None;
    }
    log_info!("[GitHub] Download completed for {}.", tool.name.to_string().bright_blue());

    // 6. Detect Downloaded File Type (using the filename-based detection)
    // We now use `detect_file_type_from_filename` because the asset's name directly tells us its type
    // which is more reliable than using the `file` command for archives with ambiguous `file` output.
    let file_type_from_name = utils::detect_file_type_from_filename(&archive_path.to_string_lossy());
    log_debug!("[GitHub] Detected downloaded file type (from filename): {}", file_type_from_name.to_string().magenta());

    // If for some reason the filename detection isn't specific enough for `extract_archive`,
    // we can still fall back to `detect_file_type` or refine `extract_archive`'s branches.
    // For now, `file_type_from_name` will be the primary source of truth for the archive type.
    let actual_file_type_for_state = utils::detect_file_type(&archive_path); // Keep this for storing accurate file type in state
    log_debug!("[GitHub] Detected downloaded file type (from `file` command for state): {}", actual_file_type_for_state.to_string().magenta());


    // 7. Determine Final Installation Path
    // The default installation path will be in the user's home directory under `bin/`.
    // We get the HOME environment variable for this.
    let home_dir = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => {
            log_error!("[GitHub] The $HOME environment variable is not set. Cannot determine installation path for {}.", tool.name.to_string().red());
            return None;
        }
    };

    // The binary's name will either be the `rename_to` field from the config, or the original tool name.
    let bin_name = tool.rename_to.clone().unwrap_or_else(|| tool.name.clone());
    // Construct the full path where the final executable will reside.
    let install_path = PathBuf::from(format!("{}/bin/{}", home_dir, bin_name));
    log_debug!("[GitHub] Target installation path for {}: {}", tool.name.to_string().bright_blue(), install_path.display().to_string().cyan());

    // 8. Install Based on File Type
    // This `match` block branches the installation logic based on what kind of file was downloaded.
    match file_type_from_name.as_str() {
        "pkg" => {
            // For macOS installer packages (.pkg files).
            log_info!("[GitHub] Installing .pkg file for {}.", tool.name.to_string().bold());
            if let Err(err) = utils::install_pkg(&archive_path) {
                log_error!("[GitHub] Failed to install .pkg for {}: {}", tool.name.to_string().red(), err);
                return None;
            }
        }
        "binary" => {
            // For standalone executable files (no archiving).
            log_debug!("[GitHub] Moving standalone binary for {}.", tool.name.to_string().bold());
            if let Err(err) = utils::move_and_rename_binary(&archive_path, &install_path) {
                log_error!("[GitHub] Failed to move binary for {}: {}", tool.name.to_string().red(), err);
                return None;
            }
            log_debug!("[GitHub] Making binary executable for {}.", tool.name.to_string().bold());
            if let Err(err) = utils::make_executable(&install_path) {
                log_error!("[GitHub] Failed to make binary executable for {}: {}", tool.name.to_string().red(), err);
                return None;
            }
        }
        // For common archive types (zip, tar.gz, tar.bz2, tar).
        // The `extract_archive` function now expects the `known_file_type` parameter.
        "zip" | "tar.gz" | "gz" | "tar.bz2" | "tar" => {
            log_debug!("[GitHub] Extracting archive for {}.", tool.name.to_string().blue());
            // Extract the archive into a temporary subdirectory.
            let extracted_path = match utils::extract_archive(&archive_path, &temp_dir, Some(&file_type_from_name)) { // *** Pass `file_type_from_name` here ***
                Ok(path) => path,
                Err(err) => {
                    log_error!("[GitHub] Failed to extract archive for {}: {}", tool.name.to_string().red(), err);
                    return None;
                }
            };
            log_debug!("[GitHub] Searching for executable in extracted contents for {}.", tool.name.to_string().blue());
            // Find the actual executable binary within the extracted contents (it might be nested).
            let executable_path = match utils::find_executable(&extracted_path) {
                Some(path) => path,
                None => {
                    log_error!("[GitHub] No executable found in the extracted archive for {}. Manual intervention may be required.", tool.name.to_string().red());
                    return None;
                }
            };

            log_debug!("[GitHub] Moving and renaming executable for {}.", tool.name.to_string().blue());
            // Move the found executable to its final installation path and potentially rename it.
            if let Err(err) = utils::move_and_rename_binary(&executable_path, &install_path) {
                log_error!("[GitHub] Failed to move extracted binary for {}: {}", tool.name.to_string().red(), err);
                return None;
            }
            log_debug!("[GitHub] Making extracted binary executable for {}.", tool.name.to_string().blue());
            // Make the final binary executable.
            if let Err(err) = utils::make_executable(&install_path) {
                log_error!("[GitHub] Failed to make extracted binary executable for {}: {}", tool.name.to_string().red(), err);
                return None;
            }
        }
        unknown => {
            // If the file type is not recognized or supported for installation.
            log_error!("[GitHub] Unsupported or unknown file type '{}' for tool {}. Cannot install.", unknown.to_string().red(), tool.name.to_string().red());
            return None;
        }
    }

    log_info!("[GitHub] Installation of {} completed successfully at {}!", tool.name.to_string().bold(), install_path.display().to_string().green());

    // 9. Return ToolState for Tracking
    // If we've reached here, the installation was successful.
    // We construct a `ToolState` object to record details about this installation
    // in our `state.json` file, so `devbox` knows it's installed.
    Some(ToolState {
        // Use the version from the config, or default to "latest" if not specified.
        version: tool.version.clone().unwrap_or_else(|| "latest".to_string()),
        // Store the final absolute path where the tool was installed.
        install_path: install_path.display().to_string().to_string(),
        // Mark that `devbox` was responsible for this installation.
        installed_by_devbox: true,
        // Record the method used for installation.
        install_method: "github".to_string(),
        // Record if the binary was renamed during installation.
        renamed_to: tool.rename_to.clone(),
        // Record the repository name 
        // Help in `sync` command to sync back
        repo: tool.repo.clone(),
        // Record the tag name 
        // Help in `sync` command to sync back
        tag: tool.tag.clone(),
        // Store the type of package that was downloaded and processed.
        // We now use the `actual_file_type_for_state` which still uses the `file` command
        // for recording the most "truthful" type for diagnostics, even if we used
        // filename for extraction logic.
        package_type: actual_file_type_for_state,
        // Pass the options from ToolEntry to ToolState.
        options: None,
    })
}
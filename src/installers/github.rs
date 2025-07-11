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
    log_debug!("[GitHub] Initiating installation process for tool: {:?}", tool.name.bold());

    // 1. Detect Current Operating System and Architecture
    // We need to know the platform to download the correct binary.
    let os = match utils::detect_os() {
        Some(os) => os,
        None => {
            log_error!("[GitHub] Unable to detect the current operating system. Aborting installation for {}.", tool.name.red());
            return None; // Cannot proceed without OS information.
        }
    };

    let arch = match utils::detect_architecture() {
        Some(arch) => arch,
        None => {
            log_error!("[GitHub] Unable to detect the current machine architecture. Aborting installation for {}.", tool.name.red());
            return None; // Cannot proceed without architecture information.
        }
    };

    log_info!("[GitHub] Detected platform for {}: {}-{}", tool.name.bold(), os.cyan(), arch.cyan());

    // 2. Validate Tool Configuration for GitHub Source
    // For GitHub installations, `tag` and `repo` fields are mandatory.
    let tag = match tool.tag.as_ref() {
        Some(t) => t,
        None => {
            log_error!("[GitHub] Configuration error: 'tag' field is missing for tool {}. Cannot download from GitHub.", tool.name.red());
            return None;
        }
    };

    let repo = match tool.repo.as_ref() {
        Some(r) => r,
        None => {
            log_error!("[GitHub] Configuration error: 'repo' field is missing for tool {}. Cannot download from GitHub.", tool.name.red());
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
            log_error!("[GitHub] Failed to fetch GitHub release for {} ({}): {}", tool.name.red(), repo.red(), e);
            return None; // Network or HTTP error.
        }
    };

    // Explicitly check the HTTP status code. GitHub API returns 404 for non-existent releases/tags.
    if response.status() >= 400 {
        log_error!("[GitHub] GitHub API returned an error status (HTTP {}) for {} release {}. Check if the repo and tag are correct.", response.status(), repo.red(), tag.red());
        return None;
    }

    // Parse the JSON response body into our `Release` struct using `serde_json`.
    let release: Release = match response.into_json() {
        Ok(json) => json,
        Err(err) => {
            log_error!("[GitHub] Failed to parse GitHub release JSON for {}: {}", tool.name.red(), err);
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
            log_info!("[GitHub] Found matching asset: {}", a.name.bold());
            a
        },
        None => {
            // If no matching asset is found, provide detailed error message including available assets.
            let available_assets = release.assets.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
            log_error!(
                "[GitHub] No suitable release asset found for platform {}-{} in repo {} tag {}. \
                 Please check the release assets on GitHub. Available assets: {:?}",
                os.red(), arch.red(), repo.red(), tag.red(), available_assets.join(", ").yellow()
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

    log_info!("[GitHub] Downloading {} to temporary location: {:?}", tool.name.bold(), archive_path.to_string_lossy().cyan());
    if let Err(err) = utils::download_file(download_url, &archive_path) {
        log_error!("[GitHub] Failed to download tool {} from {}: {}", tool.name.red(), download_url.red(), err);
        return None;
    }
    log_info!("[GitHub] Download completed for {}.", tool.name.bold());

    // 6. Detect Downloaded File Type
    // Determine how to process the downloaded file (e.g., extract zip, run installer, move binary).
    let file_type = utils::detect_file_type(&archive_path);
    log_debug!("[GitHub] Detected downloaded file type: {}", file_type.magenta());

    // 7. Determine Final Installation Path
    // The default installation path will be in the user's home directory under `bin/`.
    // We get the HOME environment variable for this.
    let home_dir = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => {
            log_error!("[GitHub] The $HOME environment variable is not set. Cannot determine installation path for {}.", tool.name.red());
            return None;
        }
    };

    // The binary's name will either be the `rename_to` field from the config, or the original tool name.
    let bin_name = tool.rename_to.clone().unwrap_or_else(|| tool.name.clone());
    // Construct the full path where the final executable will reside.
    let install_path = PathBuf::from(format!("{}/bin/{}", home_dir, bin_name));
    log_info!("[GitHub] Target installation path for {}: {:?}", tool.name.bold(), install_path.to_string_lossy().cyan());

    // 8. Install Based on File Type
    // This `match` block branches the installation logic based on what kind of file was downloaded.
    match file_type.as_str() {
        "pkg" => {
            // For macOS installer packages (.pkg files).
            log_info!("[GitHub] Installing .pkg file for {}.", tool.name.bold());
            if let Err(err) = utils::install_pkg(&archive_path) {
                log_error!("[GitHub] Failed to install .pkg for {}: {}", tool.name.red(), err);
                return None;
            }
        }
        "binary" => {
            // For standalone executable files (no archiving).
            log_info!("[GitHub] Moving standalone binary for {}.", tool.name.bold());
            if let Err(err) = utils::move_and_rename_binary(&archive_path, &install_path) {
                log_error!("[GitHub] Failed to move binary for {}: {}", tool.name.red(), err);
                return None;
            }
            log_info!("[GitHub] Making binary executable for {}.", tool.name.bold());
            if let Err(err) = utils::make_executable(&install_path) {
                log_error!("[GitHub] Failed to make binary executable for {}: {}", tool.name.red(), err);
                return None;
            }
        }
        // For common archive types (zip, tar.gz, tar.bz2, tar).
        "zip" | "tar.gz" | "gz" | "tar.bz2" | "tar" => {
            log_info!("[GitHub] Extracting archive for {}.", tool.name.bold());
            // Extract the archive into a temporary subdirectory.
            let extracted_path = match utils::extract_archive(&archive_path, &temp_dir) {
                Ok(path) => path,
                Err(err) => {
                    log_error!("[GitHub] Failed to extract archive for {}: {}", tool.name.red(), err);
                    return None;
                }
            };
            log_info!("[GitHub] Searching for executable in extracted contents for {}.", tool.name.bold());
            // Find the actual executable binary within the extracted contents (it might be nested).
            let executable_path = match utils::find_executable(&extracted_path) {
                Some(path) => path,
                None => {
                    log_error!("[GitHub] No executable found in the extracted archive for {}. Manual intervention may be required.", tool.name.red());
                    return None;
                }
            };

            log_info!("[GitHub] Moving and renaming executable for {}.", tool.name.bold());
            // Move the found executable to its final installation path and potentially rename it.
            if let Err(err) = utils::move_and_rename_binary(&executable_path, &install_path) {
                log_error!("[GitHub] Failed to move extracted binary for {}: {}", tool.name.red(), err);
                return None;
            }
            log_info!("[GitHub] Making extracted binary executable for {}.", tool.name.bold());
            // Make the final binary executable.
            if let Err(err) = utils::make_executable(&install_path) {
                log_error!("[GitHub] Failed to make extracted binary executable for {}: {}", tool.name.red(), err);
                return None;
            }
        }
        unknown => {
            // If the file type is not recognized or supported for installation.
            log_error!("[GitHub] Unsupported or unknown file type '{}' for tool {}. Cannot install.", unknown.red(), tool.name.red());
            return None;
        }
    }

    log_info!("[GitHub] Installation of {} completed successfully at {:?}!", tool.name.bold(), install_path.to_string_lossy().green());

    // 9. Return ToolState for Tracking
    // If we've reached here, the installation was successful.
    // We construct a `ToolState` object to record details about this installation
    // in our `state.json` file, so `devbox` knows it's installed.
    Some(ToolState {
        // Use the version from the config, or default to "latest" if not specified.
        version: tool.version.clone().unwrap_or_else(|| "latest".to_string()),
        // Store the final absolute path where the tool was installed.
        install_path: install_path.to_string_lossy().to_string(),
        // Mark that `devbox` was responsible for this installation.
        installed_by_devbox: true,
        // Record the method used for installation.
        install_method: "github".to_string(),
        // Record if the binary was renamed during installation.
        renamed_to: tool.rename_to.clone(),
        // Store the type of package that was downloaded and processed.
        package_type: file_type,
    })
}
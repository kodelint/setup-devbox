// This module orchestrates the installation of software tools that are distributed as GitHub
// releases. It encapsulates the complete lifecycle from fetching release metadata to placing
// the final executable, handling platform-specific asset selection, downloads, and various
// extraction/installation routines, including post-installation command execution.
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
    assets::{detect_file_type, download_file, install_pkg},
    binary::{find_executable, make_executable, move_and_rename_binary},
    compression,
    platform::{asset_matches_platform, detect_architecture, detect_os},
};
// Internal module imports:
// `Release`: Defines the structure for deserializing GitHub Release API responses.
// `ToolEntry`: Represents a single tool's configuration as defined in our `tools.yaml`,
//              providing necessary details like repository name, tag, and desired tool name.
// `ToolState`: Represents the state of an installed tool, which we persist in `state.json`
//              to track installed tools, their versions, and paths.
use crate::schema::{Release, ReleaseAsset, ToolEntry, ToolState};

use crate::libs::tool_installer::execute_post_installation_commands;
use crate::libs::utilities::assets::{current_timestamp, install_dmg};
// Custom logging macros. These are used throughout the module to provide informative output
// during the installation process, aiding in debugging and user feedback.
use crate::{log_debug, log_error, log_info};
use tempfile::Builder as TempFileBuilder;

/// Installs a software tool by fetching its release asset from GitHub.
///
/// This is the core function responsible for the GitHub-based installation flow. It orchestrates
/// several steps: platform detection, configuration validation, GitHub API interaction, asset
/// selection, download, extraction (if applicable), final placement of the executable, and
/// execution of any additional post-installation commands.
///
/// # Arguments
/// * `tool`: A reference to a `ToolEntry` struct. This `ToolEntry` contains all the
///           metadata read from the `tools.yaml` configuration file that specifies
///           how to install this particular tool from GitHub (e.g., `repo`, `tag`,
///           `name`, `rename_to`, `additional_cmd`).
///
/// # Returns
/// * `Option<ToolState>`:
///   - `Some(ToolState)`: Indicates a successful installation. The contained `ToolState`
///     struct provides details like the installed version, the absolute path to the binary,
///     the installation method, and any executed additional commands, which are then
///     persisted in our internal `state.json`.
///   - `None`: Signifies that the installation failed at some step. Detailed error logging
///     is performed before returning `None` to provide context for the failure.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    // Start the installation process with a debug log, clearly indicating which tool is being processed.
    log_debug!(
        "[GitHub Installer] Initiating installation process for tool: {}",
        tool_entry.name.to_string().bold()
    );

    // 1. Detect Current Operating System and Architecture
    // The first critical step is to determine the host's platform. GitHub releases typically
    // provide different binaries for different OS/architecture combinations (e.g., `linux-amd64`,
    // `darwin-arm64`). Without this information, we cannot select the correct asset.
    let os = match detect_os() {
        Some(os) => os,
        None => {
            // If OS detection fails, log an error and abort. This is a fundamental requirement.
            log_error!(
                "[GitHub Installer] Unable to detect the current operating system. Aborting installation for {}.",
                tool_entry.name.to_string().red()
            );
            return None;
        }
    };

    let arch = match detect_architecture() {
        Some(arch) => arch,
        None => {
            // Similarly, if architecture detection fails, log an error and abort.
            log_error!(
                "[GitHub Installer] Unable to detect the current machine architecture. Aborting installation for {}.",
                tool_entry.name.to_string().red()
            );
            return None;
        }
    };

    log_info!(
        "[GitHub Installer] Detected platform for {}: {}{}{}",
        tool_entry.name.bold(),
        os.green(),
        "-".green(),
        arch.green()
    );

    // 2. Validate Tool Configuration for GitHub Source
    // For a GitHub release installation, both `tag` (the release version) and `repo` (the GitHub
    // repository slug, e.g., "owner/repo_name") are absolutely essential. Without them, we can't
    // even form the API request to fetch release information.
    let tag = match tool_entry.tag.as_ref() {
        Some(t) => t,
        None => {
            log_error!(
                "[GitHub Installer] Configuration error: 'tag' field is missing for tool {}. Cannot download from GitHub.",
                tool_entry.name.to_string().red()
            );
            return None;
        }
    };

    let repo = match tool_entry.repo.as_ref() {
        Some(r) => r,
        None => {
            log_error!(
                "[GitHub Installer] Configuration error: 'repo' field is missing for tool {}. Cannot download from GitHub.",
                tool_entry.name.to_string().red()
            );
            return None;
        }
    };

    // 3. Fetch GitHub Release Information via API
    // Construct the GitHub API endpoint URL for the specific repository and release tag.
    // Example: https://api.github.com/repos/cli/cli/releases/tags/v2.5.0
    let api_url = format!(
        "https://api.github.com/repos/{}/releases/tags/{}",
        repo, tag
    );
    log_debug!(
        "[GitHub Installer] Fetching release information from GitHub API: {}",
        api_url.blue()
    );

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
            log_error!(
                "[GitHub Installer] Failed to fetch GitHub release for {} ({}): {}",
                tool_entry.name.to_string().red(),
                repo.to_string().red(),
                e
            );
            return None;
        }
    };

    // Beyond network errors, we must check the HTTP status code. A 4xx or 5xx status
    // indicates an API-level error, such as a non-existent repository or tag (404 Not Found).
    if response.status() >= 400 {
        log_error!(
            "[GitHub Installer] GitHub API returned an error status (HTTP {}) for {} release {}. Check if the repo and tag are correct.",
            response.status(),
            repo.to_string().red(),
            tag.to_string().red()
        );
        return None;
    }

    // // Parse the successful JSON response body into our `Release` struct.
    // // This deserialization uses `serde_json` and requires the `Release` struct
    // // to correctly mirror the expected GitHub API JSON structure.
    let release: Release = match response.into_json() {
        Ok(json) => json,
        Err(err) => {
            // Log if JSON parsing fails, indicating an unexpected API response format.
            log_error!(
                "[GitHub Installer] Failed to parse GitHub release JSON for {}: {}",
                tool_entry.name.to_string().red(),
                err
            );
            return None;
        }
    };
    // 4. Find the Correct Asset for the Current Platform
    // A GitHub release can have multiple assets (downloadable files). We need to identify
    // all specific assets that are compatible with the detected OS and architecture.
    // The `asset_matches_platform` utility function contains the logic for this.
    let mut platform_matching_assets: Vec<&ReleaseAsset> = release
        .assets
        .iter()
        .filter(|asset| asset_matches_platform(&asset.name, &os, &arch))
        .collect();

    let asset = if platform_matching_assets.is_empty() {
        // If no matching asset is found after filtering, this is a critical failure.
        // Provide informative error messages, including a list of available assets to help with debugging.
        let available_assets = release
            .assets
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>();
        log_error!(
            "[GitHub Installer] No suitable release asset found for platform {}-{} in repo {} tag {}. \
         Please check the release assets on GitHub. Available assets: {}",
            os.to_string().red(),
            arch.to_string().red(),
            repo.to_string().red(),
            tag.to_string().red(),
            available_assets.join(", ").to_string().yellow()
        );
        return None;
    } else {
        // Prioritization Logic `dmg` or `pkg` for macOS
        // Sort the platform-matching assets. We want .pkg or .dmg files to come first.
        // This is a common heuristic for macOS installations, as these formats often
        // provide a more complete and guided installation experience compared to raw binaries
        // or archives that require manual placement.
        platform_matching_assets.sort_by(|a, b| {
            let a_is_pkg_dmg = a.name.ends_with(".pkg") || a.name.ends_with(".dmg");
            let b_is_pkg_dmg = b.name.ends_with(".pkg") || b.name.ends_with(".dmg");

            match (a_is_pkg_dmg, b_is_pkg_dmg) {
                (true, false) => std::cmp::Ordering::Less, // 'a' is pkg/dmg, 'b' is not -> 'a' comes first
                (false, true) => std::cmp::Ordering::Greater, // 'b' is pkg/dmg, 'a' is not -> 'b' comes first
                _ => std::cmp::Ordering::Equal, // Both or neither are pkg/dmg -> maintain original order (or add another tie-breaker like size/name)
            }
        });

        // After sorting, the most preferred asset (pkg/dmg if present) will be at the front.
        // We can safely unwrap here because we checked `is_empty()` above.
        let selected_asset = platform_matching_assets.first().unwrap(); // Get the first (highest priority) asset

        log_debug!(
            "[GitHub Installer]Found matching asset: {}",
            selected_asset.name.bold()
        );
        selected_asset
    };

    // Once the correct asset is identified, extract its download URL.
    let download_url = &asset.browser_download_url;
    log_debug!(
        "[GitHub Installer] Download URL for selected asset: {}",
        download_url.dimmed()
    );

    // 5. Download the Asset
    // We download the asset to a temporary directory to avoid cluttering the user's system
    // with intermediate files. This temporary directory is managed by the `tempfile` crate,
    // ensuring it's cleaned up automatically when it goes out of scope.
    // Create a single, unique temporary directory for this entire installation run.
    // This ensures isolation between different tool installations.
    let install_temp_root = match TempFileBuilder::new()
        .prefix(&format!("setup-devbox-install-{}-", tool_entry.name))
        .tempdir()
    {
        Ok(dir) => dir,
        Err(e) => {
            log_error!(
                "[GitHub Installer] Failed to create temporary directory for installation: {}. Aborting for {}.",
                e,
                tool_entry.name.to_string().red()
            );
            return None;
        }
    };

    let filename = &asset.name; // Use the original filename from the asset.
    let downloaded_asset_path = install_temp_root.path().join(filename); // Construct the full path for the downloaded file.

    log_debug!(
        "[GitHub Installer] Downloading {} to temporary location: {}",
        tool_entry.name.to_string().bold(),
        downloaded_asset_path.display().to_string().cyan()
    );
    if let Err(err) = download_file(download_url, &downloaded_asset_path) {
        // Log specific download errors (e.g., network issues during download).
        log_error!(
            "[GitHub Installer] Failed to download tool {} from {}: {}",
            tool_entry.name.to_string().red(),
            download_url.to_string().red(),
            err
        );
        return None;
    }
    log_info!(
        "[GitHub Installer] Download completed for {}.",
        tool_entry.name.to_string().bright_blue()
    );

    // 6. Detect Downloaded File Type
    // We need to know the file type (e.g., "zip", "tar.gz", "binary", "pkg") to determine
    // the appropriate installation strategy (extraction, direct move, package installation).
    // `detect_file_type` is preferred here for archives because the filename
    // usually clearly indicates the compression format, which is more reliable than
    // `file` command output for archives.
    let file_type = detect_file_type(&downloaded_asset_path);
    log_debug!(
        "[GitHub Installer] Detected downloaded file type (from filename): {}",
        file_type.to_string().magenta()
    );

    // 7. Determine Final Installation Path
    // Tools are typically installed into a `bin/` directory within the user's home directory
    // (e.g., `~/.local/bin/` or `~/bin/`). We need to retrieve the `$HOME` environment variable.
    let home_dir = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => {
            // The `$HOME` variable is fundamental for user-specific installations.
            log_error!(
                "[GitHub Installer] The $HOME environment variable is not set. Cannot determine installation path for {}.",
                tool_entry.name.to_string().red()
            );
            return None;
        }
    };

    // The final executable name can be explicitly specified in `tools.yaml` via `rename_to`.
    // If not specified, we default to the tool's original `name`.
    let bin_name = tool_entry
        .rename_to
        .clone()
        .unwrap_or_else(|| tool_entry.name.clone());
    // Construct the full absolute path where the tool's executable will be placed.
    let install_path = PathBuf::from(format!("{}/bin/{}", home_dir, bin_name));
    log_debug!(
        "[GitHub Installer] Target installation path for {}: {}",
        tool_entry.name.to_string().bright_blue(),
        install_path.display().to_string().cyan()
    );
    // Introduce a mutable variable to store the actual final install path for the state.
    // Initialize it with the default binary path, then update for pkg/dmg if they install elsewhere.
    let mut final_install_path_for_state = install_path.clone();
    // This variable will hold the determined package type for the ToolState, allowing us to
    // record how the tool was installed (e.g., "binary", "macos-pkg-installer").
    let package_type_for_state: String;

    // Variable to track the working directory for additional commands execution.
    // For archives, this will be the extraction directory; for binaries/pkg/dmg, the download directory.
    let mut additional_cmd_working_dir = install_temp_root.path().to_path_buf();

    // 8. Install Based on File Type
    // This `match` statement serves as the primary dispatcher for installation logic,
    // branching based on the detected `file_type`. Each branch handles a
    // specific type of asset:
    match file_type.as_str() {
        "pkg" => {
            log_info!(
                "[GitHub Installer] Installing .pkg file for {}.",
                tool_entry.name.to_string().bold()
            );
            // Call install_pkg and capture its returned installation path.
            // .pkg installers typically install to predefined system locations (e.g., /Applications),
            // so we need to get the actual path from the installation function.
            match install_pkg(&downloaded_asset_path, &tool_entry.name) {
                Ok(path) => {
                    // Store the actual install path for ToolState, which might differ from `install_path`.
                    final_install_path_for_state = path;
                    // Set package_type to "macos-pkg-installer" to reflect the installation method.
                    package_type_for_state = "macos-pkg-installer".to_string();
                }
                Err(err) => {
                    log_error!(
                        "[GitHub Installer] Failed to install .pkg for {}: {}",
                        tool_entry.name.to_string().red(),
                        err
                    );
                    return None;
                }
            }
        }
        "dmg" => {
            log_info!(
                "[GitHub Installer] Installing .dmg file for {}.",
                tool_entry.name.to_string().bold()
            );
            // Call install_dmg and capture its returned installation path.
            // .dmg installers also install to specific locations (often /Applications or /Volumes).
            match install_dmg(&downloaded_asset_path, &tool_entry.name) {
                Ok(path) => {
                    // Store the actual install path for ToolState.
                    final_install_path_for_state = path;
                    // Set package_type to "macos-dmg-installer" to reflect the installation method.
                    package_type_for_state = "macos-dmg-installer".to_string();
                }
                Err(err) => {
                    log_error!(
                        "[GitHub Installer] Failed to install .dmg for {}: {}",
                        tool_entry.name.to_string().red(),
                        err
                    );
                    return None;
                }
            }
        }
        "binary" => {
            // Handles direct executable files (e.g., a single `.exe` or uncompressed binary).
            // These files don't need extraction; they just need to be moved and made executable.
            log_debug!(
                "[GitHub Installer] Moving standalone binary for {}.",
                tool_entry.name.to_string().bold()
            );
            if let Err(err) = move_and_rename_binary(&downloaded_asset_path, &install_path) {
                log_error!(
                    "[GitHub Installer] Failed to move binary for {}: {}",
                    tool_entry.name.to_string().red(),
                    err
                );
                return None;
            }
            log_debug!(
                "[GitHub Installer] Making binary executable for {}.",
                tool_entry.name.to_string().bold()
            );
            if let Err(err) = make_executable(&install_path) {
                log_error!(
                    "[GitHub Installer] Failed to make binary executable for {}: {}",
                    tool_entry.name.to_string().red(),
                    err
                );
                return None;
            }
            // Set package_type to "binary" as it's a direct binary installation.
            package_type_for_state = "binary".to_string();
        }
        // Handles common archive formats. For these, extraction is required, followed by
        // finding the actual executable within the extracted contents.
        "zip" | "tar.gz" | "gz" | "tar.bz2" | "tar" | "tar.xz" | "tar.bz" | "txz" | "tbz2" => {
            log_debug!(
                "[GitHub Installer] Extracting archive for {}.",
                tool_entry.name.to_string().blue()
            );
            // Extract the downloaded archive to the temporary installation root.
            let extracted_path = match compression::extract_archive(
                &downloaded_asset_path,
                &install_temp_root.path(),
                Some(&file_type),
            ) {
                Ok(path) => path,
                Err(err) => {
                    log_error!(
                        "[GitHub Installer] Failed to extract archive for {}: {}",
                        tool_entry.name.to_string().red(),
                        err
                    );
                    return None;
                }
            };

            log_debug!(
                "[GitHub Installer] Searching for executable in extracted contents for {} in {}",
                tool_entry.name.to_string().blue(),
                extracted_path.display().to_string().cyan()
            );

            // For additional commands, we want to use the same directory where we found the executable
            // or the root of the extracted content, whichever contains the actual tool files
            let content_root_path = extracted_path.clone();
            log_debug!(
                "[GitHub Installer] Archived assets root path is {}",
                content_root_path.display().to_string().cyan()
            );

            // Many archives contain nested directories (e.g., `tool-v1.0.0/bin/tool_executable`).
            // `find_executable` recursively searches the extracted contents to locate the actual binary
            // we need to install. It handles cases where the binary might be in a subdirectory.
            let executable_path = match find_executable(
                &extracted_path,
                &tool_entry.name,
                tool_entry.rename_to.as_deref(),
            ) {
                Some(path) => path,
                None => {
                    log_error!(
                        "[GitHub Installer] No executable found in the extracted archive for {}. Manual intervention may be required.",
                        tool_entry.name.to_string().red()
                    );
                    return None;
                }
            };

            // Determine the directory containing the actual tool content for additional commands
            // If the executable is in a subdirectory of the extraction, use that subdirectory as the base
            // for additional commands so they can find sibling files like 'runtime', 'config', etc.
            if let Some(parent_dir) = executable_path.parent() {
                // Check if the executable is in a bin/ subdirectory or similar
                // If so, we want to go up one level to find sibling directories like 'runtime'
                if parent_dir
                    .file_name()
                    .map(|name| name.to_str().unwrap_or(""))
                    == Some("bin")
                {
                    if let Some(grandparent) = parent_dir.parent() {
                        // Use the grandparent (e.g., helix-25.07.1/) as the content root
                        additional_cmd_working_dir = grandparent.to_path_buf();
                        log_debug!(
                            "[GitHub Installer] Using grandparent directory for additional commands: {}",
                            additional_cmd_working_dir.display().to_string().cyan()
                        );
                    } else {
                        additional_cmd_working_dir = content_root_path;
                    }
                } else {
                    // Executable is not in a bin/ directory, use its parent directory
                    additional_cmd_working_dir = parent_dir.to_path_buf();
                    log_debug!(
                        "[GitHub Installer] Using parent directory for additional commands: {}",
                        additional_cmd_working_dir.display().to_string().cyan()
                    );
                }
            } else {
                // Fallback to the extraction root if we can't determine the parent
                additional_cmd_working_dir = content_root_path;
                log_debug!(
                    "[GitHub Installer] Using extraction root for additional commands: {}",
                    additional_cmd_working_dir.display().to_string().cyan()
                );
            }
            log_debug!(
                "[GitHub Installer] Moving and renaming executable for {}.",
                tool_entry.name.to_string().blue()
            );
            // Move the located executable to its final destination and apply any `rename_to` rule.
            if let Err(err) = move_and_rename_binary(&executable_path, &install_path) {
                log_error!(
                    "[GitHub Installer] Failed to move extracted binary for {}: {}",
                    tool_entry.name.to_string().red(),
                    err
                );
                return None;
            }
            log_debug!(
                "[GitHub Installer] Making extracted binary executable for {}.",
                tool_entry.name.to_string().blue()
            );
            // Ensure the final binary has executable permissions set. This is crucial for Unix-like
            // systems to allow the user to run the installed tool directly.
            if let Err(err) = make_executable(&install_path) {
                log_error!(
                    "[GitHub Installer] Failed to make extracted binary executable for {}: {}",
                    tool_entry.name.to_string().red(),
                    err
                );
                return None;
            }
            // Set package_type to "binary" as it's a direct binary installation after extraction.
            package_type_for_state = "binary".to_string();
        }
        unknown => {
            // Catch-all for unsupported or unrecognized file types. If we download something
            // we don't know how to handle, we log an error and abort.
            log_error!(
                "[GitHub Installer] Unsupported or unknown file type '{}' for tool {}. Cannot install.",
                unknown.to_string().red(),
                tool_entry.name.to_string().red()
            );
            return None;
        }
    }

    // 9. Execute Additional Commands (if specified)
    // After the main installation is complete, execute any additional commands specified
    // in the tool configuration. These commands are often used for post-installation setup,
    // such as copying configuration files, creating directories, or setting up symbolic links.
    // Optional - failure won't stop installation
    let executed_additional_commands = execute_post_installation_commands(
        "[GitHub Installer]",
        tool_entry,
        &additional_cmd_working_dir,
    );
    // If execution reaches this point, the installation was successful.
    log_info!(
        "[GitHub Installer] Installation of {} completed successfully at {}!",
        tool_entry.name.to_string().bold(),
        final_install_path_for_state.display().to_string().green()
    );

    // 10. Return ToolState for Tracking
    // Construct a `ToolState` object to record the details of this successful installation.
    // This `ToolState` will be serialized to `state.json`, allowing `devbox` to track
    // what tools are installed, where they are, and how they were installed. This is crucial
    // for future operations like uninstallation, updates, or syncing.
    Some(ToolState {
        // The version field for tracking. Defaults to "latest" if not explicitly set in `tools.yaml`.
        version: tool_entry
            .version
            .clone()
            .unwrap_or_else(|| "latest".to_string()),
        // The canonical path where the tool's executable was installed. This is the path
        // that will be recorded in the `state.json` file.
        install_path: final_install_path_for_state.display().to_string(),
        // Flag indicating that this tool was installed by `setup-devbox`. This helps distinguish
        // between tools managed by our system and those installed manually.
        installed_by_devbox: true,
        // The method of installation, useful for future diagnostics or differing update logic.
        // In this module, it's always "GitHub".
        install_method: "github".to_string(),
        // Records if the binary was renamed during installation, storing the new name.
        renamed_to: tool_entry.rename_to.clone(),
        // Persist the GitHub repository slug, important for future sync/update checks.
        repo: tool_entry.repo.clone(),
        // Persist the GitHub tag (release version), important for future sync/update checks.
        tag: tool_entry.tag.clone(),
        // The actual package type detected by the `file` command or inferred. This is for diagnostic
        // purposes, providing the most accurate type even if the installation logic
        // used a filename-based guess (e.g., "binary", "macos-pkg-installer").
        package_type: package_type_for_state,
        // Pass any custom options defined in the `ToolEntry` to the `ToolState`.
        options: tool_entry.options.clone(),
        // For direct URL installations: The original URL from which the tool was downloaded.
        // This is important for re-downloading or verifying in the future.
        url: Some(download_url.clone()),
        // Record the timestamp when the tool was installed or updated
        last_updated: Some(current_timestamp()),
        // This field is currently `None` but could be used to store the path to an executable
        // *within* an extracted archive if `install_path` points to the archive's root.
        executable_path_after_extract: None,
        // Record any additional commands that were executed during installation.
        // This is useful for tracking what was done and potentially for cleanup during uninstall.
        additional_cmd_executed: executed_additional_commands,
    })
}

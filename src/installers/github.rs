//! # GitHub Release Installer Module
//!
//! This module provides a robust, production-grade installer for tools distributed as GitHub releases.
//! It follows the same reliability standards as official package managers with comprehensive
//! error handling, verification mechanisms, and accurate platform detection.
//!
//! ## Key Features
//!
//! - **Smart Platform Detection**: Automatically detects OS and architecture for correct asset selection
//! - **Comprehensive Asset Handling**: Supports binaries, archives (zip, tar.gz, etc.), and macOS packages (pkg, dmg)
//! - **Asset Prioritization**: Intelligently selects the best asset for the platform with macOS package preference
//! - **Comprehensive Validation**: Validates GitHub API responses, download integrity, and installation success
//! - **Smart State Tracking**: Maintains accurate installation state with version tracking
//! - **Flexible Configuration**: Supports repository specifications, version tags, and custom binary names
//! - **Post-Installation Hooks**: Executes additional setup commands after successful installation
//! - **Temporary File Management**: Properly cleans up temporary files and directories
//!
//! ## Installation Workflow
//!
//! The installer follows a meticulous 10-step process:
//!
//! 1. **Platform Detection** - Detects OS and architecture for asset selection
//! 2. **Configuration Validation** - Validates required repository and tag fields
//! 3. **GitHub API Integration** - Fetches release information from GitHub API
//! 4. **Asset Selection** - Finds and prioritizes platform-appropriate assets
//! 5. **Asset Download** - Downloads the selected asset to temporary location
//! 6. **File Type Detection** - Determines installation strategy based on file type
//! 7. **Installation Path Resolution** - Determines final binary installation path
//! 8. **Asset Processing** - Handles extraction, installation, or direct binary placement
//! 9. **Post-Installation Hooks** - Executes any additional setup commands
//! 10. **State Creation** - Creates comprehensive tool state for persistence
//!
//! ## Error Handling
//!
//! The module provides detailed error messages and logging at multiple levels:
//! - **Info**: High-level installation progress
//! - **Debug**: Detailed API calls, asset selection, and path resolution
//! - **Warn**: Non-fatal issues or warnings during installation
//! - **Error**: Installation failures with specific error codes and messages

// External crate imports
use colored::Colorize;

// Utility imports
use crate::libs::tool_installer::execute_post_installation_hooks;
use crate::libs::utilities::assets;
use crate::libs::utilities::{
    assets::detect_file_type,
    platform::{asset_matches_platform, detect_architecture, detect_os},
};

// Schema imports
use crate::schemas::common::{Release, ReleaseAsset};
use crate::schemas::state_file::ToolState;
use crate::schemas::tools::ToolEntry;

// Custom logging macros
use crate::{log_debug, log_error, log_info};

/// Installs a software tool by fetching its release asset from GitHub releases.
///
/// This function provides a robust installer for GitHub-hosted tools that mirrors the quality
/// and reliability of official package managers. It includes comprehensive validation,
/// smart asset selection, and accurate state tracking.
///
/// # Workflow
///
/// 1. **Platform Detection**: Detects OS and architecture for asset selection
/// 2. **Configuration Validation**: Validates required repository and tag fields
/// 3. **GitHub API Integration**: Fetches release information from GitHub API
/// 4. **Asset Selection**: Finds and prioritizes platform-appropriate assets
/// 5. **Asset Download**: Downloads the selected asset to temporary location
/// 6. **File Type Detection**: Determines installation strategy based on file type
/// 7. **Asset Processing**: Handles extraction, installation, or direct binary placement
/// 8. **Post-Installation Hooks**: Executes any additional setup commands
/// 9. **State Creation**: Creates comprehensive `ToolState` with all relevant metadata
///
/// # Arguments
///
/// * `tool_entry` - A reference to the `ToolEntry` struct containing tool configuration
///   - `tool_entry.name`: **Required** - The tool name
///   - `tool_entry.repo`: **Required** - GitHub repository in "owner/repo" format
///   - `tool_entry.tag`: **Required** - Release tag/version (e.g., "v1.0.0")
///   - `tool_entry.rename_to`: Optional custom binary name
///   - `tool_entry.options`: Optional additional configuration
///
/// # Returns
///
/// An `Option<ToolState>`:
/// * `Some(ToolState)` if installation was completely successful with accurate metadata
/// * `None` if any step of the installation process fails
///
/// # Examples - YAML Configuration
///
/// ```yaml
/// # GitHub CLI tool
/// # https://github.com/cli/cli
/// - name: gh
///   source: github
///   repo: cli/cli
///   tag: v2.50.0
///
/// # Kubernetes package manager with custom name
/// # https://github.com/helm/helm
/// - name: helm
///   source: github
///   repo: helm/helm
///   tag: v3.17.0
///   rename_to: helm3
///
/// # Static site generator
/// # https://github.com/gohugoio/hugo
/// - name: hugo
///   source: github
///   repo: gohugoio/hugo
///   tag: v0.140.0
/// ```
///
/// # Examples - Rust Code
///
/// ```rust
/// // Basic installation
/// let tool_entry = ToolEntry {
///     name: "gh".to_string(),
///     repo: Some("cli/cli".to_string()),
///     tag: Some("v2.50.0".to_string()),
///     rename_to: None,
///     options: None,
/// };
/// install(&tool_entry);
///
/// // Installation with custom binary name
/// let tool_entry = ToolEntry {
///     name: "helm".to_string(),
///     repo: Some("helm/helm".to_string()),
///     tag: Some("v3.17.0".to_string()),
///     rename_to: Some("helm3".to_string()),
///     options: None,
/// };
/// install(&tool_entry);
/// ```
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    log_info!(
        "[SDB::Tools::GitHubInstaller] Attempting to install tool: {}",
        tool_entry.name.bold()
    );
    log_debug!(
        "[SDB::Tools::GitHubInstaller] ToolEntry details: {:#?}",
        tool_entry
    );

    // Step 1: Detect platform (OS and architecture) for asset selection
    let (os, arch) = detect_platform()?;

    // Step 2: Validate GitHub configuration - ensure required fields are present
    let (repo, tag) = validate_github_configuration(tool_entry)?;

    // Step 3: Fetch release information from GitHub API
    log_debug!("[SDB::Tools::GitHubInstaller] Fetching release information for {repo}/{tag}");
    let release = fetch_github_release(repo, tag)?;

    // Step 4: Select appropriate asset for the detected platform
    log_debug!("[SDB::Tools::GitHubInstaller] Selecting asset for {os}-{arch}");
    let asset = select_platform_asset(&release, &os, &arch)?;

    // Step 5: Download asset to temporary location
    log_debug!(
        "[SDB::Tools::GitHubInstaller] Downloading asset: {}",
        asset.name.bold()
    );
    let (temp_dir, downloaded_path) =
        assets::download_url_asset(tool_entry, &asset.browser_download_url)?;

    // Step 6: Detect file type and determine installation strategy
    let file_type = detect_file_type(&downloaded_path);
    log_debug!(
        "[SDB::Tools::GitHubInstaller] Detected file type: {}",
        file_type.to_string().magenta()
    );

    // Step 7: Process asset based on file type (binary, archive, or macOS package)
    let (package_type, final_install_path, working_dir) =
        assets::process_asset_by_type(tool_entry, &downloaded_path, &file_type, &temp_dir)?;

    // Step 8: Execute any post-installation hooks defined in tool configuration
    log_debug!(
        "[SDB::Tools::GitHubInstaller] Executing post-installation hooks for {}",
        tool_entry.name.bold()
    );
    let executed_post_installation_hooks =
        execute_post_installation_hooks("[SDB::Tools::GitHubInstaller]", tool_entry, &working_dir);

    log_info!(
        "[SDB::Tools::GitHubInstaller] Successfully installed tool: {} (version: {})",
        tool_entry.name.bold().green(),
        tag.green()
    );

    // Step 9: Return comprehensive ToolState for state tracking and persistence
    Some(ToolState::new(
        tool_entry,
        &final_install_path,
        "github".to_string(),
        package_type,
        tool_entry.version.clone()?.to_string(),
        Some(asset.browser_download_url.clone()),
        None,
        executed_post_installation_hooks,
    ))
}

/// Detects the current platform (OS and architecture).
///
/// This function detects both the operating system and CPU architecture,
/// which are essential for selecting the correct GitHub release asset.
/// Platform detection ensures that the downloaded binary is compatible
/// with the user's system.
///
/// # Returns
///
/// * `Some((os, arch))` - A tuple containing the OS and architecture strings if both are detected
/// * `None` - If either OS or architecture detection fails
///
/// # Examples
///
/// Typical return values:
/// - `Some(("darwin", "arm64"))` - macOS on Apple Silicon
/// - `Some(("darwin", "x86_64"))` - macOS on Intel
/// - `Some(("linux", "x86_64"))` - Linux on x86_64
/// - `Some(("windows", "x86_64"))` - Windows on x86_64
fn detect_platform() -> Option<(String, String)> {
    // Detect operating system (darwin, linux, windows, etc.)
    let os = detect_os().or_else(|| {
        log_error!("[SDB::Tools::GitHubInstaller] Unable to detect operating system");
        None
    })?;

    // Detect CPU architecture (x86_64, arm64, aarch64, etc.)
    let arch = detect_architecture().or_else(|| {
        log_error!("[SDB::Tools::GitHubInstaller] Unable to detect architecture");
        None
    })?;

    log_info!(
        "[SDB::Tools::GitHubInstaller] Detected platform: {}{}{}",
        os.green(),
        "-".green(),
        arch.green()
    );

    Some((os, arch))
}

/// Validates that the tool configuration contains required GitHub fields.
///
/// This function checks that both the repository and tag fields are specified
/// in the tool configuration, as they are mandatory for GitHub release installations.
/// Without these fields, the installer cannot locate or download the correct release.
///
/// # Arguments
///
/// * `tool_entry` - The tool configuration to validate
///
/// # Returns
///
/// * `Some((repo, tag))` - References to the repository and tag strings if both are present
/// * `None` - If either field is missing, with appropriate error logging
///
/// # Configuration Requirements
///
/// - `repo`: Must be in "owner/repo" format (e.g., "cli/cli", "helm/helm")
/// - `tag`: Must match a valid release tag in the repository (e.g., "v1.0.0", "1.0.0")
fn validate_github_configuration(tool_entry: &ToolEntry) -> Option<(&String, &String)> {
    // Verify repository field is present
    let repo = tool_entry.repo.as_ref().or_else(|| {
        log_error!(
            "[SDB::Tools::GitHubInstaller] Configuration error: 'repo' field is missing for tool {}",
            tool_entry.name.red()
        );
        log_error!(
            "[SDB::Tools::GitHubInstaller] Expected format: 'repo: owner/repository' (e.g., 'repo: cli/cli')"
        );
        None
    })?;

    // Verify tag field is present
    let tag = tool_entry.tag.as_ref().or_else(|| {
        log_error!(
            "[SDB::Tools::GitHubInstaller] Configuration error: 'tag' field is missing for tool {}",
            tool_entry.name.red()
        );
        log_error!("[SDB::Tools::GitHubInstaller] Expected format: 'tag: v1.0.0' or 'tag: 1.0.0'");
        None
    })?;

    Some((repo, tag))
}

/// Fetches release information from the GitHub API.
///
/// This function makes an HTTP request to the GitHub releases API to retrieve
/// detailed information about a specific release, including all available assets.
/// The GitHub API returns comprehensive metadata about the release, which is used
/// to select and download the appropriate asset for the current platform.
///
/// # Arguments
///
/// * `repo` - The repository in "owner/repo" format (e.g., "cli/cli")
/// * `tag` - The release tag/version (e.g., "v2.50.0")
///
/// # Returns
///
/// * `Some(Release)` - Parsed release data if the API call is successful
/// * `None` - If the API call fails or returns invalid data
///
/// # API Details
///
/// - Endpoint: `https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}`
/// - User-Agent: "setup-devbox" (required by GitHub API)
/// - Response: JSON containing release metadata and asset list
///
/// # Error Handling
///
/// Failures can occur due to:
/// - Network connectivity issues
/// - Invalid repository or tag names
/// - Rate limiting (60 requests/hour for unauthenticated requests)
/// - Repository not found or private repository without authentication
fn fetch_github_release(repo: &str, tag: &str) -> Option<Release> {
    // Construct GitHub API URL for the specific release
    let api_url = format!("https://api.github.com/repos/{repo}/releases/tags/{tag}");
    log_debug!("[SDB::Tools::GitHubInstaller] API URL: {}", api_url.blue());

    // Make HTTP GET request with required User-Agent header
    let response = match ureq::get(&api_url).set("User-Agent", "setup-devbox").call() {
        Ok(resp) => resp,
        Err(e) => {
            log_error!(
                "[SDB::Tools::GitHubInstaller] Failed to fetch GitHub release for {}/{}: {}",
                repo.red(),
                tag.red(),
                e
            );
            log_error!(
                "[SDB::Tools::GitHubInstaller] This could be due to network issues, invalid repo/tag, or rate limiting"
            );
            return None;
        }
    };

    // Check for HTTP error status codes (4xx, 5xx)
    if response.status() >= 400 {
        log_error!(
            "[SDB::Tools::GitHubInstaller] GitHub API error (HTTP {}) for {}/{}",
            response.status(),
            repo.red(),
            tag.red()
        );

        // Provide helpful context for common error codes
        match response.status() {
            404 => log_error!(
                "[SDB::Tools::GitHubInstaller] Release not found. Verify the repository and tag are correct."
            ),
            403 => log_error!(
                "[SDB::Tools::GitHubInstaller] Rate limit exceeded or access forbidden. Consider authenticating for higher limits."
            ),
            _ => {}
        }
        return None;
    }

    // Parse JSON response into Release struct
    match response.into_json() {
        Ok(release) => Some(release),
        Err(err) => {
            log_error!(
                "[SDB::Tools::GitHubInstaller] Failed to parse GitHub release JSON for {}/{}: {}",
                repo.red(),
                tag.red(),
                err
            );
            log_error!("[GitHub Installer] The API response may be malformed or incomplete");
            None
        }
    }
}

/// Selects the most appropriate asset for the current platform.
///
/// This function filters release assets by platform compatibility and prioritizes
/// certain asset types to provide the best installation experience. For macOS,
/// it prefers .pkg and .dmg installers over raw binaries or archives when available,
/// as these provide better system integration and user experience.
///
/// # Arguments
///
/// * `release` - The GitHub release containing a list of available assets
/// * `os` - The target operating system (e.g., "darwin", "linux", "windows")
/// * `arch` - The target architecture (e.g., "x86_64", "arm64", "aarch64")
///
/// # Returns
///
/// * `Some(&ReleaseAsset)` - Reference to the best matching asset if found
/// * `None` - If no suitable asset is found for the platform
///
/// # Asset Selection Strategy
///
/// 1. Filter all assets for platform compatibility (OS and architecture match)
/// 2. Prioritize asset types in this order (for macOS):
///    - `.pkg` files (macOS installer packages)
///    - `.dmg` files (macOS disk images)
///    - Other formats (binaries, archives)
/// 3. Return the highest priority matching asset
///
/// # Error Handling
///
/// If no matching assets are found, the function logs all available assets
/// to help diagnose configuration or platform detection issues.
fn select_platform_asset<'a>(
    release: &'a Release,
    os: &str,
    arch: &str,
) -> Option<&'a ReleaseAsset> {
    // Filter assets to only those matching the current platform
    let mut matching_assets: Vec<&ReleaseAsset> = release
        .assets
        .iter()
        .filter(|asset| asset_matches_platform(&asset.name, os, arch))
        .collect();

    // Handle case where no assets match the platform
    if matching_assets.is_empty() {
        let available_assets: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();
        log_error!(
            "[SDB::Tools::GitHubInstaller] No suitable asset found for platform {}-{}.",
            os.red(),
            arch.red()
        );
        log_error!(
            "[SDB::Tools::GitHubInstaller] Available assets: {}",
            available_assets.join(", ").yellow()
        );
        log_error!(
            "[SDB::Tools::GitHubInstaller] This release may not support your platform, or asset naming doesn't match expected patterns"
        );
        return None;
    }

    // Sort assets to prioritize macOS packages (.pkg and .dmg files)
    // These provide better integration with macOS than raw binaries or archives
    matching_assets.sort_by(|a, b| {
        let a_is_macos_pkg = a.name.ends_with(".pkg") || a.name.ends_with(".dmg");
        let b_is_macos_pkg = b.name.ends_with(".pkg") || b.name.ends_with(".dmg");

        match (a_is_macos_pkg, b_is_macos_pkg) {
            (true, false) => std::cmp::Ordering::Less, // a is pkg/dmg, b is not - prefer a
            (false, true) => std::cmp::Ordering::Greater, // b is pkg/dmg, a is not - prefer b
            _ => std::cmp::Ordering::Equal, // Both are or neither are pkg/dmg - no preference
        }
    });

    // Select the first (highest priority) asset after sorting
    let selected_asset = matching_assets.first().unwrap();
    log_debug!(
        "[SDB::Tools::GitHubInstaller] Selected asset: {}",
        selected_asset.name.bold()
    );

    Some(selected_asset)
}

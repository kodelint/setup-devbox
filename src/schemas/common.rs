// Defines the data structures (schemas) for configuration files and the application's internal state.
// Serde traits for serialization and deserialization.
use serde::{Deserialize, Serialize};

/// Represents a downloadable asset associated with a GitHub release.
///
/// This struct captures metadata about individual files available for download
/// from a GitHub release, such as binaries, archives, or other distributables.
#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    /// The filename of the asset as it appears on GitHub.
    ///
    /// # Example
    /// ```text
    /// "tool-v1.0.0-linux-x86_64.tar.gz"
    /// "application-v2.1.0-windows-amd64.zip"
    /// ```
    pub(crate) name: String,

    /// The direct URL for downloading the asset file.
    ///
    /// This URL can be used to programmatically download the asset without
    /// requiring authentication (for public repositories) or with appropriate
    /// authentication headers for private repositories.
    ///
    /// # Example
    /// ```text
    /// "https://github.com/owner/repo/releases/download/v1.0.0/tool-v1.0.0-linux-x86_64.tar.gz"
    /// ```
    pub(crate) browser_download_url: String,
}

/// Represents a GitHub release with its associated downloadable assets.
///
/// This struct captures the release information returned by the GitHub API,
/// focusing primarily on the assets available for download. It's typically
/// used when fetching release information to locate and download specific files.
///
/// # Usage
/// ```rust
/// // Typically used with serde to deserialize GitHub API responses:
/// let release: Release = serde_json::from_str(api_response)?;
///
/// // Access assets to find specific files:
/// for asset in &release.assets {
///     if asset.name.contains("linux") {
///         println!("Found Linux asset: {}", asset.browser_download_url);
///     }
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct Release {
    /// A collection of downloadable assets associated with this release.
    ///
    /// This vector contains all files (binaries, archives, checksums, etc.)
    /// that are available for download from this specific release. The vector
    /// may be empty if the release has no associated assets.
    pub(crate) assets: Vec<ReleaseAsset>,
}

// Main Application Configuration
/// Defines the main application configuration file,
/// pointing to paths for other detailed configuration files.
/// This is the entry point configuration that references all other config files.
#[derive(Debug, Serialize, Deserialize)]
pub struct MainConfig {
    /// Optional path to `tools.yaml` configuration file
    pub tools: Option<String>,
    /// Optional path to `settings.yaml` configuration file
    pub settings: Option<String>,
    /// Optional path to `shellac.yaml` configuration file
    pub shellrc: Option<String>,
    /// Optional path to `fonts.yaml` configuration file
    pub fonts: Option<String>,
}

/// Structure representing the main config.yaml file that contains paths to other config files
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigPaths {
    pub(crate) tools: String,
    pub(crate) settings: String,
    pub(crate) shellrc: String,
    pub(crate) fonts: String,
}

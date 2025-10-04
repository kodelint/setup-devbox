//! # Common Data Structures and Configuration Schemas
//!
//! This module defines the core data structures and configuration schemas used throughout
//! the `setup-devbox` application. It includes structures for GitHub API responses,
//! main application configuration, and shared data types that are used across multiple
//! modules and components.
//!
//! ## Core Components
//!
//! - **GitHub API Structures**: Data models for interacting with GitHub Releases API
//! - **Main Configuration**: Top-level application configuration structure
//! - **Configuration Paths**: Unified structure for configuration file locations
//!
//! ## Serialization Support
//!
//! All structures implement Serde's `Serialize` and `Deserialize` traits for seamless
//! JSON and YAML serialization/deserialization, enabling easy configuration file handling
//! and API response parsing.

use serde::{Deserialize, Serialize};

// ============================================================================
// GITHUB API DATA STRUCTURES
// ============================================================================

/// Represents a downloadable asset associated with a GitHub release.
///
/// This struct captures metadata about individual files available for download
/// from a GitHub release, such as binaries, archives, or other distributables.
/// It is primarily used when parsing GitHub API responses to locate and download
/// specific release assets for tool installation.
///
/// ## Asset Types
/// Supports various asset types commonly found in GitHub releases:
/// - Compiled binaries for different platforms
/// - Source code archives (.tar.gz, .zip)
/// - Checksum files for verification
/// - Installation scripts and packages
/// - Documentation and release notes
#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    /// The filename of the asset as it appears on GitHub.
    ///
    /// This is the exact filename as uploaded to the GitHub release, which
    /// typically includes version information, platform, and architecture
    /// details for easy identification.
    ///
    /// ## Naming Patterns
    /// Common naming conventions include:
    /// - `{tool}-{version}-{platform}-{arch}.tar.gz`
    /// - `{tool}_{version}_{platform}_amd64.zip`
    /// - `{tool}-{version}-x86_64-unknown-linux-gnu.tar.xz`
    ///
    /// # Example
    /// ```text
    /// "tool-v1.0.0-linux-x86_64.tar.gz"
    /// "application-v2.1.0-windows-amd64.zip"
    /// "binary-v3.0.0-macos-arm64"
    /// ```
    pub(crate) name: String,

    /// The direct URL for downloading the asset file.
    ///
    /// This URL can be used to programmatically download the asset without
    /// requiring authentication (for public repositories) or with appropriate
    /// authentication headers for private repositories. The URL points directly
    /// to the asset file on GitHub's CDN.
    ///
    /// ## URL Structure
    /// Follows GitHub's release asset URL pattern:
    /// `https://github.com/{owner}/{repo}/releases/download/{tag}/{filename}`
    ///
    /// ## Authentication
    /// - Public repositories: No authentication required
    /// - Private repositories: Requires GitHub token with appropriate permissions
    /// - Rate limiting: Subject to GitHub API rate limits
    ///
    /// # Example
    /// ```text
    /// "https://github.com/owner/repo/releases/download/v1.0.0/tool-v1.0.0-linux-x86_64.tar.gz"
    /// "https://github.com/rust-lang/rust/releases/download/1.70.0/rust-1.70.0-x86_64-unknown-linux-gnu.tar.gz"
    /// ```
    pub(crate) browser_download_url: String,
}

/// Represents a GitHub release with its associated downloadable assets.
///
/// This struct captures the release information returned by the GitHub API,
/// focusing primarily on the assets available for download. It's typically
/// used when fetching release information to locate and download specific files
/// for tool installation from GitHub releases.
///
/// ## API Integration
/// This structure is designed to deserialize directly from GitHub API responses,
/// making it easy to work with the GitHub Releases API without custom parsing logic.
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
///
/// ## Rate Limiting Considerations
/// When using this with the GitHub API, be mindful of:
/// - API rate limits (60 requests/hour unauthenticated, 5000/hour authenticated)
/// - Conditional requests using `ETags` for caching
/// - Pagination for repositories with many releases
#[derive(Debug, Deserialize)]
pub struct Release {
    /// A collection of downloadable assets associated with this release.
    ///
    /// This vector contains all files (binaries, archives, checksums, etc.)
    /// that are available for download from this specific release. The vector
    /// may be empty if the release has no associated assets.
    ///
    /// ## Asset Filtering
    /// Typically filtered based on:
    /// - Platform-specific patterns in asset names
    /// - File extensions (`.tar.gz`, `.zip`, `.pkg`, etc.)
    /// - Architecture requirements (i`x86_64`, `arm64`, etc.)
    /// - Tool-specific naming conventions
    ///
    /// ## Empty Releases
    /// Some releases may contain only source code or documentation without
    /// pre-built binaries, resulting in an empty assets vector.
    pub(crate) assets: Vec<ReleaseAsset>,
}

// ============================================================================
// MAIN APPLICATION CONFIGURATION
// ============================================================================

/// Defines the main application configuration file structure.
///
/// This is the entry point configuration that references all other configuration
/// files. It serves as the top-level configuration that points to specialized
/// configuration files for tools, settings, shell, and fonts.
///
/// ## File Location
/// Typically named `config.yaml` and located at:
/// - `~/.setup-devbox/config.yaml` (user-specific configuration)
/// - Project-specific locations for environment-specific configurations
///
/// ## Configuration Hierarchy
/// The main config acts as a manifest that references other configuration files:
/// ```yaml
/// tools: "~/.setup-devbox/tools.yaml"
/// settings: "~/.setup-devbox/settings.yaml"
/// shellrc: "~/.setup-devbox/shellrc.yaml"
/// fonts: "~/.setup-devbox/fonts.yaml"
/// ```
///
/// ## Optional Fields
/// All fields are optional, allowing partial configurations where only some
/// aspects of the development environment are managed.
#[derive(Debug, Serialize, Deserialize)]
pub struct MainConfig {
    /// Optional path to `tools.yaml` configuration file.
    ///
    /// Specifies the location of the tool installation configuration file,
    /// which defines what tools to install and how to install them.
    ///
    /// ## Default Behavior
    /// If not specified, the system will look for `tools.yaml` in default
    /// locations or skip tool installation if no configuration is found.
    ///
    /// ## Path Expansion
    /// Supports environment variable expansion and tilde expansion:
    /// - `~/path/to/tools.yaml` → User home directory
    /// - `$HOME/path/to/tools.yaml` → User home directory
    /// - `$SDB_CONFIG/tools.yaml` → Custom config directory
    pub tools: Option<String>,

    /// Optional path to `settings.yaml` configuration file.
    ///
    /// Specifies the location of the system settings configuration file,
    /// which defines OS-specific preferences and system configurations.
    ///
    /// ## Default Behavior
    /// If not specified, the system will look for `settings.yaml` in default
    /// locations or skip settings configuration if none is found.
    pub settings: Option<String>,

    /// Optional path to `shellrc.yaml` configuration file.
    ///
    /// Specifies the location of the shell configuration file, which defines
    /// shell aliases, environment variables, and shell customization.
    ///
    /// ## Note on Field Name
    /// The field is named `shellrc` for consistency with the concept of
    /// "shell run commands" while the file is typically named `shellac.yaml`.
    ///
    /// ## Default Behavior
    /// If not specified, the system will look for `shellrc.yaml` in default
    /// locations or skip shell configuration if none is found.
    pub shellrc: Option<String>,

    /// Optional path to `fonts.yaml` configuration file.
    ///
    /// Specifies the location of the font configuration file, which defines
    /// what fonts to install and their sources.
    ///
    /// ## Default Behavior
    /// If not specified, the system will look for `fonts.yaml` in default
    /// locations or skip font installation if none is found.
    pub fonts: Option<String>,
}

// ============================================================================
// CONFIGURATION PATHS STRUCTURE
// ============================================================================

/// Structure representing the main `config.yaml` file that contains paths to other config files.
///
/// This structure provides a unified way to handle all configuration file paths
/// in a single object. It's typically used internally after parsing the main
/// configuration to resolve and validate all configuration file locations.
///
/// ## Difference from `MainConfig`
/// While `MainConfig` uses `Option<String>` for optional paths, `ConfigPaths`
/// uses `String` for required paths, representing the resolved and validated
/// configuration file locations after processing the main configuration.
///
/// ## Path Resolution
/// This structure contains the final, resolved paths after:
/// - Environment variable expansion
/// - Tilde expansion for home directory
/// - Default path fallback resolution
/// - Path validation and existence checking
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigPaths {
    /// Resolved path to the tools configuration file.
    ///
    /// After processing the main configuration, this contains the absolute
    /// path to the tools.yaml file that will be used for tool installation.
    pub(crate) tools: String,

    /// Resolved path to the settings configuration file.
    ///
    /// After processing the main configuration, this contains the absolute
    /// path to the settings.yaml file that will be used for system settings.
    pub(crate) settings: String,

    /// Resolved path to the shell configuration file.
    ///
    /// After processing the main configuration, this contains the absolute
    /// path to the shellac.yaml file that will be used for shell customization.
    pub(crate) shellrc: String,

    /// Resolved path to the fonts configuration file.
    ///
    /// After processing the main configuration, this contains the absolute
    /// path to the fonts.yaml file that will be used for font installation.
    pub(crate) fonts: String,
}

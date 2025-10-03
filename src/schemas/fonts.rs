//! # Font Configuration Schema
//!
//! This module defines the data structures for font management and installation
//! configuration. These structures are used to parse and generate `fonts.yaml`
//! configuration files that specify which fonts to download and install from
//! various sources, with support for filtering specific font styles and weights.
//!
//! ## Configuration File Structure
//!
//! The `fonts.yaml` file follows this structure:
//! ```yaml
//! fonts:
//!   - name: "FiraCode"
//!     version: "6.2"
//!     source: "github"
//!     repo: "tonsky/FiraCode"
//!     tag: "v6.2"
//!     install_only: ["Regular", "Bold", "Mono"]
//!
//!   - name: "JetBrainsMono"
//!     source: "github"
//!     repo: "JetBrains/JetBrainsMono"
//!     tag: "v2.304"
//!     install_only: ["NL", "ExtraBold"]
//! ```
//!
//! ## Font Sources
//!
//! Currently, supports fonts from GitHub releases, with architecture designed
//! for future extensibility to other font sources such as:
//! - Direct URL downloads
//! - Font repositories and CDNs
//! - Package managers (brew, apt, etc.)
//! - Local font files
//!
//! ## Font Filtering
//!
//! The `install_only` feature allows selective installation of specific font
//! styles and weights from a font family, reducing disk space usage and
//! installation time when only certain variants are needed.

use serde::{Deserialize, Serialize};

// ============================================================================
// TOP-LEVEL FONT CONFIGURATION
// ============================================================================

/// Configuration schema for `fonts.yaml`.
///
/// Defines the complete structure for managing and installing custom fonts
/// from various sources. This file configures font installation with support
/// for versioning, source specification, and selective style installation.
///
/// ## File Location
///
/// The system searches for source configuration files in this priority order:
/// - `~/.setup-devbox/configs/fonts.yaml` (default)
/// - It also supports ENV Variables
///   `SDB_CONFIG_PATH` -> `$SDB_CONFIG_PATH/configs/fonts.yaml`
///
/// ## Installation Process
/// Fonts are installed to the appropriate system font directories:
/// - **macOS**: `~/Library/Fonts/` (user) or `/Library/Fonts/` (system)
/// - **Linux**: `~/.local/share/fonts/` (user) or `/usr/share/fonts/` (system)
/// - **Windows**: `%USERPROFILE%\AppData\Local\Microsoft\Windows\Fonts\`
///
/// ## Font Management Features
/// - Version pinning for reproducible environments
/// - Source tracking for updates and provenance
/// - Selective style installation to save space
/// - Conflict detection for existing font installations
#[derive(Debug, Serialize, Deserialize)]
pub struct FontConfig {
    /// List of individual font entries to be installed.
    ///
    /// Each entry defines a font family to download and install, with
    /// specifications for source, version, and optional style filtering.
    /// Fonts are processed in the order they appear in this list.
    ///
    /// ## Installation Order
    /// Fonts are installed sequentially, which can be important for:
    /// - Dependency resolution (if some fonts depend on others)
    /// - Conflict management (later fonts may override earlier ones)
    /// - Performance optimization (grouping similar sources)
    ///
    /// ## Validation
    /// Each font entry is validated before installation to ensure:
    /// - Required fields are present for the specified source
    /// - Version strings are properly formatted
    /// - Repository URLs are valid and accessible
    pub fonts: Vec<FontEntry>,
}

// ============================================================================
// INDIVIDUAL FONT ENTRIES
// ============================================================================

/// Represents a single font entry defined by the user in `fonts.yaml`.
///
/// Each entry defines how a specific font should be downloaded and installed,
/// including source information, version requirements, and optional style
/// filtering for selective installation.
///
/// ## Font Identification
/// The `name` field serves as the primary identifier for the font family
/// and is used for state tracking, conflict detection, and display purposes.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct FontEntry {
    /// Name of the font (for identification and state tracking).
    ///
    /// This is the canonical name of the font family as it will be recognized
    /// by the system and applications. It should match the common name used
    /// by the font source when possible.
    ///
    /// ## Naming Conventions
    /// - Use the font family name without spaces or special characters
    /// - Follow the naming used by the source repository or provider
    /// - Be consistent across configurations for reliable state tracking
    ///
    /// ## Examples
    /// - `"FiraCode"` (Fira Code)
    /// - `"JetBrainsMono"` (JetBrains Mono)
    /// - `"Hack"` (Hack font)
    /// - `"SourceCodePro"` (Source Code Pro)
    ///
    /// ## State Tracking
    /// The name is used as the key in the state file to track:
    /// - Installation status and version
    /// - Installed font files and styles
    /// - Source information for updates
    pub name: String,

    /// Desired font version (optional).
    ///
    /// Specifies which version of the font to install. When not specified,
    /// the latest available version from the source will be used.
    ///
    /// ## Version Format
    /// Typically follows semantic versioning or release numbering:
    /// - `"6.2"` (`Fira Code v6.2`)
    /// - `"v2.304"` (`JetBrains Mono v2.304`)
    /// - `"3.0.1"` (`Hack v3.0.1`)
    ///
    /// ## Update Behavior
    /// - With specific version: Only updated if version changes in config
    /// - Without version (`None`): Treated as "latest", updated based on policy
    /// - "latest" string: Explicitly request latest version
    ///
    /// ## Source Compatibility
    /// Version format should match the tagging convention used by the source:
    /// - GitHub releases often use `"vX.Y.Z"` format
    /// - Some fonts use simple version numbers like `"6.2"`
    pub version: Option<String>,

    /// Source for the font (e.g., "GitHub", "nerd-fonts").
    ///
    /// Specifies where the font should be downloaded from. Determines which
    /// downloader and installation logic to use for this font.
    ///
    /// ## Supported Sources
    /// - `"github"`: GitHub releases (most common for open-source fonts)
    /// - `"nerd-fonts"`: Nerd Fonts patched fonts (specialized source)
    /// - Future support: `"url"`, `"local"`, `"package"`
    ///
    /// ## Source-Specific Requirements
    /// Each source type requires different additional fields:
    /// - **GitHub**: Requires `repo` field, optional `tag`
    /// - **Nerd-Fonts**: Requires specific repository patterns
    /// - **URL**: Would require `url` field for direct downloads
    ///
    /// ## Validation
    /// The source field is validated against supported sources, and
    /// appropriate required fields are checked for the specified source.
    pub source: String,

    /// GitHub repository for the font (if source is GitHub).
    ///
    /// Specifies the GitHub repository where the font is hosted, in the
    /// format `"owner/repository"`. Required when `source` is `"github"`.
    ///
    /// ## Repository Format
    /// Must follow GitHub's `owner/repository` naming convention:
    /// - `"tonsky/FiraCode"` (`Fira Code font`)
    /// - `"JetBrains/JetBrainsMono"` (`JetBrains Mono`)
    /// - `"source-foundry/Hack"` (`Hack font`)
    ///
    /// ## Release Discovery
    /// The system uses the GitHub API to discover available releases and
    /// assets in the specified repository, then filters for font files.
    ///
    /// ## Asset Matching
    /// Font files are identified by file extension (.ttf, .otf, .woff, .woff2)
    /// and naming patterns that match the font family name.
    pub repo: Option<String>,

    /// Specific GitHub tag/release (if source is GitHub).
    ///
    /// Specifies which release to download from the GitHub repository.
    /// When not specified, the latest release will be used.
    ///
    /// ## Tag Format
    /// Typically matches GitHub release tags:
    /// - `"v6.2"` (version tags with 'v' prefix)
    /// - `"6.2"` (version tags without prefix)
    /// - `"release-6.2"` (release-specific tags)
    /// - Commit hashes or other release identifiers
    ///
    /// ## Special Values
    /// - `None`: Use the latest release
    /// - `"latest"`: Explicitly use the latest release
    /// - Specific tag: Use that exact release
    ///
    /// ## Validation
    /// The tag must match an actual release tag in the repository, otherwise
    /// the installation will fail with a "release not found" error.
    pub tag: Option<String>,

    /// Optional list of keywords for filtering specific font files to install.
    ///
    /// This allows installing only certain font weights/styles from a font family,
    /// reducing disk space usage and installation time when only specific
    /// variants are needed.
    ///
    /// ## Filtering Behavior
    /// Only font files whose names contain any of the specified keywords
    /// will be installed. Other font files in the release will be ignored.
    ///
    /// ## Common Use Cases
    /// - Install only specific weights: `["Regular", "Bold", "Light"]`
    /// - Install only mono variants: `["Mono", "NL"]` (No-Ligatures)
    /// - Install specific styles: `["Italic", "Oblique"]`
    /// - Combination filters: `["Mono", "Bold", "Italic"]`
    ///
    /// ## Examples
    /// ```yaml
    /// install_only: ["Regular", "Bold"]       # Only Regular and Bold weights
    /// install_only: ["Mono"]                  # Only mono-spaced variants
    /// install_only: ["Italic", "Oblique"]     # Only italic styles
    /// install_only: ["NL"]                    # Only no-ligatures versions
    /// ```
    ///
    /// ## Default Behavior
    /// `#[serde(default)]` ensures that if not specified, the field is `None`
    /// and all font files from the release will be installed.
    ///
    /// ## Serialization Behavior
    /// `#[serde(skip_serializing_if = "Option::is_none")]` omits this field
    /// from serialized YAML when `None`, keeping configuration files clean.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_only: Option<Vec<String>>,
}

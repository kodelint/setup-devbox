use serde::{Deserialize, Serialize};

/// Configuration schema for `fonts.yaml`.
/// Defines the structure for managing and installing custom fonts.
/// This file configures font installation from various sources.
#[derive(Debug, Serialize, Deserialize)]
pub struct FontConfig {
    /// List of individual font entries to be installed
    pub fonts: Vec<FontEntry>,
}

/// Represents a single font entry defined by the user in `fonts.yaml`.
/// Each entry defines how a specific font should be downloaded and installed.
#[derive(Debug, Serialize, Deserialize)]
pub struct FontEntry {
    /// Name of the font (for identification and state tracking)
    pub name: String,
    /// Desired font version (optional)
    pub version: Option<String>,
    /// Source for the font (e.g., "GitHub", "nerd-fonts")
    pub source: String,
    /// GitHub repository for the font (if source is GitHub)
    pub repo: Option<String>,
    /// Specific GitHub tag/release (if source is GitHub)
    pub tag: Option<String>,
    /// Optional list of keywords for filtering specific font files to install
    /// This allows installing only certain font weights/styles from a font family
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_only: Option<Vec<String>>,
}

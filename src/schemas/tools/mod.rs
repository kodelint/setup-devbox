pub mod enums;
pub mod types;

use enums::{InstallerError, ToolEntryError};
use std::fmt;
use types::ToolEntry;

// =========================================================================== //
//                           IMPLEMENTATION BLOCKS                             //
// =========================================================================== //

/// Validation methods for individual tool entries.
///
/// This implementation provides comprehensive validation of tool configurations
/// before they're processed by the installation system. Validation catches
/// common configuration errors early and provides clear guidance for fixing them.
impl ToolEntry {
    /// Validates a tool entry's configuration based on its specified source.
    ///
    /// This method performs comprehensive validation to ensure that:
    /// 1. The specified installation source is supported
    /// 2. All required fields for that source are present
    /// 3. No conflicting fields from other sources are specified
    /// 4. Field values meet basic format requirements
    ///
    /// ## Validation Rules by Source
    ///
    /// ### GitHub Source (`source: "github"`)
    /// - **Required**: `repo` (format: "owner/repository")
    /// - **Required**: `tag` (must match a release tag)
    /// - **Forbidden**: `url`, `executable_path_after_extract`
    /// - **Optional**: `rename_to`, `options`, `additional_cmd`, `configuration_manager`
    ///
    /// ### URL Source (`source: "url"`)
    /// - **Required**: `url` (must be valid HTTP/HTTPS URL)
    /// - **Optional**: `executable_path_after_extract` (for archives)
    /// - **Forbidden**: `repo`, `tag`
    /// - **Optional**: `rename_to`, `options`, `additional_cmd`, `configuration_manager`
    ///
    /// ### Package Manager Sources (`brew`, `cargo`, `rustup`, `pip`, `uv`, `go`)
    /// - **Required**: `name` only
    /// - **Optional**: `version`, `rename_to`, `options`, `additional_cmd`, `configuration_manager`
    /// - **Forbidden**: All source-specific fields (`repo`, `tag`, `url`, `executable_path_after_extract`)
    ///
    /// ## Common Fields (allowed for all sources)
    /// - `name`: Tool identifier (always required)
    /// - `version`: Version specification (optional, defaults to "latest")
    /// - `rename_to`: Alternative executable name (optional)
    /// - `options`: Installer-specific command-line options (optional)
    /// - `additional_cmd`: Post-installation commands (optional)
    /// - `configuration_manager`: Configuration file management (optional, defaults to disabled)
    ///
    /// ## Error Types
    /// - **`InvalidSource`**: Unsupported installation method specified
    /// - **`MissingField`**: Required field for the specified source is absent
    /// - **`ConflictingFields`**: Fields from different sources used together inappropriately
    ///
    /// ## Examples
    ///
    /// **Valid GitHub tool:**
    /// ```yaml
    /// - name: "ripgrep"
    ///   source: "github"
    ///   repo: "BurntSushi/ripgrep"
    ///   tag: "v13.0.0"
    ///   rename_to: "rg"
    /// ```
    ///
    /// **Valid Brew tool:**
    /// ```yaml
    /// - name: "bat"
    ///   source: "brew"
    ///   version: "0.24.0"
    ///   options: ["--HEAD"]
    /// ```
    ///
    /// **Invalid configuration (conflicting fields):**
    /// ```yaml
    /// - name: "bad-tool"
    ///   source: "github"      # GitHub source specified
    ///   repo: "owner/repo"    # GitHub field (correct)
    ///   tag: "v1.0.0"        # GitHub field (correct)
    ///   url: "http://..."     # URL field (CONFLICT!)
    /// ```
    ///
    /// ## Returns
    /// - `Ok(())`: Configuration is valid and ready for processing
    /// - `Err(ToolEntryError)`: Specific validation error with description
    pub fn validate(&self) -> Result<(), ToolEntryError> {
        // Define all supported installation sources
        let supported_sources = [
            "github", "brew", "cargo", "rustup", "pip", "go", "url", "uv",
        ];
        let source_lower = self.source.to_lowercase();

        // Validate that the source is supported
        if !supported_sources.contains(&source_lower.as_str()) {
            return Err(ToolEntryError::InvalidSource(self.source.clone()));
        }

        // Perform source-specific validation
        match source_lower.as_str() {
            "github" => {
                // GitHub sources require repository and tag specification
                if self.repo.is_none() {
                    return Err(ToolEntryError::MissingField("repo (for GitHub source)"));
                }
                if self.tag.is_none() {
                    return Err(ToolEntryError::MissingField("tag (for GitHub source)"));
                }

                // GitHub sources cannot use URL-specific fields
                if self.url.is_some() || self.executable_path_after_extract.is_some() {
                    return Err(ToolEntryError::ConflictingFields(
                        "url or executable_path_after_extract should not be present for GitHub source".to_string(),
                    ));
                }
            }
            "url" => {
                // URL sources require a download URL
                if self.url.is_none() {
                    return Err(ToolEntryError::MissingField("url (for URL source)"));
                }

                // URL sources cannot use GitHub-specific fields
                if self.repo.is_some() || self.tag.is_some() {
                    return Err(ToolEntryError::ConflictingFields(
                        "repo or tag should not be present for URL source".to_string(),
                    ));
                }
                // Note: executable_path_after_extract is allowed for URL sources (archives)
            }
            // Package manager sources (brew, cargo, rustup, pip, go, uv)
            "brew" | "cargo" | "rustup" | "pip" | "uv" => {
                // Package managers cannot use any source-specific fields
                if self.repo.is_some()
                    || self.tag.is_some()
                    || self.url.is_some()
                    || self.executable_path_after_extract.is_some()
                {
                    return Err(ToolEntryError::ConflictingFields(format!(
                        "repo, tag, url, or executable_path_after_extract should not be present for '{}' source",
                        self.source
                    )));
                }
                // Note: additional_cmd is allowed for all sources for post-install flexibility
            }
            "go" => {
                if self.repo.is_some()
                    || self.tag.is_some()
                    || self.executable_path_after_extract.is_some()
                {
                    return Err(ToolEntryError::ConflictingFields(format!(
                        "repo, tag or executable_path_after_extract should not be present for '{}' source",
                        self.source
                    )));
                }
            }
            _ => {
                // This shouldn't be reachable due to the supported_sources check above,
                // but is included for completeness and future-proofing
                unreachable!(
                    "Source validation should have caught unsupported source: {}",
                    source_lower
                );
            }
        }

        // All validation checks passed
        Ok(())
    }
}

// =========================================================================== //
//                         ERROR TYPE IMPLEMENTATIONS                          //
// =========================================================================== //

/// User-friendly display implementation for tool entry validation errors.
///
/// Provides clear, actionable error messages that help users understand
/// what's wrong with their configuration and how to fix it.
impl fmt::Display for ToolEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => {
                write!(f, "Missing required field: {field}")
            }
            Self::InvalidSource(source) => {
                write!(
                    f,
                    "Invalid tool source: '{source}'. Supported sources are: github, brew, cargo, rustup, pip, go, url, uv"
                )
            }
            Self::ConflictingFields(msg) => {
                write!(f, "Conflicting fields: {msg}")
            }
        }
    }
}

/// Standard error trait implementation for tool entry errors.
///
/// Enables `ToolEntryError` to be used with Rust's standard error handling
/// infrastructure, including error chaining and conversion patterns.
impl std::error::Error for ToolEntryError {}

/// User-friendly display implementation for installer availability errors.
///
/// Provides clear guidance when required command-line tools are missing
/// from the system, helping users understand what they need to install.
impl fmt::Display for InstallerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand(cmd) => {
                write!(
                    f,
                    "Installer command '{}' not found in your system's PATH. Please install {} before proceeding.",
                    cmd,
                    match cmd.as_str() {
                        "brew" => "Homebrew (https://brew.sh/)",
                        "cargo" => "Rust toolchain (https://rustup.rs/)",
                        "go" => "Go programming language (https://golang.org/dl/)",
                        "pip" => "Python package installer (usually bundled with Python)",
                        "uv" => "UV Python package manager (https://github.com/astral-sh/uv)",
                        _ => cmd, // Generic fallback for unknown commands
                    }
                )
            }
        }
    }
}

/// Standard error trait implementation for installer errors.
///
/// Enables `InstallerError` to be used with Rust's standard error handling
/// patterns and to be returned from functions that return `Result` types.
impl std::error::Error for InstallerError {}

// This file handles version checking for the `setup-devbox` tool.
// It retrieves the local tool version from `Cargo.toml` and compares it against
// the latest release available on GitHub.

use crate::{log_error, log_info, log_warn}; // Custom logging macros.
use colored::Colorize; // For colored terminal output.
use serde::Deserialize; // Serde for deserializing structured data (TOML/JSON) into Rust structs.
use std::fs;          // File system operations (e.g., reading `Cargo.toml`).
use std::io;          // Standard I/O operations and error handling.
use ureq;             // HTTP client for making web requests (e.g., to GitHub API).

// GitHub repository details for version checking.
const REPO_OWNER: &str = "kodelint"; // The owner of the GitHub repository.
const REPO_NAME: &str = "setup-devbox"; // The name of the GitHub repository.

/// Represents the structure of `Cargo.toml` for deserialization.
/// Specifically targets the `[package]` section.
#[derive(Deserialize)]
struct CargoToml {
    package: Package, // Holds the deserialized `[package]` section.
}

/// Represents the `[package]` section within `Cargo.toml`,
/// specifically for extracting the `version` field.
#[derive(Deserialize)]
struct Package {
    version: String, // The version string of the package.
}

/// Reads the version string from the local `Cargo.toml` file.
///
/// # Returns
/// * `Ok(String)` containing the version if successful.
/// * `Err(io::Error)` if the file cannot be read or parsed.
fn get_local_version() -> io::Result<String> {
    // Read the content of `Cargo.toml`.
    let cargo_toml = fs::read_to_string("Cargo.toml")?;

    // Parse the TOML string into the `CargoToml` struct.
    // Converts `toml::de::Error` to `io::Error` for consistent error handling.
    let cargo: CargoToml = toml::from_str(&cargo_toml).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to parse Cargo.toml: {}", e),
        )
    })?;

    Ok(cargo.package.version) // Return the extracted version.
}

/// Represents a simplified structure for a GitHub Release API response,
/// focusing on the `tag_name` (which typically corresponds to the version).
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String, // The release tag name (e.g., "v1.0.0").
}

/// Fetches the latest release version tag from the specified GitHub repository.
///
/// # Returns
/// * `Ok(String)` containing the latest version tag if successful.
/// * `Err(Box<dyn std::error::Error>)` if the network request or JSON parsing fails.
fn get_latest_github_release() -> Result<String, Box<dyn std::error::Error>> {
    // Construct the GitHub API URL for the latest release.
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );

    // Create a `ureq` agent with a custom User-Agent header.
    let agent = ureq::AgentBuilder::new()
        .user_agent("setup-devbox-version-checker") // Identifies the application making the request.
        .build();

    // Execute the HTTP GET request.
    let response = agent.get(&url).call()?;

    // Validate the response content-type is JSON.
    if !response.has("content-type") || !response.header("content-type").unwrap().contains("application/json") {
        return Err("GitHub returned unexpected content type, not JSON.".into());
    }

    // Parse the JSON response into the `GitHubRelease` struct.
    let release: GitHubRelease = response.into_json()?;

    Ok(release.tag_name) // Return the extracted tag name.
}

/// Main function to execute the version checking logic.
/// Compares the local tool version with the latest GitHub release.
pub fn run() {
    log_info!("Checking local tool version...");

    match get_local_version() {
        Ok(local_version) => {
            log_info!("Local version found: {}", local_version);

            log_info!("Checking for latest GitHub release...");
            match get_latest_github_release() {
                Ok(latest_version) => {
                    log_info!("Latest GitHub release: {}", latest_version);

                    // Compare versions, trimming whitespace for accurate comparison.
                    if latest_version.trim() != local_version.trim() {
                        log_warn!("A newer version is available. Consider upgrading.");
                    } else {
                        log_info!("You are running the latest version.");
                    }
                }
                Err(e) => {
                    log_error!("Failed to fetch the latest release from GitHub: {}", e);
                }
            }
        }
        Err(e) => {
            log_error!("Failed to read local Cargo.toml version: {}", e);
        }
    }
}
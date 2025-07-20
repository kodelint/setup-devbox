// This file handles version checking for the `setup-devbox` tool.
// It retrieves the local tool version from `Cargo.toml` and compares it against
// the latest release available on GitHub. This is crucial for informing users
// if their installed version is outdated, promoting upgrades.

// Custom logging macros: These provide a consistent way to output messages
// at different severity levels (debug, info, error, warn) throughout the application.
use crate::{log_debug, log_error, log_info, log_warn};
// For colored terminal output: Enhances readability of log messages.
use colored::Colorize;
// Serde for deserializing structured data: Used to parse TOML (for `Cargo.toml`)
// and JSON (for GitHub API responses) into Rust structs.
use serde::Deserialize;
// Standard library for file system operations: Needed to read the `Cargo.toml` file.
use std::fs;
// Standard I/O operations and error handling: Used for file operations and general error types.
use std::io;
// HTTP client for making web requests: Essential for communicating with the GitHub API.
use ureq;

// GitHub repository details for version checking.
// These constants define the specific repository to check for releases.
const REPO_OWNER: &str = "kodelint"; // The GitHub username or organization that owns the repository.
const REPO_NAME: &str = "setup-devbox"; // The name of the repository to check for releases.

/// Represents the top-level structure of a `Cargo.toml` file relevant to this module.
/// This struct is used by Serde to deserialize the TOML content.
#[derive(Deserialize)]
struct CargoToml {
    // The `package` field directly maps to the `[package]` section in `Cargo.toml`.
    package: Package,
}

/// Represents the `[package]` section within `Cargo.toml`.
/// Specifically designed to extract the `version` field.
#[derive(Deserialize)]
struct Package {
    // The `version` field holds the package's version string, e.g., "0.1.0".
    version: String,
}

/// Reads the version string from the local `Cargo.toml` file.
///
/// This function performs the following steps:
/// 1. Reads the entire content of the `Cargo.toml` file into a string.
/// 2. Parses the TOML string into the `CargoToml` struct using `toml::from_str`.
/// 3. Extracts and returns the `version` field from the parsed structure.
///
/// # Returns
/// * `Ok(String)`: A `String` containing the version number (e.g., "0.1.0") if the file is read and parsed successfully.
/// * `Err(io::Error)`: An `io::Error` if:
///   - The `Cargo.toml` file cannot be found or read (e.g., file system permissions).
///   - The content of `Cargo.toml` is malformed or cannot be deserialized into the expected `CargoToml` structure.
fn get_local_version() -> io::Result<String> {
    log_debug!("Attempting to read local Cargo.toml...");
    // `fs::read_to_string("Cargo.toml")?` attempts to read the file.
    // The `?` operator propagates any `io::Error` directly from this call.
    let cargo_toml = fs::read_to_string("Cargo.toml")?;
    log_debug!("Successfully read Cargo.toml content.");

    // `toml::from_str(&cargo_toml)` parses the TOML string.
    // `.map_err(...)` is used to convert the `toml::de::Error` (if parsing fails)
    // into an `io::Error`, ensuring a consistent error return type for the function.
    let cargo: CargoToml = toml::from_str(&cargo_toml).map_err(|e| {
        log_error!("Failed to parse Cargo.toml: {}", e);
        io::Error::new(
            io::ErrorKind::Other, // Use `ErrorKind::Other` for parsing errors.
            format!("Failed to parse Cargo.toml: {}", e), // Detailed error message.
        )
    })?;

    log_debug!(
        "Successfully parsed Cargo.toml. Local version: {}",
        cargo.package.version
    );
    Ok(cargo.package.version) // Return the extracted version string.
}

/// Represents a simplified structure for a GitHub Release API response.
/// We are primarily interested in the `tag_name` field, which typically
/// holds the release version (e.g., "v1.0.0", "1.2.3").
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String, // The tag associated with the release.
}

/// Fetches the latest release version tag from the specified GitHub repository.
///
/// This function constructs the GitHub API endpoint for the latest release,
/// performs an HTTP GET request, and parses the JSON response to extract the `tag_name`.
///
/// # Returns
/// * `Ok(String)`: A `String` containing the latest release tag (e.g., "v1.0.0") if successful.
/// * `Err(Box<dyn std::error::Error>)`: A boxed error if:
///   - The HTTP request fails (e.g., network issues, invalid URL).
///   - The response content type is not JSON.
///   - The JSON response cannot be parsed into the `GitHubRelease` struct.
fn get_latest_github_release() -> Result<String, Box<dyn std::error::Error>> {
    log_debug!("Constructing GitHub API URL for latest release.");
    // Construct the GitHub API URL for fetching the latest release.
    // Uses `REPO_OWNER` and `REPO_NAME` constants.
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );
    log_debug!("GitHub API URL: {}", url.blue());

    // Create a `ureq` agent.
    // Setting a `User-Agent` header is important for making polite and identifiable
    // requests to web APIs, as many APIs require or prefer it.
    let agent = ureq::AgentBuilder::new()
        .user_agent("setup-devbox-version-checker") // Custom User-Agent string.
        .build();

    log_debug!("Making HTTP request to GitHub API...");
    // Execute the HTTP GET request. `call()?` sends the request and
    // propagates any `ureq::Error` if the request fails.
    let response = agent.get(&url).call()?;
    log_debug!(
        "Received response from GitHub API. Status: {}",
        response.status()
    );

    // Validate the response content-type.
    // Ensures that we're actually receiving JSON before attempting to parse it.
    if !response.has("content-type")
        || !response
            .header("content-type")
            .unwrap()
            .contains("application/json")
    {
        log_error!(
            "GitHub returned unexpected content type: {:?}",
            response.header("content-type")
        );
        return Err("GitHub returned unexpected content type, not JSON.".into());
    }
    log_debug!("Content-Type is JSON.");

    // Parse the JSON response body into the `GitHubRelease` struct.
    // `into_json()?` attempts to deserialize the response body.
    // Propagates any `serde_json::Error` or `ureq::Error` if deserialization fails.
    let release: GitHubRelease = response.into_json()?;
    log_debug!(
        "Successfully parsed GitHub release JSON. Tag: {}",
        release.tag_name
    );

    Ok(release.tag_name) // Return the extracted `tag_name`.
}

/// Normalizes a version string for robust comparison.
///
/// This function performs several steps to ensure consistent comparison
/// between version strings, addressing common variations like "0.1.0" vs "v0.1.0".
///
/// Steps:
/// 1. `trim()`: Removes leading and trailing whitespace.
/// 2. `trim_start_matches(|c| c == 'v' || c == 'V')`: Removes a leading 'v' or 'V'
///    character. This handles both lowercase and uppercase 'v' prefixes.
/// 3. `chars().filter(|c| c.is_ascii()).collect::<String>()`: Filters out any
///    non-ASCII characters that might be present (though less common in versions).
///    This creates a new `String`.
/// 4. `to_ascii_lowercase()`: Converts the entire resulting string to lowercase ASCII.
///    This ensures that any remaining characters (like "Alpha" vs "alpha") are consistent.
///
/// # Arguments
/// * `version`: A string slice (`&str`) representing the version to normalize.
///
/// # Returns
/// * `String`: The normalized version string.
fn normalize_version(version: &str) -> String {
    log_debug!("Normalizing version: '{}'", version.cyan());
    let normalized = version
        .trim() // Step 1: Remove leading/trailing whitespace.
        .trim_start_matches(|c| c == 'v' || c == 'V') // Step 2: Strip 'v' or 'V' prefix (case-insensitive due to this logic).
        .chars()
        .filter(|c| c.is_ascii()) // Step 3: Filter out non-ASCII characters.
        .collect::<String>() // Collect into a new String.
        .to_ascii_lowercase(); // Step 4: Convert to lowercase ASCII.
    log_debug!("Normalized result: '{}'", normalized.green());
    normalized
}

/// Main function to execute the version checking logic.
///
/// This function orchestrates the entire version check process:
/// 1. Calls `get_local_version()` to retrieve the version from `Cargo.toml`.
/// 2. Calls `get_latest_github_release()` to fetch the latest version from GitHub.
/// 3. Normalizes both versions using `normalize_version()`.
/// 4. Compares the normalized versions to determine if an upgrade is available.
/// 5. Logs appropriate messages to the user based on the comparison result.
pub fn run() {
    log_info!("Comparing the local and latest versions...");
    // Attempt to get the local version.
    match get_local_version() {
        Ok(local_version) => {
            // Attempt to get the latest GitHub release version.
            match get_latest_github_release() {
                Ok(latest_version) => {
                    log_info!(
                        "Local Version: {} and Latest GitHub Release: {}",
                        local_version.bright_yellow().bold(),
                        latest_version.bright_green().bold()
                    );

                    // Normalize both versions for a fair comparison.
                    let norm_local = normalize_version(&local_version);
                    let norm_latest = normalize_version(&latest_version);

                    log_debug!("Final normalized local: '{}'", norm_local.yellow());
                    log_debug!("Final normalized latest: '{}'", norm_latest.green());

                    // Perform the comparison using the normalized versions.
                    if norm_local != norm_latest {
                        log_warn!(
                            "A newer version is available (Local: {}, Latest: {}). Consider upgrading.",
                            local_version.bold(), // Display original versions to user in warning.
                            latest_version.bold()
                        );
                    } else {
                        log_info!(
                            "{}",
                            "You are running the latest version.".bright_blue().bold()
                        );
                    }
                }
                Err(e) => {
                    // Log error if fetching the latest release fails.
                    log_error!("Failed to fetch the latest release from GitHub: {}", e);
                }
            }
        }
        Err(e) => {
            // Log error if reading local Cargo.toml fails.
            log_error!("Failed to read local Cargo.toml version: {}", e);
        }
    }
}

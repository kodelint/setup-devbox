// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_warn};
// The 'colored' crate helps us make our console output look pretty and readab
use colored::Colorize;

/// Checks if a given asset filename from a GitHub release (or similar source)
/// is likely compatible with the current operating system and architecture.
/// This is how `setup-devbox` intelligently selects the correct download asset
/// from a list of available files. It performs fuzzy matching using aliases.
///
/// # Arguments
/// * `filename`: The full filename (`&str`) of the asset (e.g., "mytool_1.0.0_macOS_arm64.tar.gz").
/// * `os`: The current operating system as a normalized string (e.g., "macos").
/// * `arch`: The current architecture as a normalized string (e.g., "arm64").
///
/// # Returns
/// * `bool`: `true` if the filename contains recognizable keywords for the platform's OS and architecture,
///           considering aliases and a special Rosetta 2 fallback for macOS ARM64. `false` otherwise.
pub fn asset_matches_platform(filename: &str, os: &str, arch: &str) -> bool {
    // Convert inputs to lowercase for case-insensitive comparison.
    let asset_name_lower = filename.to_lowercase();
    let os_lower = os.to_lowercase();
    let arch_lower = arch.to_lowercase();

    // 1. Check for OS match:
    // Iterate through all known aliases for the current OS. If any alias is found
    // as a substring within the asset filename, it's considered an OS match.
    let os_matches = os_aliases(&os_lower)
        .iter()
        .any(|alias| asset_name_lower.contains(alias));

    // If no OS match, immediately return false. No need to check architecture.
    if !os_matches {
        log_debug!("[Utils] Asset '{}' does not match OS '{}'", filename.dimmed(), os);
        return false;
    }

    // 2. Check for Architecture match:
    // Iterate through all known aliases for the current architecture. If any alias is found
    // as a substring within the asset filename, it's considered an architecture match.
    let arch_matches = arch_aliases(&arch_lower)
        .iter()
        .any(|alias| asset_name_lower.contains(alias));

    // 3. Special consideration for macOS ARM64 (aarch64) with Rosetta 2 fallback:
    // If the target is macOS ARM64, and the asset filename contains "x86_64" (Intel architecture)
    // but *does not* contain "arm64" or "aarch64" (explicit ARM64),
    // it's considered a potential match because macOS can run x86_64 binaries via Rosetta 2 emulation.
    let rosetta_fallback = (os_lower == "macos" && arch_lower == "arm64") &&
        asset_name_lower.contains("x86_64") &&
        !(asset_name_lower.contains("arm64") || asset_name_lower.contains("aarch64"));

    // If neither a direct architecture match nor the Rosetta fallback condition is met, return false.
    if !(arch_matches || rosetta_fallback) {
        log_debug!("[Utils] Asset '{}' does not match architecture '{}' (and no Rosetta fallback).", filename.dimmed(), arch);
        return false;
    }

    // 4. Optional: Exclude common source, debug, or checksum files.
    // These files are usually not the actual executable binaries we want to download.
    // This helps in picking the actual binary release.
    if asset_name_lower.contains("src") ||
        asset_name_lower.contains("source") ||
        asset_name_lower.contains("debug") ||
        asset_name_lower.contains("checksum") ||
        asset_name_lower.contains("sha256") ||
        asset_name_lower.contains("tar.gz.sig") || // Common signature file for tar.gz
        asset_name_lower.ends_with(".asc") {      // Common detached signature file extension
        log_debug!("[Utils] Asset '{}' excluded due to containing common non-binary keywords.", filename.dimmed());
        return false;
    }

    // If all checks pass, the asset is considered a match for the current platform.
    log_debug!("[Utils] Asset '{}' matches platform (OS: {}, ARCH: {}) -> {}", filename.dimmed(), os.cyan(), arch.magenta(), "true".bold());
    true
}

/// Helper function: Provides a list of common alternative names (aliases) for a given CPU architecture.
/// This is used internally by `asset_matches_platform` to handle different naming conventions
/// for architectures in release asset filenames (e.g., "aarch64" vs "arm64").
///
/// # Arguments
/// * `arch`: A string slice representing a normalized architecture name (e.g., "arm64", "x86_64").
///
/// # Returns
/// * `Vec<String>`: A vector of strings containing the input architecture name and its known aliases.
fn arch_aliases(arch: &str) -> Vec<String> {
    match arch.to_lowercase().as_str() {
        "arm64" => vec!["arm64", "aarch64"] // Aliases for ARM 64-bit.
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        "x86_64" => vec!["x86_64", "amd64"] // Aliases for x86 64-bit.
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        other => vec![other.to_string()], // For unknown architecture, just return the input string.
    }
}

/// Helper function: Provides a list of common alternative names (aliases) for a given operating system.
/// This is used internally by `asset_matches_platform` to improve the flexibility of matching
/// GitHub release asset filenames, which might use various naming conventions for the same OS.
///
/// # Arguments
/// * `os`: A string slice representing a normalized OS name (e.g., "macos", "linux").
///
/// # Returns
/// * `Vec<String>`: A vector of strings containing the input OS name and its known aliases.
fn os_aliases(os: &str) -> Vec<String> {
    match os.to_lowercase().as_str() {
        "macos" => vec!["macos", "darwin", "apple-darwin", "macosx"] // Aliases for macOS.
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        "linux" => vec!["linux"] // Aliases for Linux (currently just "linux" itself).
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        "windows" => vec!["windows", "win32", "win64"] // Aliases for Windows.
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        other => vec![other.to_string()], // For unknown OS, just return the input string.
    }
}

/// Detects the current machine's CPU architecture (e.g., "arm64", "x86_64").
/// This is vital for downloading the correct version of a binary from GitHub releases
/// or other sources that provide platform-specific builds.
///
/// # Returns
/// * `Option<String>`:
///   - `Some(String)` containing the detected architecture as a canonical string
///     (e.g., "arm64" for "aarch64", "x86_64" for "amd64").
///   - `None` if detection somehow fails, though `std::env::consts::ARCH` is generally reliable.
pub fn detect_architecture() -> Option<String> {
    // `std::env::consts::ARCH` provides the target architecture Rust was compiled for
    // (e.g., "aarch64", "x86_64"). This is highly reliable for the running binary.
    // We then pass it to `normalize_arch` to get a consistent string format.
    Some(normalize_arch(std::env::consts::ARCH).to_string())
}

/// Detects the current operating system (e.g., "macos", "linux", "windows").
/// Similar to architecture detection, this is crucial for finding the right software release
/// assets that are built for the specific OS.
///
/// # Returns
/// * `Option<String>`:
///   - `Some(String)` containing the detected OS as a canonical string
///     (e.g., "macOS" for "darwin", "windows" for "win32").
///   - `None` if detection somehow fails, though `std::env::consts::OS` is generally reliable.
pub fn detect_os() -> Option<String> {
    // `std::env::consts::OS` provides the target operating system Rust was compiled for
    // (e.g., "macOS", "linux", "windows").
    // We then pass it to `normalize_os` to get a consistent string format.
    Some(normalize_os(std::env::consts::OS).to_string())
}

/// Normalizes various input strings for operating systems into a consistent, lowercase format.
/// This helps `setup-devbox` deal with different ways OS names might appear in asset names
/// (e.g., "macOS", "Darwin", "OSX") or from system information, mapping them to a common set.
///
/// # Arguments
/// * `os`: An input string (`&str`) representing an OS (e.g., "macOS", "darwin", "Linux").
///
/// # Returns
/// * `String`: The normalized OS string (e.g., "macOS", "linux", "windows").
///             If the input is not a known alias, the lowercase version of the input is returned.
pub fn normalize_os(os: &str) -> String {
    // Convert the input OS string to lowercase for case-insensitive matching.
    match os.to_lowercase().as_str() {
        "macos" | "darwin" | "apple-darwin" => "macos".to_string(), // Map various macOS/Darwin names to "macos".
        "linux" => "linux".to_string(),                             // Linux is typically straightforward.
        "windows" | "win32" | "win64" => "windows".to_string(),     // Map various Windows names to "windows".
        other => {
            // If we encounter an unknown OS variant, log a warning.
            // We return the lowercase version of the unknown string as-is,
            // hoping it might still match some asset names.
            log_warn!("[Utils] Unknown OS variant '{}', using as-is. This might cause issues with asset matching.", other.purple());
            other.to_string()
        }
    }
}

/// Normalizes various input strings for CPU architectures into a consistent, lowercase format.
/// This ensures `setup-devbox` can correctly match architectures (e.g., "aarch64" vs "arm64",
/// or "amd64" vs "x86_64") when parsing asset names from releases.
///
/// # Arguments
/// * `arch`: An input string (`&str`) representing an architecture (e.g., "AARCH64", "x86_64", "amd64").
///
/// # Returns
/// * `String`: The normalized architecture string (e.g., "arm64", "x86_64").
///             If the input is not a known alias, the lowercase version of the input is returned.
pub fn normalize_arch(arch: &str) -> String {
    // Convert the input architecture string to lowercase for case-insensitive matching.
    match arch.to_lowercase().as_str() {
        "aarch64" | "arm64" => "arm64".to_string(), // Map ARM 64-bit variants to "arm64".
        "amd64" | "x86_64" => "x86_64".to_string(), // Map AMD/Intel 64-bit variants to "x86_64".
        other => {
            // If we encounter an unknown architecture variant, log a warning.
            // We return the lowercase version of the unknown string as-is.
            log_warn!("[Utils] Unknown ARCH variant '{}', using as-is. This might cause issues with asset matching.", other.purple());
            other.to_string()
        }
    }
}
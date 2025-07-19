// Internal module imports:
// These `use` statements bring necessary types and functions into scope for this module.

// `ToolEntry`: Represents a single tool's configuration as defined in your `tools.yaml` file.
//              It's a struct that contains all possible configuration fields for a tool,
//              such as name, version, source, URL, repository, etc.
// `ToolState`: Represents the actual state of an *installed* tool. This struct is used to
//              persist information about installed tools in the application's `state.json` file.
//              It helps `setup-devbox` track what's installed, its version, and where it's located.
use crate::schema::{ToolEntry, ToolState};
// Custom logging macros:
// These macros (`log_debug!`, `log_error!`, `log_info!`, `log_warn!`) provide a
// consistent and structured way to output messages at different severity levels.
// They help in debugging, providing user feedback, and indicating critical issues.
use crate::{log_debug, log_error, log_info, log_warn};
// Removed ToolEntryFull as ToolEntry is a struct
// `Colorize` trait from the `colored` crate:
// This trait extends string types, allowing them to be easily colored for improved
// readability in terminal output. For example, `my_string.bold().red()`.
use colored::Colorize;
// `dirs` crate:
// Used to find common user directories, such as the home directory, which is essential
// for determining where `setup-devbox` should store its data and installed tools.
use dirs;
// Standard library `fs` module:
// Provides functions for interacting with the file system, such as creating directories,
// reading/writing files, and managing permissions.
use std::fs;
// `PathBuf` from the standard library `path` module:
// A versatile type for representing and manipulating file paths in an OS-agnostic way.
use std::path::PathBuf;

// Internal utility imports:
// `download_file` function:
// A utility from `crate::libs::utilities::assets` responsible for securely downloading
// a file from a given URL to a specified local path.
use crate::libs::utilities::assets::download_file;
// `extract_archive` function:
// A utility from `crate::libs::utilities::compression` that handles the decompression
// and extraction of various archive formats (e.g., .zip, .tar.gz) to a target directory.
use crate::libs::utilities::compression::extract_archive;


/// Installs a tool by directly downloading it from a specified URL.
///
/// This function serves as the core logic for installing tools that are made available
/// via a direct download link. It handles the entire lifecycle:
/// 1. Validating the input `tool_entry` for a URL source.
/// 2. Determining appropriate installation paths within the `setup-devbox` directory.
/// 3. Downloading the tool's archive or binary.
/// 4. Extracting the contents if it's an archive, or copying if it's a standalone binary.
/// 5. Cleaning up temporary download files.
/// 6. Attempting to set executable permissions for the primary binary on Unix-like systems.
/// 7. Recording the successful installation by returning a `ToolState` object.
///
/// # Arguments
/// * `tool_entry`: A reference (`&`) to a `ToolEntry` struct instance. This contains all
///   the configuration details for the tool to be installed, including the critical
///   `url` field for direct URL installations.
///
/// # Returns
/// An `Option<ToolState>`:
/// * `Some(ToolState)`: Returned if the tool was successfully downloaded, extracted/copied,
///   and its state recorded. This `ToolState` object contains all relevant information
///   about the newly installed tool.
/// * `None`: Returned if any step of the installation process fails (e.g., missing URL,
///   download error, extraction error, directory creation failure). Error details are
///   logged internally.
pub fn install(tool_entry: &ToolEntry) -> Option<ToolState> {
    // Step 1: Validate ToolEntry Fields
    // The `tool_entry` is now correctly recognized as a `struct`,
    // so we can directly access its fields.

    // Attempt to retrieve the download URL from the `tool_entry`.
    // `tool_entry.url` is an `Option<String>`, so `&tool_entry.url` gives `Option<&String>`.
    // The `let Some(...) else { ... }` syntax provides a concise way to unwrap the `Option`
    // or execute a block of code if it's `None`.
    let Some(download_url_str) = &tool_entry.url else {
        // If `url` is `None`, log an error because a URL is mandatory for this installer.
        log_error!(
            "Tool '{}' has 'url' source but no URL provided in configuration.",
            tool_entry.name.bold().red() // Access `name` directly from the `tool_entry` struct.
        );
        return None; // Abort installation.
    };

    // Log debug information about the upcoming installation.
    // This helps in tracing the application's flow during development or troubleshooting.
    log_debug!(
        "Attempting to install tool '{}' from URL: {}",
        tool_entry.name.bold(), // Tool name displayed in bold.
        download_url_str // The URL to be downloaded.
    );

    // Step 2: Determine Installation Paths

    // Get the user's home directory. This is the base for all `setup-devbox`'s internal
    // directories and installations to ensure consistency and proper cleanup.
    let Some(home_dir) = dirs::home_dir() else {
        // If the home directory cannot be determined (rare, but possible in some environments),
        // log an error and abort.
        log_error!("Could not determine home directory to install tool.");
        return None;
    };

    // Define the base installation directory for all tools managed by `setup-devbox`.
    // This typically resolves to `~/.setup-devbox/tools/`.
    let tools_base_dir = home_dir.join(".setup-devbox").join("tools");
    // Attempt to create the base tools directory. `create_dir_all` ensures that all
    // necessary parent directories are also created if they don't exist.
    if let Err(e) = fs::create_dir_all(&tools_base_dir) {
        // If directory creation fails, log the error with path and details.
        log_error!(
            "Failed to create installation directory '{}': {}",
            tools_base_dir.display(), // Display path for user readability.
            e.to_string().red()       // Error message in red.
        );
        return None; // Abort installation.
    }

    // Determine the specific target directory for *this* tool.
    // This path will be `~/.setup-devbox/tools/<tool_name>/`.
    // Each tool gets its own isolated directory for cleaner management.
    let tool_install_dir = tools_base_dir.join(&tool_entry.name); // Use `tool_entry.name`.
    // Attempt to create the tool-specific installation directory.
    if let Err(e) = fs::create_dir_all(&tool_install_dir) {
        log_error!(
            "Failed to create tool-specific installation directory '{}': {}",
            tool_install_dir.display(),
            e.to_string().red()
        );
        return None; // Abort installation.
    }

    // Step 3: Download the Tool

    // Extract the filename from the download URL. This filename will be used for
    // temporary storage of the downloaded file.
    // Example: For `https://example.com/tool-v1.0.tar.gz`, `filename` becomes "tool-v1.0.tar.gz".
    let filename = PathBuf::from(download_url_str)
        .file_name() // Get the last component of the path (the filename).
        .map(|s| s.to_string_lossy().into_owned()) // Convert OsStr to owned String.
        .unwrap_or_else(|| "downloaded_file".to_string()); // Fallback name if URL doesn't have a filename.

    // Define the full path where the downloaded file will be temporarily saved.
    // This is within the tool's specific installation directory.
    let temp_download_path = tool_install_dir.join(&filename);

    // Inform the user about the download progress.
    log_info!(
        "Downloading '{}' from {} to {}...",
        tool_entry.name.bold(), // Tool name.
        download_url_str.cyan(),     // URL in cyan for emphasis.
        temp_download_path.display() // Local temporary path.
    );

    // Call the `download_file` utility function to perform the actual download.
    // This function handles the HTTP request and saving the response body to a file.
    if let Err(e) = download_file(download_url_str, &temp_download_path) {
        // If download fails, log an error and abort.
        log_error!(
            "Failed to download file from '{}': {}",
            download_url_str.red(),
            e.to_string().red()
        );
        return None; // Abort installation.
    }

    // Step 4: Extract/Process the Downloaded File

    // Initialize `package_type` to a default. This will be stored in `ToolState`
    // to indicate what kind of package was installed (e.g., "zip-archive", "binary").
    let mut package_type = "binary".to_string(); // Default assumption: a single binary file.
    // `final_extracted_path` will hold the path to the directory where the tool's
    // main contents are ultimately placed after extraction or copying.
    let final_extracted_path: PathBuf;

    // Determine the type of the downloaded file based on its extension.
    // This `match` statement helps `extract_archive` know how to handle the file.
    let known_file_type = match filename.to_lowercase().as_str() {
        s if s.ends_with(".tar.gz") => Some("tar.gz"),
        s if s.ends_with(".zip") => Some("zip"),
        s if s.ends_with(".gz") => Some("gz"),
        s if s.ends_with(".tar.bz2") => Some("tar.bz2"),
        s if s.ends_with(".tar") => Some("tar"),
        s if s.ends_with(".pkg") => Some("pkg"),
        _ => None, // If extension not directly recognized, let `extract_archive` try to auto-detect.
    };

    // If a known archive type is detected:
    if known_file_type.is_some() {
        log_info!(
            "Attempting to extract archive using `extract_archive` (type: {})...",
            known_file_type.as_deref().unwrap_or("None") // Log the detected archive type.
        );
        // Update `package_type` to reflect that it was an archive.
        package_type = format!("{}-archive", known_file_type.unwrap_or("unknown"));

        // Call the `extract_archive` function.
        // It takes the path to the downloaded archive, the target directory for extraction,
        // and the optional known file type hint.
        match extract_archive(&temp_download_path, &tool_install_dir, known_file_type) {
            Ok(extracted_dir) => {
                // If extraction is successful, `extracted_dir` is the path to the directory
                // where contents were placed (often a subdirectory like `tool_install_dir/extracted`).
                final_extracted_path = extracted_dir;
                log_debug!(
                    "Archive extracted to: {}",
                    final_extracted_path.display()
                );
            }
            Err(e) => {
                // If extraction fails, log an error.
                log_error!(
                    "Failed to extract archive from '{}': {}",
                    temp_download_path.display().to_string().red(),
                    e.to_string().red()
                );
                // Attempt to clean up the partially downloaded/extracted file.
                let _ = fs::remove_file(&temp_download_path);
                return None; // Abort installation.
            }
        }
    } else {
        // If the downloaded file is not a recognized archive (e.g., it's a direct binary executable):
        log_info!("Treating downloaded file as a direct binary or unknown type. Copying to installation directory using `extract_archive` (binary mode).");
        // Use `extract_archive` with `Some("binary")` to tell it to simply copy the file.
        // `extract_archive` will place the binary into a subdirectory, typically `tool_install_dir/extracted/`.
        match extract_archive(&temp_download_path, &tool_install_dir, Some("binary")) {
            Ok(copied_path) => {
                // `copied_path` will be the path to the new location of the binary.
                final_extracted_path = copied_path;
                log_debug!(
                    "Binary copied to: {}",
                    final_extracted_path.display()
                );
            }
            Err(e) => {
                // If copying fails, log an error.
                log_error!(
                    "Failed to copy binary '{}': {}",
                    temp_download_path.display().to_string().red(),
                    e.to_string().red()
                );
                let _ = fs::remove_file(&temp_download_path); // Clean up.
                return None; // Abort installation.
            }
        }
    }

    // Step 5: Clean Up Temporary Files

    // After processing (downloading and extracting/coping), the original temporary
    // downloaded file is no longer needed. Attempt to remove it.
    if temp_download_path.is_file() {
        if let Err(e) = fs::remove_file(&temp_download_path) {
            // If cleanup fails, log a warning but don't abort the installation,
            // as the main task (installation) was successful.
            log_warn!(
                "Failed to remove temporary download '{}': {}",
                temp_download_path.display().to_string().yellow(),
                e.to_string().yellow()
            );
        } else {
            log_debug!(
                "Removed temporary download '{}'.",
                temp_download_path.display()
            );
        }
    }

    // Step 6: Set Executable Permissions (Unix-like systems only)

    // This block is conditionally compiled only for Unix-like operating systems (Linux, macOS).
    // Windows handles executables differently (via `.exe` extension), so this is not applicable there.
    #[cfg(unix)]
    {
        // Determine the path to the executable *after* extraction.
        // Prioritize `executable_path_after_extract` if provided in `tools.yaml`.
        // This allows users to specify the exact path to the executable within the archive.
        let target_executable_path = if let Some(exec_path_relative) = &tool_entry.executable_path_after_extract {
            // If `executable_path_after_extract` is present, resolve it relative to `tool_install_dir`.
            // The user-provided path is relative to the *root* of the extracted contents.
            tool_install_dir.join(exec_path_relative)
        } else {
            // Fallback heuristic: If no specific path is given, assume the executable
            // is named after the tool and is directly inside the `final_extracted_path` (e.g., the `extracted` subdirectory).
            final_extracted_path.join(&tool_entry.name)
        };

        // Check if the determined executable path actually points to a file.
        if target_executable_path.is_file() {
            use std::os::unix::fs::PermissionsExt; // Trait for Unix-specific permission methods.

            let metadata = fs::metadata(&target_executable_path);
            if let Ok(mut perms) = metadata.map(|m| m.permissions()) {
                let mode = perms.mode(); // Get the current file mode (permissions bits).
                // Check if any execute bit (user, group, or others) is NOT set (0o111 means execute for all).
                if (mode & 0o111) == 0 {
                    // If no execute bit is set, set user, group, and others to read/write/execute (0o755).
                    perms.set_mode(mode | 0o755);
                    // Apply the new permissions to the file.
                    if let Err(e) = fs::set_permissions(&target_executable_path, perms) {
                        log_warn!(
                            "Failed to set executable permissions for '{}': {}",
                            target_executable_path.display(),
                            e.to_string().yellow()
                        );
                    } else {
                        log_debug!(
                            "Set executable permissions for '{}'.",
                            target_executable_path.display()
                        );
                    }
                }
            }
        } else {
            // If `target_executable_path` does not exist or is not a file.
            log_debug!("No direct executable found at expected path: {}. Skipping permission set.", target_executable_path.display());
            // If the user *did* specify `executable_path_after_extract` but it didn't lead to a file,
            // issue a warning to prompt them to check their configuration.
            if tool_entry.executable_path_after_extract.is_some() {
                log_warn!("Configured 'executable_path_after_extract' did not lead to a file: {}. Please verify your tools.yaml configuration.", tool_entry.executable_path_after_extract.as_ref().unwrap().yellow());
            }
        }
    }

    // Step 7: Construct and Return ToolState

    // Create a `ToolState` instance to record the details of the successful installation.
    // This state will be serialized and saved to `state.json` to keep track of installed tools.
    Some(ToolState {
        // Use the version from `tool_entry`. If `version` is `None` in the config,
        // default to "unknown".
        version: tool_entry.version.clone().unwrap_or_else(|| "unknown".to_string()),
        // The `install_path` points to the tool's base directory within `~/.setup-devbox/tools/`.
        install_path: tool_install_dir.to_string_lossy().into_owned(),
        // Mark that this tool was installed by `setup-devbox`.
        installed_by_devbox: true,
        // Specify the installation method for logging and future reference.
        install_method: "direct-url".to_string(),
        // Record if the tool was renamed during installation.
        renamed_to: tool_entry.rename_to.clone(),
        // Record the detected or assigned package type (e.g., "zip-archive", "binary").
        package_type,
        // For direct URL installs, `repo` and `tag` are typically not applicable, so they are `None`.
        repo: None,
        tag: None,
        // Store the original download URL in the state for potential re-downloads or verification.
        url: Some(download_url_str.clone()),
        // Clone any additional options provided in the configuration.
        options: tool_entry.options.clone(),
        // Store the `executable_path_after_extract` if it was specified in the configuration.
        // This is crucial for `setup-devbox` to later locate the actual executable.
        executable_path_after_extract: tool_entry.executable_path_after_extract.clone(),
    })
}
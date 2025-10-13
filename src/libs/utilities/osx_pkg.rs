// ============================================================================
//                          STANDARD LIBRARY DEPENDENCIES
// ============================================================================

#[cfg(target_os = "macos")]
use std::ffi::OsStr;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
#[cfg(target_os = "macos")]
use std::{fs, io};

// ============================================================================
//                             EXTERNAL DEPENDENCIES
// ============================================================================

#[cfg(target_os = "macos")]
use colored::Colorize;

// ============================================================================
//                              INTERNAL IMPORTS
// ============================================================================

#[cfg(target_os = "macos")]
use crate::{log_debug, log_error, log_info, log_warn};


/// Installs a software from a .pkg file on macOS.
/// This is a dummy implementation; your actual function needs to:
/// 1. Execute the `installer` command with the .pkg file.
/// 2. Determine and return the actual installation path (e.g., /Applications/AppName.app).
///
/// # Arguments
/// * `pkg_path`: The path to the .pkg file.
/// * `tool_name`: The name of the tool, used to guess the installation path.
///
/// # Returns
/// * `io::Result<PathBuf>`: `Ok(PathBuf)` if the PKG was installed successfully,
///   `Err(io::Error)` otherwise.
#[cfg(target_os = "macos")]
pub fn install_pkg(
    pkg_path: &Path,
    tool_name: &str,
    tool_renamed_to: &Option<String>,
) -> io::Result<PathBuf> {
    log_info!(
        "[macOS Installer] Initiating .pkg installation for: {}",
        pkg_path.display().to_string().bold()
    );
    log_info!("[macOS Installer] Executing .pkg installer (may require admin privileges)...");

    let installer_output = Command::new("sudo")
        .arg("installer")
        .arg("-pkg")
        .arg(pkg_path)
        .arg("-target")
        .arg("/") // Install to the root volume
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !installer_output.status.success() {
        let stderr = String::from_utf8_lossy(&installer_output.stderr);
        log_error!("[macOS Installer] Failed to install .pkg: {}", stderr.red());
        return Err(std::io::Error::other(format!(
            "Failed to install .pkg: {stderr}"
        )));
    }


    let check_cli_paths = |name: &str| -> Option<PathBuf> {
        // Check A: CLI Tool Root Directory (e.g., /usr/local/go)
        let cli_root_path = PathBuf::from(format!("/usr/local/{name}"));
        if cli_root_path.exists() && cli_root_path.is_dir() {
            log_debug!(
                "[macOS Installer] Found CLI tool root directory for '{}' at: {}",
                name.cyan(),
                cli_root_path.display()
            );
            return Some(cli_root_path);
        }

        // Check B: CLI Binary in /usr/local/bin
        let cli_bin_path = PathBuf::from(format!("/usr/local/bin/{name}"));
        if cli_bin_path.exists() {
            log_debug!(
                "[macOS Installer] Found CLI binary for '{}' at: {}",
                name.cyan(),
                cli_bin_path.display()
            );
            return Some(cli_bin_path);
        }

        None
    };

    let mut inferred_install_path = None;

    // 1. Check for application bundles in /Applications (common for GUI apps)
    let app_path = PathBuf::from(format!("/Applications/{tool_name}.app"));
    if app_path.exists() {
        log_debug!(
            "[macOS Installer] Found application bundle at: {}",
            app_path.display()
        );
        inferred_install_path = Some(app_path);
    }

    // 2. If not an app bundle, check common CLI tool directories using the *original tool name*.
    if inferred_install_path.is_none() {
        inferred_install_path = check_cli_paths(tool_name);
    }

    // 3. If still not found, check using the 'tool_renamed_to' alias.
    if inferred_install_path.is_none() {
        if let Some(alias) = tool_renamed_to.as_ref() {
            log_debug!(
                "[macOS Installer] Primary path checks failed. Checking alias '{}'...",
                alias.cyan()
            );
            inferred_install_path = check_cli_paths(alias);
        }
    }

    // 4. Fallback if no specific path was found, or if the tool name doesn't lead to a direct match.
    // This is the least specific guess.
    let final_path = inferred_install_path.unwrap_or_else(|| {
        log_warn!(
            "[macOS Installer] Unable to precisely determine install path for '{}' PKG. \
             Returning a generic fallback path. For critical tools, consider manually verifying \
             the installation path or adding an explicit 'install_path' if that feature becomes available.",
            tool_name.green()
        );
        // Defaulting to /usr/local/bin/<tool_name> as a very common CLI install location.
        PathBuf::from(format!("/usr/local/bin/{tool_name}"))
    });

    log_info!(
        "[macOS Installer] PKG for {} installed successfully. Inferred install path: {}",
        tool_name.green(),
        final_path.display().to_string().green()
    );
    Ok(final_path)
}


/// Installs a software from a .dmg (Disk Image) file on macOS.
///
/// This function attempts to:
/// 1. Mount the .dmg file.
/// 2. Search for either a .pkg installer or a .app bundle within the mounted volume,
///    prioritizing .pkg if both are present.
/// 3. If a .pkg is found, it calls `install_pkg` to install it.
/// 4. If a .app is found, it's copied to the `/Applications` directory.
/// 5. Unmount the .dmg file, **reliably**, regardless of installation success or failure.
///
/// # Arguments
/// * `dmg_path`: The path to the .dmg file.
/// * `app_name`: The expected name of the application (e.g., "Zed") to correctly
///   find and copy the `.app` bundle (e.g., "Zed.app").
///
/// # Returns
/// * `io::Result<PathBuf>`: `Ok(PathBuf)` if the DMG was processed successfully,
///   containing the final installation path; `Err(io::Error)` otherwise.
#[cfg(target_os = "macos")]
pub fn install_dmg(
    dmg_path: &Path,
    app_name: &str,
    tool_renamed_to: &Option<String>,
) -> io::Result<PathBuf> {
    log_info!(
        "[macOS Installer] Initiating .dmg installation for: {}",
        dmg_path.display().to_string().bold()
    );

    if !dmg_path.exists() || !dmg_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "DMG file does not exist or is not a file: {}",
                dmg_path.display()
            ),
        ));
    }

    let mounted_path: Option<PathBuf>;

    log_debug!("[macOS Installer] Mounting DMG: {}", dmg_path.display());
    let hdiutil_output = Command::new("sudo")
        .arg("hdiutil")
        .arg("attach")
        .arg("-nobrowse")
        .arg("-plist")
        .arg("-readonly")
        .arg("-noverify")
        .arg(dmg_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !hdiutil_output.status.success() {
        let stderr = String::from_utf8_lossy(&hdiutil_output.stderr);
        log_error!("[macOS Installer] Failed to mount DMG: {}", stderr.red());
        return Err(std::io::Error::other(format!(
            "[macOS Installer] Failed to mount DMG: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&hdiutil_output.stdout);
    if let Some(path_str) = extract_mounted_path_from_hdiutil_plist(&stdout) {
        let path = PathBuf::from(path_str);
        if path.exists() && path.is_dir() {
            log_info!(
                "[macOS Installer] DMG mounted successfully at: {}",
                path.display().to_string().green()
            );
            mounted_path = Some(path);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "hdiutil reported successful mount, but path does not exist or is not a directory: {}",
                    path.display()
                ),
            ));
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse mounted path from hdiutil output for {}",
                dmg_path.display()
            ),
        ));
    }


    let mounted_volume_path = mounted_path.ok_or_else(|| {
        std::io::Error::other("DMG was not mounted or mounted path could not be determined.")
    })?;

    //  Perform Installation and ensure unmount happens
    let install_result: io::Result<PathBuf> = (|| {
        // Changed closure return type to PathBuf
        let mut pkg_found: Option<PathBuf> = None;
        let mut app_found: Option<PathBuf> = None;

        for entry in fs::read_dir(&mounted_volume_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension() == Some(OsStr::new("pkg")) {
                pkg_found = Some(path);
                break;
            } else if path.extension() == Some(OsStr::new("app")) {
                app_found = Some(path);
            }
        }

        if let Some(pkg_path) = pkg_found {
            log_info!(
                "[macOS Installer] Found .pkg installer: {}",
                pkg_path.display().to_string().bold()
            );
            log_info!(
                "[macOS Installer] Executing .pkg installer (may require admin privileges)..."
            );
            // Call install_pkg and return its result (which is PathBuf)
            install_pkg(&pkg_path, app_name, tool_renamed_to)
        } else if let Some(app_path) = app_found {
            log_info!(
                "[macOS Installer] Found .app bundle: {}",
                app_path.display().to_string().bold()
            );
            let target_app_path = PathBuf::from("/Applications").join(format!("{app_name}.app"));

            if target_app_path.exists() {
                log_info!(
                    "[macOS Installer] Removing existing app at: {}",
                    target_app_path.display().to_string().yellow()
                );
                // Use sudo rm -rf for permission issues
                let rm_output = Command::new("sudo")
                    .arg("rm")
                    .arg("-rf") // Force recursively delete
                    .arg(&target_app_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()?;

                if !rm_output.status.success() {
                    let stderr = String::from_utf8_lossy(&rm_output.stderr);
                    log_error!(
                        "[macOS Installer] Failed to remove existing app {}: {}",
                        target_app_path.display(),
                        stderr.red()
                    );
                    return Err(std::io::Error::other(format!(
                        "Failed to remove existing app {}: {stderr}",
                        target_app_path.display()
                    )));
                }
                log_info!("[macOS Installer] Existing app removed successfully.");
            }

            log_debug!(
                "[macOS Installer] Copying .app to: {}",
                target_app_path.display()
            );
            let cp_output = Command::new("sudo")
                .arg("cp")
                .arg("-R")
                .arg(&app_path)
                .arg(Path::new("/Applications"))
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()?;

            if !cp_output.status.success() {
                let stderr = String::from_utf8_lossy(&cp_output.stderr);
                log_error!(
                    "[macOS Installer] Failed to copy .app to /Applications: {}",
                    stderr.red()
                );
                return Err(std::io::Error::other(format!(
                    "Failed to copy .app: {stderr}"
                )));
            }
            log_info!(
                "[macOS Installer] .app copied successfully to {}",
                target_app_path.display().to_string().green()
            );
            Ok(target_app_path) // Return the path for .app
        } else {
            log_warn!(
                "[macOS Installer] No .pkg or .app found in DMG: {}. Manual intervention may be required.",
                mounted_volume_path.display()
            );
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "No installable .app or .pkg found in DMG: {}",
                    mounted_volume_path.display()
                ),
            ))
        }
    })();

    // Unmount the DMG (always attempt, regardless of install_result)
    match unmount_dmg(&mounted_volume_path) {
        Ok(_) => log_debug!("[macOS Installer] DMG unmounted successfully."),
        Err(e) => {
            log_error!(
                "[macOS Installer] Failed to unmount DMG {}: {}",
                mounted_volume_path.display(),
                e.to_string().red()
            );
            if install_result.is_ok() {
                return Err(e);
            }
        }
    }

    log_info!(
        "[macOS Installer] .dmg installation process completed for: {}",
        dmg_path.display().to_string().green()
    );
    // Return the result of the installation process (which includes the PathBuf)
    install_result
}

/// Helper function to unmount a DMG.
///
/// # Arguments
/// * `mount_path`: The path where the DMG is mounted.
///
/// # Returns
/// * `io::Result<()>`: `Ok(())` if the DMG was unmounted successfully,
///   `Err(io::Error)` otherwise.
#[cfg(target_os = "macos")]
fn unmount_dmg(mount_path: &Path) -> io::Result<()> {
    log_debug!(
        "[macOS Installer] Attempting to unmount DMG from: {}",
        mount_path.display()
    );
    let detach_output = Command::new("sudo")
        .arg("hdiutil")
        .arg("detach")
        .arg("-force") // Force detach in case of busy errors
        .arg(mount_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !detach_output.status.success() {
        let stderr = String::from_utf8_lossy(&detach_output.stderr);
        return Err(std::io::Error::other(format!(
            "Failed to unmount DMG {}: {}",
            mount_path.display(),
            stderr
        )));
    }
    log_debug!("[macOS Installer] DMG unmounted successfully.");
    Ok(())
}

/// Helper to extract the mounted path from hdiutil's XML (plist) output.
///
/// This function parses the XML output from `hdiutil attach -plist` to find the
/// `<string>` value associated with the `<key>mount-point</key>`.
///
/// # Arguments
/// * `plist_output`: The `&str` containing the XML (plist) output from `hdiutil attach -plist`.
///
/// # Returns
/// * `Option<String>`: The mounted path as a `String` if found, otherwise `None`.
#[cfg(target_os = "macos")]
fn extract_mounted_path_from_hdiutil_plist(plist_output: &str) -> Option<String> {
    // A simple line-by-line search for the mount-point key and its subsequent string value.
    // For more complex plist structures, using a dedicated plist parser crate would be ideal.
    let mut lines = plist_output.lines().map(|s| s.trim());
    while let Some(line) = lines.next() {
        if line == "<key>mount-point</key>" {
            if let Some(path_line) = lines.next() {
                // The mount path is typically enclosed in <string> tags
                if path_line.starts_with("<string>") && path_line.ends_with("</string>") {
                    return Some(path_line[8..path_line.len() - 9].to_string());
                }
            }
        }
    }
    None
}

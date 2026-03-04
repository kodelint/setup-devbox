//! # Backup Utility
//!
//! This module provides functionality for backing up the application configuration directory.
//! It creates a timestamped zip archive of the configuration files before major changes.

use crate::{log_debug, log_info};
use chrono::Local;
use std::env;
use std::fs::{self, File};
use std::io::{self};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipWriter;
use zip::write::FileOptions;

/// Backs up the specified directory into a timestamped zip file.
pub fn backup_directory(src_dir: &Path) -> io::Result<PathBuf> {
    if !src_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Source directory does not exist",
        ));
    }

    // 1. Determine backup directory path from env var or default
    let backup_dir = match env::var("SDB_CONFIG_BACKUP_RETENTION_PATH") {
        Ok(path_str) if !path_str.trim().is_empty() => PathBuf::from(path_str),
        _ => src_dir.join(".backup"),
    };

    fs::create_dir_all(&backup_dir)?;

    // 2. Generate timestamped filename
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("backup_{}.zip", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    log_info!("[SDB::Backup] Creating backup: {}", backup_path.display());

    // 3. Create the zip archive
    let file = File::create(&backup_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored) // Simple storage for backups
        .unix_permissions(0o755);

    for entry in WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            // Don't backup the backup directory itself!
            !e.path().starts_with(&backup_dir) && e.path().is_file()
        })
    {
        let path = entry.path();
        let name = path.strip_prefix(src_dir).unwrap_or(path);

        log_debug!("[SDB::Backup] Adding to archive: {:?}", name);

        // zip.start_file takes a string path, so we use to_string_lossy.
        // On Windows it will have \, we should replace it with / for cross-platform zip format.
        #[cfg(windows)]
        let name_str = name.to_string_lossy().replace("\\", "/");
        #[cfg(not(windows))]
        let name_str = name.to_string_lossy();

        zip.start_file(name_str, options)?;
        let mut f = File::open(path)?;
        io::copy(&mut f, &mut zip)?;
    }

    zip.finish()?;

    log_info!(
        "[SDB::Backup] Backup successfully created at {}",
        backup_path.display()
    );

    // 4. Enforce backup retention policy
    enforce_retention_policy(&backup_dir);

    Ok(backup_path)
}

fn enforce_retention_policy(backup_dir: &Path) {
    let retention: usize = env::var("SDB_CONFIG_BACKUP_RETENTION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7);

    let mut backups: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_valid_backup = path.is_file()
                && path.extension().is_some_and(|ext| ext == "zip")
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| name.starts_with("backup_"));

            if is_valid_backup {
                backups.push(path);
            }
        }
    }

    // Sort backups by name (lexicographical sort of timestamps will order them oldest first)
    backups.sort();

    if backups.len() > retention {
        let to_delete = backups.len() - retention;
        for path in backups.iter().take(to_delete) {
            log_debug!(
                "[SDB::Backup] Discarding old backup due to retention policy: {}",
                path.display()
            );
            let _ = fs::remove_file(path);
        }
    }
}

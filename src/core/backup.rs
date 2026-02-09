//! # Backup Utility
//!
//! This module provides functionality for backing up the application configuration directory.
//! It creates a timestamped zip archive of the configuration files before major changes.

use crate::{log_debug, log_info};
use chrono::Local;
use std::fs::{self, File};
use std::io::{self};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipWriter;
use zip::write::FileOptions;

/// Backs up the specified directory into a timestamped zip file in a '.backup' subdirectory.
pub fn backup_directory(src_dir: &Path) -> io::Result<PathBuf> {
    if !src_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Source directory does not exist",
        ));
    }

    // 1. Create .backup directory inside src_dir or next to it
    // Let's put it in src_dir/.backup
    let backup_dir = src_dir.join(".backup");
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
            // Don't backup the .backup directory itself!
            !e.path().starts_with(&backup_dir) && e.path().is_file()
        })
    {
        let path = entry.path();
        let name = path.strip_prefix(src_dir).unwrap_or(path);

        log_debug!("[SDB::Backup] Adding to archive: {:?}", name);

        zip.start_file(name.to_string_lossy(), options)?;
        let mut f = File::open(path)?;
        io::copy(&mut f, &mut zip)?;
    }

    zip.finish()?;

    log_info!(
        "[SDB::Backup] Backup successfully created at {}",
        backup_path.display()
    );
    Ok(backup_path)
}

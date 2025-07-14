// This module re-exports components related to compression, specifically
// the Gzip decoder for handling gzipped archives.

// Re-export GzDecoder from the `flate2` crate.
// `pub(crate)` means it's publicly accessible within the crate but not outside it.
pub(crate) use flate2::read::GzDecoder;
// Our custom utility tools from assets 
use crate::libs::utilities::assets::detect_file_type;
// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_info};
use bzip2::read::BzDecoder;
// The 'colored' crate helps us make our console output look pretty and readab
use colored::Colorize;
// For file system operations: creating directories, reading files, etc.
// `std::fs` provides functions for interacting with the file system.
use std::fs;
// For creating and interacting with files.
// `std::fs::{File, OpenOptions}` allows for fine-grained control over file opening and creation.
use std::fs::File;
// For working with file paths, specifically to construct installation paths.
// `std::path::Path` is a powerful type for working with file paths in a robust way.
// `std::path::PathBuf` provides an OS-agnostic way to build and manipulate file paths.
use std::path::{Path, PathBuf};
// To get environment variables, like the temporary directory or home directory.
// `std::env` provides functions to interact with the process's environment.
// `std::io` contains core input/output functionalities and error types.
use std::io;
// For extracting tar archives.
// The `tar` crate provides functionality to read and write tar archives.
pub(crate) use tar::Archive;
// For extracting zip archives.
// The `zip` crate provides functionality to read and write zip archives.
use zip::ZipArchive;

/// Extracts the contents of a compressed archive (zip, tar.gz, etc.) into a new subdirectory
/// within the specified destination path. This is a core utility for unpacking downloaded tools.
/// The extracted contents will be placed in a new directory named "extracted" inside `dest`.
///
/// # Arguments
/// * `src`: The path (`&Path`) to the compressed archive file that needs to be extracted.
/// * `dest`: The parent directory (`&Path`) where the *extracted* content should be placed.
///           A new subdirectory named "extracted" will be created inside this `dest` path.
/// * `known_file_type`: An `Option<&str>`. If `Some(type_str)` is provided, it tells the function
///   the exact type of the archive (e.g., "zip", "tar.gz"), bypassing internal detection.
///   This is useful when the caller already knows the type (e.g., from a GitHub asset name),
///   which can be faster or more accurate than re-detecting. If `None`, `detect_file_type` is used.
///
/// # Returns
/// * `io::Result<PathBuf>`:
///   - `Ok(PathBuf)` with the path to the newly created "extracted" directory if extraction was successful.
///   - An `io::Error` if extraction fails, the archive type is unsupported, or any I/O operation fails.
pub fn extract_archive(src: &Path, dest: &Path, known_file_type: Option<&str>) -> io::Result<PathBuf> {
    log_debug!("[Utils] Extracting archive {:?} into {:?}", src.to_string_lossy().blue(), dest.to_string_lossy().cyan());

    // Determine the file type to guide the extraction process.
    // If `known_file_type` is provided (i.e., `Some(ft)`), use that.
    // Otherwise, fall back to `detect_file_type` which uses the `file` command.
    let file_type = if let Some(ft) = known_file_type {
        log_debug!("[Utils] Using known file type from argument: {}", ft.green());
        ft.to_string()
    } else {
        log_debug!("[Utils] No known file type provided. Auto-detecting using 'file' command...");
        detect_file_type(src)
    };

    // Create a specific subdirectory named "extracted" inside the `dest` directory.
    // This keeps extracted contents organized and prevents clutter in the main temporary directory.
    // `fs::create_dir_all` creates all necessary parent directories if they don't exist.
    // The `?` operator propagates any I/O error (e.g., permission denied) from directory creation.
    let extracted_path = dest.join("extracted");
    fs::create_dir_all(&extracted_path)?;

    // Use a `match` statement to handle different archive types.
    match file_type.as_str() {
        "zip" => {
            // Open the source zip file.
            let file = File::open(src)?;
            // Create a new `ZipArchive` reader from the opened file.
            let mut archive = ZipArchive::new(file)?;
            // Extract all contents of the zip archive into the `extracted_path`.
            archive.extract(&extracted_path)?;
            log_debug!("[Utils] Zip archive extracted successfully.");
        }
        "tar.gz" => { // Handle specific `tar.gz` files.
            // Open the gzipped tar file.
            let tar_gz = File::open(src)?;
            // Create a `GzDecoder` to decompress the gzip stream.
            let decompressor = GzDecoder::new(tar_gz);
            // Create a `tar::Archive` reader from the decompressed stream.
            let mut archive = Archive::new(decompressor);
            // Unpack all contents of the tar archive into the `extracted_path`.
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar.gz archive extracted successfully.");
        }
        "gz" => { // Handle pure `.gz` files (not tarred, typically a single compressed file).
            log_info!("[Utils] Decompressing plain GZ file. Contents will be the original file without tar extraction.");
            let gz_file = File::open(src)?;
            let mut decompressor = GzDecoder::new(gz_file);
            // Determine the output file path by removing the ".gz" extension from the source filename.
            let output_file_path = extracted_path.join(src.file_stem().unwrap_or_default());
            let mut output_file = File::create(&output_file_path)?;
            // Copy the decompressed data from the `GzDecoder` to the new output file.
            io::copy(&mut decompressor, &mut output_file)?;
            log_debug!("[Utils] GZ file decompressed successfully to {:?}", output_file_path.display());
        }
        "tar.bz2" => {
            // Open the bzipped tar file.
            let tar_bz2 = File::open(src)?;
            // Create a `BzDecoder` to decompress the bzip2 stream.
            let decompressor = BzDecoder::new(tar_bz2);
            // Create a `tar::Archive` reader from the decompressed stream.
            let mut archive = Archive::new(decompressor);
            // Unpack all contents of the tar archive into the `extracted_path`.
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar.bz2 archive extracted successfully.");
        }
        "tar" => { // Handle plain `.tar` archives (uncompressed).
            // Open the tar file.
            let tar = File::open(src)?;
            // Create a `tar::Archive` reader directly from the file.
            let mut archive = Archive::new(tar);
            // Unpack all contents of the tar archive into the `extracted_path`.
            archive.unpack(&extracted_path)?;
            log_debug!("[Utils] Tar archive extracted successfully.");
        }
        "binary" => { // For standalone binaries like a .exe or uncompressed Mac binary.
            // In this case, "extraction" means simply copying the binary to the `extracted_path`.
            log_info!("[Utils] Copying detected 'binary' directly to extraction path.");
            // Get the filename part from the source path.
            let file_name = src.file_name().ok_or_else(|| {
                // If the source path doesn't have a filename, return an error.
                io::Error::new(io::ErrorKind::InvalidInput, "Source path has no filename")
            })?;
            // Copy the source file to the `extracted_path` maintaining its original filename.
            fs::copy(src, extracted_path.join(file_name))?;
            log_debug!("[Utils] Binary copied successfully to {:?}", extracted_path.join(file_name).display());
        }
        "pkg" => { // For macOS `.pkg` files, which are installers, not archives to unpack in the traditional sense.
            // We copy them to the extracted path so they are available for installation later.
            log_info!("[Utils] Detected .pkg installer. Copying directly to extraction path for installation.");
            let file_name = src.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Source path has no filename")
            })?;
            // Copy the `.pkg` file to the `extracted_path`.
            fs::copy(src, extracted_path.join(file_name))?;
            log_debug!("[Utils] .pkg file copied successfully to {:?}", extracted_path.join(file_name).display());
        }
        _ => {
            // If the `file_type` string does not match any of the supported types.
            log_error!("[Utils] Unsupported archive type '{}' for extraction: {:?}", file_type.red(), src);
            // Return an `io::Error` indicating that the archive type is not supported.
            return Err(io::Error::new(
                io::ErrorKind::InvalidData, // `InvalidData` is suitable for unsupported file formats.
                format!("Unsupported archive type: {}", file_type),
            ));
        }
    }

    // Log a success message with the path to the extracted contents.
    log_debug!("[Utils] ✨ Archive contents available at: {:?}", extracted_path.to_string_lossy().green());
    Ok(extracted_path) // Return the path to the directory where contents were extracted.
}
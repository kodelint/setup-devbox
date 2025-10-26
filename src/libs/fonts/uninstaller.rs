// =========================================================================== //
//                          STANDARD LIBRARY DEPENDENCIES                      //
// =========================================================================== //
use std::fs;

// =========================================================================== //
//                             EXTERNAL DEPENDENCIES                           //
// =========================================================================== //
use colored::Colorize;

// =========================================================================== //
//                              INTERNAL IMPORTS                               //
// =========================================================================== //
use crate::libs::tools::uninstaller::executors::RemovalResult;
use crate::schemas::common::RemovalOrchestrator;
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::state_file::FontState;
use crate::{log_debug, log_error, log_info, log_warn};
// =========================================================================== //
//                           REMOVAL ORCHESTRATOR                              //
// =========================================================================== //

impl<'a> RemovalOrchestrator<'a> {
    /// Removes a font from the system.
    ///
    /// This method handles the complete font removal process:
    /// 1. Locates the font in state
    /// 2. Removes all font files from the fonts directory
    /// 3. Removes the font from state
    /// 4. Removes the font from configuration YAML
    ///
    /// # Arguments
    ///
    /// * `font_name` - Name of the font to remove
    ///
    /// # Returns
    ///
    /// A RemovalResult indicating success, not found, or failure
    ///
    /// # Font File Handling
    ///
    /// Fonts are typically installed as multiple .ttf files (regular, bold, italic, etc.)
    /// all containing the font name. This method removes all matching font files.
    pub fn remove_font(&mut self, font_name: &str) -> RemovalResult {
        log_info!("[SDB::Remove::Font] Removing: {}", font_name.cyan());

        // Step 1: Check if font exists in state
        let font_state = match self.state.fonts.get(font_name) {
            Some(state) => state.clone(),
            None => {
                log_warn!(
                    "[SDB::Remove::Font] Not found in state: {}",
                    font_name.yellow()
                );
                return RemovalResult::NotFound;
            }
        };

        // Step 2: Remove font files from the file system
        if let Err(e) = self.remove_font_files(&font_state) {
            log_error!("[SDB::Remove::Font] Failed to remove files: {}", e.red());
            return RemovalResult::Failed(e);
        }

        // Step 3: Remove from state
        self.state.fonts.remove(font_name);
        log_debug!("[SDB::Remove::Font] Removed from state: {}", font_name);

        // Step 4: Remove from configuration YAML
        if let Err(e) = self
            .cleaner
            .remove_list_item("fonts.yaml", "fonts:", "name:", font_name)
        {
            log_warn!("[SDB::Remove::Font] YAML cleanup warning: {}", e.yellow());
        }

        RemovalResult::Removed
    }

    /// Removes all font files associated with a font.
    ///
    /// # Arguments
    ///
    /// * `font_state` - State information for the font
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All font files removed (or none found)
    /// * `Err(String)` - Failed to access fonts directory or remove files
    ///
    /// # Implementation Details
    ///
    /// Font files are stored in a system fonts directory (typically ~/Library/Fonts
    /// on macOS). This method:
    /// 1. Locates the fonts directory
    /// 2. Searches for all .ttf files containing the font name
    /// 3. Removes each matching file
    ///
    /// If no font files are found, a warning is logged but this is not an error.
    fn remove_font_files(&self, font_state: &FontState) -> Result<(), String> {
        let fonts_dir = PathResolver::get_font_installation_dir()
            .map_err(|e| format!("Failed to get fonts directory: {e}"))?;

        log_debug!("[SDB::Remove::Font] Searching in: {}", fonts_dir.display());

        let mut removed_count = 0;

        // Read all files in the fonts directory
        let entries = fs::read_dir(&fonts_dir).map_err(|e| {
            format!(
                "Failed to read fonts directory {}: {}",
                fonts_dir.display(),
                e
            )
        })?;

        // Find and remove all font files matching this font name
        for entry in entries {
            let entry = entry.map_err(|e| {
                format!(
                    "Failed to read directory entry in {}: {}",
                    fonts_dir.display(),
                    e
                )
            })?;

            let path = entry.path();

            // Check if this is a font file matching our criteria
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|file_name| {
                        file_name.contains(&font_state.name) && file_name.ends_with(".ttf")
                    })
                    .unwrap_or(false)
            {
                fs::remove_file(&path)
                    .map_err(|e| format!("Failed to remove font file {}: {}", path.display(), e))?;
                log_info!(
                    "[SDB::Remove::Font] Deleted: {}",
                    path.display().to_string().cyan()
                );
                removed_count += 1;
            }
        }

        if removed_count == 0 {
            log_warn!(
                "[SDB::Remove::Font] No font files found for: {}",
                font_state.name.yellow()
            );
        } else {
            log_info!(
                "[SDB::Remove::Font] Removed {} file(s) for: {}",
                removed_count.to_string().cyan(),
                font_state.name.cyan()
            );
        }

        Ok(())
    }
}

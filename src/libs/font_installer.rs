use std::path::PathBuf; // Provides `PathBuf` for working with file paths.
use colored::Colorize; // Imports the `Colorize` trait for adding color to console output.
// Adjust the import path for logging macros if they are not directly in `crate::` but, for example, in `crate::utils::logging`.
// Assuming they are still at the top-level crate import for now.
use crate::{log_debug, log_error, log_info}; // Custom logging macros for various log levels.

// The path to the font installer module will change because it's now under `installers`.
// It was already `crate::installers::fonts;` so this line remains the same as `font_installer.rs` calls it.
use crate::installers::fonts; // Imports the `fonts` module, which contains the actual font installation logic.

use crate::schema::{DevBoxState, FontConfig}; // Imports `DevBoxState` for application state management and `FontConfig` for font-specific configuration.
// If `state_management` is also in `src/libs/`, its path would be `crate::libs::state_management::save_devbox_state;`
// For now, assuming it's `crate::libs::state_management::save_devbox_state` or a top-level `crate::state_management`.
// Based on your original file, it's `crate::libs::state_management::save_devbox_state`, so it remains the same.
use crate::libs::state_management::save_devbox_state; // Imports the function to save the `DevBoxState`.


/// Installs fonts based on the provided configuration and updates the application state.
///
/// This function iterates through each font defined in `fonts_cfg`, checks if it's
/// already installed according to `state`, and delegates to the `fonts` installer
/// for new fonts. It also handles state persistence.
///
/// # Arguments
/// * `fonts_cfg`: A `FontConfig` struct containing the list of fonts to install.
/// * `state`: A mutable reference to the `DevBoxState` to update installed fonts.
/// * `state_path_resolved`: The `PathBuf` to the `state.json` file for saving.
pub fn install_fonts(fonts_cfg: FontConfig, state: &mut DevBoxState, state_path_resolved: &PathBuf) {
    log_info!("[Fonts] Processing Font Installations..."); // Informative log that font installation process has started.
    log_debug!("Entering install_fonts() function."); // Debug log for function entry.

    let mut fonts_updated = false; // Flag to track if any fonts were newly installed or updated.
    let mut skipped_fonts: Vec<String> = Vec::new(); // A vector to store names of fonts that were skipped because they were already installed.

    // Iterate over each font definition in the `fonts_cfg`.
    for font in &fonts_cfg.fonts {
        log_debug!("[Fonts] Considering font: {:?}", font.name.bold()); // Debug log for the current font being considered.
        // Check if the font is already present in the `DevBoxState`.
        if !state.fonts.contains_key(&font.name) {
            print!("\n"); // Print a newline for better console formatting.
            eprintln!("{}", "==============================================================================================".bright_blue()); // Print a separator for visual clarity.
            log_info!("[Fonts] Installing {}...", font.name.bold().cyan()); // Informative log about the font being installed.
            // Call the actual font installation logic from the `fonts` installer module.
            if let Some(font_state) = fonts::install(font) {
                state.fonts.insert(font_state.name.clone(), font_state); // Insert the new font's state into the `DevBoxState`.
                fonts_updated = true; // Set the flag to true as a font was installed.
                log_info!("[Fonts] Successfully installed {}.", font.name.bold().green()); // Success log for the font installation.
                eprintln!("{}", "===============================================================================================".bright_blue()); // Print another separator.
                print!("\n"); // Print a newline.
            } else {
                // Log an error if the font installation failed.
                log_error!(
                    "Failed to install font: {}. Please review previous logs for specific errors during installation.",
                    font.name.bold().red()
                );
            }
        } else {
            skipped_fonts.push(font.name.clone()); // Add the font to the skipped list if already installed.
            log_debug!("[Fonts] Font '{}' is already recorded as installed. Added to skipped list.", font.name.blue()); // Debug log for skipped font.
        }
    }

    // After iterating through all fonts, check if any were skipped.
    if !skipped_fonts.is_empty() {
        let skipped_fonts_str = skipped_fonts.join(", "); // Join skipped font names for a single log message.
        log_info!(
            "[Fonts] The following fonts were already recorded as installed and were skipped: {}",
            skipped_fonts_str.blue() // Informative log about skipped fonts.
        );
    } else {
        log_debug!("[Fonts] No fonts were skipped as they were not found in the state."); // Debug log if no fonts were skipped.
    }

    // If any fonts were installed or updated, save the `DevBoxState`.
    if fonts_updated {
        log_info!("[Fonts] Font state updated. Saving current DevBox state..."); // Informative log before saving state.
        if !save_devbox_state(state, state_path_resolved) {
            log_error!("Failed to save state after font installations. Data loss risk!"); // Error log if state saving fails.
        }
        log_info!("[StateSave] State saved successfully after font updates."); // Success log for state saving.
    } else {
        log_info!("[Fonts] No new fonts installed or state changes detected for fonts."); // Informative log if no state changes occurred.
    }
    eprintln!(); // Print a newline for final console formatting.
    log_debug!("Exiting install_fonts() function."); // Debug log for function exit.
}
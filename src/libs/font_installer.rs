use std::path::PathBuf;
use colored::Colorize;
use crate::{log_debug, log_error, log_info};
use crate::installers::fonts;
use crate::schema::{DevBoxState, FontConfig};
use crate::libs::state_management::save_devbox_state;

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
    log_info!("[Fonts] Processing Font Installations...");
    log_debug!("Entering install_fonts() function.");

    let mut fonts_updated = false;
    let mut skipped_fonts: Vec<String> = Vec::new();

    for font in &fonts_cfg.fonts {
        log_debug!("[Fonts] Considering font: {:?}", font.name.bold());
        if !state.fonts.contains_key(&font.name) {
            print!("\n");
            eprintln!("{}", "==============================================================================================".bright_blue());
            log_info!("[Fonts] Installing new font: {}", font.name.bold());
            log_debug!("[Fonts] Full configuration details for font '{}': {:?}", font.name, font);

            let installation_result = fonts::install(font);

            if let Some(font_state) = installation_result {
                state.fonts.insert(font.name.clone(), font_state);
                fonts_updated = true;
                log_info!("[Fonts] Successfully installed font: {}", font.name.bold().green());
                eprintln!("{}", "==============================================================================================".bright_blue());
                print!("\n");
            } else {
                log_error!(
                    "Failed to install font: {}. Please review previous logs for specific errors during installation.",
                    font.name.bold().red()
                );
            }
        } else {
            skipped_fonts.push(font.name.clone());
            log_debug!("[Fonts] Font '{}' is already recorded as installed. Added to skipped list.", font.name.blue());
        }
    }

    if !skipped_fonts.is_empty() {
        let skipped_fonts_str = skipped_fonts.join(", ");
        log_info!(
            "[Fonts] The following fonts were already recorded as installed and were skipped: {}",
            skipped_fonts_str.blue()
        );
    } else {
        log_debug!("[Fonts] No fonts were skipped as they were not found in the state.");
    }

    if fonts_updated {
        log_info!("[Fonts] Font state updated. Saving current DevBox state...");
        if !save_devbox_state(state, state_path_resolved) {
            log_error!("Failed to save state after font installations. Data loss risk!");
        }
        log_info!("[StateSave] State saved successfully after font updates.");
    } else {
        log_info!("[Fonts] No new fonts installed or state changes detected for fonts.");
    }
    eprintln!();
    log_debug!("Exiting install_fonts() function.");
}

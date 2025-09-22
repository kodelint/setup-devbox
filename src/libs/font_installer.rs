use colored::Colorize;
use std::path::PathBuf;
// Provides `PathBuf` for working with file paths. // Imports the `Colorize` trait for adding color to console output.
// Adjust the import path for logging macros if they are not directly in `crate::` but, for example, in `crate::libutils::logging`.
// Assuming they are still at the top-level crate import for now.
use crate::{log_debug, log_error, log_info};
// Custom logging macros for various log levels.

// The path to the font installer module will change because it's now under `installers`.
// It was already `crate::installers::fonts;` so this line remains the same as `font_installer.rs` calls it.
// Imports the `fonts` module, which contains the actual font installation logic.
use crate::installers::fonts;
// Imports `DevBoxState` for application state management and `FontConfig` for font-specific configuration.
use crate::schemas::fonts::FontConfig;
use crate::schemas::state_file::DevBoxState;
// Imports the function to save the `DevBoxState`.
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
pub fn install_fonts(
    fonts_cfg: FontConfig,
    state: &mut DevBoxState,
    state_path_resolved: &PathBuf,
) {
    eprintln!("\n");
    eprintln!("{}:", "FONTS".bright_yellow().bold());
    eprintln!("{}\n", "=".repeat(7).bright_yellow());
    log_debug!("Entering install_fonts() function."); // Debug log to indicate entry into the `install_fonts` function.

    // A boolean flag initialized to `false`. This flag will be set to `true`
    // if any fonts are newly installed or updated during this function call.
    let mut fonts_updated = false;
    // A mutable vector named `skipped_fonts` to store the names of fonts
    // that are found to be already installed and thus skipped from re-installation.
    let mut skipped_fonts: Vec<String> = Vec::new();

    // Iterate over each `font` definition in the `fonts_cfg`.
    // `&fonts_cfg.fonts` creates an immutable reference to the vector of font configurations,
    // allowing iteration without consuming or moving the `fonts_cfg` data.
    for font in &fonts_cfg.fonts {
        // Log at debug level the name of the current font being considered for installation.
        // `font.name.bold()` makes the font name bold in the debug output for better readability.
        log_debug!("[Fonts] Considering font: {}", font.name.bold());
        // Check if the font is already present in the `DevBoxState`.
        // `!state.fonts.contains_key(&font.name)` evaluates to `true` if the font's name
        // is NOT found as a key in the `state.fonts` HashMap, indicating that it needs to be installed.
        if !state.fonts.contains_key(&font.name) {
            // Print a newline character to the standard output for better console formatting
            // and visual separation between log blocks.
            println!("\n");
            // Print a separator line to the standard error output for strong visual clarity,
            // signaling the start of a new font installation block. The line is colored bright blue.
            eprintln!("{}", "==============================================================================================".bright_blue());
            // Log an informative message to the user about the specific font being installed.
            // The font's name is displayed in bold cyan for emphasis.
            log_info!("[Fonts] Installing {}...", font.name.bold().cyan());
            // Call the actual font installation logic from the `fonts` installer module.
            // `fonts::install(font)` is expected to perform the system-level font installation.
            // It returns an `Option<FontState>`: `Some(font_state)` on success with the installed font's state, or `None` on failure.
            if let Some(font_state) = fonts::install(font) {
                // If `install` returns `Some(font_state)`, it means the font was successfully installed.
                // Insert the new font's state into the `DevBoxState`'s `fonts` HashMap.
                // `font_state.name.clone()` is used as the key to store the `font_state` value.
                state.fonts.insert(font_state.name.clone(), font_state);
                // Set the `fonts_updated` flag to `true` to indicate that a change occurred in the state,
                // which will trigger a state save later.
                fonts_updated = true;
                // Log a success message for the font installation, displaying the font's name in bold green.
                log_info!(
                    "[Fonts] Successfully installed {}.",
                    font.name.bold().green()
                );
                // Print another separator line to the standard error output, in bright blue,
                // to visually close the font installation block.
                eprintln!("{}", "===============================================================================================".bright_blue());
                println!("\n"); // Print a newline for additional visual spacing.
            } else {
                // If `fonts::install(font)` returned `None`, it indicates that the font installation failed.
                // Log an error message, prompting the user to review earlier logs for more specific
                // details about why the installation might have failed.
                log_error!(
                    "Failed to install font: {}. Please review previous logs for specific errors during installation.",
                    font.name.bold().red() // Display the failed font's name in bold red.
                );
            }
        } else {
            // This block is executed if `!state.fonts.contains_key(&font.name)` is `false`,
            // meaning the font is already recorded in the `DevBoxState`.
            // Add the current font's name to the `skipped_fonts` list.
            skipped_fonts.push(font.name.clone());
            // Log a debug message indicating that the font was skipped because it was already found
            // in the state, displaying its name in blue.
            log_debug!(
                "[Fonts] Font '{}' is already recorded as installed. Added to skipped list.",
                font.name.blue()
            );
        }
    }

    // After iterating through all fonts in `fonts_cfg`,
    // this block checks if any fonts were skipped.
    if !skipped_fonts.is_empty() {
        // If the `skipped_fonts` vector is not empty,
        // join the font names into a comma-separated string.
        let skipped_fonts_str = skipped_fonts.join(", ");
        // Log an informative message to the user, listing all the fonts that were skipped.
        log_info!(
            "[Fonts] The following fonts were already recorded as installed and were skipped: {}",
            skipped_fonts_str.blue() // Display the list of skipped fonts in blue.
        );
    } else {
        // If the `skipped_fonts` vector is empty, it means all fonts were either installed/updated
        // or an attempt was made. Log a debug message to this effect.
        log_debug!("[Fonts] No fonts were skipped as they were not found in the state.");
    }

    // If any fonts were installed or updated during this function call (`fonts_updated` is `true`),
    // then the application state needs to be saved to persist these changes.
    if fonts_updated {
        // Informative log before initiating the state saving process.
        log_info!("[Fonts] Font state updated. Saving current DevBox state...");
        // Call the `save_devbox_state` function from the `state_management` module.
        // This function serializes the `state` (which now includes the new/updated fonts)
        // to the `state_path_resolved` file. It returns `false` if the save operation fails.
        if !save_devbox_state(state, state_path_resolved) {
            // If `save_devbox_state` returns `false`, log a critical error,
            // as failure to save the state can lead to loss of installed font information.
            log_error!("Failed to save state after font installations. Data loss risk!");
        } else {
            // If `save_devbox_state` returns `true`, log a success message for the state saving.
            log_info!("[StateSave] State saved successfully after font updates.");
        }
    } else {
        // If `fonts_updated` is `false`, no new fonts were installed or existing ones updated.
        // Log an informative message that no state changes related to fonts were detected,
        // so no saving action was necessary.
        log_info!("[Fonts] No new fonts installed or state changes detected for fonts.");
    }
    // Print a final newline for consistent console output spacing.
    eprintln!();
    // Debug log to indicate successful exit from the `install_fonts` function.
    log_debug!("Exiting install_fonts() function.");
}

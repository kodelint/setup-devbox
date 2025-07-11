// This module acts as the "shell configuration manager" for your setup-devbox.
// Its core job is to intelligently update your shell's startup files (like `.zshrc` or `.bashrc`).
// It ensures that any necessary configurations and aliases you've defined are added,
// but it's smart enough not to add duplicates, keeping your RC files clean and efficient.

// We're bringing in the blueprints (schemas) for how we expect to see
// your `AliasEntry` and `ShellRc` configurations structured. These are
// likely defined in your `schema.rs` file.
use crate::schema::{AliasEntry, ShellRc};
// We're importing specific helper functions from our `utils` module.
// These are like specialized tools that handle parts of the work:
// `append_to_rc_file`: for adding new lines to the shell RC file.
// `get_rc_file`: for figuring out which RC file to target.
// `read_rc_lines`: for reading the existing content of an RC file.
use crate::utils::{append_to_rc_file, get_rc_file, read_rc_lines};
// These are our custom logging tools! They let us print messages
// to the console at different detail levels:
// `log_debug`: for highly detailed messages, usually for troubleshooting.
// `log_info`: for general updates about what's happening.
// `log_warn`: for non-critical issues that you should be aware of.
use crate::{log_debug, log_info, log_warn};
// This wonderful crate allows us to make our terminal output
// much more readable by adding colors!
use colored::Colorize;


/// This is the main orchestrator for applying shell configurations and aliases.
/// Think of `apply_shellrc` as the "chief manager" for your shell setup.
/// It takes your desired shell configurations (`shellrc` and `aliases`)
/// from your `shellrc.yaml` file and thoughtfully updates your shell's
/// runtime configuration file (like `.zshrc` or `.bashrc`).
///
/// Here's the game plan this function follows:
/// 1.  **Identify the right RC file**: It first figures out which specific
///     shell RC file (e.g., `.zshrc` for Zsh) needs to be modified.
/// 2.  **Read existing content**: It then reads everything that's already
///     inside that RC file. This is crucial for avoiding duplicates.
/// 3.  **Compare and collect new configs**: It meticulously checks your
///     `raw_configs` and `aliases` from your `shellrc.yaml` against
///     what's already in the RC file. Only truly *new* or *missing* entries get selected.
/// 4.  **Append new entries**: If there are any fresh configurations or aliases,
///     it carefully appends them to the end of your RC file.
///
/// # Arguments
/// * `shellrc`: A blueprint (`ShellRc` struct) of your desired shell configuration.
///              It tells us which shell to target and provides raw configuration snippets.
/// * `aliases`: A list (`AliasEntry` slice) of specific command shortcuts (aliases)
///              you want to set up in your shell.
pub fn apply_shellrc(shellrc: &ShellRc, aliases: &[AliasEntry]) {
    // Let's kick things off with a friendly message, telling the user which shell
    // we're focusing on for configuration.
    log_info!("[ShellRC] Starting the process to apply configurations for shell: {}", shellrc.shell.bold());
    // For deeper dives, let's log the full details of the shell configuration
    // and the aliases we've received. This is great for debugging!
    log_debug!("[ShellRC] Received ShellRc configuration details: {:?}", shellrc);
    log_debug!("[ShellRC] Received aliases to consider: {:?}", aliases);

    // Step 1: Identify the Correct RC File
    // We need to pinpoint the exact shell RC file (like `.zshrc` or `.bashrc`)
    // that corresponds to the shell specified in your `shellrc` configuration.
    // The `get_rc_file` helper function does this detective work for us.
    let Some(rc_path) = get_rc_file(&shellrc.shell) else {
        // If `get_rc_file` comes up empty (returns `None`), it means the shell
        // you've specified isn't one we currently know how to handle.
        // We'll log a warning and gracefully stop this operation for this shell.
        log_warn!(
            "[ShellRC] Skipping shell configuration: Unsupported or unrecognized shell '{}'. Our current mission only covers 'zsh' and 'bash'.",
            shellrc.shell.red()
        );
        return; // We can't proceed, so let's exit this function.
    };
    // Success! We've found the target RC file. Let's let everyone know.
    log_info!("[ShellRC] Identified the target RC file path: {}", rc_path.display().to_string().cyan());
    log_debug!("[ShellRC] Preparing to read the existing content of the RC file for smart merging.");

    // Step 2: Read Existing RC File Content
    // To avoid creating duplicate entries, we absolutely need to know what's already
    // inside the RC file. The `read_rc_lines` helper reads every line for us.
    let existing_lines = read_rc_lines(&rc_path);
    // A quick debug check to confirm how many lines we successfully read.
    log_debug!(
        "[ShellRC] Successfully read {} existing line(s) from {}.",
        existing_lines.len().to_string().bold(),
        rc_path.display()
    );
    // For extreme debugging situations, you could uncomment the line below to see
    // a snippet of the existing content. Be careful with large files!
    // log_debug!("[ShellRC] Existing RC file content (first 10 lines): {:?}", existing_lines.iter().take(10).collect::<Vec<_>>());

    // This is where we'll collect all the brand-new lines that need to be added.
    // It starts empty and gets populated as we compare.
    let mut new_lines_to_add: Vec<String> = vec![];
    log_debug!("[ShellRC] Awaiting new configuration entries; 'new_lines_to_add' is currently empty.");

    // Step 3a: Process Raw Configurations from `shellrc.yaml`
    log_debug!("[ShellRC] Now, let's carefully review the 'raw_configs' from your shellrc.yaml.");
    // We'll go through each raw configuration snippet you've provided.
    for raw_config_entry in &shellrc.raw_configs {
        // First, we tidy up the raw configuration by removing any extra spaces
        // from the beginning or end. This helps with accurate comparisons.
        let trimmed_raw = raw_config_entry.trim();
        log_debug!("[ShellRC] Examining raw config entry: '{}'", trimmed_raw.dimmed());

        // This is the core logic for avoiding duplicates:
        // We check if *any* of the `existing_lines` in the RC file already `contains`
        // our `trimmed_raw` configuration. Using `contains` makes this check robust
        // against slight variations like extra spaces or comments on the same line in the RC file.
        if !existing_lines.iter().any(|existing_line| existing_line.contains(trimmed_raw)) {
            // Hooray! This raw config is not found in the existing file. It's truly new!
            log_info!("[ShellRC] Discovered a new raw configuration! Adding: {}", trimmed_raw.green());
            new_lines_to_add.push(raw_config_entry.clone()); // Add the original (untrimmed) line to our list.
        } else {
            // No need to add this one; it's already there!
            log_debug!("[ShellRC] Raw config '{}' is already happily living in your RC file. Skipping.", trimmed_raw.yellow());
        }
    }
    log_debug!("[ShellRC] Finished checking all 'raw_configs'.");

    // Step 3b: Process Aliases from `shellrc.yaml`
    log_debug!("[ShellRC] Next up: let's look at your custom 'aliases' from shellrc.yaml.");
    // Now, we do a similar check for each alias you've defined.
    for alias_entry in aliases {
        // We first format the alias exactly how it would appear in a shell RC file:
        // `alias <name>='<value>'`. This ensures our comparison is accurate.
        let alias_line_to_check = format!("alias {}='{}'", alias_entry.name, alias_entry.value);
        log_debug!("[ShellRC] Examining alias definition: '{}'", alias_line_to_check.dimmed());

        // Again, we check if this exact alias definition already `contains`
        // within any of the `existing_lines` in the RC file.
        if !existing_lines.iter().any(|existing_line| existing_line.contains(&alias_line_to_check)) {
            // Fantastic! This alias is new and not in the RC file yet.
            log_info!("[ShellRC] Found a new alias to add! It's: {}", alias_line_to_check.green());
            new_lines_to_add.push(alias_line_to_check); // Add the freshly formatted alias line to our list.
        } else {
            // This alias is already set up; no action needed.
            log_debug!("[ShellRC] Alias '{}' is already defined in your RC file. Skipping.", alias_entry.name.yellow());
        }
    }
    log_debug!("[ShellRC] All 'aliases' have been processed.");

    // Step 4: Append New Lines to RC File (if necessary)
    // After all the checks, we look at our `new_lines_to_add` list.
    // Is there anything new to write to the RC file?
    if new_lines_to_add.is_empty() {
        // If the list is empty, it means your RC file is already up-to-date!
        log_info!(
            "[ShellRC] Good news! No new configurations or aliases were found for {:?}. Your RC file remains untouched.",
            rc_path.display().to_string().cyan()
        );
    } else {
        // Ah, there are new lines to add! Let's get to work.
        log_info!(
            "[ShellRC] Identified {} new line(s) to add. Appending them to {}.",
            new_lines_to_add.len().to_string().bold(),
            rc_path.display().to_string().cyan()
        );
        // We call the `append_to_rc_file` helper to safely write these new lines.
        match append_to_rc_file(&rc_path, new_lines_to_add) {
            Ok(_) => {
                // Success! The file has been updated.
                log_info!(
                    "[ShellRC] RC file {} updated successfully! For these changes to take effect, remember to reload your shell (e.g., type `source {}` in your terminal).",
                    rc_path.display().to_string().green(),
                    rc_path.file_name().unwrap().to_string_lossy().blue() // Just display the filename for the source command
                );
                log_debug!("[ShellRC] Append operation finished with no issues.");
            }
            Err(err) => {
                // Uh oh! Something went wrong while trying to write to the file.
                // We'll alert the user with a warning, suggesting they check permissions.
                log_warn!(
                    "[ShellRC] Failed to write new configurations to RC file {}: {}. Please double-check your file permissions and try again.",
                    rc_path.display().to_string().red(),
                    err.to_string().red()
                );
            }
        }
    }
    log_debug!("[ShellRC] Shell configuration application process for `apply_shellrc` has concluded.");
}
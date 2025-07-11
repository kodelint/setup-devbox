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
use crate::utils::{append_to_rc_file, contains_multiline_block, get_rc_file, read_rc_lines};
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
    log_debug!("[Shell Config] Starting the process to apply configurations for shell: {}", shellrc.shell.bold());
    // For deeper dives, let's log the full details of the shell configuration
    // and the aliases we've received. This is great for debugging!
    // Pretty print shellrc (ShellRC struct)
    match serde_json::to_string_pretty(shellrc) {
        Ok(pretty_shellrc) => {
            log_debug!("[Shell Config] Received ShellRc configuration details:\n{}", pretty_shellrc);
        },
        Err(e) => {
            log_warn!("[Shell Config] Failed to pretty-print ShellRc for debug log: {}", e);
            log_debug!("[Shell Config] Received ShellRc configuration details: {:?}", shellrc); // Fallback to debug
        }
    }

    // Pretty print aliases (slice of AliasEntry)
    match serde_json::to_string_pretty(aliases) {
        Ok(pretty_aliases) => {
            log_debug!("[Shell Config] Received aliases to consider:\n{}", pretty_aliases);
        },
        Err(e) => {
            log_warn!("[Shell Config] Failed to pretty-print aliases for debug log: {}", e);
            log_debug!("[Shell Config] Received aliases to consider: {:?}", aliases); // Fallback to debug
        }
    }

    // Step 1: Identify the Correct RC File
    // We need to pinpoint the exact shell RC file (like `.zshrc` or `.bashrc`)
    // that corresponds to the shell specified in your `shellrc` configuration.
    // The `get_rc_file` helper function does this detective work for us.
    let Some(rc_path) = get_rc_file(&shellrc.shell) else {
        // If `get_rc_file` comes up empty (returns `None`), it means the shell
        // you've specified isn't one we currently know how to handle.
        // We'll log a warning and gracefully stop this operation for this shell.
        log_warn!(
            "[Shell Config] Skipping shell configuration: Unsupported or unrecognized shell '{}'. Our current mission only covers 'zsh' and 'bash'.",
            shellrc.shell.red()
        );
        return; // We can't proceed, so let's exit this function.
    };
    // Success! We've found the target RC file. Let's let everyone know.
    log_debug!("[Shell Config] Identified the target RC file path: {}", rc_path.display().to_string().cyan());
    log_debug!("[Shell Config] Preparing to read the existing content of the RC file for smart merging.");

    // Step 2: Read Existing RC File Content
    // To avoid creating duplicate entries, we absolutely need to know what's already
    // inside the RC file. The `read_rc_lines` helper reads every line for us.
    let existing_lines = read_rc_lines(&rc_path);
    // A quick debug check to confirm how many lines we successfully read.
    log_debug!(
        "[Shell Config] Successfully read {} existing line(s) from {}.",
        existing_lines.len().to_string().bold(),
        rc_path.display()
    );
    // For extreme debugging situations, you could uncomment the line below to see
    // a snippet of the existing content. Be careful with large files!
    // log_debug!("[ShellRC] Existing RC file content (first 10 lines): {:?}", existing_lines.iter().take(10).collect::<Vec<_>>());

    // This is where we'll collect all the brand-new lines that need to be added.
    // It starts empty and gets populated as we compare.
    let mut new_lines_to_add: Vec<String> = vec![];
    log_debug!("[Shell Config] Awaiting new configuration entries; 'new_lines_to_add' is currently empty.");

    // Step 3a: Process Raw Configurations from `shellrc.yaml`
    log_debug!("[Shell Config] Now, let's carefully review the 'raw_configs' from your shellrc.yaml.");
    // We'll go through each raw configuration snippet you've provided.
    log_debug!("[Shell Config] Now, let's carefully review the 'raw_configs' from your shellrc.yaml.");
    for raw_config_entry in &shellrc.raw_configs {
        // Split the raw_config_entry into individual lines for comparison
        let raw_config_lines: Vec<String> = raw_config_entry
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty() && !s.trim().starts_with('#')) // Apply similar filtering as read_rc_lines
            .collect();

        // If the parsed config block is empty, skip it.
        if raw_config_lines.is_empty() {
            log_debug!("[Shell Config] Raw config entry was empty or only comments/whitespace. Skipping.");
            continue;
        }

        log_debug!(
            "[Shell Config] Examining raw config entry ({} lines): '{}'",
            raw_config_lines.len().to_string().dimmed(),
            raw_config_lines[0].trim().dimmed() // Just show the first line for debug
        );

        if !contains_multiline_block(&existing_lines, &raw_config_lines) {
            // Hooray! This raw config is not found in the existing file. It's truly new!
            log_info!("[Shell Config] Discovered a new raw configuration! Adding:\n{}", raw_config_entry.green());
            new_lines_to_add.push(raw_config_entry.clone()); // Add the original (untrimmed) raw config
        } else {
            // No need to add this one; it's already there!
            log_debug!("[Shell Config] Raw config is already happily living in your RC file. Skipping.");
        }
    }
    log_debug!("[Shell Config] Finished checking all 'raw_configs'.");

    // Step 3b: Process Aliases from `shellrc.yaml`
    log_debug!("[Shell Config] Next up: let's look at your custom 'aliases' from shellrc.yaml.");
    // Now, we do a similar check for each alias you've defined.
    for alias_entry in aliases {
        // We first format the alias exactly how it would appear in a shell RC file:
        // `alias <name>='<value>'`. This ensures our comparison is accurate.
        let alias_line_to_check = format!("alias {}='{}'", alias_entry.name, alias_entry.value);
        log_debug!("[Shell Config] Examining alias definition: '{}'", alias_line_to_check.dimmed());

        // Again, we check if this exact alias definition already `contains`
        // within any of the `existing_lines` in the RC file.
        if !existing_lines.iter().any(|existing_line| existing_line.contains(&alias_line_to_check)) {
            // Fantastic! This alias is new and not in the RC file yet.
            log_info!("[Shell Config] Found a new alias to add! It's: {}", alias_line_to_check.green());
            new_lines_to_add.push(alias_line_to_check); // Add the freshly formatted alias line to our list.
        } else {
            // This alias is already set up; no action needed.
            log_debug!("[Shell Config] Alias '{}' is already defined in your RC file. Skipping.", alias_entry.name.yellow());
        }
    }
    log_debug!("[Shell Config] All 'aliases' have been processed.");

    // Step 4: Append New Lines to RC File (if necessary)
    // After all the checks, we look at our `new_lines_to_add` list.
    // Is there anything new to write to the RC file?
    if new_lines_to_add.is_empty() {
        // If the list is empty, it means your RC file is already up-to-date!
        log_info!(
            "[Shell Config] No new configurations or aliases were found for {}.",
            rc_path.display().to_string().cyan()
        );
    } else {
        // Ah, there are new lines to add! Let's get to work.
        log_info!(
            "[Shell Config] Identified {} new line(s) to add. Appending them to {}.",
            new_lines_to_add.len().to_string().bold(),
            rc_path.display().to_string().cyan()
        );
        // We call the `append_to_rc_file` helper to safely write these new lines.
        match append_to_rc_file(&rc_path, new_lines_to_add) {
            Ok(_) => {
                // Success! The file has been updated.
                log_info!(
                    "[Shell Config] RC file {} updated successfully!",
                    rc_path.display().to_string().green()// Just display the filename for the source command
                );
                log_debug!("[Shell Config] Append operation finished with no issues.");
            }
            Err(err) => {
                // Uh oh! Something went wrong while trying to write to the file.
                // We'll alert the user with a warning, suggesting they check permissions.
                log_warn!(
                    "[Shell Config] Failed to write new configurations to RC file {}: {}. Please double-check your file permissions and try again.",
                    rc_path.display().to_string().red(),
                    err.to_string().red()
                );
            }
        }
    }
    log_debug!("[Shell Config] Shell configuration application process for `apply_shellrc` has concluded.");
}
use crate::libs::shell_manager::{
    ensure_sections_exist, insert_into_section, is_command_update, log_section_stats,
    normalize_command, parse_existing_sections, section_header_name,
};
use crate::libs::utilities::file_operations::{
    get_rc_file, read_rc_file, remove_rc_file, source_rc_file, write_rc_file,
};
use crate::libs::utilities::platform::is_env_var_set;
use crate::schemas::shell_configuration::{
    AliasEntry, ConfigSection, RunCommandEntry, ShellConfig,
};
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Applies shell configurations (run commands and aliases) to the user's shell RC file.
/// This function serves as the main entry point for configuring shell environments.
///
/// # Arguments
/// * `shell_cfg` - A `ShellConfig` struct containing shell type, run commands, and aliases
///
/// # Behavior
/// - Determines the appropriate RC file path based on shell type
/// - Processes run commands and aliases
/// - Sources the updated RC file to apply changes immediately
/// - Handles unsupported shells gracefully with warning messages
pub fn apply_shell_configs(shell_cfg: ShellConfig) {
    eprintln!("{}:", "Shell Configuration".bright_yellow().bold());
    println!("{}\n", "=".repeat(20).bright_yellow());
    log_info!("[Shell Config] Applying Shell Configurations...");

    let Some(rc_path) = get_rc_file(&shell_cfg.run_commands.shell) else {
        log_warn!(
            "[Shell Config] Unsupported shell '{}'. Skipping configuration.",
            shell_cfg.run_commands.shell.red()
        );
        return;
    };

    log_debug!(
        "[Shell Config] Target RC file: {}",
        rc_path.display().to_string().cyan()
    );

    // Process run commands and aliases
    if let Err(e) = process_shell_config(
        &rc_path,
        &shell_cfg.run_commands.run_commands,
        &shell_cfg.aliases,
    ) {
        log_error!(
            "[Shell Config] Failed to process shell configuration: {}",
            e
        );
        return;
    }

    // Source the updated RC file
    if let Err(e) = source_rc_file(&shell_cfg.run_commands.shell, &rc_path) {
        log_warn!(
            "[Shell Config] Failed to source RC file: {}",
            e.to_string().yellow()
        );
    }
}

/// Main function to process all shell configurations with intelligent update detection
/// and regeneration capabilities when updates are detected.
///
/// # Arguments
/// * `rc_path` - Path to the shell RC file
/// * `run_commands` - Slice of run command entries to process
/// * `aliases` - Slice of alias entries to process
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Result indicating success or failure
///
/// # Algorithm
/// 1. Read existing RC file content
/// 2. Parse existing managed sections
/// 3. Check if any commands/aliases need updates (not just additions)
/// 4. If updates detected and regeneration enabled, perform full regeneration
/// 5. Otherwise, process normally with append-only operations
/// 6. Write changes if any modifications were made
fn process_shell_config(
    rc_path: &Path,
    run_commands: &[RunCommandEntry],
    aliases: &[AliasEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lines = read_rc_file(rc_path);

    // Parse existing managed sections
    let existing_content = parse_existing_sections(&lines);

    // Check if we need to do a full regeneration first
    let needs_regeneration = check_for_updates(run_commands, aliases, &existing_content);

    if needs_regeneration && is_env_var_set("SDB_RESET_SHELLRC_FILE") {
        log_info!("[Shell Config] Updates detected - regenerating Shell RC file");

        // Remove the file and start fresh
        if let Err(e) = remove_rc_file(rc_path) {
            log_error!(
                "[Shell Config] Failed to remove RC file before reset: {}",
                e
            );
            return Err(Box::new(e));
        }

        // Start with empty lines for complete regeneration
        lines = Vec::new();

        // Ensure managed sections exist in the fresh file
        ensure_sections_exist(&mut lines, run_commands, aliases);

        // Process all commands as new additions (no duplicate checking against old content)
        // Bypass any checks and recreates the Shell RC File as new
        // Needs `SDB_RESET_SHELLRC_FILE` to be set
        process_run_commands_after_reset(&mut lines, run_commands);
        process_aliases_after_reset(&mut lines, aliases);

        final_write(rc_path, &lines)?;
    } else if needs_regeneration {
        // Updates detected but env var not set - just warn
        log_warn!("[Shell Config] Updates detected but file regeneration disabled");
        log_warn!(
            "[Shell Config] Set Environment Variable: {} to automate regeneration",
            "SDB_RESET_SHELLRC_FILE".yellow()
        );
    } else {
        // No updates needed, process normally
        ensure_sections_exist(&mut lines, run_commands, aliases);

        let mut changes_made = false;
        changes_made |= process_run_commands(&mut lines, run_commands, &existing_content);
        changes_made |= process_aliases(&mut lines, aliases, &existing_content);

        if changes_made {
            final_write(rc_path, &lines)?;
        } else {
            log_info!("[Shell Config] No changes needed - all configurations are up to date");
        }
    }

    Ok(())
}

/// Checks if any commands or aliases need actual updates (not just new additions)
/// This function distinguishes between new commands and updates to existing ones.
///
/// # Arguments
/// * `run_commands` - Slice of run command entries to check
/// * `aliases` - Slice of alias entries to check
/// * `existing_content` - HashMap of existing content by section
///
/// # Returns
/// * `bool` - True if any updates are detected, false otherwise
///
/// # Logic
/// - For run commands: Checks if command with same "type" but different value exists
/// - For aliases: Checks if alias with same name but different value exists
/// - Returns true only for actual updates, not for new additions
fn check_for_updates(
    run_commands: &[RunCommandEntry],
    aliases: &[AliasEntry],
    existing_content: &HashMap<ConfigSection, HashSet<String>>,
) -> bool {
    // Check run commands for actual updates (not exact duplicates)
    for run_command in run_commands {
        let command = run_command.command.trim();
        if command.is_empty() {
            continue;
        }

        let normalized_command = normalize_command(command);
        let empty_set = HashSet::new();
        let existing_in_section = existing_content
            .get(&run_command.section)
            .unwrap_or(&empty_set);

        // Skip if exact command already exists
        if existing_in_section.contains(&normalized_command) {
            continue;
        }

        // Check if this is a true update (same command type but different value)
        if is_command_update(command, existing_in_section) {
            return true;
        }
    }

    // Check aliases for actual updates (not exact duplicates)
    let empty_set = HashSet::new();
    let existing_aliases = existing_content
        .get(&ConfigSection::Aliases)
        .unwrap_or(&empty_set);

    for alias in aliases {
        let alias_line = format!("alias {}='{}'", alias.name, alias.value);
        let normalized_alias = normalize_command(&alias_line);

        // Skip if exact alias already exists
        if existing_aliases.contains(&normalized_alias) {
            continue;
        }

        // Check if this is a true update (same alias name but different value)
        let has_same_name = existing_aliases
            .iter()
            .any(|existing| existing.starts_with(&format!("alias {}", alias.name)));

        if has_same_name {
            return true; // This is a real update, not a duplicate
        }
    }

    false
}

/// Process run commands for a fresh file (no existing content check)
/// Used during full regeneration when starting from an empty file.
///
/// # Arguments
/// * `lines` - Mutable reference to the lines vector being built
/// * `run_commands` - Slice of run command entries to process
///
/// # Features
/// - Handles duplicate commands within the same regeneration session
/// - Tracks section statistics for logging
/// - Uses normalized commands for duplicate detection
fn process_run_commands_after_reset(lines: &mut Vec<String>, run_commands: &[RunCommandEntry]) {
    let mut section_stats: HashMap<ConfigSection, u32> = HashMap::new();
    let mut processed_commands: HashSet<String> = HashSet::new();

    for run_command in run_commands {
        let section = &run_command.section;
        let command = run_command.command.trim();

        if command.is_empty() {
            continue;
        }

        let normalized_command = normalize_command(command);

        // Check if we've already processed this exact command
        if processed_commands.contains(&normalized_command) {
            log_debug!(
                "[Shell Config] Skipping duplicate command: {}",
                command.dimmed()
            );
            continue;
        }

        if insert_into_section(lines, command, section) {
            processed_commands.insert(normalized_command);
            *section_stats.entry(section.clone()).or_insert(0) += 1;
            log_info!(
                "[Shell Config] Added command to {} section: {}",
                section_header_name(section).cyan(),
                command.green()
            );
        }
    }

    log_section_stats(&section_stats);
}

/// Process aliases for a fresh file (no existing content check)
/// Used during full regeneration when starting from an empty file.
///
/// # Arguments
/// * `lines` - Mutable reference to the lines vector being built
/// * `aliases` - Slice of alias entries to process
///
/// # Features
/// - Handles duplicate aliases within the same regeneration session
/// - Uses normalized aliases for duplicate detection
/// - Tracks total aliases added for logging
fn process_aliases_after_reset(lines: &mut Vec<String>, aliases: &[AliasEntry]) {
    let mut added = 0;
    let mut processed_aliases: HashSet<String> = HashSet::new();

    for alias in aliases {
        let alias_line = format!("alias {}='{}'", alias.name, alias.value);
        let normalized_alias = normalize_command(&alias_line);

        // Check if we've already processed this exact alias
        if processed_aliases.contains(&normalized_alias) {
            log_debug!(
                "[Shell Config] Skipping duplicate alias: {}",
                alias_line.dimmed()
            );
            continue;
        }

        if insert_into_section(lines, &alias_line, &ConfigSection::Aliases) {
            processed_aliases.insert(normalized_alias);
            added += 1;
            log_info!("[Shell Config] Added alias: {}", alias_line.green());
        }
    }

    if added > 0 {
        log_info!(
            "[Shell Config] Aliases section: {} added",
            added.to_string().cyan()
        );
    }
}

/// Processes run command entries in append mode (normal operation)
/// Only adds new commands that don't already exist in the RC file.
///
/// # Arguments
/// * `lines` - Mutable reference to the lines vector
/// * `run_commands` - Slice of run command entries to process
/// * `existing_content` - HashMap of existing content by section
///
/// # Returns
/// * `bool` - True if any changes were made, false otherwise
///
/// # Behavior
/// - Skips empty commands
/// - Skips commands that already exist exactly
/// - Warns if updates are detected (should not happen in append mode)
/// - Tracks section statistics for logging
fn process_run_commands(
    lines: &mut Vec<String>,
    run_commands: &[RunCommandEntry],
    existing_content: &HashMap<ConfigSection, HashSet<String>>,
) -> bool {
    let mut changes_made = false;
    let mut section_stats: HashMap<ConfigSection, u32> = HashMap::new();

    for run_command in run_commands {
        let section = &run_command.section;
        let command = run_command.command.trim();

        if command.is_empty() {
            continue;
        }

        // Normalize for comparison with existing content
        let normalized_command = normalize_command(command);
        let empty_set = HashSet::new();
        // Get existing commands in this section
        let existing_in_section = existing_content.get(section).unwrap_or(&empty_set);

        // Check if this exact command already exists in the RC file
        if existing_in_section.contains(&normalized_command) {
            log_debug!(
                "[Shell Config] Command already exists in {} section: {}",
                section_header_name(section).dimmed(),
                command.dimmed()
            );
            continue;
        }

        // Check if this would be an update to an existing command (different value for same "type")
        let is_update = is_command_update(command, existing_in_section);

        if is_update {
            // This should not happen in normal append mode since we check for updates earlier
            log_warn!("[Shell Config] Update detected in append mode - this shouldn't happen");
            continue;
        } else if insert_into_section(lines, command, section) {
            *section_stats.entry(section.clone()).or_insert(0) += 1;
            changes_made = true;
            log_info!(
                "[Shell Config] Added command to {} section: {}",
                section_header_name(section).cyan(),
                command.green()
            );
        }
    }
    // Log statistics about commands added to each section
    log_section_stats(&section_stats);
    changes_made
}

/// Processes alias entries in append mode (normal operation)
/// Only adds new aliases that don't already exist in the RC file.
///
/// # Arguments
/// * `lines` - Mutable reference to the lines vector
/// * `aliases` - Slice of alias entries to process
/// * `existing_content` - HashMap of existing content by section
///
/// # Returns
/// * `bool` - True if any changes were made, false otherwise
///
/// # Behavior
/// - Skips aliases that already exist exactly
/// - Warns if updates are detected (should not happen in append mode)
/// - Tracks total aliases added for logging
fn process_aliases(
    lines: &mut Vec<String>,
    aliases: &[AliasEntry],
    existing_content: &HashMap<ConfigSection, HashSet<String>>,
) -> bool {
    let mut changes_made = false;
    let mut added = 0;

    let empty_set = HashSet::new();
    let existing_aliases = existing_content
        .get(&ConfigSection::Aliases)
        .unwrap_or(&empty_set);

    for alias in aliases {
        let alias_line = format!("alias {}='{}'", alias.name, alias.value);
        let normalized_alias = normalize_command(&alias_line);

        // Check if this exact alias already exists
        if existing_aliases.contains(&normalized_alias) {
            log_debug!(
                "[Shell Config] Alias already exists: {}",
                alias_line.dimmed()
            );
            continue;
        }

        // Check if this is an update
        let is_update = existing_aliases
            .iter()
            .any(|existing| existing.starts_with(&format!("alias {}", alias.name)));

        // Check if an alias with the same name but different value exists
        if is_update {
            // This should not happen in normal append mode since we check for updates earlier
            log_warn!("[Shell Config] Update detected in append mode - this shouldn't happen");
            continue;
        } else {
            // This is a new alias, add it to the aliases section
            if insert_into_section(lines, &alias_line, &ConfigSection::Aliases) {
                // Update count and track changes
                added += 1;
                changes_made = true;
                log_info!("[Shell Config] Added alias: {}", alias_line.green());
            }
        }
    }

    if added > 0 {
        log_info!(
            "[Shell Config] Aliases section: {} added",
            added.to_string().cyan()
        );
    }

    changes_made
}

/// Writes the final configuration to the RC file with proper error handling
///
/// # Arguments
/// * `rc_path` - Path to the RC file to write
/// * `lines` - Slice of strings representing the file content
///
/// # Returns
/// * `Result<(), std::io::Error>` - Success or IO error
///
/// # Behavior
/// - Attempts to write the file content
/// - Logs appropriate success/error messages
/// - Returns the IO error for proper error propagation
fn final_write(rc_path: &Path, lines: &[String]) -> Result<(), std::io::Error> {
    write_rc_file(rc_path, lines).inspect_err(|e| {
        log_warn!(
            "[Shell Config] Failed to write RC file: {}",
            e.to_string().red()
        );
    })?;

    log_info!("[Shell Config] Successfully updated shell configuration");
    Ok(())
}

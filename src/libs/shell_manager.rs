use crate::schemas::shell_configuration::{AliasEntry, ConfigSection, RunCommandEntry};
use crate::{log_debug, log_info};
use colored::Colorize;
use std::collections::{HashMap, HashSet};

/// Checks if a command is an update to an existing one rather than a new command
/// This is important for detecting when we need to regenerate the entire file vs just appending
///
/// # Arguments
/// * `command` - The new command to check
/// * `existing_commands` - Set of existing normalized commands in the same section
///
/// # Returns
/// * `bool` - True if this command updates an existing one, false if it's new
///
/// # Logic
/// - For export commands: Checks if the same variable name already exists with a different value
/// - For other commands: Currently returns false (could be extended for other patterns)

pub fn is_command_update(command: &str, existing_commands: &HashSet<String>) -> bool {
    // For export commands, check if we're updating the same variable
    if command.starts_with("export ") {
        if let Some(equals_pos) = command.find('=') {
            let var_name = &command[7..equals_pos]; // Skip "export "
            return existing_commands
                .iter()
                .any(|existing| existing.starts_with(&format!("export {}", var_name)));
        }
    }

    false
}

/// Detects which managed section a header line belongs to by parsing the header content
/// This helps identify the different managed sections in the RC file
///
/// # Arguments
/// * `line` - A line from the RC file that might be a section header
///
/// # Returns
/// * `Option<ConfigSection>` - The detected section if it's a managed header, None otherwise
pub fn detect_section_from_header(line: &str) -> Option<ConfigSection> {
    let trimmed = line.trim();

    // Check for each type of managed section header with the management signature
    if trimmed.contains("Paths Section") && trimmed.contains("Managed by setup-devbox") {
        Some(ConfigSection::Paths)
    } else if trimmed.contains("Evals Section") && trimmed.contains("Managed by setup-devbox") {
        Some(ConfigSection::Evals)
    } else if trimmed.contains("Exports Section") && trimmed.contains("Managed by setup-devbox") {
        Some(ConfigSection::Exports)
    } else if trimmed.contains("Other Section") && trimmed.contains("Managed by setup-devbox") {
        Some(ConfigSection::Other)
    } else if trimmed.contains("Aliases Section") && trimmed.contains("Managed by setup-devbox") {
        Some(ConfigSection::Aliases)
    } else if trimmed.contains("Functions Section") && trimmed.contains("Managed by setup-devbox") {
        Some(ConfigSection::Functions)
    } else {
        None
    }
}

/// Inserts content into a specific managed section of the RC file
/// This handles the actual placement of new commands/aliases in the correct section
///
/// # Arguments
/// * `lines` - Mutable reference to all lines in the RC file
/// * `content` - The content to insert (maybe multiple lines)
/// * `section` - The section where the content should be inserted
///
/// # Returns
/// * `bool` - True if insertion was successful, false if section wasn't found
///
/// # Process
/// 1. Find the start of the target section
/// 2. Find the end of the section (where to append new content)
/// 3. Insert the content at the correct position
pub fn insert_into_section(
    lines: &mut Vec<String>,
    content: &str,
    section: &ConfigSection,
) -> bool {
    let Some(section_start) = find_section_start(lines, section) else {
        return false;
    };

    // Find the end of this section (where to append new content)
    let mut insert_pos = section_start + 1;

    // Skip existing content in this section to find the end
    while insert_pos < lines.len() {
        let line = lines[insert_pos].trim();

        // Stop if we hit another section header
        if line.starts_with("# ") && line.contains("Section - Managed by setup-devbox") {
            break;
        }
        // Stop if we hit the end marker
        if line.starts_with("# End of setup-devbox managed content") {
            break;
        }
        // Skip empty lines at the end of section
        if line.is_empty() && insert_pos + 1 < lines.len() {
            let next_line = lines[insert_pos + 1].trim();
            if next_line.starts_with("# ")
                && next_line.contains("Section - Managed by setup-devbox")
            {
                break;
            }
            if next_line.starts_with("# End of setup-devbox managed content") {
                break;
            }
        }

        insert_pos += 1;
    }

    // Insert lines in correct order at the end of the section
    for line in content.lines() {
        lines.insert(insert_pos, line.to_string());
        insert_pos += 1; // Move insertion point for next line
    }
    // Successfully inserted
    true
}

/// Finds the line number where a specific section starts
/// This helps locate managed sections within the RC file content
///
/// # Arguments
/// * `lines` - All lines from the RC file
/// * `section` - The section to find
///
/// # Returns
/// * `Option<usize>` - Line number where section starts, or None if not found
pub fn find_section_start(lines: &[String], section: &ConfigSection) -> Option<usize> {
    let section_header = create_section_header(section);

    for (i, line) in lines.iter().enumerate() {
        if line.trim() == section_header {
            return Some(i);
        }
    }
    None
}

/// Creates a standardized section header string for a given section
/// This ensures consistent formatting across all managed sections
///
/// # Arguments
/// * `section` - The section to create a header for
///
/// # Returns
/// * `String` - The formatted section header line

pub fn create_section_header(section: &ConfigSection) -> String {
    format!(
        "# {} Section - Managed by setup-devbox",
        section_header_name(section)
    )
}

/// Gets the display name for a section used in header creation
/// This provides the human-readable section name
///
/// # Arguments
/// * `section` - The section to get the name for
///
/// # Returns
/// * `&'static str` - The display name of the section
pub fn section_header_name(section: &ConfigSection) -> &'static str {
    match section {
        ConfigSection::Paths => "Paths",
        ConfigSection::Evals => "Evals",
        ConfigSection::Exports => "Exports",
        ConfigSection::Other => "Other",
        ConfigSection::Functions => "Functions",
        ConfigSection::Aliases => "Aliases",
    }
}

/// Normalizes commands for consistent comparison
/// This handles variations in spacing, quoting, etc., to detect duplicates accurately
///
/// # Arguments
/// * `command` - The command to normalize
///
/// # Returns
/// * `String` - The normalized version of the command
///
/// # Normalization Rules
/// - Export commands: Keep only "export VAR_NAME" (strip the value)
/// - Alias commands: Keep only "alias NAME" (strip the value)
/// - Other commands: Normalize whitespace (collapse multiple spaces to single)
pub fn normalize_command(command: &str) -> String {
    // For exports, normalize the variable name extraction
    if command.starts_with("export ") {
        if let Some(equals_pos) = command.find('=') {
            let var_name = &command[7..equals_pos]; // Skip "export "
            return format!("export {}", var_name);
        }
    }

    // For aliases, normalize the alias name
    if command.starts_with("alias ") {
        if let Some(equals_pos) = command.find('=') {
            let alias_name = &command[6..equals_pos]; // Skip "alias "
            return format!("alias {}", alias_name);
        }
    }

    // For other commands, just normalize whitespace
    // Split on whitespace and rejoin with single spaces
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parses existing content from all managed sections in the RC file
/// This builds a map of what commands already exist in each section
///
/// # Arguments
/// * `lines` - All lines from the RC file
///
/// # Returns
/// * `HashMap<ConfigSection, HashSet<String>>` - Map of section to normalized existing commands
///
/// # Process
/// 1. Scans through all lines
/// 2. Detects section headers to track current section
/// 3. Collects non-comment, non-empty lines from managed sections
/// 4. Normalizes commands for consistent comparison
pub fn parse_existing_sections(lines: &[String]) -> HashMap<ConfigSection, HashSet<String>> {
    let mut existing: HashMap<ConfigSection, HashSet<String>> = HashMap::new();
    let mut current_section: Option<ConfigSection> = None;

    for line in lines {
        if let Some(section) = detect_section_from_header(line) {
            current_section = Some(section);
            continue;
        }

        // Reset section if we hit a non-managed section or significant gap
        if line.trim().starts_with("# ") && !line.contains("Managed by setup-devbox") {
            current_section = None;
            continue;
        }

        // If we're inside a managed section and this is a content line (not comment or empty)
        if let Some(section) = &current_section {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("#") {
                existing
                    .entry(section.clone())
                    .or_insert_with(HashSet::new)
                    .insert(normalize_command(trimmed));
            }
        }
    }

    existing
}

/// Ensures all needed sections exist in the RC file
/// This creates missing sections that are required by the current configuration
///
/// # Arguments
/// * `lines` - Mutable reference to all lines in the RC file
/// * `run_commands` - All run commands that need to be configured
/// * `aliases` - All aliases that need to be configured
///
/// # Process
/// 1. Determines which sections are needed based on the configuration
/// 2. Checks if each needed section exists
/// 3. Creates missing sections in the proper order
pub fn ensure_sections_exist(
    lines: &mut Vec<String>,
    run_commands: &[RunCommandEntry],
    aliases: &[AliasEntry],
) {
    let section_order = [
        ConfigSection::Paths,
        ConfigSection::Evals,
        ConfigSection::Exports,
        ConfigSection::Other,
        ConfigSection::Functions,
        ConfigSection::Aliases,
    ];

    // Collect all sections that are needed based on the configuration
    let mut needed_sections = HashSet::new();
    for cmd in run_commands {
        needed_sections.insert(cmd.section.clone());
    }
    if !aliases.is_empty() {
        needed_sections.insert(ConfigSection::Aliases);
    }

    // Add missing sections in order
    for section in section_order {
        if needed_sections.contains(&section) && find_section_start(lines, &section).is_none() {
            add_section_header(lines, &section);
        }
    }
}

/// Adds a section header at the appropriate location in the RC file
/// This maintains the proper section ordering when creating new sections
///
/// # Arguments
/// * `lines` - Mutable reference to all lines in the RC file
/// * `section` - The section to add
///
/// # Process
/// 1. Determines the correct position based on section ordering
/// 2. Adds appropriate spacing before the section
/// 3. Inserts the section header
/// 4. Logs the creation for visibility
pub fn add_section_header(lines: &mut Vec<String>, section: &ConfigSection) {
    // Define the preferred order of sections
    let section_order = [
        ConfigSection::Paths,
        ConfigSection::Evals,
        ConfigSection::Exports,
        ConfigSection::Other,
        ConfigSection::Functions,
        ConfigSection::Aliases,
    ];

    // Find the position of this section in the preferred order
    let target_index = section_order.iter().position(|s| s == section).unwrap_or(0);

    // Find where to insert this section
    let mut insert_pos = lines.len();

    for (i, line) in lines.iter().enumerate() {
        if let Some(existing_section) = detect_section_from_header(line) {
            let existing_index = section_order
                .iter()
                .position(|s| s == &existing_section)
                .unwrap_or(99);
            if existing_index > target_index {
                insert_pos = i;
                break;
            }
        }
    }

    // Add spacing before section if needed
    if insert_pos > 0 && !lines[insert_pos - 1].trim().is_empty() {
        lines.insert(insert_pos, "".to_string());
        insert_pos += 1;
    }

    lines.insert(insert_pos, create_section_header(section));
    log_debug!(
        "[Shell Config] Created {} section",
        section_header_name(section).cyan()
    );
}

/// Helper function to log statistics about commands added to each section
/// This provides user feedback about what was configured
///
/// # Arguments
/// * `section_stats` - Map of section to number of commands added
pub fn log_section_stats(section_stats: &HashMap<ConfigSection, u32>) {
    for (section, added) in section_stats {
        if *added > 0 {
            log_info!(
                "[Shell Config] {} section: {} added",
                section_header_name(section).cyan(),
                added.to_string().cyan()
            );
        }
    }
}

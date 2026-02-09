//! # Check Updates Module
//!
//! This module provides the functionality for the `check-updates` command.
//! It reads the `tools.yaml` configuration file, iterates through the defined tools,
//! and checks for the latest available versions using the appropriate installer.
//! The results are then displayed in a structured format in the console,
//! separated into two tables: "Updates Available" and "Manual Check Required".

use crate::config::load_single_config;
use crate::engine::installers::factory::InstallerFactory;
use crate::log_info;
use crate::schemas::path_resolver::PathResolver;
use colored::Colorize;
use prettytable::{Cell, Row, Table};

/// # `run`
///
/// This is the main entry point for the `check-updates` command.
///
/// ## Functionality
///
/// 1. **Load Configuration**: It loads the `tools.yaml` configuration file from the
///    default or specified configuration directory.
/// 2. **Initialize Installers**: It creates an `InstallerFactory` to get access
///    to the different tool installers (e.g., GitHub, Brew, Pip, UV, Go, Cargo, Rustup, URL).
/// 3. **Iterate Tools**: It iterates through each tool defined in the loaded `tools.yaml`
///    configuration.
/// 4. **Check Versions**: For each tool, it performs the following:
///    - **Skip "latest" or "N/A"**: If the configured version is "latest" or "N/A", the tool
///      is automatically added to the "Manual Check Required" table.
///    - **Get Latest Version**: It calls the `get_latest_version` method of the appropriate
///      installer to find the latest available version.
///    - **Version Normalization**: Both the configured and latest versions are normalized
///      (e.g., stripping a leading 'v' prefix) for accurate comparison.
///    - **Comparison and Categorization**:
///      - If a newer version is available and it's not a "Skipped" status, the tool
///        is added to the `updates_available_rows` list. The latest version is colored green.
///      - If the latest version check results in a "Skipped" message (e.g., for `uv tool`
///        mode or unversioned tools), or if an error occurs during detection, the tool
///        is added to the `manual_check_rows` list. Error messages are colored red.
///      - If the tool is up-to-date (normalized versions are the same), it is not added
///        to either table, reducing clutter.
/// 5. **Display Results**: After processing all tools, it prints two separate tables to
///    the console if their respective row lists are not empty:
///    - **"Updates Available"**: Lists tools where a newer version is found.
///      Columns: "Tool Name", "Configured Version", "Latest Version" (green if updated).
///    - **"Manual Check Required"**: Lists tools that were skipped or encountered errors
///      during version detection.
///      Columns: "Tool Name", "Configured Version", "Status" (with skipped/error messages).
///
/// ## Side Effects
///
/// - Prints formatted tables and informational messages to the console.
/// - Reads the `tools.yaml` file from the configuration directory.
pub fn run() {
    log_info!("[SDB::CheckUpdates] Checking for updates for tools defined in tools.yaml...");

    let paths = PathResolver::new(None, None).unwrap();
    let tools_yaml_path = paths.configs_dir().join("tools.yaml");
    let config_filename = "tools.yaml";

    let parsed_configs = load_single_config(&tools_yaml_path, config_filename);
    let installer_factory = InstallerFactory::new();

    if let Some(tools_cfg) = parsed_configs.tools {
        let mut updates_available_rows = Vec::new();
        let mut manual_check_rows = Vec::new();

        for tool in tools_cfg.tools {
            let current_version = tool.version.as_deref().unwrap_or("N/A").to_string();

            if current_version.to_lowercase() == "latest" || current_version == "N/A" {
                manual_check_rows.push(Row::new(vec![
                    Cell::new(&tool.name),
                    Cell::new(&current_version),
                    Cell::new("Skipped (version is 'latest' or not specified)"),
                ]));
                continue;
            }

            let source_type = &tool.source;

            if let Some(installer) = installer_factory.get_installer(source_type) {
                match installer.get_latest_version(&tool) {
                    Ok(latest_version) => {
                        let normalized_current = current_version
                            .strip_prefix('v')
                            .unwrap_or(&current_version);
                        let normalized_latest =
                            latest_version.strip_prefix('v').unwrap_or(&latest_version);

                        if normalized_current != normalized_latest
                            && !latest_version.starts_with("Skipped")
                        {
                            updates_available_rows.push(Row::new(vec![
                                Cell::new(&tool.name),
                                Cell::new(&current_version),
                                Cell::new(&latest_version).style_spec("Fg"),
                            ]));
                        } else if latest_version.starts_with("Skipped") {
                            manual_check_rows.push(Row::new(vec![
                                Cell::new(&tool.name),
                                Cell::new(&current_version),
                                Cell::new(&latest_version),
                            ]));
                        }
                    }
                    Err(e) => {
                        manual_check_rows.push(Row::new(vec![
                            Cell::new(&tool.name),
                            Cell::new(&current_version),
                            Cell::new(&format!("Error: {}", e)).style_spec("Fr"),
                        ]));
                    }
                }
            } else {
                manual_check_rows.push(Row::new(vec![
                    Cell::new(&tool.name),
                    Cell::new(&current_version),
                    Cell::new("No installer found for this source type").style_spec("Fr"),
                ]));
            }
        }

        if !updates_available_rows.is_empty() {
            println!("\n{}", "Updates Available".bold().green());
            let mut updates_table = Table::new();
            updates_table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
            updates_table.add_row(Row::new(vec![
                Cell::new("Tool Name").style_spec("b"),
                Cell::new("Configured Version").style_spec("b"),
                Cell::new("Latest Version").style_spec("b"),
            ]));
            for row in updates_available_rows {
                updates_table.add_row(row);
            }
            updates_table.printstd();
        }

        if !manual_check_rows.is_empty() {
            println!("\n{}", "Manual Check Required".bold().yellow());
            let mut manual_table = Table::new();
            manual_table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
            manual_table.add_row(Row::new(vec![
                Cell::new("Tool Name").style_spec("b"),
                Cell::new("Configured Version").style_spec("b"),
                Cell::new("Status").style_spec("b"),
            ]));
            for row in manual_check_rows {
                manual_table.add_row(row);
            }
            manual_table.printstd();
        }
    } else {
        log_info!("[SDB::CheckUpdates] No tools found in tools.yaml to check for updates.");
    }
}

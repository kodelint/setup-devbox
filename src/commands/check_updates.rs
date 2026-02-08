use crate::config::load_single_config;
use crate::schemas::path_resolver::PathResolver;
use colored::Colorize;
use prettytable::{Cell, Row, Table};

use crate::engine::installers::factory::InstallerFactory;
use crate::log_info;

pub fn run() {
    log_info!("[SDB::CheckUpdates] Checking for updates for tools defined in tools.yaml...");

    let paths = PathResolver::new(None, None).unwrap();
    let tools_yaml_path = paths.configs_dir().join("tools.yaml");
    let config_filename = "tools.yaml";

    let parsed_configs = load_single_config(&tools_yaml_path, config_filename);
    let installer_factory = InstallerFactory::new();

    if let Some(tools_cfg) = parsed_configs.tools {
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);
        table.add_row(Row::new(vec![
            Cell::new("Tool Name").style_spec("b"),
            Cell::new("Configured Version").style_spec("b"),
            Cell::new("Latest Version").style_spec("b"),
        ]));

        for tool in tools_cfg.tools {
            let current_version = tool.version.as_deref().unwrap_or("N/A").to_string();

            if current_version.to_lowercase() == "latest" || current_version == "N/A" {
                table.add_row(Row::new(vec![
                    Cell::new(&tool.name),
                    Cell::new(&current_version),
                    Cell::new("Skipped (version is 'latest' or not specified)"),
                ]));
                continue;
            }

            let latest_version_cell;
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
                            latest_version_cell = Cell::new(&latest_version.green().to_string());
                        } else {
                            latest_version_cell = Cell::new(&latest_version);
                        }
                    }
                    Err(e) => {
                        latest_version_cell = Cell::new(&format!("Error: {}", e).red().to_string());
                    }
                }
            } else {
                latest_version_cell =
                    Cell::new(&"No installer found for this source type".red().to_string());
            }

            table.add_row(Row::new(vec![
                Cell::new(&tool.name),
                Cell::new(&current_version),
                latest_version_cell,
            ]));
        }
        println!("\n");
        table.printstd();
    } else {
        log_info!("[SDB::CheckUpdates] No tools found in tools.yaml to check for updates.");
    }
}

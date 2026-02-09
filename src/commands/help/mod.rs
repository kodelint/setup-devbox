pub mod add_help;
pub mod check_updates_help;
pub mod edit_help;
pub mod generate_help;
pub mod installers_help;
pub mod now_help;
pub mod remove_help;
pub mod reset_help;
pub mod sync_config_help;

use self::add_help::show_add_help;
use self::check_updates_help::show_check_updates_help;
use self::edit_help::show_edit_help;
use self::generate_help::show_generate_help;
use self::installers_help::{add_supported_installers, show_installers_help};
use self::now_help::show_now_help;
use self::remove_help::show_remove_help;
use self::reset_help::show_reset_help;
use self::sync_config_help::show_sync_config_help;
use colored::Colorize;
use std::fmt::Write;

pub fn run(topic: Option<String>, detailed: bool, filter: Option<String>) {
    match topic.as_deref() {
        Some("add") => show_add_help(detailed),
        Some("edit") => show_edit_help(detailed),
        Some("generate") => show_generate_help(detailed),
        Some("installers") => show_installers_help(detailed, filter),
        Some("now") => show_now_help(detailed),
        Some("remove") => show_remove_help(detailed),
        Some("reset") => show_reset_help(detailed),
        Some("sync-config" | "sync_config") => show_sync_config_help(detailed),
        Some("check-updates") => show_check_updates_help(detailed),
        Some("version") => show_version_help(detailed),
        Some(unknown) => {
            show_unknown_topic_error(unknown);
            std::process::exit(1);
        }
        None => show_general_help(),
    }
}

fn show_unknown_topic_error(topic: &str) {
    eprintln!("{}: Unknown help topic '{}'", "Error".red(), topic);
    println!("\n{}", "Available help topics:".bold().yellow());

    const TOPICS: [(&str, &str); 10] = [
        ("add", "Show help for the 'add' command"),
        ("edit", "Show help for the 'edit' command"),
        ("generate", "Show help for the 'generate' command"),
        ("installers", "Show all supported installers"),
        ("now", "Show help for the 'now' command"),
        ("remove", "Show help for the 'remove' command"),
        ("reset", "Show help for the 'reset' command"),
        ("sync-config", "Show help for the 'sync-config' command"),
        ("check-updates", "Show help for the 'check-updates' command"),
        ("version", "Show help for the 'version' command"),
    ];

    let max_width = TOPICS
        .iter()
        .map(|(topic, _)| topic.len())
        .max()
        .unwrap_or(0);

    for (topic, desc) in &TOPICS {
        println!("  • {:width$} - {}", topic.cyan(), desc, width = max_width);
    }
}

fn show_general_help() {
    let mut output = String::with_capacity(4096);

    let _ = writeln!(output, "\n{}", "SETUP-DEVBOX:".bold().bright_blue());
    let _ = writeln!(output, "{}", "-------------".bold().blue());
    let _ = writeln!(
        output,
        "  Helps orchestrating development environments with automated tool installation,"
    );
    let _ = writeln!(
        output,
        "  standardized configurations, and reproducible setup workflows.\n"
    );

    add_commands_info(&mut output);
    add_supported_installers(&mut output);
    add_usage_info(&mut output);

    print!("{}", output);
}

fn add_commands_info(output: &mut String) {
    let _ = writeln!(output, "{}", "Commands:".bold().yellow());

    const COMMANDS: [(&str, &str); 10] = [
        (
            "now",
            "Installs and Configures Tools, Fonts, OS Settings and Shell Configs",
        ),
        ("generate", "Generates default configuration files"),
        (
            "sync-config",
            "Synchronizes or generates configurations from a state file",
        ),
        (
            "edit",
            "Edit configuration files or state file in your preferred editor",
        ),
        (
            "add",
            "Add a new tool, font, setting, or alias to configuration files",
        ),
        (
            "remove",
            "Remove an installed tool, font, alias, or setting",
        ),
        (
            "reset",
            "Reset the installation state (wipes entries from state file)",
        ),
        (
            "check-updates",
            "Checks for updates for all tools defined in tools.yaml",
        ),
        ("help", "Show detailed help for commands and installers"),
        ("version", "Show the current version of the tool"),
    ];

    let max_width = COMMANDS.iter().map(|(cmd, _)| cmd.len()).max().unwrap_or(0);

    for (cmd, desc) in &COMMANDS {
        let _ = writeln!(
            output,
            "  • {:width$} - {}",
            cmd.cyan(),
            desc,
            width = max_width
        );
    }
    let _ = writeln!(output);
}

fn add_usage_info(output: &mut String) {
    let _ = writeln!(output, "{}", "Usage:".bold().yellow());
    let _ = writeln!(output, "  setup-devbox [OPTIONS] <COMMAND>\n");
}

fn show_version_help(detailed: bool) {
    println!("{}", "setup-devbox version".bold().blue());
    if detailed {
        println!("Shows version information.");
    }
}

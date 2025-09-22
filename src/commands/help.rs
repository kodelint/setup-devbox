//! This module provides the `help` command functionality for the `setup-devbox` application.
//! It handles displaying various types of help information, including general usage,
//! detailed command explanations, and a list of supported installers.
//!
//! The module is structured to dispatch the help command based on user-provided topics,
//! and it includes functions for generating formatted, colored output for the terminal.

use crate::help_details::edit_help::show_edit_help;
use crate::help_details::generate_help::show_generate_help;
use crate::help_details::installers_help::{add_supported_installers, show_installers_help};
use crate::help_details::now_help::show_now_help;
use crate::help_details::sync_config_help::show_sync_config_help;
use colored::Colorize;
use std::fmt::Write;

/// Dispatches the main help command based on the provided topic.
///
/// This is the entry point for all help-related operations. It uses a `match` statement
/// to route the command to the appropriate display function based on the `topic` argument.
///
/// # Arguments
///
/// * `topic` - An `Option<String>` representing the specific topic the user wants help with.
///   If `None`, the general help message is displayed.
/// * `detailed` - A `bool` flag indicating whether to show a more detailed version of the help.
/// * `filter` - An `Option<String>` used to filter the output, currently only for the
///   `installers` topic.
pub fn run(topic: Option<String>, detailed: bool, filter: Option<String>) {
    match topic.as_deref() {
        Some("edit") => show_edit_help(detailed),
        Some("generate") => show_generate_help(detailed),
        Some("installers") => show_installers_help(detailed, filter),
        Some("now") => show_now_help(detailed),
        Some("sync-config") | Some("sync_config") => show_sync_config_help(detailed),
        Some("version") => show_version_help(detailed),
        // If the topic is provided but is not one of the known topics, show an error and exit.
        Some(unknown) => {
            show_unknown_topic_error(unknown);
            std::process::exit(1);
        }
        // If no topic is provided, display the default general help message.
        None => show_general_help(),
    }
}

/// Displays an error message for an unknown help topic and lists available topics.
///
/// This function is called when a user requests help for a topic that doesn't exist.
/// It prints a clear error message in red and then provides a list of valid topics
/// to guide the user.
///
/// # Arguments
///
/// * `topic` - The name of the unknown topic provided by the user.
fn show_unknown_topic_error(topic: &str) {
    eprintln!("{}: Unknown help topic '{}'", "Error".red(), topic);
    println!("\n{}", "Available help topics:".bold().yellow());

    // A constant array to store available topics and their short descriptions.
    const TOPICS: [(&str, &str); 5] = [
        ("installers", "Show all supported installers"),
        ("now", "Show help for the 'now' command"),
        ("generate", "Show help for the 'generate' command"),
        ("sync-config", "Show help for the 'sync-config' command"),
        ("version", "Show help for the 'version' command"),
    ];

    // Find the longest topic name for padding
    let max_width = TOPICS
        .iter()
        .map(|(topic, _)| topic.len())
        .max()
        .unwrap_or(0);

    // Iterate through the topics and print them in a formatted list.
    for (topic, desc) in &TOPICS {
        println!("  • {:width$} - {}", topic.cyan(), desc, width = max_width);
    }
}

/// Displays general help information about the `setup-devbox` tool.
///
/// This function serves as the main help page, providing a high-level overview of the
/// tool's purpose, scope, supported installers, environment variables, and available commands.
/// It builds the entire output string in memory before printing it to the console for efficiency.
fn show_general_help() {
    // Allocate a string with a reasonable initial capacity to minimize reallocations.
    let mut output = String::with_capacity(4096);

    // Use `writeln!` to build the formatted output string.
    writeln!(output, "\n{}", "SETUP-DEVBOX:".bold().bright_blue()).unwrap();
    writeln!(output, "{}", "-------------".bold().blue()).unwrap();
    writeln!(
        output,
        "  Helps orchestrating development environments with automated tool installation,"
    )
    .unwrap();
    writeln!(
        output,
        "  standardized configurations, and reproducible setup workflows.\n"
    )
    .unwrap();

    // Add the tool's scope section.
    writeln!(output, "{}", "Scope:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  • {} - no system-wide installations.",
        "[User-level operations only]".bold(),
    )
    .unwrap();
    writeln!(
        output,
        "  • Development environment foundation, not project dependencies."
    )
    .unwrap();
    writeln!(
        output,
        "  • Project packages managed by dedicated tools (cargo, pip, uv, etc.)."
    )
    .unwrap();
    writeln!(
        output,
        "  • Clean separation between tool installation and package management.\n"
    )
    .unwrap();

    // Call helper functions to add other sections to the output string.
    add_supported_installers(&mut output);
    add_environment_variables(&mut output);
    add_usage_info(&mut output);
    add_detailed_help_info(&mut output);

    // Print the fully built string to the console.
    print!("{output}");
}

/// Adds the "Supported Environment Variables" section to a mutable string.
///
/// This helper function lists the environment variables that can be used to
/// configure the `setup-devbox` tool's behavior.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_environment_variables(output: &mut String) {
    writeln!(
        output,
        "{}",
        "Supported Environment Variables:".bold().yellow()
    )
    .unwrap();

    let env_vars = [
        ("SDB_CONFIG_DIR", "Path for SDB Configuration directory."),
        (
            "SDB_RESET_SHELLRC_FILE",
            "\"true\" or \"false\" (see: \"setup-devbox help installers\" more details)",
        ),
    ];

    // Find the longest variable name for padding
    let max_width = env_vars
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(0);

    for (var_name, desc) in &env_vars {
        writeln!(
            output,
            "  • {:width$} - {}",
            var_name.bold().cyan(),
            desc,
            width = max_width
        )
        .unwrap();
    }
    writeln!(output).unwrap();
}

/// Adds the "Usage" and "Available Commands" sections to a mutable string.
///
/// This function provides a quick reference for the tool's command-line usage
/// and lists all available top-level commands.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_usage_info(output: &mut String) {
    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(output, "  setup-devbox [OPTIONS] <COMMAND>\n").unwrap();

    writeln!(output, "{}", "Available Commands:".bold().yellow()).unwrap();
    // A constant array for commands and their descriptions.
    let commands = [
        ("version", "Show the current Version of the tool"),
        (
            "now",
            "Installs and Configures Tools, Fonts, OS Settings and Shell Configs",
        ),
        ("generate", "Generates default configuration files"),
        (
            "sync-config",
            "Synchronizes or generates configurations from a state file",
        ),
        ("help", "Show detailed help for commands and installers"),
    ];

    // Find the longest command name for padding
    let max_width = commands.iter().map(|(cmd, _)| cmd.len()).max().unwrap_or(0);

    // Iterate and print each command.
    for (cmd, desc) in &commands {
        writeln!(
            output,
            "  • {:width$} - {}",
            cmd.green(),
            desc,
            width = max_width
        )
        .unwrap();
    }
    writeln!(output).unwrap();

    // Add global options section.
    writeln!(output, "{}", "Global Options:".bold().yellow()).unwrap();

    let options = [
        ("-d, --debug", "Enables detailed debug output"),
        ("-h, --help", "Print help"),
    ];

    // Find the longest option name for padding
    let max_width = options.iter().map(|(opt, _)| opt.len()).max().unwrap_or(0);

    for (opt, desc) in &options {
        writeln!(
            output,
            "  {:width$} - {}",
            opt.cyan(),
            desc,
            width = max_width + 2 // +2 for the bullet and space
        )
        .unwrap();
    }
    writeln!(output).unwrap();
}

/// Adds the "Detailed help information" section to a mutable string.
///
/// This section explains how to access more specific help topics and provides
/// examples of the `help` command usage.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_detailed_help_info(output: &mut String) {
    writeln!(
        output,
        "{}",
        "Detailed help information about SETUP-DEVBOX"
            .bold()
            .bright_blue()
    )
    .unwrap();
    writeln!(
        output,
        "{}",
        "------------------------------------------------\n"
            .blue()
            .bright_blue()
    )
    .unwrap();

    // Explain usage for help topics.
    writeln!(output, "{}", "Usages for [Help Topics]:".bold().white()).unwrap();
    writeln!(output, "  • setup-devbox help [TOPICS] <<Help Options>>\n").unwrap();

    // List available help topics with descriptions.
    writeln!(output, "{}", "Available Help Topics:".bold().white()).unwrap();
    let topics = [
        (
            "installers",
            "Show all supported installers and their details",
        ),
        ("now", "Show detailed help for installation command"),
        ("generate", "Show help for configuration generation"),
        ("sync-config", "Show help for configuration synchronization"),
        ("version", "Show version information"),
    ];

    // Find the longest topic name for padding
    let max_width = topics
        .iter()
        .map(|(topic, _)| topic.len())
        .max()
        .unwrap_or(0);

    for (topic, desc) in &topics {
        writeln!(
            output,
            "  • {:width$} - {}",
            topic.cyan(),
            desc,
            width = max_width
        )
        .unwrap();
    }
    writeln!(output).unwrap();

    // Describe the available options for the help command itself.
    writeln!(output, "{}", "Help Options:".bold().white()).unwrap();

    let help_options = [
        ("--detailed", "Show comprehensive information with examples"),
        ("--filter <type>", "Filter results (for installers topic)"),
    ];

    // Find the longest option name for padding
    let max_width = help_options
        .iter()
        .map(|(opt, _)| opt.len())
        .max()
        .unwrap_or(0);

    for (opt, desc) in &help_options {
        writeln!(
            output,
            "  • {:width$} - {}",
            opt.cyan(),
            desc,
            width = max_width
        )
        .unwrap();
    }

    writeln!(output, "{}: All Topics supports \"{}\" and installers optionally also support \"{}\" for specific installer\n",
             "Note".italic(), "--detailed".cyan(), "--filter <<installer_name>>".cyan()).unwrap();

    // Provide concrete examples for using the help command.
    writeln!(output, "{}", "Help Examples:".bold().white()).unwrap();
    let examples = [
        "• setup-devbox help installers",
        "• setup-devbox help installers --detailed",
        "• setup-devbox help installers --filter brew --detailed",
        "• setup-devbox help now --detailed",
    ];

    for example in &examples {
        writeln!(output, "  {}", example.italic()).unwrap();
    }
}

/// Displays help for the `version` command.
///
/// This function provides information on the `version` command, which shows
/// the tool's version and build details.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show comprehensive information.
fn show_version_help(detailed: bool) {
    println!("{}", "setup-devbox version".bold().blue());
    println!("Display version information for setup-devbox.\n");

    println!("{}", "Usage:".bold());
    println!("  setup-devbox version\n");

    if detailed {
        println!("{}", "Description:".bold());
        println!("Shows the current version of setup-devbox along with build information");
        println!("and supported features. Useful for troubleshooting and ensuring you're");
        println!("running the latest version.\n");

        // List the information that the command typically displays.
        println!("{}", "Information Displayed:".bold());
        let info_items = [
            "Version number",
            "Build date and commit hash (if available)",
            "Supported installer types",
            "Configuration file format version",
        ];

        for item in &info_items {
            println!("  • {item}");
        }
        println!();
    }

    // Provide a simple example.
    println!("{}", "Example:".bold());
    println!("  setup-devbox version");
}

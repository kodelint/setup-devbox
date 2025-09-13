//! This module provides the `help` command functionality for the `setup-devbox` application.
//! It handles displaying various types of help information, including general usage,
//! detailed command explanations, and a list of supported installers.
//!
//! The module is structured to dispatch the help command based on user-provided topics,
//! and it includes functions for generating formatted, colored output for the terminal.

use colored::Colorize;
use std::fmt::Write;
// Assuming this schema is defined elsewhere and contains the InstallerRegistry.
// The use of 'crate::' indicates it's a module within the same project.
use crate::schemas::help_schema::InstallerRegistry;

/// Dispatches the main help command based on the provided topic.
///
/// This is the entry point for all help-related operations. It uses a `match` statement
/// to route the command to the appropriate display function based on the `topic` argument.
///
/// # Arguments
///
/// * `topic` - An `Option<String>` representing the specific topic the user wants help with.
///             If `None`, the general help message is displayed.
/// * `detailed` - A `bool` flag indicating whether to show a more detailed version of the help.
/// * `filter` - An `Option<String>` used to filter the output, currently only for the
///              `installers` topic.
pub fn run(topic: Option<String>, detailed: bool, filter: Option<String>) {
    match topic.as_deref() {
        Some("installers") => show_installers_help(detailed, filter),
        Some("now") => show_now_help(detailed),
        Some("generate") => show_generate_help(detailed),
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

    // Iterate through the topics and print them in a formatted list.
    for (topic, desc) in &TOPICS {
        println!("  • {} - {}", topic.cyan(), desc);
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
        "  • {} - {}",
        "User-level operations only".bold(),
        "no system-wide installations.".dimmed()
    )
    .unwrap();
    writeln!(
        output,
        "  • {}",
        "Development environment foundation, not project dependencies."
    )
    .unwrap();
    writeln!(
        output,
        "  • {}",
        "Project packages managed by dedicated tools (cargo, pip, uv, etc.)."
    )
    .unwrap();
    writeln!(
        output,
        "  • {}\n",
        "Clean separation between tool installation and package management."
    )
    .unwrap();

    // Call helper functions to add other sections to the output string.
    add_supported_installers(&mut output);
    add_environment_variables(&mut output);
    add_usage_info(&mut output);
    add_detailed_help_info(&mut output);

    // Print the fully built string to the console.
    print!("{}", output);
}

/// Adds the "Supported Installers" section to a mutable string.
///
/// This is a helper function that populates a `String` with a list of all
/// supported installers, their descriptions, and any special notes.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_supported_installers(output: &mut String) {
    writeln!(output, "{}", "Supported Installers:".bold().yellow()).unwrap();

    // A constant array of installer information: (Name, Description, Extra Info).
    const INSTALLERS: [(&str, &str, &str); 10] = [
        ("Brew", "Package manager for macOS/Linux (Homebrew)", ""),
        ("Cargo", "Rust package manager for crates and binaries.", ""),
        ("Fonts", "Nerd Fonts installer from GitHub releases.", ""),
        ("Github", "Download tools from GitHub releases.", ""),
        ("Go", "Go package installer using", "go install"),
        ("Pip", "Python package installer using pip.", ""),
        ("Rustup", "Rust toolchain installer and manager.", ""),
        ("shell", "Shell configs", ".zshrc, .bashrc"),
        ("URL", "Download and install from direct URLs.", ""),
        (
            "UV",
            "Ultra-fast Python package installer",
            "tool/pip/python",
        ),
    ];

    // Iterate and format each installer entry.
    for (name, desc, extra) in &INSTALLERS {
        write!(output, "  • {} - {}", name.bold().cyan(), desc).unwrap();
        // Append extra information if it exists, with specific formatting for certain installers.
        if !extra.is_empty() {
            if *name == "Go" {
                write!(output, " '{}'.", extra.cyan()).unwrap();
            } else if *name == "shell" {
                write!(output, " ({} etc) and aliases.", extra.cyan()).unwrap();
            } else if *name == "UV" {
                write!(output, " ({} modes).", extra.cyan()).unwrap();
            }
        }
        writeln!(output).unwrap();
    }
    writeln!(output).unwrap();
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
    writeln!(
        output,
        "  • {} - Path for SDB Configuration directory.",
        "SDB_CONFIG_DIR".bold().cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  • {} - \"{}\" or \"{}\" (see: \"setup-devbox help installers\" more details)\n",
        "SDB_RESET_SHELLRC_FILE".bold().cyan(),
        "true".cyan(),
        "false".cyan()
    )
    .unwrap();
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

    // Iterate and print each command.
    for (cmd, desc) in &commands {
        writeln!(output, "  • {} - {}", cmd.green(), desc).unwrap();
    }
    writeln!(output).unwrap();

    // Add global options section.
    writeln!(output, "{}", "Global Options:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} - Enables detailed debug output",
        "-d, --debug".cyan()
    )
    .unwrap();
    writeln!(output, "  {} - Print help\n", "-h, --help".cyan()).unwrap();
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

    for (topic, desc) in &topics {
        writeln!(output, "  • {} - {}", topic.cyan(), desc).unwrap();
    }
    writeln!(output).unwrap();

    // Describe the available options for the help command itself.
    writeln!(output, "{}", "Help Options:".bold().white()).unwrap();
    writeln!(
        output,
        "  {} - Show comprehensive information with examples",
        "--detailed".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {} - Filter results (for installers topic)",
        "--filter <type>".cyan()
    )
    .unwrap();
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

/// Displays help for supported installers with optional filtering and detail.
///
/// This function retrieves a list of all supported installers from the `InstallerRegistry`
/// and formats the output. It respects the `--detailed` and `--filter` flags to
/// provide tailored information to the user.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show detailed information about each installer.
/// * `filter` - An `Option<String>` to filter the list of installers by name.
fn show_installers_help(detailed: bool, filter: Option<String>) {
    println!("\n{}", "Supported Installers".bold().blue());
    println!("{}", "--------------------".bold().blue());

    // Print a general description based on the `detailed` flag.
    if detailed {
        println!(
            "Comprehensive guide to all available installers with examples and configuration options.\n"
        );
    } else {
        println!("Overview of available installers for setting up your development environment.\n");
    }

    // Retrieve all installers from the registry.
    let installers = InstallerRegistry::get_all();
    // Normalize the filter string to lowercase for case-insensitive matching.
    let filter_str = filter.as_ref().map(|s| s.to_lowercase());

    // Iterate through the installers and display them, applying the filter if it exists.
    for installer in installers {
        if let Some(ref filter) = filter_str {
            // Skip installers that don't match the filter.
            if !installer.name.to_lowercase().contains(filter) {
                continue;
            }
        }

        // Call the `display` method on the installer object to print its details.
        installer.display(detailed);
    }

    // Print a tip if a filter was used.
    if filter.is_some() {
        println!(
            "\n{} Use without --filter to see all installers.",
            "Tip:".yellow()
        );
    }
}

/// Displays help for the `now` command.
///
/// This function explains the purpose, usage, and options for the `now` command,
/// which is responsible for installing and configuring the environment. It can
/// show either a basic or a detailed view.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show comprehensive information about the command.
fn show_now_help(detailed: bool) {
    let mut output = String::with_capacity(2048);

    writeln!(output, "\n{}", "setup-devbox now".bold().blue()).unwrap();
    writeln!(output, "{}", "--------------------".bold().blue()).unwrap();
    writeln!(
        output,
        "Install and configure your development environment based on configuration files.\n"
    )
    .unwrap();

    // Add usage and options to the output string.
    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(output, "  setup-devbox now [OPTIONS]\n").unwrap();

    writeln!(output, "{}", "Options:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} Use custom configuration file (default: {})",
        "--config <PATH>".cyan(),
        "~/.setup-devbox/config.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {} Use custom state file (default: {})",
        "--state <PATH>".cyan(),
        "~/.setup-devbox/state.json".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {} Force update all tools marked as '{}' version\n",
        "--update-latest".cyan(),
        "@latest".cyan()
    )
    .unwrap();

    // Conditionally add detailed or basic information based on the flag.
    if detailed {
        add_now_detailed_info(&mut output);
    } else {
        add_now_basic_examples(&mut output);
    }

    print!("{}", output);
}

/// Adds detailed information for the `now` command to a mutable string.
///
/// This function provides a deeper dive into the `now` command's functionality,
/// explaining its role in state management and the files it uses.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_now_detailed_info(output: &mut String) {
    writeln!(output, "{}", "Detailed Description:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  The 'now' command is the main installation command that reads your configuration"
    )
    .unwrap();
    writeln!(
        output,
        "  file and installs all specified tools, fonts, and configurations. It maintains"
    )
    .unwrap();
    writeln!(
        output,
        "  a state file to track what's been installed and their versions.\n"
    )
    .unwrap();

    // Explain the configuration file and its components.
    writeln!(output, "{}", "Configuration File:".bold().yellow()).unwrap();
    writeln!(
        output,
        "The configuration file ({}) defines what should be installed:",
        "config.yaml".cyan()
    )
    .unwrap();
    let config_items = [
        ("tools.yaml", "Tools and their versions"),
        ("fonts.yaml", "Fonts to install"),
        ("shellrc.yaml", "Shell configurations"),
        ("settings.yaml", "OS-specific settings"),
    ];

    for (file, desc) in &config_items {
        writeln!(output, "  • {} ({})", desc, file.cyan()).unwrap();
    }
    writeln!(output).unwrap();

    // Explain the state management file.
    writeln!(output, "{}", "State Management:".bold().yellow()).unwrap();
    writeln!(output, "The state file ({}) tracks:", "state.yaml".cyan()).unwrap();
    let state_items = [
        "Currently installed tools and versions",
        "Installation timestamps",
        "Success/failure status of installations",
    ];

    for item in &state_items {
        writeln!(output, "  • {}", item).unwrap();
    }
    writeln!(output).unwrap();

    // Add examples and behavior information from other helper functions.
    add_now_detailed_examples(output);
    add_now_behavior_info(output);
}

/// Adds detailed examples for the `now` command to a mutable string.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_now_detailed_examples(output: &mut String) {
    writeln!(output, "{}", "Examples:".bold().yellow()).unwrap();
    let examples = [
        "setup-devbox now",
        "setup-devbox now --config ./my-config.yaml",
        "setup-devbox now --update-latest",
        "setup-devbox now --config custom.yaml --state custom-state.json",
    ];

    for example in &examples {
        writeln!(output, "  {}", example).unwrap();
    }
    writeln!(output).unwrap();
}

/// Adds behavior information for the `now` command to a mutable string.
///
/// This function explains how the command manages installations, including
/// skipping, updating, and state file backups.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_now_behavior_info(output: &mut String) {
    writeln!(output, "{}", "Behavior:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  • Skips already installed tools unless versions differ"
    )
    .unwrap();
    writeln!(
        output,
        "  • Updates tools marked with '{}' only after configured time period",
        "latest".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  • Use {} to force update all tools '{}' version immediately",
        "--update-latest".italic().cyan(),
        "@latest".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  • Creates backup of state file before major changes"
    )
    .unwrap();
}

/// Adds basic examples for the `now` command to a mutable string.
///
/// This is a simple version of the examples, shown when `detailed` is false.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_now_basic_examples(output: &mut String) {
    writeln!(output, "{}", "Examples:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  setup-devbox now                    # Use default config"
    )
    .unwrap();
    writeln!(
        output,
        "  setup-devbox now --update-latest    # Force update latest versions"
    )
    .unwrap();
}

/// Displays help for the `generate` command.
///
/// This function explains the purpose, usage, and options for the `generate` command,
/// which creates starter configuration files.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show comprehensive information.
fn show_generate_help(detailed: bool) {
    println!("{}", "setup-devbox generate".bold().blue());
    println!("Generate default configuration and state files to get started.\n");

    println!("{}", "Usage:".bold());
    println!("  setup-devbox generate [OPTIONS]\n");

    println!("{}", "Options:".bold());
    println!(
        "  --config <PATH>  Save configuration to custom path (default: ~/.setup-devbox/config.yaml)"
    );
    println!(
        "  --state <PATH>   Save state file to custom path (default: ~/.setup-devbox/state.json)\n"
    );

    if detailed {
        show_generate_detailed_help();
    } else {
        show_generate_basic_examples();
    }
}

/// Shows detailed help for the `generate` command.
fn show_generate_detailed_help() {
    println!("{}", "Detailed Description:".bold());
    println!("The 'generate' command creates template configuration files that you can");
    println!("customize for your development environment. It's the recommended way to");
    println!("get started with setup-devbox.\n");

    // List the files that are created.
    println!("{}", "Generated Files:".bold());
    println!("  • config.yaml - Main configuration with example tools and settings");
    println!("  • state.json  - Empty state file to track installations\n");

    // Explain the contents of the generated configuration.
    println!("{}", "Configuration Structure:".bold());
    println!("The generated config includes examples for:");
    let config_examples = [
        "Common development tools (git, docker, node, etc.)",
        "Popular programming languages and their toolchains",
        "Essential fonts for development",
        "Shell configurations and aliases",
        "OS-specific tweaks and settings",
    ];

    for example in &config_examples {
        println!("  • {}", example);
    }
    println!();

    // Provide usage examples.
    println!("{}", "Examples:".bold());
    println!("  setup-devbox generate");
    println!("  setup-devbox generate --config ./project-config.yaml");
    println!("  setup-devbox generate --config dev.yaml --state dev-state.json\n");

    // Outline the typical workflow for a new user.
    println!("{}", "Next Steps:".bold());
    println!("  1. Run 'setup-devbox generate' to create default configs");
    println!("  2. Edit the generated config.yaml to match your needs");
    println!("  3. Run 'setup-devbox now' to install everything");
}

/// Shows basic examples for the `generate` command.
fn show_generate_basic_examples() {
    println!("{}", "Examples:".bold());
    println!("  setup-devbox generate                    # Create default configs");
    println!("  setup-devbox generate --config custom.yaml # Custom location");
}

/// Displays help for the `sync-config` command.
///
/// This function explains the purpose, usage, and options for the `sync-config` command,
/// which reconstructs configuration files from a state file.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show comprehensive information.
fn show_sync_config_help(detailed: bool) {
    println!("{}", "setup-devbox sync-config".bold().blue());
    println!("Synchronize or generate configurations from an existing state file.\n");

    println!("{}", "Usage:".bold());
    println!("  setup-devbox sync-config [OPTIONS]\n");

    println!("{}", "Options:".bold());
    println!("  --state <PATH>      Path to state file (default: ~/.setup-devbox/state.json)");
    println!(
        "  --output-dir <PATH> Output directory for configs (default: ~/.setup-devbox/configs)\n"
    );

    if detailed {
        show_sync_config_detailed_help();
    } else {
        show_sync_config_basic_examples();
    }
}

/// Shows detailed help for the `sync-config` command.
fn show_sync_config_detailed_help() {
    println!("{}", "Detailed Description:".bold());
    println!("The 'sync-config' command helps you recreate configuration files from");
    println!("an existing state file. This is useful for:");
    let use_cases = [
        "Recovering lost configuration files",
        "Sharing your setup across multiple machines",
        "Creating configuration templates from working setups",
    ];

    for case in &use_cases {
        println!("  • {}", case);
    }
    println!();

    // Provide additional use cases.
    println!("{}", "Use Cases:".bold());
    let additional_cases = [
        "Backup and restore configurations",
        "Team configuration sharing",
        "Migration between development environments",
        "Creating standardized setups",
    ];

    for case in &additional_cases {
        println!("  • {}", case);
    }
    println!();

    // Describe the generated output.
    println!("{}", "Generated Output:".bold());
    println!("Creates configuration files based on your state file:");
    let outputs = [
        "Reconstructed config.yaml with current tool versions",
        "Shell configuration files (.zshrc, .bashrc)",
        "Tool-specific configuration files",
        "Documentation of your current setup",
    ];

    for output in &outputs {
        println!("  • {}", output);
    }
    println!();

    // Provide usage examples.
    println!("{}", "Examples:".bold());
    println!("  setup-devbox sync-config");
    println!("  setup-devbox sync-config --state ./backup-state.json");
    println!("  setup-devbox sync-config --output-dir ./team-configs");
    println!("  setup-devbox sync-config --state prod.json --output-dir ./prod-configs\n");

    // Explain the typical workflow.
    println!("{}", "Workflow:".bold());
    println!("  1. Run 'setup-devbox now' to install and track your tools");
    println!("  2. Use 'setup-devbox sync-config' to generate shareable configs");
    println!("  3. Share the generated configs with your team or other machines");
}

/// Shows basic examples for the `sync-config` command.
fn show_sync_config_basic_examples() {
    println!("{}", "Examples:".bold());
    println!("  setup-devbox sync-config                 # Use default paths");
    println!("  setup-devbox sync-config --state backup.json # From backup");
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
            println!("  • {}", item);
        }
        println!();
    }

    // Provide a simple example.
    println!("{}", "Example:".bold());
    println!("  setup-devbox version");
}

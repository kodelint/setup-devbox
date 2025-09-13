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
use crate::schemas::help_schema::{InstallerRegistry, format_yaml_content};

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
        "  • {} - {}",
        "[User-level operations only]".bold(),
        "no system-wide installations."
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

    // Find the longest installer name for padding
    let max_width = INSTALLERS
        .iter()
        .map(|(name, _, _)| name.len())
        .max()
        .unwrap_or(0);

    for (name, desc, extra) in &INSTALLERS {
        write!(
            output,
            "  • {:width$} - {}",
            name.bold().cyan(),
            desc,
            width = max_width
        )
        .unwrap();
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
    let mut matched_installers = Vec::new();

    // Iterate through the installers and display them, applying the filter if it exists.
    for installer in &installers {
        if let Some(ref filter) = filter_str {
            // Skip installers that don't match the filter.
            if !installer.name.to_lowercase().contains(filter) {
                continue;
            }
        }

        // Call the `display` method on the installer object to print its details.
        installer.display(detailed);
        matched_installers.push(installer);
    }

    // If a filter was used but no installers matched, show available installers
    if let Some(ref original_filter) = filter {
        if matched_installers.is_empty() {
            println!(
                "{}: No installer found matching '{}'",
                "Error".red(),
                original_filter
            );
            println!("\n{}", "Available installers:".bold().yellow());

            // Get all installers again to show the complete list
            let all_installers = InstallerRegistry::get_all();

            // Find the longest installer name for padding
            let max_width = all_installers
                .iter()
                .map(|i| i.name.len())
                .max()
                .unwrap_or(0);

            for installer in all_installers {
                println!(
                    "  • {:width$} - {}",
                    installer.name.cyan(),
                    installer.description,
                    width = max_width
                );
            }
        } else {
            println!(
                "\n{} Use without --filter to see all installers.",
                "Tip:".yellow()
            );
        }
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
        "  {} Force update all tools marked as '{}' version. Overrides the configuration \"update_latest_only_after\"\n",
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
        "  files and installs all specified tools, fonts, and configurations. It maintains"
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
        "The configuration file ({}) is collection other configuration files paths:",
        "config.yaml".cyan()
    )
    .unwrap();
    let config_items = [
        (
            "tools.yaml",
            "Tools and their versions needs to be installed",
        ),
        ("fonts.yaml", "Fonts to install"),
        ("shellrc.yaml", "Shell configurations to be applied"),
        ("settings.yaml", "OS-specific settings changes to be made"),
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

    // Add behavior and examples information from other helper functions.
    add_now_behavior_info(output);
    add_now_detailed_examples(output);
}

/// Adds detailed examples for the `now` command to a mutable string.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
fn add_now_detailed_examples(output: &mut String) {
    writeln!(output, "\n{}", "Command Examples:".bold().yellow()).unwrap();
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

    add_configuration_examples(output);
}

/// Adds configuration file examples to a mutable string.
///
/// This function provides detailed examples of what the configuration files look like,
/// showing the structure and format users should follow.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
///
fn add_configuration_examples(output: &mut String) {
    writeln!(output, "{}", "Configuration Examples:".bold().yellow()).unwrap();
    writeln!(
        output,
        "Here are examples of the configuration files structure:\n"
    )
    .unwrap();

    // Main config.yaml example
    writeln!(output, "{}:\n", "config.yaml".bold().cyan()).unwrap();
    writeln!(output, "{}", "```yaml".green()).unwrap();
    let config_yaml = r#"# Tells SDB where to find the tools configuration
tools: /Users/<<user>>/.config/setup-devbox/configs/tools.yaml
# Tells SDB where to find the settings configuration
settings: /Users/<<user>>/.config/setup-devbox/configs/settings.yaml
# Tells SDB where to find the shell configuration
shellrc: /Users/<<user>>/.config/setup-devbox/configs/shellrc.yaml
# Tells SDB where to find the fonts configuration
fonts: /Users/<<user>>/.config/setup-devbox/configs/fonts.yaml"#;
    write!(output, "{}", format_yaml_content(config_yaml)).unwrap();
    writeln!(output, "{}\n", "```".green()).unwrap();

    // tools.yaml example
    writeln!(output, "{}:\n", "tools.yaml".bold().cyan()).unwrap();
    writeln!(output, "{}", "```yaml".green()).unwrap();
    let tools_yaml = r#"update_latest_only_after: "7 days"
tools:

  # This will install rustup
  - name: rustup
    source: brew # Install rustup via Homebrew

  # Example: Install pyenv
  - name: pyenv
    source: brew
    options:
      - --head"#;
    write!(output, "{}", format_yaml_content(tools_yaml)).unwrap();
    writeln!(output, "{}\n", "```".green()).unwrap();

    // fonts.yaml example
    writeln!(output, "{}:\n", "fonts.yaml".bold().cyan()).unwrap();
    writeln!(output, "{}", "```yaml".green()).unwrap();
    let fonts_yaml = r#"fonts:

  # name: name of the font
  # version: specific version of the font
  # source: from where to download the fonts, (GitHub only)
  # repo: repository name
  # tag: the specific release tag for this font version
  # install_only: only install the one mentioned, default is `all`
  - name: 0xProto
    version: "3.4.0"
    source: github
    repo: ryanoasis/nerd-fonts
    tag: v3.4.0
    install_only: ['regular', 'Mono']"#;
    write!(output, "{}", format_yaml_content(fonts_yaml)).unwrap();
    writeln!(output, "{}\n", "```".green()).unwrap();

    // shellrc.yaml example
    writeln!(output, "{}:\n", "shellrc.yaml".bold().cyan()).unwrap();
    writeln!(output, "{}", "```yaml".green()).unwrap();
    let shellrc_yaml = r#"run_commands:
  # Supported Shells: `zsh` and `bash`
  shell: "zsh" # or "bash"
  run_commands:
    # Exports Section - Environment variables
    # Any environment variables which needs to be exported
    - command: |
        export EDITOR="zed"
        export VISUAL="zed"
      section: Exports

    # Paths Section - PATH modifications
    - command: export PATH="$HOME/bin:$PATH"
      section: Paths

    # Evals Section - Command evaluations
    - command: eval "$(pyenv init - zsh)"
      section: Evals
  aliases:
    # Alias Section - For all aliases
    - name: sd
      value: setup-devbox"#;
    write!(output, "{}", format_yaml_content(shellrc_yaml)).unwrap();
    writeln!(output, "{}\n", "```".green()).unwrap();

    // settings.yaml example
    writeln!(output, "{}:\n", "settings.yaml".bold().cyan()).unwrap();
    writeln!(output, "{}", "```yaml".green()).unwrap();
    let settings_yaml = r#"settings:
  macos: # Settings specifically for macOS
    # A common domain for global macOS settings
    - domain: NSGlobalDomain
      # The specific setting key: show file extensions
      key: AppleShowAllExtensions
      # Set its value to true
      value: "true"
      # This setting expects a boolean value
      type: bool"#;
    write!(output, "{}", format_yaml_content(settings_yaml)).unwrap();
    writeln!(output, "{}\n", "```".green()).unwrap();
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
///
/// The `generate` command creates template configuration files that you can
/// customize for your development environment. It's the recommended way to
/// get started with setup-devbox.
///
/// **Generated Files:**
///   - `config.yaml`
///   - `tools.yaml`
///   - `fonts.yaml`
///   - `settings.yaml`
///   - `shellrc.yaml`
///   - `state.json`
///
fn show_generate_detailed_help() {
    println!("{}", "Detailed Description:".bold());
    println!("The 'generate' command creates template configuration files that you can");
    println!("customize for your development environment. It's the recommended way to");
    println!("get started with setup-devbox.\n");

    // List the files that are created.
    println!("{}", "Generated Files:".bold());
    println!(
        "  • config.yaml    - Main configuration with references to other configuration files"
    );
    println!("  • tools.yaml     - Tools configuration with example tools");
    println!("  • fonts.yaml     - Fonts configuration with example fonts");
    println!("  • shellrc.yaml   - Shell configuration with example shell configuration");
    println!(
        "  • settings.yaml  - OS Setting configuration with example configuration (Supported OS: macOS"
    );
    println!("  • state.json     - Empty state file to track installations\n");

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
///
/// The 'sync-config' command helps you recreate configuration files from an existing state file.
/// This is useful for:
///   1. Recovering lost configuration files
///   2. Sharing your setup across multiple machines (state.json)
///   3. Creating configuration templates from working setups
///
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

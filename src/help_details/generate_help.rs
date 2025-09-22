use colored::Colorize;

/// Displays help for the `generate` command.
///
/// This function explains the purpose, usage, and options for the `generate` command,
/// which creates starter configuration files.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show comprehensive information.
pub fn show_generate_help(detailed: bool) {
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
pub fn show_generate_detailed_help() {
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
        println!("  • {example}");
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
pub fn show_generate_basic_examples() {
    println!("{}", "Examples:".bold());
    println!("  setup-devbox generate                    # Create default configs");
    println!("  setup-devbox generate --config custom.yaml # Custom location");
}

use colored::Colorize;

/// Displays help for the `sync-config` command.
///
/// This function explains the purpose, usage, and options for the `sync-config` command,
/// which reconstructs configuration files from a state file.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show comprehensive information.
pub fn show_sync_config_help(detailed: bool) {
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
pub fn show_sync_config_detailed_help() {
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
pub fn show_sync_config_basic_examples() {
    println!("{}", "Examples:".bold());
    println!("  setup-devbox sync-config                 # Use default paths");
    println!("  setup-devbox sync-config --state backup.json # From backup");
}

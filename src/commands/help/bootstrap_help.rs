use colored::Colorize;

/// Displays help for the `bootstrap` command.
///
/// This function explains the purpose, usage, and options for the `bootstrap` command,
/// providing both basic and detailed information depending on the `detailed` flag.
///
/// # Arguments
///
/// * `detailed` - If true, shows extended information and examples.
pub fn show_bootstrap_help(detailed: bool) {
    println!("{}", "setup-devbox bootstrap".bold().blue());
    println!(
        "Bootstraps the development environment by generating default configurations and installing Homebrew.\n"
    );

    println!("{}", "Usage:".bold().yellow());
    println!("  setup-devbox bootstrap [OPTIONS]\n");

    println!("{}", "Options:".bold().yellow());
    println!(
        "  --config <PATH>  Optional path to save generated configuration files (Overrides $SDB_CONFIG_PATH)"
    );
    println!(
        "  --state <PATH>   Optional path to save the generated state file (Overrides $SDB_STATE_FILE_PATH)"
    );
    println!("  --detailed       Show this detailed help information\n");

    println!("{}", "Environment Variables:".bold().yellow());
    println!(
        "  SDB_CONFIG_PATH       Base directory for configurations (default: ~/.setup-devbox)"
    );
    println!("  SDB_STATE_FILE_PATH   Specific path for the state file\n");

    if detailed {
        show_bootstrap_detailed_help();
    } else {
        show_bootstrap_basic_examples();
    }
}

/// Shows detailed help for the `bootstrap` command.
///
/// Provides in-depth information about what files are created, what they contain,
/// and how to use them to customize the development environment.
pub fn show_bootstrap_detailed_help() {
    println!("{}", "Description:".bold().yellow());
    println!("The 'bootstrap' command initializes your development environment by:");
    println!("1. Creating a set of template configuration files with sensible defaults.");
    println!("2. Checking for and installing Homebrew if it's not already present.");
    println!("\nThis is typically the first command you run when setting up a new machine");
    println!("or starting to use setup-devbox for the first time.\n");

    println!("{}", "Generated Files:".bold().yellow());
    println!(
        "  • {} - Main configuration file linking all others",
        "config.yaml".cyan()
    );
    println!(
        "  • {} - Definitions for development tools and packages",
        "tools.yaml".cyan()
    );
    println!(
        "  • {} - macOS system settings and preferences",
        "settings.yaml".cyan()
    );
    println!(
        "  • {} - Shell aliases and initialization commands",
        "shellrc.yaml".cyan()
    );
    println!(
        "  • {} - Font installation configurations",
        "fonts.yaml".cyan()
    );
    println!("\nConfiguration directory resolution precedence:");
    println!("  1. User provided --config <PATH> argument");
    println!("  2. Environment variable $SDB_CONFIG_PATH/configs/");
    println!("  3. Default: ~/.setup-devbox/configs/");
    println!("\nExisting files are NEVER overwritten, so your manual changes are safe.\n");

    println!("{}", "Advanced Examples:".bold().yellow());
    println!("  setup-devbox bootstrap");
    println!("  setup-devbox bootstrap --config ./project-configs");
    println!("  setup-devbox bootstrap --config dev --state dev-state.json\n");

    println!("{}", "Workflow:".bold().yellow());
    println!("  1. Run 'setup-devbox bootstrap' to initialize your environment.");
    println!("  2. Homebrew will be installed automatically if needed.");
    println!("  3. Review and customize the generated YAML files in ~/.setup-devbox/configs.");
    println!("  4. Run 'setup-devbox now' to apply the configurations.");
}

/// Shows basic examples for the `bootstrap` command.
pub fn show_bootstrap_basic_examples() {
    println!("{}", "Examples:".bold().yellow());
    println!("  setup-devbox bootstrap                    # Default setup in ~/.setup-devbox");
    println!("  setup-devbox bootstrap --config ./custom  # Save configs to custom directory\n");
}

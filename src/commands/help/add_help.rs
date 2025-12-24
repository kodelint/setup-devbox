use colored::Colorize;
use std::fmt::Write;

/// Displays comprehensive help for the add command with optional detail level.
///
/// This function shows detailed information about the add command, including
/// all subcommands, their options, and usage examples.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show detailed information with examples.
pub fn show_add_help(detailed: bool) {
    let mut output = String::with_capacity(4096);

    writeln!(output, "\n{}", "setup-devbox add".bold().blue()).unwrap();
    writeln!(output, "{}", "--------------------".bold().blue()).unwrap();
    writeln!(
        output,
        "Add new tools, fonts, settings, or aliases to your configuration files.\n"
    )
    .unwrap();

    // Overview
    writeln!(output, "{}", "Overview:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  The {} command lets you add new items to your configuration files without",
        "add".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  manually editing YAML. After adding an item, it automatically runs"
    )
    .unwrap();
    writeln!(
        output,
        "  {} to apply the changes immediately.\n",
        "setup-devbox now".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "Available Subcommands:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}        - Add a new tool to {} configuration",
        "tool".green(),
        "tools.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}        - Add a new font to {} configuration",
        "font".green(),
        "fonts.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}     - Add a new macOS setting to {} configuration",
        "setting".green(),
        "settings.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}       - Add a new shell alias to {} configuration\n",
        "alias".green(),
        "shellrc.yaml".cyan()
    )
    .unwrap();

    // Conditionally add detailed or basic information based on the flag.
    if detailed {
        add_add_detailed_info(&mut output);
    } else {
        add_add_basic_info(&mut output);
    }

    print!("{output}");
}

/// Adds basic information about the add command subcommands.
fn add_add_basic_info(output: &mut String) {
    writeln!(output, "{}", "Basic Usage:".bold().yellow()).unwrap();
    writeln!(output).unwrap();

    writeln!(output, "  {} Add a tool:", "•".bold()).unwrap();
    writeln!(
        output,
        "  {}",
        "    setup-devbox add tool --name <NAME> --version <VERSION> --source <SOURCE>"
            .cyan()
            .italic()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(output, "  {} Add a font:", "•".bold()).unwrap();
    writeln!(
        output,
        "  {}",
        "    setup-devbox add font --name <NAME> --version <VERSION> --repo <REPO> --tag <TAG>"
            .cyan()
            .italic()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(output, "  {} Add a setting:", "•".bold()).unwrap();
    writeln!(
        output,
        "  {}",
        "    setup-devbox add setting --domain <DOMAIN> --key <KEY> --value <VALUE> --value-type <TYPE>".cyan().italic()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(output, "  {} Add an alias:", "•".bold()).unwrap();
    writeln!(
        output,
        "  {}",
        "    setup-devbox add alias --name <NAME> --value <VALUE>"
            .cyan()
            .italic()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "{} Use {} for detailed examples and all available options.",
        "Tip:".yellow(),
        "setup-devbox help add --detailed".cyan()
    )
    .unwrap();
    writeln!(output).unwrap();
}

/// Adds detailed information about all add command subcommands with examples.
fn add_add_detailed_info(output: &mut String) {
    // Add Tool
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(output, "{}", "ADD TOOL".bold().green()).unwrap();
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "\n{}\n",
        "Add a new tool to your tools.yaml configuration.".dimmed()
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} {} {} [OPTIONS]\n",
        "setup-devbox".cyan(),
        "add".cyan(),
        "tool".green()
    )
    .unwrap();

    writeln!(output, "{}", "Required Options:".bold().yellow()).unwrap();
    writeln!(output, "  {}  Tool name", "--name <NAME>".cyan()).unwrap();
    writeln!(
        output,
        "  {}  Version (e.g., '1.0.0' or 'latest')",
        "--version <VERSION>".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}  Source type [brew, github, rustup, cargo, pip, go, url, uv]\n",
        "--source <SOURCE>".cyan()
    )
    .unwrap();

    writeln!(
        output,
        "{}",
        "GitHub Source Options (required for --source github):"
            .bold()
            .yellow()
    )
    .unwrap();
    writeln!(
        output,
        "  {}  Repository in format 'owner/repo'",
        "--repo <REPO>".cyan()
    )
    .unwrap();
    writeln!(output, "  {}  Release tag\n", "--tag <TAG>".cyan()).unwrap();

    writeln!(output, "{}", "Optional Options:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}  Rename binary to different name",
        "--rename-to <NAME>".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}  Additional command to run (can be used multiple times)",
        "--additional-cmd <CMD>".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}  Enable configuration manager tracking",
        "--enable-config-manager".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}  Config file path to track (can be used multiple times)\n",
        "--config-paths <PATH>".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "Examples:".bold().magenta()).unwrap();

    writeln!(
        output,
        "\n  {} Add a tool from GitHub with additional setup:",
        "1.".bold()
    )
    .unwrap();
    writeln!(output, "     {} add tool \\", "setup-devbox".cyan()).unwrap();
    writeln!(output, "       --name helix \\").unwrap();
    writeln!(output, "       --version 25.07.1 \\").unwrap();
    writeln!(output, "       --source github \\").unwrap();
    writeln!(output, "       --repo helix-editor/helix \\").unwrap();
    writeln!(output, "       --tag 25.07.1 \\").unwrap();
    writeln!(output, "       --rename-to hx \\").unwrap();
    writeln!(
        output,
        "       --additional-cmd \"mkdir -p $HOME/.config/helix\" \\"
    )
    .unwrap();
    writeln!(
        output,
        "       --additional-cmd \"cp -r runtime $HOME/.config/helix\" \\"
    )
    .unwrap();
    writeln!(output, "       --enable-config-manager \\").unwrap();
    writeln!(
        output,
        "       --config-paths \"$HOME/.config/helix/config.toml\""
    )
    .unwrap();

    writeln!(output, "\n  {} Add a tool from Homebrew:", "2.".bold()).unwrap();
    writeln!(
        output,
        "     {} add tool --name bat --version latest --source brew",
        "setup-devbox".cyan()
    )
    .unwrap();

    writeln!(
        output,
        "\n  {} Add a Rust tool from crates.io:",
        "3.".bold()
    )
    .unwrap();
    writeln!(
        output,
        "     {} add tool --name ripgrep --version 14.1.1 --source cargo\n",
        "setup-devbox".cyan()
    )
    .unwrap();

    // Add Font
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(output, "{}", "ADD FONT".bold().green()).unwrap();
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "\n{}\n",
        "Add a new font to your fonts.yaml configuration.".dimmed()
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} {} {} [OPTIONS]\n",
        "setup-devbox".cyan(),
        "add".cyan(),
        "font".green()
    )
    .unwrap();

    writeln!(output, "{}", "Required Options:".bold().yellow()).unwrap();
    writeln!(output, "  {}  Font name", "--name <NAME>".cyan()).unwrap();
    writeln!(output, "  {}  Font version", "--version <VERSION>".cyan()).unwrap();
    writeln!(
        output,
        "  {}  Repository in format 'owner/repo'",
        "--repo <REPO>".cyan()
    )
    .unwrap();
    writeln!(output, "  {}  Release tag\n", "--tag <TAG>".cyan()).unwrap();

    writeln!(output, "{}", "Optional Options:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}  Source type (default: github)",
        "--source <SOURCE>".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}  Font variants to install (can be used multiple times)\n",
        "--install-only <VARIANT>".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "Examples:".bold().magenta()).unwrap();

    writeln!(
        output,
        "\n  {} Add a Nerd Font with specific variants:",
        "1.".bold()
    )
    .unwrap();
    writeln!(output, "     {} add font \\", "setup-devbox".cyan()).unwrap();
    writeln!(output, "       --name 0xProto \\").unwrap();
    writeln!(output, "       --version 3.4.0 \\").unwrap();
    writeln!(output, "       --repo ryanoasis/nerd-fonts \\").unwrap();
    writeln!(output, "       --tag v3.4.0 \\").unwrap();
    writeln!(output, "       --install-only regular \\").unwrap();
    writeln!(output, "       --install-only Mono").unwrap();

    writeln!(output, "\n  {} Add JetBrainsMono font:", "2.".bold()).unwrap();
    writeln!(output, "     {} add font \\", "setup-devbox".cyan()).unwrap();
    writeln!(output, "       --name JetBrainsMono \\").unwrap();
    writeln!(output, "       --version 3.4.0 \\").unwrap();
    writeln!(output, "       --repo ryanoasis/nerd-fonts \\").unwrap();
    writeln!(output, "       --tag v3.4.0\n").unwrap();

    // Add Setting
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(output, "{}", "ADD SETTING".bold().green()).unwrap();
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "\n{}\n",
        "Add a new macOS system setting to your settings.yaml configuration.".dimmed()
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} {} {} [OPTIONS]\n",
        "setup-devbox".cyan(),
        "add".cyan(),
        "setting".green()
    )
    .unwrap();

    writeln!(output, "{}", "Required Options:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}  Setting domain (e.g., NSGlobalDomain, com.apple.finder)",
        "--domain <DOMAIN>".cyan()
    )
    .unwrap();
    writeln!(output, "  {}  Setting key name", "--key <KEY>".cyan()).unwrap();
    writeln!(output, "  {}  Setting value", "--value <VALUE>".cyan()).unwrap();
    writeln!(
        output,
        "  {}  Value type [bool, string, int, float]\n",
        "--value-type <TYPE>".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "Examples:".bold().magenta()).unwrap();

    writeln!(output, "\n  {} Show all file extensions:", "1.".bold()).unwrap();
    writeln!(output, "     {} add setting \\", "setup-devbox".cyan()).unwrap();
    writeln!(output, "       --domain NSGlobalDomain \\").unwrap();
    writeln!(output, "       --key AppleShowAllExtensions \\").unwrap();
    writeln!(output, "       --value true \\").unwrap();
    writeln!(output, "       --value-type bool").unwrap();

    writeln!(output, "\n  {} Set Finder default view:", "2.".bold()).unwrap();
    writeln!(output, "     {} add setting \\", "setup-devbox".cyan()).unwrap();
    writeln!(output, "       --domain com.apple.finder \\").unwrap();
    writeln!(output, "       --key FXPreferredViewStyle \\").unwrap();
    writeln!(output, "       --value Nlsv \\").unwrap();
    writeln!(output, "       --value-type string").unwrap();

    writeln!(output, "\n  {} Set dock icon size:", "3.".bold()).unwrap();
    writeln!(output, "     {} add setting \\", "setup-devbox".cyan()).unwrap();
    writeln!(output, "       --domain com.apple.dock \\").unwrap();
    writeln!(output, "       --key tilesize \\").unwrap();
    writeln!(output, "       --value 48 \\").unwrap();
    writeln!(output, "       --value-type int\n").unwrap();

    // Add Alias
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(output, "{}", "ADD ALIAS".bold().green()).unwrap();
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "\n{}\n",
        "Add a new shell alias to your shellrc.yaml configuration.".dimmed()
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} {} {} [OPTIONS]\n",
        "setup-devbox".cyan(),
        "add".cyan(),
        "alias".green()
    )
    .unwrap();

    writeln!(output, "{}", "Required Options:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}  Alias name (the shortcut)",
        "--name <NAME>".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}  Alias value (the command it expands to)\n",
        "--value <VALUE>".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "Examples:".bold().magenta()).unwrap();

    writeln!(output, "\n  {} Replace cat with bat:", "1.".bold()).unwrap();
    writeln!(
        output,
        "     {} add alias --name cat --value \"bat --paging=never\"",
        "setup-devbox".cyan()
    )
    .unwrap();

    writeln!(output, "\n  {} Quick config editing:", "2.".bold()).unwrap();
    writeln!(
        output,
        "     {} add alias --name config --value \"zed $HOME/.config\"",
        "setup-devbox".cyan()
    )
    .unwrap();

    writeln!(output, "\n  {} Better ls command:", "3.".bold()).unwrap();
    writeln!(
        output,
        "     {} add alias --name ls --value lsd",
        "setup-devbox".cyan()
    )
    .unwrap();

    writeln!(output, "\n  {} Shortcut for setup-devbox:", "4.".bold()).unwrap();
    writeln!(
        output,
        "     {} add alias --name sd --value setup-devbox\n",
        "setup-devbox".cyan()
    )
    .unwrap();

    // Important Notes
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(output, "{}", "IMPORTANT NOTES".bold().yellow()).unwrap();
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} All configuration files must exist before using the add command.",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "    Run {} first if they don't exist.",
        "setup-devbox generate".cyan()
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} The {} environment variable must be set.",
        "•".bold(),
        "SDB_CONFIG_PATH".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "    It should point to your setup-devbox directory."
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} After adding an item, {} runs automatically.",
        "•".bold(),
        "setup-devbox now".cyan()
    )
    .unwrap();
    writeln!(output, "    This applies your changes immediately.").unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} For GitHub sources, both {} and {} are required.",
        "•".bold(),
        "--repo".cyan(),
        "--tag".cyan()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(output, "{}", "Configuration File Locations".bold().yellow()).unwrap();
    writeln!(output, "{}", "━".repeat(80).bright_black()).unwrap();
    writeln!(output, "\n  Configuration files are located at:").unwrap();
    writeln!(
        output,
        "    Configs:  {}",
        "$SDB_CONFIG_PATH/configs/configs.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "    Tools:    {}",
        "$SDB_CONFIG_PATH/configs/tools.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "    Fonts:    {}",
        "$SDB_CONFIG_PATH/configs/fonts.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "    Settings: {}",
        "$SDB_CONFIG_PATH/configs/settings.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "    Shell:    {}\n",
        "$SDB_CONFIG_PATH/configs/shellrc.yaml".cyan()
    )
    .unwrap();
}

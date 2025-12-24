use colored::Colorize;
use std::fmt::Write;

/// Displays comprehensive help for the remove command with optional detail level.
///
/// This function shows detailed information about the remove command, including
/// all subcommands, their options, and usage examples.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show detailed information with examples.
pub fn show_remove_help(detailed: bool) {
    let mut output = String::with_capacity(4096);

    writeln!(output, "\n{}", "setup-devbox remove".bold().blue()).unwrap();
    writeln!(output, "{}", "--------------------".bold().blue()).unwrap();
    writeln!(
        output,
        "Remove installed tools, fonts, aliases, or settings from your system.\n"
    )
    .unwrap();

    // Overview
    writeln!(output, "{}", "Overview:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  The {} command lets you cleanly uninstall items that were installed",
        "remove".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  through setup-devbox. It handles uninstallation, cleans up configuration"
    )
    .unwrap();
    writeln!(
        output,
        "  files, and updates the state tracking to keep everything in sync.\n"
    )
    .unwrap();

    writeln!(output, "{}", "Available Subcommands:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}        - Remove an installed tool and clean up its configuration",
        "tool".green()
    )
    .unwrap();
    writeln!(
        output,
        "  {}        - Remove an installed font from your system",
        "font".green()
    )
    .unwrap();
    writeln!(
        output,
        "  {}       - WIP: Remove a shell alias from {} configuration",
        "alias".green(),
        "shellrc.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}     - WIP: Remove a macOS setting from {} configuration\n",
        "setting".green(),
        "settings.yaml".cyan()
    )
    .unwrap();

    // Conditionally add detailed or basic information based on the flag.
    if detailed {
        add_remove_detailed_info(&mut output);
    } else {
        add_remove_basic_info(&mut output);
    }

    print!("{output}");
}

/// Adds basic information about the remove command subcommands.
fn add_remove_basic_info(output: &mut String) {
    writeln!(output, "{}", "Basic Usage:".bold().yellow()).unwrap();
    writeln!(output).unwrap();

    writeln!(output, "  {} Remove a tool:", "•".bold()).unwrap();
    writeln!(
        output,
        "  {}",
        "    setup-devbox remove tool <TOOL_NAME>".cyan().italic()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(output, "  {} Remove a font:", "•".bold()).unwrap();
    writeln!(
        output,
        "  {}",
        "    setup-devbox remove font <FONT_NAME>".cyan().italic()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "  {} {} Remove an alias:",
        "•".bold(),
        "[WIP]".yellow()
    )
    .unwrap();
    writeln!(
        output,
        "  {}",
        "    setup-devbox remove alias <ALIAS_NAME>".cyan().italic()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "  {} {} Remove a setting:",
        "•".bold(),
        "[WIP]".yellow()
    )
    .unwrap();
    writeln!(
        output,
        "  {}",
        "    setup-devbox remove setting <DOMAIN> <KEY>"
            .cyan()
            .italic()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "{} Use {} for detailed examples and behavior information.",
        "Tip:".yellow(),
        "setup-devbox help remove --detailed".cyan()
    )
    .unwrap();
    writeln!(output).unwrap();
}

/// Adds detailed information about all remove command subcommands with examples.
fn add_remove_detailed_info(output: &mut String) {
    // Remove Tool
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(output, "{}", "REMOVE TOOL".bold().green()).unwrap();
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "\n{}\n",
        "Remove an installed tool from your system.".dimmed()
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} {} {} <TOOL_NAME>\n",
        "setup-devbox".cyan(),
        "remove".cyan(),
        "tool".green()
    )
    .unwrap();

    writeln!(output, "{}", "Arguments:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}  Name of the tool to remove (or its alias if it was renamed)\n",
        "<TOOL_NAME>".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "What Gets Removed:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} The tool binary/package is uninstalled using the appropriate method",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "  {} Tool entry is removed from {}",
        "•".bold(),
        "tools.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {} State tracking is updated in {}",
        "•".bold(),
        "state.json".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {} Managed configuration files are cleaned up (if config manager was enabled)\n",
        "•".bold()
    )
    .unwrap();

    writeln!(output, "{}", "Supported Installers:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  Tools installed via: cargo, pip, brew, go, rustup, uv, github, or url\n"
    )
    .unwrap();

    writeln!(output, "{}", "Examples:".bold().magenta()).unwrap();

    writeln!(output, "\n  {} Remove a tool by name:", "1.".bold()).unwrap();
    writeln!(output, "     {} remove tool ripgrep", "setup-devbox".cyan()).unwrap();

    writeln!(output, "\n  {} Remove a renamed tool:", "2.".bold()).unwrap();
    writeln!(output, "     {} remove tool hx", "setup-devbox".cyan()).unwrap();
    writeln!(
        output,
        "     {} If helix was renamed to 'hx', this removes it",
        "Note:".dimmed()
    )
    .unwrap();

    writeln!(output, "\n  {} Remove a Homebrew package:", "3.".bold()).unwrap();
    writeln!(output, "     {} remove tool bat", "setup-devbox".cyan()).unwrap();

    writeln!(output, "\n  {} Remove a Cargo-installed tool:", "4.".bold()).unwrap();
    writeln!(output, "     {} remove tool fd", "setup-devbox".cyan()).unwrap();
    writeln!(output).unwrap();

    // Remove Font
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(output, "{}", "REMOVE FONT".bold().green()).unwrap();
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "\n{}\n",
        "Remove an installed font from your system.".dimmed()
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} {} {} <FONT_NAME>\n",
        "setup-devbox".cyan(),
        "remove".cyan(),
        "font".green()
    )
    .unwrap();

    writeln!(output, "{}", "Arguments:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}  Name of the font to remove\n",
        "<FONT_NAME>".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "What Gets Removed:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} All .ttf files containing the font name in the fonts directory",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "  {} Font entry is removed from {}",
        "•".bold(),
        "fonts.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {} State tracking is updated in {}\n",
        "•".bold(),
        "state.json".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "Examples:".bold().magenta()).unwrap();

    writeln!(output, "\n  {} Remove JetBrainsMono font:", "1.".bold()).unwrap();
    writeln!(
        output,
        "     {} remove font JetBrainsMono",
        "setup-devbox".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "     {} Removes all files like: JetBrainsMono-Regular.ttf, JetBrainsMono-Bold.ttf, etc.",
        "Result:".dimmed()
    )
    .unwrap();

    writeln!(output, "\n  {} Remove 0xProto font:", "2.".bold()).unwrap();
    writeln!(
        output,
        "     {} remove font 0xProto\n",
        "setup-devbox".cyan()
    )
    .unwrap();

    // Remove Alias
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "{} {}",
        "[WIP]".yellow(),
        "REMOVE ALIAS".bold().green()
    )
    .unwrap();
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "\n{}\n",
        "Remove a shell alias from your shellrc.yaml configuration.".dimmed()
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} {} {} <ALIAS_NAME>\n",
        "setup-devbox".cyan(),
        "remove".cyan(),
        "alias".green()
    )
    .unwrap();

    writeln!(output, "{}", "Arguments:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}  Name of the alias to remove\n",
        "<ALIAS_NAME>".cyan()
    )
    .unwrap();

    writeln!(output, "{}", "What Gets Removed:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} Alias definition is removed from {}",
        "•".bold(),
        "shellrc.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {} Changes take effect after shell restart or config reload\n",
        "•".bold()
    )
    .unwrap();

    writeln!(output, "{}", "Examples:".bold().magenta()).unwrap();

    writeln!(output, "\n  {} Remove 'll' alias:", "1.".bold()).unwrap();
    writeln!(output, "     {} remove alias ll", "setup-devbox".cyan()).unwrap();

    writeln!(output, "\n  {} Remove custom 'cat' alias:", "2.".bold()).unwrap();
    writeln!(output, "     {} remove alias cat", "setup-devbox".cyan()).unwrap();

    writeln!(output, "\n  {} Remove 'sd' shortcut alias:", "3.".bold()).unwrap();
    writeln!(output, "     {} remove alias sd\n", "setup-devbox".cyan()).unwrap();

    // Remove Setting
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "{} {}",
        "[WIP]".yellow(),
        "REMOVE SETTING".bold().green()
    )
    .unwrap();
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(
        output,
        "\n{}\n",
        "Remove a macOS system setting from your settings.yaml configuration.".dimmed()
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} {} {} <DOMAIN> <KEY>\n",
        "setup-devbox".cyan(),
        "remove".cyan(),
        "setting".green()
    )
    .unwrap();

    writeln!(output, "{}", "Arguments:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}  Setting domain (e.g., NSGlobalDomain, com.apple.finder)",
        "<DOMAIN>".cyan()
    )
    .unwrap();
    writeln!(output, "  {}  Setting key name\n", "<KEY>".cyan()).unwrap();

    writeln!(output, "{}", "What Gets Removed:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {} Setting definition is removed from {}",
        "•".bold(),
        "settings.yaml".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {} The actual macOS preference is NOT reverted automatically",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "  {} You'll need to manually change it back in System Preferences if desired\n",
        "•".bold()
    )
    .unwrap();

    writeln!(output, "{}", "Examples:".bold().magenta()).unwrap();

    writeln!(output, "\n  {} Remove dock autohide setting:", "1.".bold()).unwrap();
    writeln!(
        output,
        "     {} remove setting com.apple.dock autohide",
        "setup-devbox".cyan()
    )
    .unwrap();

    writeln!(
        output,
        "\n  {} Remove file extension visibility:",
        "2.".bold()
    )
    .unwrap();
    writeln!(
        output,
        "     {} remove setting NSGlobalDomain AppleShowAllExtensions",
        "setup-devbox".cyan()
    )
    .unwrap();

    writeln!(output, "\n  {} Remove Finder view style:", "3.".bold()).unwrap();
    writeln!(
        output,
        "     {} remove setting com.apple.finder FXPreferredViewStyle\n",
        "setup-devbox".cyan()
    )
    .unwrap();

    // Important Notes
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(output, "{}", "IMPORTANT NOTES".bold().yellow()).unwrap();
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} The remove command updates configuration files and state tracking.",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "    This keeps your setup-devbox configuration in sync with your system."
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} For tools, the uninstallation method matches the installation method.",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "    (brew uninstall, cargo uninstall, pip uninstall, etc.)"
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} Tools can be referenced by original name or renamed alias.",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "    Both {} and {} work if helix was renamed to hx.",
        "remove tool helix".cyan(),
        "remove tool hx".cyan()
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} Font removal deletes ALL variants (regular, bold, italic, etc.).",
        "•".bold()
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} Removing settings only updates the config file.",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "    It does NOT revert the actual macOS system preference."
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} Removing aliases only updates the config file.",
        "•".bold()
    )
    .unwrap();
    writeln!(
        output,
        "    Restart your shell or reload config for changes to take effect."
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "  {} If an item is not found, you'll see a warning instead of an error.",
        "•".bold()
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(output, "{}", "Removal Summary".bold().yellow()).unwrap();
    writeln!(output, "{}", "─".repeat(80).bright_black()).unwrap();
    writeln!(output, "\n  After removal, you'll see a summary showing:").unwrap();
    writeln!(output, "    {} Items successfully removed", "✓".green()).unwrap();
    writeln!(
        output,
        "    {} Items not found in configuration",
        "⚠".yellow()
    )
    .unwrap();
    writeln!(
        output,
        "    {} Items that failed to remove with reasons\n",
        "✗".red()
    )
    .unwrap();
}

use crate::schemas::help::format_yaml_content;
use colored::Colorize;
use std::fmt::Write;

/// Displays help for the `now` command.
///
/// This function explains the purpose, usage, and options for the `now` command,
/// which is responsible for installing and configuring the environment. It can
/// show either a basic or a detailed view.
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show comprehensive information about the command.
pub fn show_now_help(detailed: bool) {
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
pub fn add_now_detailed_info(output: &mut String) {
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
pub fn add_now_detailed_examples(output: &mut String) {
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
pub fn add_configuration_examples(output: &mut String) {
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
pub fn add_now_behavior_info(output: &mut String) {
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
pub fn add_now_basic_examples(output: &mut String) {
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

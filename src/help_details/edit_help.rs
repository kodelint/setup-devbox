use colored::Colorize;
use std::fmt::Write;
/// Displays help for the `edit` command.
///
/// This function explains the purpose, usage, and options for the `edit` command,
/// which is facilitates editing configuration and state file
///
/// # Arguments
///
/// * `detailed` - A `bool` flag to show comprehensive information about the command.
pub fn show_edit_help(detailed: bool) {
    let mut output = String::with_capacity(2048);

    writeln!(output, "\n{}", "setup-devbox edit".bold().blue()).unwrap();
    writeln!(output, "{}", "--------------------".bold().blue()).unwrap();
    writeln!(output, "Edit setup-devbox configuration files.\n").unwrap();

    // Add usage and options to the output string.
    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(output, "  setup-devbox edit [OPTIONS]\n").unwrap();
    writeln!(output, "{}", "Options:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  {}: Edit the state file. Use this option with caution.",
        "--state".cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  {}: Edit a specific configuration file. [Supported Options: {}, {}, {} and {}]\n",
        "--config <CONFIG_TYPE>".cyan(),
        "tools".italic().cyan(),
        "fonts".italic().cyan(),
        "shell".italic().cyan(),
        "settings".italic().cyan()
    )
    .unwrap();

    // Conditionally add detailed or basic information based on the flag.
    if detailed {
        add_edit_detailed_info(&mut output);
    } else {
        add_edit_basic_examples(&mut output);
    }

    print!("{}", output);
}

/// Adds detailed information for the `edit` command to a mutable string.
///
/// This function provides a deeper dive into the `edit` command's functionality,
/// explaining its role in state management and the files it uses.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
pub fn add_edit_detailed_info(output: &mut String) {
    writeln!(output, "{}", "Detailed Description:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  The '{}' command allows you to edit {} for your \
    development environment.",
        "edit".italic().cyan(),
        "core configuration files".bold()
    )
    .unwrap();
    writeln!(
        output,
        "  These files are used to manage tools, fonts, shell configurations, \
    and operating system settings."
    )
    .unwrap();
    writeln!(
        output,
        "  The command can also be used to edit the {}, though this should be \
    done with caution\n",
        "state file".bold()
    )
    .unwrap();
    // Add behavior and examples information from other helper functions.
    add_edit_behavior_info(output);
    add_edit_basic_examples(output);
}

/// Adds behavior information for the `edit` command to a mutable string.
///
/// This function explains how the command manages editing the configuration
/// and state files.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
pub fn add_edit_behavior_info(output: &mut String) {
    writeln!(output, "{}", "Behavior:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  • The command opens the specified file using the editor defined by your {} environment variable.",
        "$EDITOR".italic().cyan()).unwrap();
    writeln!(
        output,
        "  • It monitors the file's {} to detect any changes made during the editing session.",
        "SHA".bold()
    )
    .unwrap();
    writeln!(
        output,
        "  • If changes are detected, the {} command is automatically executed to apply them.",
        "now".italic().cyan()
    )
    .unwrap();
    writeln!(
        output,
        "  • If no changes are made to the file, the command exits without further action.\n"
    )
    .unwrap();
}

/// Adds detailed examples for the `edit` command to a mutable string.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
pub fn add_edit_basic_examples(output: &mut String) {
    writeln!(output, "{}", "Command Examples:".bold().yellow()).unwrap();
    let examples = [
        "setup-devbox edit --config tools",
        "setup-devbox edit --config fonts",
        "setup-devbox edit --config shell",
        "setup-devbox edit --config settings",
        "setup-devbox edit --state",
    ];

    for example in &examples {
        writeln!(output, "  {}", example).unwrap();
    }
    writeln!(output).unwrap();
}

use colored::Colorize;
use std::fmt::Write;

pub fn show_check_updates_help(detailed: bool) {
    let mut output = String::with_capacity(2048);

    writeln!(output, "\n{}", "setup-devbox check-updates".bold().blue()).unwrap();
    writeln!(output, "{}", "---------------------------".bold().blue()).unwrap();
    writeln!(
        output,
        "Checks for updates for all tools defined in tools.yaml.\n"
    )
    .unwrap();

    writeln!(output, "{}", "Usage:".bold().yellow()).unwrap();
    writeln!(output, "  setup-devbox check-updates\n").unwrap();

    if detailed {
        writeln!(output, "{}", "Detailed Description:".bold().yellow()).unwrap();
        writeln!(
            output,
            "  The 'check-updates' command reads your 'tools.yaml' configuration file and checks"
        )
        .unwrap();
        writeln!(
            output,
            "  for new versions of the tools you have specified. It then prints a table with the"
        )
        .unwrap();
        writeln!(
            output,
            "  tool name, the currently configured version, and the latest available version.\n"
        )
        .unwrap();
    }

    writeln!(output, "{}", "Examples:".bold().yellow()).unwrap();
    writeln!(
        output,
        "  setup-devbox check-updates       # Check for updates for all tools"
    )
    .unwrap();

    print!("{output}");
}

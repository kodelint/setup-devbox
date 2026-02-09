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
            "  for new versions of the tools you have specified. The results are presented in two tables:\n"
        )
        .unwrap();
        writeln!(output, "  {}:", "Updates Available".bold().green()).unwrap();
        writeln!(
            output,
            "    This table lists tools for which a newer version has been detected. It shows the"
        )
        .unwrap();
        writeln!(
            output,
            "    tool name, your configured version, and the latest available version (highlighted in green)."
        )
        .unwrap();
        writeln!(output, "\n  {}:", "Manual Check Required".bold().yellow()).unwrap();
        writeln!(
            output,
            "    This table lists tools that require manual attention. This includes tools whose"
        )
        .unwrap();
        writeln!(
            output,
            "    version is specified as 'latest' or 'N/A', tools for which the installer does not"
        )
        .unwrap();
        writeln!(
            output,
            "    support automatic version checks (e.g., 'uv tool' mode), or tools that encountered"
        )
        .unwrap();
        writeln!(
            output,
            "    an error during the version detection process. Up-to-date tools are not listed to reduce clutter.\n"
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

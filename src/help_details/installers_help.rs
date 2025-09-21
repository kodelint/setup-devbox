use crate::schemas::help::InstallerRegistry;
use colored::Colorize;
use std::fmt::Write;

/// Adds the "Supported Installers" section to a mutable string.
///
/// This is a helper function that populates a `String` with a list of all
/// supported installers, their descriptions, and any special notes.
///
/// # Arguments
///
/// * `output` - A mutable reference to the `String` where the content will be appended.
pub fn add_supported_installers(output: &mut String) {
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
pub fn show_installers_help(detailed: bool, filter: Option<String>) {
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

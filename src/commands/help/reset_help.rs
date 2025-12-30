use colored::Colorize;

pub fn show_reset_help(detailed: bool) {
    println!("{}", "setup-devbox reset".bold().blue());
    println!("Reset the installation state.");
    println!();
    println!("Wipes entries from the state file without uninstalling the actual tools.");
    println!("This forces the next 'now' run to re-verify or re-install everything.");
    println!();
    println!("{}", "Usage:".bold().yellow());
    println!("  setup-devbox reset [OPTIONS]");
    println!();
    println!("{}", "Options:".bold().yellow());
    println!("  --tool <NAME>   Optional name of a specific tool to reset in the state.");
    println!("  --all           Reset the entire state file (wipes everything).");
    println!("  --state <PATH>  Optional path to a custom state file.");

    if detailed {
        println!();
        println!("{}", "Examples:".bold().yellow());
        println!("  # Reset the entire state file (useful for fresh starts)");
        println!("  setup-devbox reset --all");
        println!();
        println!("  # Reset a specific tool (forces re-verification/install of that tool)");
        println!("  setup-devbox reset --tool starship");
    }
}

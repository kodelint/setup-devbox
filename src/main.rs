mod commands;

use clap::{Parser, Subcommand};
use commands::{generate, now, sync, version};

#[derive(Parser)]
#[command(name = "setup-devbox")]
#[command(about = "Setup development environment with ease", long_about = None)]
struct Cli {
    /// Turn debugging information on
    #[arg(short, long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print version information
    Version,
    /// Run setup now
    Now {
        #[arg(long)]
        config: Option<String>,
        #[arg(long)]
        state: Option<String>,
    },
    /// Generate a config
    Generate {
        #[arg(long)]
        config: Option<String>,
        #[arg(long)]
        state: Option<String>,
    },
    /// Sync the current state
    Sync {
        #[arg(long)]
        state: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => version::run(),
        Commands::Now { config, state } => now::run(config, state),
        Commands::Generate { config, state } => generate::run(config, state),
        Commands::Sync { state } => sync::run(state),
    }
}

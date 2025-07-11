// This is the heart of our `setup-devbox` application!
// It's like the central control panel that decides what our program should do
// based on the commands we give it from the terminal.

// We're bringing in a few custom modules that handle specific parts of our application.
// Think of these as specialized workshops where different tasks are performed.
mod commands;   // This module contains the logic for each specific command (like 'now', 'generate', 'sync').
mod utils;      // General utility functions that might be useful across different parts of the application.
mod schema;     // Defines the structure of our configuration files (e.g., how a 'tool' or 'font' is described).
mod logger;     // Handles setting up and managing our application's logging (debug, info, error messages).
mod installers; // Contains the logic for installing various types of software (like GitHub tools, Homebrew packages, fonts).
// Import `colored` crate for adding color to terminal output, which enhances
// the readability of log messages for the user.
use colored::Colorize;
// Standard library module for interacting with the operating system's environment variables.

// We're using the 'clap' crate for parsing command-line arguments.
// It helps us define what commands and options our application accepts from the user.
use clap::{Parser, Subcommand};
// Specifically importing the 'run' functions from our 'commands' module.
// Each of these corresponds to a subcommand that the user can execute.
use commands::{generate, now, sync, version};

/// The main structure that defines our command-line interface (CLI) for 'setup-devbox'.
/// We're using `#[derive(Parser)]` from the `clap` crate to automatically
/// generate the code needed to parse arguments from the command line.
#[derive(Parser)]
// We're giving our application a friendly name that will appear in help messages.
#[command(name = "setup-devbox")]
// A brief description of what 'setup-devbox' does. This is shown in short help.
#[command(about = "Setup development environment with ease", long_about = None)]
struct Cli {
    /// This argument allows users to turn on debugging information.
    /// When they run `setup-devbox --debug` or `setup-devbox -d`,
    /// our logger will show much more detailed messages, which is super helpful for troubleshooting!
    #[arg(short, long, global = true)] // `-d` is short, `--debug` is long, `global = true` means it works for any subcommand.
    debug: bool,

    /// This is where we define the subcommands that 'setup-devbox' can execute.
    /// Think of subcommands as different actions the user can ask the program to perform.
    #[command(subcommand)]
    command: Commands, // This field will hold the specific subcommand and its arguments.
}

/// This `enum` lists all the available subcommands for our `setup-devbox` application.
/// Each variant here corresponds to an action the user can choose.
#[derive(Subcommand)]
enum Commands {
    /// The 'version' subcommand:
    /// When you type `setup-devbox version`, this tells you which version of the tool you're running.
    Version,
    /// The 'now' subcommand:
    /// This is probably the most exciting one! It tells `setup-devbox` to run the full setup process right away.
    Now {
        /// An optional argument to specify a custom path to the configuration file.
        /// If not provided, 'setup-devbox' will look for a default config file.
        #[arg(long)] // This means the user would type `--config /path/to/my_config.yaml`
        config: Option<String>,
        /// An optional argument to specify a custom path to the state file.
        /// The state file keeps track of what 'setup-devbox' has already installed.
        #[arg(long)] // This means the user would type `--state /path/to/my_state.json`
        state: Option<String>,
    },
    /// The 'generate' subcommand:
    /// This command helps you get started by generating a sample configuration file.
    /// It's super handy if you're new to 'setup-devbox'!
    Generate {
        /// Just like with 'now', you can specify where to generate the new config file.
        #[arg(long)]
        config: Option<String>,
        /// And where to potentially place a default state file alongside it.
        #[arg(long)]
        state: Option<String>,
    },
    /// The 'sync' subcommand:
    /// This command ensures that your installed tools and fonts match what's defined
    /// in your configuration. It's great for keeping things up-to-date!
    Sync {
        /// You can tell 'sync' which state file to use for its operations.
        #[arg(long)]
        state: Option<String>,
    },
}

/// This is the main entry point of our entire application.
/// When you run `setup-devbox` from your terminal, execution begins right here!
fn main() {
    // First things first, we parse the command-line arguments the user provided.
    // 'clap' takes care of all the heavy lifting, converting raw arguments into our `Cli` struct.
    let cli = Cli::parse();
    log_debug!("[main] Command line arguments successfully parsed.");
    log_debug!("[main] Debug mode requested: {}", cli.debug);

    // Initialize our custom logger. This is crucial because it allows us to control
    // how verbose our application's output is. If `cli.debug` is true, we'll see
    // all those juicy debug messages!
    logger::init(cli.debug);
    log_debug!("[main] Logger initialized. Debug output is now active if enabled.");

    // Now, we use a `match` statement to figure out which subcommand the user
    // actually invoked. This is like a dispatcher, sending control to the
    // appropriate function in our 'commands' module.
    match cli.command {
        // If the user typed `setup-devbox version`...
        Commands::Version => {
            log_debug!("[main] 'Version' subcommand detected. Calling version::run().");
            version::run(); // ...we call the function that handles version display.
        }
        // If the user typed `setup-devbox now` (perhaps with --config or --state)...
        Commands::Now { config, state } => {
            log_debug!("[main] 'Now' subcommand detected.");
            // We pass along the optional config and state paths to the 'now' command's run function.
            log_debug!("[main] 'Now' subcommand received config path: {:?}", config);
            log_debug!("[main] 'Now' subcommand received state path: {:?}", state);
            now::run(config, state); // ...we execute the main setup logic.
        }
        // If the user typed `setup-devbox generate`...
        Commands::Generate { config, state } => {
            log_debug!("[main] 'Generate' subcommand detected.");
            log_debug!("[main] 'Generate' subcommand received config path: {:?}", config);
            log_debug!("[main] 'Generate' subcommand received state path: {:?}", state);
            generate::run(config, state); // ...we run the config generation process.
        }
        // If the user typed `setup-devbox sync`...
        Commands::Sync { state } => {
            log_debug!("[main] 'Sync' subcommand detected.");
            log_debug!("[main] 'Sync' subcommand received state path: {:?}", state);
            sync::run(state); // ...we initiate the state synchronization.
        }
    }
    log_debug!("[main] Command execution completed. Exiting application.");
}
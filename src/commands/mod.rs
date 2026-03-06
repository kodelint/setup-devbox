// Register application subcommands.
// Each module corresponds to a specific `setup-devbox` command-line action.

// Adding a component (tool, font, setting or alias) from command line.
pub mod add;
pub mod add_interactive;
pub mod check_updates;
// Help with editing configuration and state file.
pub mod edit;
// Manages the creation of default configuration files and initial setup.
pub mod bootstrap;
// Generates help command
pub mod help;
// Orchestrates the main setup and installation process.;
pub mod now;
// Reset the installation state
pub mod reset;
// Remove a component (tool, font, setting or alias) from command line
pub mod remove;
// Sync configuration files from state file
pub mod sync;
// Displays the version of SDB
pub mod version;

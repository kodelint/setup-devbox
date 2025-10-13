// Register application subcommands.
// Each module corresponds to a specific `setup-devbox` command-line action.

// Adding a component (tool, font, setting or alias) from command line.
pub mod add;
// Help with editing configuration and state file.
pub mod edit;
// Manages the creation of default configuration files.
pub mod generate;
// Generates help command
pub mod help;
// Orchestrates the main setup and installation process.;
pub mod now;
// Remove a component (tool, font, setting or alias) from command line
pub mod remove;
// Sync configuration files from state file
pub mod sync;
// Displays the version of SDB
pub mod version;

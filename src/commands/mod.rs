// Register application subcommands.
// Each module corresponds to a specific `setup-devbox` command-line action.
pub mod add;
pub mod edit;
pub mod generate; // Manages the creation of default configuration files.
pub mod help;
pub mod now; // Orchestrates the main setup and installation process.
pub mod sync;
pub mod version;

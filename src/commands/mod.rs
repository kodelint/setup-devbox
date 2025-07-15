// Register application subcommands.
// Each module corresponds to a specific `setup-devbox` command-line action.

pub mod version;  // Handles displaying the tool's version and checking for updates.
pub mod generate; // Manages the creation of default configuration files.
pub mod now;      // Orchestrates the main setup and installation process.
pub mod sync;     // Provides functionality to synchronize state with configuration files.
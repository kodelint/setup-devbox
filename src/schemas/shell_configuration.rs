use serde::{Deserialize, Serialize};

/// Represents Shell run commands configuration with section-based organization
/// This structure defines how shell commands should be organized in the RC file
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellRunCommands {
    /// Type of shell (e.g., "bash", "zsh") - determines which RC file to use
    pub shell: String,
    /// List of run command entries organized by section
    pub run_commands: Vec<RunCommandEntry>,
}

/// Represents a single run command entry with section information
/// Each entry defines a shell command and which section it belongs to
#[derive(Debug, Serialize, Deserialize)]
pub struct RunCommandEntry {
    /// The actual shell command to be added to the RC file
    pub command: String,
    /// Which section this command belongs to (organizes commands in the RC file)
    pub section: ConfigSection,
}

/// Configuration schema for `shellac.yaml`.
/// Defines the structure for shell environment customization (shell run commands and aliases).
/// This file configures the user's shell environment with custom commands and aliases.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Configuration for shell run commands (exports, evals, paths, etc.)
    pub run_commands: ShellRunCommands,
    /// List of shell aliases to be created
    pub aliases: Vec<AliasEntry>,
}

/// Defines the possible sections for organizing shell run commands
/// Each section groups related types of shell commands for better organization
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ConfigSection {
    Exports,   // Environment variable exports (export VAR=value)
    Aliases,   // Shell command aliases (alias ll='ls -la')
    Evals,     // Commands that need to be evaluated (eval "$(starship init bash)")
    Functions, // Shell function definitions
    Paths,     // PATH modifications (export PATH=$PATH:/new/path)
    Other,     // Miscellaneous commands that don't fit other categories
}

/// Represents a single command alias entry in `shellac.yaml`.
/// Defines a shell alias that maps a short name to a longer command
#[derive(Debug, Serialize, Deserialize)]
pub struct AliasEntry {
    /// The alias name (what the user types in the shell)
    pub name: String,
    /// The command the alias expands to (what gets executed)
    pub value: String,
}

//! # Shell Configuration Schema
//!
//! This module defines the data structures for shell environment customization,
//! including shell run commands, aliases, and section-based organization.
//! These structures are used to parse and generate `shellrc.yaml` configuration files
//! that customize the user's shell environment with custom commands and aliases.
//!
//! ## Configuration File Structure
//!
//! The `shellrc.yaml` file follows this structure:
//! ```yaml
//! run_commands:
//!   shell: "zsh"
//!   run_commands:
//!     - command: 'export EDITOR="nvim"'
//!       section: Exports
//!     - command: 'eval "$(starship init zsh)"'
//!       section: Evals
//!
//! aliases:
//!   - name: "ll"
//!     value: "ls -la"
//!   - name: "gst"
//!     value: "git status"
//! ```
//!
//! ## Organization Benefits
//!
//! The section-based organization provides:
//! - **Structured RC files**: Commands are grouped logically in the generated shell configuration
//! - **Maintainability**: Related commands are kept together for easier management
//! - **Readability**: Well-organized shell configuration files are easier to understand
//! - **Consistency**: Standardized organization across different shell environments

use serde::{Deserialize, Serialize};

// ============================================================================
// SHELL RUN COMMANDS CONFIGURATION
// ============================================================================

/// Represents Shell run commands configuration with section-based organization.
///
/// This structure defines how shell commands should be organized in the RC file
/// with logical grouping of related commands into sections for better maintainability
/// and readability of the generated shell configuration.
///
/// ## Shell Support
/// Supports multiple shell types including bash, zsh, fish, and others.
/// The shell type determines which RC file (.bashrc, .zshrc, etc.) is modified.
///
/// ## Section Organization
/// Commands are organized into logical sections that are preserved in the
/// generated shell configuration file, creating a well-structured RC file.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellRunCommands {
    /// Type of shell (e.g., `"bash"`, `"zsh"`) - determines which RC file to use.
    ///
    /// The shell type controls:
    /// - Which RC file is modified (`.bashrc`, `.zshrc`, `.config/fish/config.fish`)
    /// - Shell-specific syntax and command formatting
    /// - Comment headers and section organization
    ///
    /// ## Supported Shells
    /// - `"bash"`: Bourne Again SHell (Linux/macOS)
    /// - `"zsh"`: Z Shell (macOS default, popular alternative)
    /// - Other shells with RC file support
    pub shell: String,

    /// List of run command entries organized by section.
    ///
    /// Each entry defines a shell command and its logical section grouping.
    /// Commands are processed in order and grouped by section in the output file.
    ///
    /// ## Command Processing
    /// - Commands are validated for basic shell syntax safety
    /// - Duplicate commands within the same section are detected and handled
    /// - Section headers are automatically generated in the output
    /// - Commands are organized by section for readability
    pub run_commands: Vec<RunCommandEntry>,
}

// ============================================================================
// INDIVIDUAL COMMAND ENTRIES
// ============================================================================

/// Represents a single run command entry with section information.
///
/// Each entry defines a shell command and which section it belongs to,
/// enabling logical grouping of related commands in the generated shell
/// configuration file.
///
/// ## Command Types
/// Supports various shell command types including:
/// - Environment variable exports (`export VAR=value`)
/// - Command evaluations (`eval "$(command init)"`)
/// - Path modifications (`export PATH=$PATH:/new/path`)
/// - Function definitions
/// - Alias definitions (though aliases have a separate dedicated system)
/// - Miscellaneous shell commands
#[derive(Debug, Serialize, Deserialize)]
pub struct RunCommandEntry {
    /// The actual shell command to be added to the RC file.
    ///
    /// This is the exact command that will be inserted into the shell
    /// configuration file. It should be valid shell syntax for the
    /// specified shell type.
    ///
    /// ## Examples
    /// - `"export EDITOR='nvim'"`
    /// - `"eval \"$(starship init zsh)\""`
    /// - `"export PATH=\"$HOME/.local/bin:$PATH\""`
    /// - `"alias ll='ls -la'"`
    /// - `"function mkcd() { mkdir -p \"$1\" && cd \"$1\"; }"`
    ///
    /// ## Validation
    /// Commands are validated for basic syntax safety but should be
    /// written carefully as they execute in the user's shell environment.
    pub command: String,

    /// Which section this command belongs to (organizes commands in the RC file).
    ///
    /// Sections provide logical grouping of related commands, making the
    /// generated shell configuration file more organized and maintainable.
    ///
    /// ## Section Benefits
    /// - Related commands are grouped together under section headers
    /// - Improves readability of the generated RC file
    /// - Makes it easier to find and modify specific types of commands
    /// - Provides consistent organization across different shell configurations
    ///
    /// # Examples
    /// Generate `.zshrc` file will look like below:
    /// ```zsh
    /// ## Paths Section - Managed by setup-devbox
    /// export PATH="$HOME/.local/bin:$PATH"
    /// export PATH="$HOME/.cargo/bin:$PATH"
    ///
    /// ## Evals Section - Managed by setup-devbox
    /// eval "$(starship init zsh)"
    /// eval "$(atuin init zsh --disable-up-arrow)"
    ///
    /// ## Exports Section - Managed by setup-devbox
    /// export EDITOR="zed"
    /// export PYENV_ROOT="$HOME/.pyenv"
    ///
    /// ## Other Section - Managed by setup-devbox
    /// source $HOME/.config/secrets.zsh
    ///
    /// ## Aliases Section - Managed by setup-devbox
    /// alias cat='bat --paging=never'
    /// ```
    pub section: ConfigSection,
}

// ============================================================================
// TOP-LEVEL SHELL CONFIGURATION
// ============================================================================

/// Configuration schema for `shellrc.yaml`.
///
/// Defines the complete structure for shell environment customization,
/// including both shell run commands and aliases. This file configures
/// the user's shell environment with custom commands, environment variables,
/// and command aliases.
///
/// ## File Location
/// Typically located at:
/// - `~/.setup-devbox/configs/shellrc.yaml` (user-specific configuration)
/// - It also supports ENV Variables
///   `SDB_CONFIG_PATH` -> `$SDB_CONFIG_PATH/configs/shellrc.yaml`
///
/// ## Integration with Shell Setup
/// The configuration is processed during environment setup and the commands
/// are added to the appropriate shell RC file based on the detected shell.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Configuration for shell run commands (exports, evals, paths, etc.).
    ///
    /// Defines the shell-specific commands that should be added to the RC file,
    /// organized into logical sections for better maintainability and readability.
    ///
    /// ## Command Categories
    /// Includes environment variable exports, command evaluations, path
    /// modifications, function definitions, and miscellaneous commands.
    pub run_commands: ShellRunCommands,

    /// List of shell aliases to be created.
    ///
    /// Shell aliases provide short names for frequently used commands,
    /// improving productivity and reducing typing in the shell.
    ///
    /// ## Alias Processing
    /// Aliases are added to the shell RC file in a dedicated aliases section,
    /// ensuring they are available in new shell sessions.
    pub aliases: Vec<AliasEntry>,
}

// ============================================================================
// CONFIGURATION SECTIONS
// ============================================================================

/// Defines the possible sections for organizing shell run commands.
///
/// Each section groups related types of shell commands for better organization
/// and readability in the generated shell configuration file. Sections are
/// represented as headers in the output file with appropriate comments.
///
/// ## Section Ordering
/// Sections are typically ordered in the output file as:
/// 1. `Paths` - PATH modifications
/// 2. `Evals` - Command evaluations
/// 3. `Exports` - Environment variables
/// 4. `Other` - Miscellaneous commands
/// 5. `Aliases` - Command aliases (though most aliases use the dedicated system)
/// 6. `Functions` - Shell functions
///
/// This ordering follows shell best practices and ensures proper variable
/// resolution and command availability.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ConfigSection {
    /// Environment variable exports (`export VAR=value`).
    ///
    /// Used for setting environment variables that should be available
    /// in shell sessions and to child processes.
    ///
    /// ## Examples
    /// ```yaml
    /// run_commands:
    ///   shell: "zsh" # or "bash"
    ///   run_commands:
    ///     - command: |
    ///         export EDITOR="zed"
    ///         export VISUAL="zed"
    ///       section: Exports
    ///     - command: export PYENV_ROOT="$HOME/.pyenv"
    ///       section: Exports
    /// ```
    Exports,

    /// Shell command aliases (`alias ll='ls -la'`).
    ///
    /// Note: Most aliases should use the dedicated `aliases` field rather
    /// than run commands for better organization and management.
    ///
    /// ## Usage
    /// Primarily for aliases that need to be grouped with other run commands
    /// or for backward compatibility with existing configurations.
    ///
    /// # Examples
    /// ```yaml
    /// aliases:
    ///   - name: gl
    ///     value: g log --oneline --graph --decorate --all
    ///   - name: glog
    ///     value: r log --pretty=format:"%C(auto)%h%d %s %C(blue)(%cr) %C(green)<%an>" --graph --all
    /// ```
    Aliases,

    /// Commands that need to be evaluated (`eval "$(starship init bash)"`).
    ///
    /// Used for commands that generate shell configuration dynamically
    /// and need to be evaluated in the current shell context.
    ///
    /// ## Examples
    /// ```yaml
    /// run_commands:
    ///   shell: "zsh" # or "bash"
    ///   run_commands:
    ///      - command: eval "$(pyenv init - zsh)"
    ///        section: Evals
    ///      - command: eval "$(pyenv virtualenv-init -)"
    ///        section: Evals
    /// ```
    Evals,

    /// Shell function definitions.
    ///
    /// Used for defining custom shell functions that provide complex
    /// functionality or combine multiple commands.
    ///
    /// ## Examples
    /// ```shell
    /// function mkcd() {
    ///   mkdir -p "$1" && cd "$1"
    /// }
    /// ```
    Functions,

    /// PATH modifications (`export PATH=$PATH:/new/path`).
    ///
    /// Used for adding directories to the shell PATH environment variable,
    /// making executables in those directories available without full paths.
    ///
    /// ## Examples
    /// ```yaml
    /// run_commands:
    ///   shell: "zsh" # or "bash"
    ///   run_commands:
    ///     - command: export PATH="$HOME/.local/bin:$PATH"
    ///       section: Paths
    ///     - command: export PATH="$HOME/bin:$PATH"
    ///       section: Paths
    /// ```
    Paths,

    /// Miscellaneous commands that don't fit other categories.
    ///
    /// A catch-all section for commands that don't logically belong to
    /// the other defined sections but still need to be in the RC file.
    ///
    /// ## Usage
    /// Should be used sparingly - most commands should fit into the
    /// other defined sections for better organization.
    ///
    /// # Examples
    /// ```yaml
    /// run_commands:
    ///   shell: "zsh" # or "bash"
    ///   run_commands:
    ///     - command: source $HOME/.config/additional-stuff.zsh
    ///       section: Other
    /// ```
    Other,
}

// ============================================================================
// SHELL ALIAS DEFINITIONS
// ============================================================================

/// Represents a single command alias entry in `shellrc.yaml`.
///
/// Defines a shell alias that maps a short, memorable name to a longer
/// or more complex command, improving shell productivity and reducing
/// typing effort for frequently used commands.
#[derive(Debug, Serialize, Deserialize)]
pub struct AliasEntry {
    /// The alias name (what the user types in the shell).
    pub name: String,

    /// The command the alias expands to (what gets executed).
    ///
    /// This is the full command that will be executed when the alias
    /// is invoked. It can include flags, arguments, and complex command
    /// sequences using shell features like pipes and redirection.
    ///
    /// ## Examples
    /// - Simple command: `"ls -la"`
    /// - Command with flags: `"git status --short --branch"`
    /// - Complex sequence: `"git add . && git commit -m \"quick save\" && git push"`
    /// - Piped commands: `"docker ps -a | grep exited"`
    pub value: String,
}

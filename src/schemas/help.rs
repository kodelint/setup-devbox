//! # Help System and Installer Documentation
//!
//! This module provides comprehensive help and documentation for all supported
//! installers in the `setup-devbox` system. It includes structured information
//! about each installer, usage examples, configuration options, and environment
//! variables, all formatted with colored output for enhanced readability.
//!
//! ## Features
//!
//! - **Colorized Output**: Syntax highlighting for YAML examples and structured formatting
//! - **Detailed Documentation**: Comprehensive information for each installer type
//! - **Environment Variable Support**: Documentation of relevant environment variables
//! - **Configuration Options**: Clear explanation of all available configuration parameters
//! - **Examples**: Practical usage examples for each installer
//!
//! ## Usage
//!
//! The help system can be accessed through command-line help commands:
//! ```bash
//! setup-devbox help now --detailed                         # Detailed help for Brew installer
//! setup-devbox help installers --detailed --filter brew    # Detailed help for all installers
//! ```

use colored::Colorize;

// ============================================================================
// INSTALLER INFORMATION STRUCTURE
// ============================================================================

/// Represents the structured information for a single installer.
///
/// This struct holds all the details required to display comprehensive help
/// for a specific installer, including its name, description, usage examples,
/// and available configuration options. It serves as a complete documentation
/// unit for each supported installation method.
///
/// ## Documentation Components
/// - **Name**: Display name of the installer
/// - **Description**: Overview of what the installer does
/// - **Environment Variables**: Optional environment variable documentation
/// - **Examples**: Formatted YAML usage examples with syntax highlighting
/// - **Options**: Configuration options and their descriptions
///
/// ## Display Modes
/// Supports both brief overview and detailed documentation modes,
/// allowing users to get either a quick summary or comprehensive information.
#[derive(Clone)]
pub struct InstallerInfo {
    /// The display name of the installer (e.g., "Brew Installer").
    ///
    /// Used as the primary identifier in help output and should be
    /// clear and descriptive while remaining concise.
    pub(crate) name: &'static str,

    /// A short description of what the installer does.
    ///
    /// Provides a brief overview of the installer's purpose, capabilities,
    /// and typical use cases. Should be informative but concise.
    pub(crate) description: &'static str,

    /// Optional section for environment variables related to the installer.
    ///
    /// Contains documentation about environment variables that affect
    /// the installer's behavior, including their purpose and usage.
    env_variables: Option<&'static str>,

    /// Function to generate formatted YAML examples with colors.
    ///
    /// A function pointer that returns YAML content demonstrating
    /// typical usage patterns for the installer. The content is
    /// automatically formatted with syntax highlighting.
    examples_fn: fn() -> String,

    /// A slice of strings describing the configuration options available.
    ///
    /// Each string describes a configuration option, typically in the
    /// format "option_name: description". Lines starting with '#' are
    /// treated as comments and formatted differently.
    options: &'static [&'static str],
}

// ============================================================================
// YAML FORMATTING AND SYNTAX HIGHLIGHTING
// ============================================================================

/// Formats a given YAML content string for display with syntax highlighting.
///
/// This function iterates through each line of the input `yaml_content` and applies
/// specific text formatting (e.g., color, dimming) to different YAML elements
/// such as keys, values, comments, and list items. The formatting uses the
/// `colored` crate to provide visual distinction between different YAML constructs.
///
/// ## Syntax Highlighting Rules
/// - **Comments** (`# comment`): Dimmed text
/// - **Keys** (`key:`): Blue text
/// - **Values**: White text
/// - **List items** (`- item`): Cyan text
/// - **Colons and punctuation**: White text for separation
///
/// ## Line Processing Logic
/// The function processes lines in this order:
/// 1. Comments (lines starting with `#`)
/// 2. Key-value pairs (lines containing `:` not starting with `-`)
/// 3. List items (lines starting with `-`)
/// 4. Fallback for other content
///
/// # Arguments
///
/// * `yaml_content` - A string slice containing the raw YAML content to be formatted.
///
/// # Returns
///
/// A `String` containing the formatted YAML content with applied highlighting.
pub fn format_yaml_content(yaml_content: &str) -> String {
    // Initialize a new empty string to build the formatted output.
    let mut formatted = String::new();

    // Iterate over each line of the input string. The `lines()` method
    // splits the string by newline characters.
    for line in yaml_content.lines() {
        // Process each line to determine its type and apply the appropriate formatting.
        let processed_line = if line.trim().starts_with('#') {
            // This block handles comments.
            // A comment is identified by a '#' as the first non-whitespace character.
            // We want to dim the comment text while preserving any leading whitespace.
            let trimmed = line.trim_start();
            let whitespace = &line[..line.len() - trimmed.len()];
            format!("{}{}", whitespace, trimmed.dimmed())
        } else if line.contains(':') && !line.trim_start().starts_with('-') {
            // This block handles standard key-value pairs that are not list items.
            // A key-value pair is identified by containing a ':' and not starting
            // with a '-' (which would indicate a list item).
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                // If the split was successful, format the key in blue and the
                // rest of the line (colon and value) in white.
                format!("{}{}{}", parts[0].blue(), ":".white(), parts[1].white())
            } else {
                // Fallback: If for some reason the split fails, just return the
                // original line. This handles malformed lines gracefully.
                line.to_string()
            }
        } else if line.trim_start().starts_with('-') {
            // This block handles YAML list items.
            // A list item is identified by a '-' as the first non-whitespace character.
            let trimmed = line.trim_start();
            let whitespace = &line[..line.len() - trimmed.len()];

            if trimmed.contains(':') {
                // This sub-block handles list items that contain a key-value pair,
                // e.g., `- key: value`. The `-` is formatted in cyan.
                let dash_and_rest: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if dash_and_rest.len() == 2 {
                    let rest = dash_and_rest[1];
                    if rest.contains(':') {
                        let key_value: Vec<&str> = rest.splitn(2, ':').collect();
                        if key_value.len() == 2 {
                            // Format: `-` in cyan, `key` in blue, `:value` in white.
                            format!(
                                "{}{} {}{}{}",
                                whitespace,
                                "-".cyan(),
                                key_value[0].blue(),
                                ":".white(),
                                key_value[1].white()
                            )
                        } else {
                            // Fallback for unexpected format after the dash.
                            format!("{}{} {}", whitespace, "-".cyan(), rest.white())
                        }
                    } else {
                        // Fallback for a list item that doesn't contain a key-value.
                        format!("{}{} {}", whitespace, "-".cyan(), rest.white())
                    }
                } else {
                    // Fallback for an invalid list item structure.
                    line.to_string()
                }
            } else {
                // This sub-block handles simple list items without a key-value,
                // e.g., `- simple_value`. The entire line is formatted in cyan.
                format!("{}{}", whitespace, trimmed.cyan())
            }
        } else {
            // This is a catch-all for any other lines that don't match the above
            // patterns, such as empty lines or non-standard YAML. The line
            // is returned as-is without any special formatting.
            line.to_string()
        };

        // Append the processed line to the formatted string, followed by a newline character.
        formatted.push_str(&processed_line);
        formatted.push('\n');
    }
    // Return the final formatted string.
    formatted
}

// ============================================================================
// INSTALLER INFORMATION DISPLAY
// ============================================================================

impl InstallerInfo {
    /// Displays the formatted installer information to the console.
    ///
    /// The output includes the installer's name and description. If the `detailed`
    /// flag is set to `true`, it also prints examples, environment variables,
    /// and configuration options with appropriate formatting and colors.
    ///
    /// ## Output Structure
    /// - **Brief mode**: Name, description only
    /// - **Detailed mode**: Name, description, examples, env vars, options
    ///
    /// ## Formatting Features
    /// - Color-coded sections for better readability
    /// - Syntax-highlighted YAML examples
    /// - Proper indentation and section organization
    /// - Dimmed comments and emphasized key information
    ///
    /// # Arguments
    ///
    /// * `detailed` - A `bool` indicating whether to show the full, detailed
    ///                help for the installer.
    pub(crate) fn display(&self, detailed: bool) {
        // Print the installer name with a bullet point.
        println!("{} {}", "•".green().bold(), self.name.bold().blue());
        // Print a separating line for visual clarity.
        println!("{}", "  -----------------".bold().blue());
        // Print the main description.
        println!("  {}\n", self.description);

        // Check if the detailed view is requested.
        if detailed {
            // Generate and display formatted examples
            let examples_output = (self.examples_fn)();
            if !examples_output.trim().is_empty() {
                println!("  {}", "Examples:".italic().yellow());
                println!("  {}", "```yaml".green());
                // Indent the YAML content to align with the Examples header
                for line in format_yaml_content(&examples_output).lines() {
                    println!("  {}", line);
                }
                println!("  {}", "```".green());
                println!();
            }

            // If environment variables are specified, print them.
            if let Some(env_vars) = self.env_variables {
                println!("  {}", "Environment variables:".italic().yellow());
                for line in env_vars.lines() {
                    if let Some((key, value)) = line.split_once(':') {
                        let key = key.trim();
                        let value = value.trim();

                        // Check if the key is all uppercase
                        if key
                            .chars()
                            .all(|c| c.is_uppercase() || c == '_' || c.is_numeric())
                        {
                            println!("    {}: {}", key.blue().bold(), value.white());
                        } else {
                            // For non-uppercase keys, use dimmed styling
                            println!("    {}", line.dimmed());
                        }
                    } else {
                        // If the line doesn't contain ':', print it dimmed
                        println!("    {}", line.dimmed());
                    }
                }
                println!();
            }

            // If configuration options are available, print them.
            if !self.options.is_empty() {
                println!("  {}", "Configuration options:".italic().yellow());
                for option in self.options {
                    let trimmed_option = option.trim();

                    // Check if the line is a comment (starts with #)
                    if trimmed_option.starts_with('#') {
                        println!("    {}", option.dimmed());
                    } else if let Some((key, value)) = option.split_once(':') {
                        let key = key.trim();
                        let value = value.trim();

                        println!("    {}: {}", key.green().bold(), value.white());
                    } else {
                        // If the line doesn't contain ':', print it dimmed
                        println!("    {}", option.dimmed());
                    }
                }
                println!();
            }
        }
    }
}

// ============================================================================
// INSTALLER REGISTRY
// ============================================================================

/// A central registry for all supported installer information.
///
/// This struct acts as a container for methods that return `InstallerInfo`
/// for each installer type. It provides a single point of access to the
/// help data for the `setup-devbox` application, ensuring consistent
/// documentation across all installation methods.
///
/// ## Registry Benefits
/// - Centralized documentation management
/// - Consistent formatting and information structure
/// - Easy addition of new installers
/// - Comprehensive overview of all supported installation methods
///
/// ## Supported Installers
/// Includes installers for package managers, language tools, direct downloads,
/// and system configuration methods covering the complete tool installation
/// ecosystem supported by `setup-devbox`.
pub struct InstallerRegistry;

impl InstallerRegistry {
    /// Retrieves a vector containing the `InstallerInfo` for all supported installers.
    ///
    /// This function is the primary way for other modules to access the
    /// help data for all installer types. It calls a private function for
    /// each installer to build the complete list.
    ///
    /// ## Installer Coverage
    /// Returns documentation for all currently supported installation methods,
    /// providing users with a comprehensive overview of available options.
    ///
    /// # Returns
    ///
    /// A `Vec<InstallerInfo>` containing all installer help data.
    pub(crate) fn get_all() -> Vec<InstallerInfo> {
        vec![
            Self::brew_installer(),
            Self::cargo_installer(),
            Self::fonts_installer(),
            Self::github_installer(),
            Self::go_installer(),
            Self::pip_installer(),
            Self::rustup_installer(),
            Self::shell_installer(),
            Self::url_installer(),
            Self::uv_installer(),
        ]
    }

    /// Returns the help information for the Homebrew installer.
    ///
    /// Provides documentation for installing tools using Homebrew, the
    /// popular package manager for macOS and Linux.
    fn brew_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Brew",
            description: "Package manager for macOS/Linux (Homebrew). Installs CLI tools, applications, and libraries.",
            env_variables: None,
            examples_fn: || {
                r#"- name: pyenv
  source: brew
  options:
    - --head

# Install `git` using `brew`
# source is `brew` registry
# version is latest
- name: git
  source: brew
  version: latest


# options to provide addtional flags to `brew` for installation
- name: docker
  source: brew
  version: 24.0.6
  options:
    - --cask"#
                    .to_string()
            },
            options: &[
                "name: Tool name (required)",
                "source: brew (required)",
                "version: Specific version or 'latest'",
                "options: List of brew-specific flags (--cask, --head, etc.)",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the Cargo installer.
    ///
    /// Provides documentation for installing Rust crates and binaries using
    /// Cargo, the Rust package manager and build system.
    fn cargo_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Cargo",
            description: "Rust package manager for crates and binaries. Installs Rust tools and applications.",
            env_variables: None,
            examples_fn: || {
                r#"- name: lsd
  source: cargo
  version: 1.1.5

- name: uv
  source: cargo
  version: 0.8.17
  options:
    - --git https://github.com/astral-sh/uv

- name: ripgrep
  source: cargo
  version: latest
  options:
    - --features=simd-accel"#
                    .to_string()
            },
            options: &[
                "name: Crate name (required)",
                "source: cargo (required)",
                "version: Specific version or 'latest'",
                "options: List of cargo install flags (--git, --features, etc.)",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the Fonts installer.
    ///
    /// Provides documentation for installing Nerd Fonts from GitHub releases,
    /// supporting programming fonts with icon glyphs for development environments.
    fn fonts_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Fonts",
            description: "Nerd Fonts installer from GitHub releases. Downloads and installs programming fonts with icons.\
            Presently only GitHub is supported.",
            env_variables: None,
            examples_fn: || {
                r#"- name: 0xProto
  source: github
  version: "3.4.0"
  repo: repo: ryanoasis/nerd-fonts
  tag: v3.4.0
  install_only: ['regular', 'Mono']"#
                    .to_string()
            },
            options: &[
                "name: Font name from Nerd Fonts collection (required)",
                "source: github (required)",
                "version: Specific release version or 'latest'",
                "repo: GitHub repository (required)",
                "tag: GitHub Release tag (optional)",
                "install_only: Only install the mentioned font style, default: all",
                "options: Font-specific installation options",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the GitHub installer.
    ///
    /// Provides documentation for downloading and installing tools directly
    /// from GitHub releases, supporting various archive formats and platforms.
    fn github_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Github",
            description: "Download and install tools directly from GitHub releases. Supports various archive formats.",
            env_variables: None,
            examples_fn: || {
                r#"- name: helix
  version: 25.07.1
  source: github
  repo: helix-editor/helix
  tag: 25.07.1
  rename_to: hx
  additional_cmd:
    - mkdir -p $HOME/.config/helix
    - cp -r runtime $HOME/.config/helix

- name: zed
  version: 0.200.1-pre
  source: github
  repo: zed-industries/zed
  tag: v0.200.1-pre
  rename_to: zed"#
                    .to_string()
            },
            options: &[
                "name: Tool name (required)",
                "source: github (required)",
                "repo: GitHub repository (owner/name format)",
                "version: Release version",
                "tag: Specific git tag or 'latest'",
                "rename_to: Rename binary after installation",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the Go installer.
    ///
    /// Provides documentation for installing Go-based tools and applications
    /// using the `go install` command with package import paths.
    fn go_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Go",
            description: "Go package installer using 'go install'. Installs Go-based tools and applications.",
            env_variables: None,
            examples_fn: || {
                r#"- name: gopls
  source: go
  version: latest
  url: golang.org/x/tools/gopls

- name: goreleaser
  source: go
  version: latest
  url: github.com/goreleaser/goreleaser

- name: goimports
  source: go
  version: v0.1.12
  url: golang.org/x/tools/cmd/goimports"#
                    .to_string()
            },
            options: &[
                "name: Tool name (required)",
                "source: go (required)",
                "url: Go package import path (required)",
                "version: Package version (@latest, @v1.2.3, etc.)",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the Pip installer.
    ///
    /// Provides documentation for installing Python packages using pip,
    /// supporting both libraries and command-line tools.
    fn pip_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Pip",
            description: "Python package installer using pip. Installs Python libraries and command-line tools.",
            env_variables: None,
            examples_fn: || {
                r#"- name: black
  source: pip
  version: 22.3.0

- name: django
  source: pip
  version: latest
  options:
    - --user

- name: numpy
  source: pip
  version: 1.24.0
  options:
    - --upgrade
    - --user"#
                    .to_string()
            },
            options: &[
                "name: Package name (required)",
                "source: pip (required)",
                "version: Specific version or 'latest'",
                "options: List of pip install flags (--user, --upgrade, etc.)",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the Rustup installer.
    ///
    /// Provides documentation for managing Rust toolchains, components,
    /// and versions using the rustup toolchain manager.
    fn rustup_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Rustup",
            description: "Rust toolchain installer and manager. Manages Rust versions, targets, and components.",
            env_variables: None,
            examples_fn: || {
                r#"- name: rust
  source: rustup
  version: stable
  options:
    - rust-src
    - clippy
    - rustfmt
    - rust-analyzer

- name: rust-nightly
  source: rustup
  version: nightly
  options:
    - rustfmt
    - clippy"#
                    .to_string()
            },
            options: &[
                "name: Toolchain identifier (required)",
                "source: rustup (required)",
                "version: Rust toolchain (stable, beta, nightly, or specific version)",
                "options: List of components to install (rust-src, clippy, rustfmt, etc.)",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the Shell installer.
    ///
    /// Provides documentation for managing shell configuration files (.zshrc, .bashrc)
    /// and shell aliases, including environment variable management and command organization.
    fn shell_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Shell",
            description: "Shell configuration manager. Handles .zshrc, .bashrc files and shell aliases.",
            env_variables: Some(
                r#"SDB_RESET_SHELLRC_FILE: "true||false"

Purpose:
========
This feature ensures that updates to existing shell configuration elements
(aliases, exports, commands) are properly applied by completely regenerating the
shell configuration file rather than attempting to modify individual entries in place.

The Problem: In-Place Modification Challenges
=============================================
When users update existing shell configuration elements, several issues arise:

1. Identification Difficulty:
    There's no reliable way to programmatically identify which specific line or section
    corresponds to an existing alias, export, or command that needs modification.
2. Conflict Resolution:
     Attempting to modify existing entries risks:
     - Partial updates leaving broken configurations
     - Duplicate entries if both old and new versions remain
     - Syntax errors from improper text manipulation
     - Missing dependencies if related commands aren't updated together
3. Consistency Concerns:
     Manual edits might not follow the same formatting, commenting, or organizational
     structure as automated entries.

The Solution: Complete Regeneration
===================================
Instead of attempting risky in-place modifications, the system:
- Deletes the existing shell configuration file (.zshrc, .bashrc, etc.)
- Recreates the file from scratch with all current configuration elements
- Preserves the integrity and consistency of the entire configuration

How It Works:
=============
1. Default Behavior (SDB_RESET_SHELLRC_FILE = "false"):
   - New configuration elements are appended to the existing file
   - Existing elements remain unchanged
   - This is safe but may lead to duplicates or outdated entries
2. Automatic Regeneration (SDB_RESET_SHELLRC_FILE = "true"):
   - The entire shell config file is recreated when updates occur
   - All configuration elements are written fresh in a consistent format
   - Ensures no duplicates, outdated entries, or formatting inconsistencies"#,
            ),
            examples_fn: || {
                r#"run_commands:
  shell: "zsh" # or "bash"
  run_commands:
    # Exports Section - Environment variables
    - command: |
        export EDITOR="zed"
        export VISUAL="zed"
      section: Exports
    - command: export UV_CONFIG_FILE="$HOME/.config/uv/uv.toml"
      section: Exports
    - command: export PYENV_ROOT="$HOME/.pyenv"
      section: Exports

    # Paths Section - PATH modifications
    - command: export PATH="$HOME/bin:$PATH"
      section: Paths
    - command: export PATH="$HOME/.local/bin:$PATH"
      section: Paths
    - command: export PATH="$(go env GOPATH)/bin:$PATH"
      section: Paths

    # Evals Section - Command evaluations
    - command: eval "$(pyenv init - zsh)"
      section: Evals
    - command: eval "$(atuin init zsh --disable-up-arrow)"
      section: Evals

    # Other Section - Miscellaneous configurations
    - command: source $HOME/.config/secrets.zsh
      section: Other

aliases:
  - name: cat # Replace `cat` with `bat`
    value: bat --paging=never
  - name: ls
    value: lsd

  # Git Aliases
  - name: g
    value: git
  - name: gs
    value: g status

# Implementation Details:
# =======================
# The Shell Installer is engineered to generate shell configuration files with a structured, modular
# architecture organized into distinct functional sections. The implementation follows this
# organizational framework:
#
#    Configuration Sections:
#        1. Exports Section - Environment variable declarations
#        2. Paths Section - System and custom PATH configurations
#        3. Evals Section - Command evaluations and initialization routines
#        4. Other Section - Miscellaneous configurations and customizations
#        5. Aliases Section - Command alias definitions
#
#    Execution Priority:
#        The installer employs a deliberate execution sequence that prioritizes the Paths Section to ensure
#        proper initialization of the $PATH environment variable before processing subsequent sections.
#        This strategic ordering guarantees that all path-dependent operations execute with correctly
#        resolved binary and script locations."#.to_string()
            },
            options: &[
                "shell: Target shell (zsh, bash)",
                "run_commands: List of shell commands organized by sections",
                "  - command: Shell command to execute",
                "  - section: Organization section (Exports, Paths, Evals, Other)",
                "aliases: List of shell aliases",
                "  - name: Alias name",
                "  - value: Command the alias expands to",
            ],
        }
    }

    /// Returns the help information for the URL installer.
    ///
    /// Provides documentation for installing software from direct download URLs,
    /// supporting various download types including packages, scripts, and archives.
    fn url_installer() -> InstallerInfo {
        InstallerInfo {
            name: "URL",
            description: "Direct URL installer. Downloads and installs software from direct download URLs.",
            env_variables: None,
            examples_fn: || {
                r#"- name: go
  source: url
  version: 1.24.5
  url: https://go.dev/dl/go1.24.5.darwin-amd64.pkg

- name: docker-install
  source: url
  url: https://get.docker.com/
  options:
    - --script

- name: custom-tool
  source: url
  url: https://example.com/tool.tar.gz
  options:
    - --binary=tool
    - --checksum=sha256:abc123..."#
                    .to_string()
            },
            options: &[
                "name: Tool name (required)",
                "source: url (required)",
                "url: Download URL (required)",
                "version: Version identifier (optional)",
                "options: Installation flags (--script, --binary, --checksum, etc.) (optional)",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the UV installer.
    ///
    /// Provides documentation for the UV Python package manager, supporting
    /// multiple modes including tool installation, pip compatibility, and
    /// Python version management.
    fn uv_installer() -> InstallerInfo {
        InstallerInfo {
            name: "UV",
            description: "Ultra-fast Python package installer with tool, pip, and python modes for modern Python development.",
            env_variables: None,
            examples_fn: || {
                r#"- name: cpython-3.13.7
  source: uv
  version: 3.13.7
  options:
    - --mode=python
  additional_cmd:
    - uv python update-shell

- name: ruff
  source: uv
  version: latest
  options:
    - --mode=tool

- name: django
  source: uv
  version: 4.2.0
  options:
    - --mode=pip"#
                    .to_string()
            },
            options: &[
                "name: Package/Python name (required)",
                "source: uv (required)",
                "version: Version or Python version (required)",
                "options: UV mode flags (--mode=tool/pip/python) (required)",
                "additional_cmd: Commands to run after installation (optional)",
            ],
        }
    }
}

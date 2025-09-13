use colored::Colorize;

/// Represents the structured information for a single installer.
///
/// This struct holds all the details required to display comprehensive help
/// for a specific installer, including its name, description, usage examples,
/// and available configuration options.
#[derive(Clone)]
pub struct InstallerInfo {
    /// The display name of the installer (e.g., "Brew Installer").
    pub(crate) name: &'static str,
    /// A short description of what the installer does.
    description: &'static str,
    /// Optional section for environment variables related to the installer.
    env_variables: Option<&'static str>,
    /// A slice of strings representing usage examples in YAML format.
    examples: &'static [&'static str],
    /// A slice of strings describing the configuration options available.
    options: &'static [&'static str],
}

impl InstallerInfo {
    /// Displays the formatted installer information to the console.
    ///
    /// The output includes the installer's name and description. If the `detailed`
    /// flag is set to `true`, it also prints examples, environment variables,
    /// and configuration options.
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
            // If examples are available, print them in a dedicated section.
            if !self.examples.is_empty() {
                println!("  {}", "Examples:".italic().yellow());
                for example in self.examples {
                    println!("    {}", example.dimmed());
                }
            }

            // If environment variables are specified, print them.
            if let Some(env_vars) = self.env_variables {
                println!("\n  {}", "Environment variables:".italic().yellow());
                for line in env_vars.lines() {
                    println!("    {}", line.dimmed());
                }
            }

            // If configuration options are available, print them.
            if !self.options.is_empty() {
                println!("\n  {}", "Configuration options:".italic().yellow());
                for option in self.options {
                    println!("    {}", option.dimmed());
                }
            }
            println!();
        }
    }
}

/// A central registry for all supported installer information.
///
/// This struct acts as a container for methods that return `InstallerInfo`
/// for each installer type. It provides a single point of access to the
/// help data for the `setup-devbox` application.
pub struct InstallerRegistry;

impl InstallerRegistry {
    /// Retrieves a vector containing the `InstallerInfo` for all supported installers.
    ///
    /// This function is the primary way for other modules to access the
    /// help data for all installer types. It calls a private function for
    /// each installer to build the complete list.
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
    fn brew_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Brew Installer",
            description: "Package manager for macOS/Linux (Homebrew). Installs CLI tools, applications, and libraries.",
            env_variables: None,
            examples: &[
                "  - name: pyenv",
                "    source: brew",
                "    options:",
                "      - --head",
                "",
                "  - name: git",
                "    source: brew",
                "    version: latest",
                "",
                "  - name: docker",
                "    source: brew",
                "    version: 24.0.6",
                "    options:",
                "      - --cask",
            ],
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
    fn cargo_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Cargo Installer",
            description: "Rust package manager for crates and binaries. Installs Rust tools and applications.",
            env_variables: None,
            examples: &[
                "  - name: lsd",
                "    source: cargo",
                "    version: 1.1.5",
                "",
                "  - name: uv",
                "    source: cargo",
                "    version: 0.8.17",
                "    options:",
                "      - --git https://github.com/astral-sh/uv",
                "",
                "  - name: ripgrep",
                "    source: cargo",
                "    version: latest",
                "    options:",
                "      - --features=simd-accel",
            ],
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
    fn fonts_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Fonts Installer",
            description: "Nerd Fonts installer from GitHub releases. Downloads and installs programming fonts with icons.",
            env_variables: None,
            examples: &[
                "  - name: FiraCode",
                "    source: fonts",
                "    version: latest",
                "",
                "  - name: JetBrainsMono",
                "    source: fonts",
                "    version: v3.0.0",
                "",
                "  - name: SourceCodePro",
                "    source: fonts",
                "    version: v2.038R-ro/1.058R-it",
            ],
            options: &[
                "name: Font name from Nerd Fonts collection (required)",
                "source: fonts (required)",
                "version: Specific release version or 'latest'",
                "options: Font-specific installation options",
                "additional_cmd: List of commands to run after installation (optional)",
            ],
        }
    }

    /// Returns the help information for the GitHub installer.
    fn github_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Github Installer",
            description: "Download and install tools directly from GitHub releases. Supports various archive formats.",
            env_variables: None,
            examples: &[
                "  - name: helix",
                "    version: 25.07.1",
                "    source: github",
                "    repo: helix-editor/helix",
                "    tag: 25.07.1",
                "    rename_to: hx",
                "    additional_cmd:",
                "      - mkdir -p $HOME/.config/helix",
                "      - cp -r runtime $HOME/.config/helix",
                "",
                "  - name: zed",
                "    version: 0.200.1-pre",
                "    source: github",
                "    repo: zed-industries/zed",
                "    tag: v0.200.1-pre",
                "    rename_to: zed",
            ],
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
    fn go_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Go Installer",
            description: "Go package installer using 'go install'. Installs Go-based tools and applications.",
            env_variables: None,
            examples: &[
                "  - name: gopls",
                "    source: go",
                "    version: latest",
                "    url: golang.org/x/tools/gopls",
                "",
                "  - name: goreleaser",
                "    source: go",
                "    version: latest",
                "    url: github.com/goreleaser/goreleaser",
                "",
                "  - name: goimports",
                "    source: go",
                "    version: v0.1.12",
                "    url: golang.org/x/tools/cmd/goimports",
            ],
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
    fn pip_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Pip Installer",
            description: "Python package installer using pip. Installs Python libraries and command-line tools.",
            env_variables: None,
            examples: &[
                "  - name: black",
                "    source: pip",
                "    version: 22.3.0",
                "",
                "  - name: django",
                "    source: pip",
                "    version: latest",
                "    options:",
                "      - --user",
                "",
                "  - name: numpy",
                "    source: pip",
                "    version: 1.24.0",
                "    options:",
                "      - --upgrade",
                "      - --user",
            ],
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
    fn rustup_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Rustup Installer",
            description: "Rust toolchain installer and manager. Manages Rust versions, targets, and components.",
            env_variables: None,
            examples: &[
                "  - name: rust",
                "    source: rustup",
                "    version: stable",
                "    options:",
                "      - rust-src",
                "      - clippy",
                "      - rustfmt",
                "      - rust-analyzer",
                "",
                "  - name: rust-nightly",
                "    source: rustup",
                "    version: nightly",
                "    options:",
                "      - rustfmt",
                "      - clippy",
            ],
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
    fn shell_installer() -> InstallerInfo {
        InstallerInfo {
            name: "Shell Installer",
            description: "Shell configuration manager. Handles .zshrc, .bashrc files and shell aliases.",
            env_variables: Some(
                r#"SDB_RESET_SHELLRC_FILE: "true||false"

      Purpose:
        This feature ensures that updates to existing shell configuration elements
        (aliases, exports, commands) are properly applied by completely regenerating the
        shell configuration file rather than attempting to modify individual entries in place.

      The Problem: In-Place Modification Challenges
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
        Instead of attempting risky in-place modifications, the system:
        - Deletes the existing shell configuration file (.zshrc, .bashrc, etc.)
        - Recreates the file from scratch with all current configuration elements
        - Preserves the integrity and consistency of the entire configuration

      How It Works:
        1. Default Behavior (SDB_RESET_SHELLRC_FILE = "false"):
           - New configuration elements are appended to the existing file
           - Existing elements remain unchanged
           - This is safe but may lead to duplicates or outdated entries
        2. Automatic Regeneration (SDB_RESET_SHELLRC_FILE = "true"):
           - The entire shell config file is recreated when updates occur
           - All configuration elements are written fresh in a consistent format
           - Ensures no duplicates, outdated entries, or formatting inconsistencies"#,
            ),
            examples: &[
                "run_commands:",
                "  shell: \"zsh\" # or \"bash\"",
                "  run_commands:",
                "    # Exports Section - Environment variables",
                "    - command: |",
                "        export EDITOR=\"zed\"",
                "        export VISUAL=\"zed\"",
                "      section: Exports",
                "    - command: export UV_CONFIG_FILE=\"$HOME/.config/uv/uv.toml\"",
                "      section: Exports",
                "    - command: export PYENV_ROOT=\"$HOME/.pyenv\"",
                "      section: Exports",
                "",
                "    # Paths Section - PATH modifications",
                "    - command: export PATH=\"$HOME/bin:$PATH\"",
                "      section: Paths",
                "    - command: export PATH=\"$HOME/.local/bin:$PATH\"",
                "      section: Paths",
                "    - command: export PATH=\"$(go env GOPATH)/bin:$PATH\"",
                "      section: Paths",
                "",
                "    # Evals Section - Command evaluations",
                "    - command: eval \"$(pyenv init - zsh)\"",
                "      section: Evals",
                "    - command: eval \"$(atuin init zsh --disable-up-arrow)\"",
                "      section: Evals",
                "",
                "    # Other Section - Miscellaneous configurations",
                "    - command: source $HOME/.config/secrets.zsh",
                "      section: Other",
                "",
                "aliases:",
                "  - name: cat # Replace `cat` with `bat`",
                "    value: bat --paging=never",
                "  - name: config",
                "    value: zed $HOME/.config",
                "  - name: ls",
                "    value: lsd",
                "",
                "  # Git Aliases",
                "  - name: g",
                "    value: git",
                "  - name: gs",
                "    value: g status",
            ],
            options: &[
                "shell: Target shell (zsh, bash, fish)",
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
    fn url_installer() -> InstallerInfo {
        InstallerInfo {
            name: "URL Installer",
            description: "Direct URL installer. Downloads and installs software from direct download URLs.",
            env_variables: None,
            examples: &[
                "  - name: go",
                "    source: url",
                "    version: 1.24.5",
                "    url: https://go.dev/dl/go1.24.5.darwin-amd64.pkg",
                "",
                "  - name: docker-install",
                "    source: url",
                "    url: https://get.docker.com/",
                "    options:",
                "      - --script",
                "",
                "  - name: custom-tool",
                "    source: url",
                "    url: https://example.com/tool.tar.gz",
                "    options:",
                "      - --binary=tool",
                "      - --checksum=sha256:abc123...",
            ],
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
    fn uv_installer() -> InstallerInfo {
        InstallerInfo {
            name: "UV Installer",
            description: "Ultra-fast Python package installer with tool, pip, and python modes for modern Python development.",
            env_variables: None,
            examples: &[
                "  - name: cpython-3.13.7",
                "    source: uv",
                "    version: 3.13.7",
                "    options:",
                "      - --mode=python",
                "    additional_cmd:",
                "      - uv python update-shell",
                "",
                "  - name: ruff",
                "    source: uv",
                "    version: latest",
                "    options:",
                "      - --mode=tool",
                "",
                "  - name: django",
                "    source: uv",
                "    version: 4.2.0",
                "    options:",
                "      - --mode=pip",
            ],
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

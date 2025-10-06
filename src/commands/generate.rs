//! # Configuration File Generation Module
//!
//! This module provides functionality for generating default configuration files
//! for setup-devbox, offering users a solid foundation to start managing their
//! development environment.
//!
//! ## Purpose
//!
//! The generate command serves multiple use cases:
//! - **New User Onboarding**: Creates initial configuration templates with sensible defaults
//! - **Configuration Reset**: Allows users to regenerate specific config files
//! - **Recovery**: Helps restore missing configuration files without losing existing ones
//!
//! ## Design Philosophy
//!
//! - **Non-Destructive**: Never overwrites existing files to preserve user customizations
//! - **Comprehensive**: Generates all required configuration files in one command
//! - **Well-Documented**: Templates include inline comments explaining each option
//! - **Extensible**: Easy to add new configuration file types
//!
//! ## Generated Files
//!
//! 1. `config.yaml` - Main configuration file with references to all others
//! 2. `tools.yaml` - Development tools and installation configurations
//! 3. `settings.yaml` - OS-specific system settings
//! 4. `shellrc.yaml` - Shell initialization and aliases
//! 5. `fonts.yaml` - Font installation configurations

use crate::{log_debug, log_error, log_info};
use colored::Colorize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

// ============================================================================
// CONFIGURATION FILE DEFINITIONS
// ============================================================================

/// Standard filenames for generated configuration files
///
/// These constants ensure consistency across the codebase and make it easy
/// to reference configuration files by their canonical names.
mod filenames {
    pub const TOOLS: &str = "tools.yaml";
    pub const SETTINGS: &str = "settings.yaml";
    pub const SHELLRC: &str = "shellrc.yaml";
    pub const FONTS: &str = "fonts.yaml";
    pub const CONFIG: &str = "config.yaml";
}

// ============================================================================
// TEMPLATE DEFINITIONS
// ============================================================================

/// Configuration file templates with sensible defaults and documentation
///
/// Each template is designed to be:
/// - Self-documenting with inline comments
/// - Ready to use with minimal changes
/// - Educational for new users learning the configuration format
mod templates {
    use std::path::Path;

    /// Tools configuration template with common development tools
    ///
    /// Includes examples for:
    /// - Homebrew package manager installation
    /// - Git version control
    /// - Language toolchains (Rust, Go, Python)
    /// - CLI utilities
    pub const TOOLS: &str = r#"tools:
  ############################################################
  # Tools Configuration                                      #
  # This file defines all development tools to install      #
  # Uncomment and configure the tools you need              #
  ############################################################

  # Package Manager: Homebrew
  # Homebrew is the foundation for many other installations
  - name: brew
    version: 4.5.10
    source: github
    repo: Homebrew/brew
    tag: 4.5.10

  # Version Control: Git
  # Essential for most development workflows
  - name: git
    source: brew
    version: latest

  # Example: Rust Toolchain
  # Uncomment if you develop in Rust
  # - name: rustup
  #   source: brew

  # Example: Python Version Manager
  # Uncomment if you need multiple Python versions
  # - name: pyenv
  #   source: brew
  #   options:
  #     - --head

  # Example: Python Virtual Environment Manager
  # Works with pyenv for isolated Python environments
  # - name: pyenv-virtualenv
  #   source: brew
  #   options:
  #     - --head

  # Example: Rust Installation
  # Install the stable toolchain with common components
  # - name: rust
  #   source: rustup
  #   version: stable
  #   options:
  #     - rust-src      # Standard library source
  #     - clippy        # Linter
  #     - rustfmt       # Code formatter
  #     - rust-analyzer # Language server

  # Example: Go Programming Language
  # Update URL for your architecture (Intel vs Apple Silicon)
  # - name: go
  #   source: url
  #   url: https://go.dev/dl/go1.24.5.darwin-amd64.pkg

  # Example: Cargo Installer
  # ```yaml
  # `uv` - A single tool to replace pip, pip-tools, pipx, poetry, pyenv, twine, virtualenv, and more.
  # https://docs.astral.sh/uv/
  # - name: uv
  #   source: cargo
  #   version: 0.8.17
  #   options:
  #     - --git https://github.com/astral-sh/uv
  #   configuration_manager:
  #   enabled: true
  #   tools_configuration_paths:
  #     - $HOME/.config/uv/uv.toml

  # Example: GitHub CLI
  # Powerful command-line interface for GitHub
  # - name: cli
  #   version: "2.74.0"
  #   source: github
  #   repo: cli/cli
  #   tag: v2.74.0
  #   rename_to: gh
"#;

    /// macOS system settings template
    ///
    /// Configures system preferences via the `defaults` command.
    /// Currently includes examples for Finder customization.
    pub const SETTINGS: &str = r#"settings:
  macos:
    # Show all file extensions in Finder
    # Helps prevent confusion about file types
    - domain: NSGlobalDomain
      key: AppleShowAllExtensions
      value: "true"
      value_type: bool

    # Show hidden files in Finder
    # Useful for developers who need access to dotfiles
    - domain: com.apple.finder
      key: AppleShowAllFiles
      value: "true"
      value_type: bool

    # Additional setting examples (uncomment to use):

    # Faster key repeat rate
    # - domain: NSGlobalDomain
    #   key: KeyRepeat
    #   value: "1"
    #   value_type: int

    # Shorter delay until key repeat
    # - domain: NSGlobalDomain
    #   key: InitialKeyRepeat
    #   value: "15"
    #   value_type: int
"#;

    /// Shell configuration template
    ///
    /// Provides structure for:
    /// - Shell initialization commands
    /// - PATH modifications
    /// - Environment variable exports
    /// - Command aliases
    pub const SHELLRC: &str = r#"run_commands:
  shell: zsh
  run_commands:
    # Add custom bin directory to PATH
    - command: export PATH=$HOME/bin:$PATH
      section: PATH

    # Add Rust cargo binaries to PATH
    - command: export PATH=$HOME/.cargo/bin:$PATH
      section: PATH

    # Add Go binaries to PATH (if installed manually)
    # - command: export PATH=/usr/local/go/bin:$PATH
    #   section: PATH

    # Initialize Starship prompt (modern, fast shell prompt)
    - command: eval "$(starship init zsh)"
      section: Initialization

    # Configure pyenv (Python version manager)
    - command: export PYENV_ROOT="$HOME/.pyenv"
      section: Exports

    - command: '[[ -d $PYENV_ROOT/bin ]] && export PATH="$PYENV_ROOT/bin:$PATH"'
      section: PATH

    - command: eval "$(pyenv init - zsh)"
      section: Initialization

    - command: eval "$(pyenv virtualenv-init -)"
      section: Initialization

aliases:
  # Use bat instead of cat (syntax highlighting)
  - name: cat
    value: bat

  # Quick navigation to Go source directory
  - name: gocode
    value: cd $HOME/go/src/

  # Additional alias examples (uncomment to use):

  # Quick navigation to projects directory
  # - name: proj
  #   value: cd $HOME/projects

  # List files with details
  # - name: ll
  #   value: ls -lah
"#;

    /// Fonts configuration template
    ///
    /// Demonstrates font installation from GitHub releases.
    /// Includes popular developer fonts from Nerd Fonts collection.
    pub const FONTS: &str = r#"fonts:
  # 0xProto: Modern monospace font optimized for code
  - name: 0xProto
    version: "2.304"
    source: github
    repo: ryanoasis/nerd-fonts
    tag: v3.4.0

  # Additional font examples (uncomment to use):

  # FiraCode: Font with programming ligatures
  # - name: FiraCode
  #   version: "6.2"
  #   source: github
  #   repo: tonsky/FiraCode
  #   tag: "6.2"

  # JetBrainsMono: Font by JetBrains
  # - name: JetBrainsMono
  #   version: "2.304"
  #   source: github
  #   repo: ryanoasis/nerd-fonts
  #   tag: v3.4.0
"#;

    /// Main configuration file template generator
    ///
    /// Unlike other templates, this one is generated dynamically to include
    /// the actual paths being used by the application.
    ///
    /// # Arguments
    ///
    /// * `configs_dir` - The directory containing all configuration files
    ///
    /// # Returns
    ///
    /// A formatted string containing the main config template with actual paths
    pub fn config(configs_dir: &Path) -> String {
        format!(
            r#"# Main Configuration File
# This file references all other configuration files

tools: {}
settings: {}
shellrc: {}
fonts: {}
"#,
            configs_dir.join("tools.yaml").display(),
            configs_dir.join("settings.yaml").display(),
            configs_dir.join("shellrc.yaml").display(),
            configs_dir.join("fonts.yaml").display()
        )
    }
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// Errors that can occur during configuration file generation
#[derive(Debug)]
pub enum GenerateError {
    /// Failed to create directory
    DirectoryCreation(std::io::Error),

    /// Failed to create file
    FileCreation(std::io::Error),

    /// Failed to write content to file
    FileWrite(std::io::Error),
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerateError::DirectoryCreation(e) => {
                write!(f, "Failed to create directory: {}", e.to_string().red())
            }
            GenerateError::FileCreation(e) => {
                write!(f, "Failed to create file: {}", e.to_string().red())
            }
            GenerateError::FileWrite(e) => {
                write!(f, "Failed to write to file: {}", e.to_string().red())
            }
        }
    }
}

impl std::error::Error for GenerateError {}

/// Result type for generation operations
type GenerateResult<T> = Result<T, GenerateError>;

// ============================================================================
// FILE GENERATOR
// ============================================================================

/// Represents a single configuration file to be generated
///
/// This struct encapsulates all information needed to generate a specific
/// configuration file, including its name, content template, and metadata.
#[derive(Debug)]
struct ConfigFile {
    /// Filename (e.g., "tools.yaml")
    filename: &'static str,

    /// Template content to write (or generator function result)
    content: String,

    /// Human-readable description of the file's purpose
    description: &'static str,
}

impl ConfigFile {
    /// Creates a new ConfigFile definition with static content
    ///
    /// # Arguments
    ///
    /// * `filename` - The name of the file to generate
    /// * `content` - The template content to write
    /// * `description` - Human-readable description for logging
    fn new(filename: &'static str, content: &'static str, description: &'static str) -> Self {
        ConfigFile {
            filename,
            content: content.to_string(),
            description,
        }
    }

    /// Creates a new ConfigFile definition with dynamically generated content
    ///
    /// # Arguments
    ///
    /// * `filename` - The name of the file to generate
    /// * `content` - The generated content string
    /// * `description` - Human-readable description for logging
    fn with_generated_content(
        filename: &'static str,
        content: String,
        description: &'static str,
    ) -> Self {
        ConfigFile {
            filename,
            content,
            description,
        }
    }

    /// Generates this configuration file in the specified directory
    ///
    /// This method handles the complete file generation process:
    /// 1. Check if file already exists (skip if it does)
    /// 2. Create the file
    /// 3. Write template content
    /// 4. Log success or failure
    ///
    /// # Arguments
    ///
    /// * `config_dir` - Target directory for the file
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if file was created
    /// - `Ok(false)` if file already exists (skipped)
    /// - `Err(GenerateError)` if creation or writing failed
    fn generate(&self, config_dir: &Path) -> GenerateResult<bool> {
        let file_path = config_dir.join(self.filename);

        // Skip if file already exists to preserve user modifications
        if file_path.exists() {
            log_info!(
                "[Generate] Skipping existing file: {} (preserving your changes)",
                file_path.display().to_string().yellow()
            );
            return Ok(false);
        }

        log_debug!(
            "[Generate] Creating {} ({})",
            self.filename.cyan(),
            self.description
        );

        // Create and write to file
        let mut file = fs::File::create(&file_path).map_err(GenerateError::FileCreation)?;

        file.write_all(self.content.as_bytes())
            .map_err(GenerateError::FileWrite)?;

        log_info!(
            "[Generate] Created: {}",
            file_path.display().to_string().green()
        );

        Ok(true)
    }
}

// ============================================================================
// CONFIGURATION GENERATOR
// ============================================================================

/// Orchestrates the generation of all configuration files
///
/// This struct manages the complete configuration generation workflow,
/// including directory setup, file generation, and error handling.
pub struct ConfigGenerator {
    /// Target directory for generated files (typically ~/.setup-devbox/configs)
    configs_dir: PathBuf,

    /// List of all configuration files to generate
    config_files: Vec<ConfigFile>,
}

impl ConfigGenerator {
    /// Creates a new ConfigGenerator for the specified directory
    ///
    /// # Arguments
    ///
    /// * `configs_dir` - Directory where configuration files will be generated
    ///
    /// # Returns
    ///
    /// A ConfigGenerator instance ready to generate files
    pub fn new(configs_dir: PathBuf) -> Self {
        // Generate the main config content with actual paths
        let config_content = templates::config(&configs_dir);

        // Define all configuration files to generate
        // Order matters: main config should be generated last
        let config_files = vec![
            ConfigFile::new(
                filenames::TOOLS,
                templates::TOOLS,
                "Development tools configuration",
            ),
            ConfigFile::new(
                filenames::SETTINGS,
                templates::SETTINGS,
                "OS-specific system settings",
            ),
            ConfigFile::new(
                filenames::SHELLRC,
                templates::SHELLRC,
                "Shell initialization and aliases",
            ),
            ConfigFile::new(
                filenames::FONTS,
                templates::FONTS,
                "Font installation configuration",
            ),
            ConfigFile::with_generated_content(
                filenames::CONFIG,
                config_content,
                "Main configuration file",
            ),
        ];

        ConfigGenerator {
            configs_dir,
            config_files,
        }
    }

    /// Ensures the configuration directory exists
    ///
    /// Creates the directory and any necessary parent directories if they
    /// don't already exist.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if directory exists or was created successfully
    /// - `Err(GenerateError)` if directory creation failed
    fn ensure_directory_exists(&self) -> GenerateResult<()> {
        if !self.configs_dir.exists() {
            log_debug!(
                "[Generate] Creating config directory: {}",
                self.configs_dir.display()
            );

            fs::create_dir_all(&self.configs_dir).map_err(GenerateError::DirectoryCreation)?;

            log_info!(
                "[Generate] Created directory: {}",
                self.configs_dir.display().to_string().green()
            );
        } else {
            log_debug!(
                "[Generate] Directory already exists: {}",
                self.configs_dir.display()
            );
        }

        Ok(())
    }

    /// Executes the complete configuration generation process
    ///
    /// This method orchestrates the entire workflow:
    /// 1. Ensure configuration directory exists
    /// 2. Generate each configuration file
    /// 3. Track and report statistics
    ///
    /// ## Non-Destructive Behavior
    ///
    /// Files are never overwritten. If a file already exists, it is skipped
    /// to preserve user customizations. This allows safe re-running of the
    /// generate command.
    ///
    /// # Returns
    ///
    /// A summary of the generation operation including counts of created,
    /// skipped, and failed files.
    ///
    /// # Errors
    ///
    /// Returns `GenerateError` if directory creation fails. Individual file
    /// generation errors are logged but don't stop the process.
    pub fn generate(&self) -> GenerateResult<GenerationSummary> {
        log_info!(
            "[Generate] Starting configuration generation in: {}",
            self.configs_dir.display().to_string().cyan()
        );

        // Ensure target directory exists
        self.ensure_directory_exists()?;

        // Generate each configuration file
        let mut summary = GenerationSummary::new();

        for config_file in &self.config_files {
            match config_file.generate(&self.configs_dir) {
                Ok(true) => {
                    summary.created += 1;
                }
                Ok(false) => {
                    summary.skipped += 1;
                }
                Err(e) => {
                    log_error!(
                        "[Generate] Failed to generate {}: {}",
                        config_file.filename.red(),
                        e
                    );
                    summary.failed += 1;
                }
            }
        }

        log_info!("[Generate] Configuration generation completed");
        log_debug!("[Generate] Summary: {}", summary);

        Ok(summary)
    }
}

// ============================================================================
// GENERATION SUMMARY
// ============================================================================

/// Statistics about the configuration generation process
///
/// Tracks how many files were created, skipped, or failed during generation.
/// This provides useful feedback to users about what happened.
#[derive(Debug, Default)]
pub struct GenerationSummary {
    /// Number of files successfully created
    pub created: usize,

    /// Number of files skipped (already existed)
    pub skipped: usize,

    /// Number of files that failed to generate
    pub failed: usize,
}

impl GenerationSummary {
    fn new() -> Self {
        Self::default()
    }

    /// Total number of files processed
    pub fn total(&self) -> usize {
        self.created + self.skipped + self.failed
    }

    /// Whether the generation was completely successful
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }
}

impl std::fmt::Display for GenerationSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "created: {}, skipped: {}, failed: {}, total: {}",
            self.created,
            self.skipped,
            self.failed,
            self.total()
        )
    }
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Main entry point for the generate command
///
/// This function serves as the CLI command handler for configuration
/// generation. It coordinates the entire workflow and provides user-friendly
/// feedback for both success and failure scenarios.
///
/// ## Workflow
///
/// 1. Accept optional config and state paths (from PathResolver)
/// 2. Create ConfigGenerator instance with provided paths
/// 3. Execute generation process
/// 4. Report results to user
///
/// ## User Feedback
///
/// Success output includes:
/// - Summary of files created, skipped, and any failures
/// - Color-coded status messages
/// - Total files processed
/// - Next steps guidance
///
/// Partial failure output includes:
/// - Details about which files failed
/// - Summary statistics
/// - Non-zero exit code
///
/// Complete failure output includes:
/// - Error message with details
/// - Helpful guidance for resolution
/// - Non-zero exit code
///
/// # Arguments
///
/// * `config_dir` - Directory where configuration files will be generated
///   (typically from `PathResolver::configs_dir()`)
/// * `_state_path` - State file path (unused but accepted for API consistency)
///
/// # Exit Behavior
///
/// - **Success**: Returns normally after printing summary
/// - **Partial Failure**: Continues but reports issues and exits with code 1
/// - **Complete Failure**: Exits with code 1 after error message
///
/// # Example Usage
///
/// ```rust
/// let paths = PathResolver::new(None, None)?;
/// generate::run(paths.configs_dir(), paths.state_file().to_path_buf());
/// ```
///
/// # Example Output (Success)
///
/// ```text
/// [INFO] Starting configuration generation in: /Users/user/.setup-devbox/configs
/// [INFO] Created: /Users/user/.setup-devbox/configs/tools.yaml
/// [INFO] Created: /Users/user/.setup-devbox/configs/settings.yaml
/// [INFO] Skipping existing file: /Users/user/.setup-devbox/configs/shellrc.yaml
/// [INFO] Created: /Users/user/.setup-devbox/configs/fonts.yaml
/// [INFO] Created: /Users/user/.setup-devbox/configs/config.yaml
/// [INFO] Configuration generation completed
///
/// ================================================================================
/// Generation Summary:
///   Created: 4 files
///   Skipped: 1 file (already existed)
///   Failed:  0 files
///   Total:   5 files processed
/// ================================================================================
///
/// Configuration files are ready to use!
/// Next step: Review and customize the generated files for your needs.
/// ```
pub fn run(config_dir: PathBuf, _state_path: PathBuf) {
    log_debug!(
        "[Generate] Command invoked with config_dir: {}",
        config_dir.display()
    );

    // Create generator with the provided configs directory
    let generator = ConfigGenerator::new(config_dir);

    match generator.generate() {
        Ok(summary) => {
            // Print formatted summary
            println!("\n{}", "=".repeat(80).blue());
            println!("{}", "Generation Summary:".bold());
            println!(
                "  {}: {} file{}",
                "Created".green(),
                summary.created,
                if summary.created == 1 { "" } else { "s" }
            );
            println!(
                "  {}: {} file{} (already existed)",
                "Skipped".yellow(),
                summary.skipped,
                if summary.skipped == 1 { "" } else { "s" }
            );

            if summary.failed > 0 {
                println!(
                    "  {}: {} file{}",
                    "Failed".red(),
                    summary.failed,
                    if summary.failed == 1 { "" } else { "s" }
                );
            }

            println!("  {}: {} files processed", "Total".cyan(), summary.total());
            println!("{}\n", "=".repeat(80).blue());

            if summary.is_success() {
                println!("{}", "Configuration files are ready to use!".green());
                println!(
                    "{}",
                    "Next step: Review and customize the generated files for your needs.".cyan()
                );
            } else {
                println!(
                    "{}",
                    "⚠ Some files failed to generate. Check logs above for details.".yellow()
                );
                std::process::exit(1);
            }
        }
        Err(e) => {
            log_error!("[Generate] Fatal error: {}", e);
            eprintln!(
                "\n{} {}",
                "✗ Generation failed:".red().bold(),
                e.to_string().red()
            );
            eprintln!(
                "{}",
                "  Unable to create configuration directory or files.".yellow()
            );
            std::process::exit(1);
        }
    }

    log_debug!("[Generate] Command execution completed");
}

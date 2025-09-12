// This file handles the generation of default configuration files for `setup-devbox`.
// It provides initial, sensible config templates for new users or for resetting configurations.

use crate::libs::utilities::misc_utils::expand_tilde; // Helper to expand '~' in paths.
use crate::{log_debug, log_error, log_info}; // Custom logging macros.
use colored::Colorize; // For colored console output.
use std::fs; // File system operations (create directories, create files).
use std::io::Write; // Trait for writing data to files.
use std::path::Path; // Type for working with file paths.

// Constants defining standard filenames for generated configuration files.
const TOOLS_FILE: &str = "tools.yaml";
const SETTINGS_FILE: &str = "settings.yaml";
const SHELLRC_FILE: &str = "shellrc.yaml";
const FONTS_FILE: &str = "fonts.yaml";
const CONFIG_FILE: &str = "config.yaml";

// Default YAML content for configuration templates.

/// Default content for `tools.yaml`.
const TOOLS_TEMPLATE: &str = r#"tools:
  ############################################################
  # Generate command assume that the base system is vanilla  #
  # Based on that assumption it, generate the bare minimun   #
  # Which can be extended as you please                      #
  ############################################################

  # Install Brew: Brew Installer (using github)
  - name: brew
    version: 4.5.10
    source: github
    # This is look at the GitHub repository `Homebrew/brew` Release page
    # And it will download the `Homebrew-4.5.10.pkg`
    repo: Homebrew/brew
    tag: 4.5.10

  # Core Development Tools (Common for most setups)
  # Usually it is not available in the GitHub
  # Uncomment and configure the tools you need.
  - name: git
    source: brew # Install git using Homebrew
    version: latest # Or a specific version, e.g., "2.45.2"

  # Example: Rust Toolchain Installer
  # Uncomment if you develop in Rust.
  # This will install rustup
  # - name: rustup
  #   source: brew # Install rustup via Homebrew

  # Example: Install pyenv
  # Uncomment if you develop in in python and want `pyenv`.
  # - name: pyenv
  #   source: brew
  #   options:
  #     - --head

  # Example: Install pyenv-virtualenv
  # Uncomment if you develop in in python and want `pyenv-virtualenv`.
  # - name: pyenv-virtualenv
  #   source: brew
  #   options:
  #     - --head

  # Install rust and other rust tools
  # - name: rust
    # Specifies that 'rustup' should be used for installation
    # source: rustup
    # Targets your existing 'stable' toolchain. rustup will update it if needed.
    # version: stable
    # List of components to install with the 'stable' toolchain
    # options:
      # Source code for the Rust standard library, useful for IDEs
      # - rust-src
      # A linter to catch common mistakes and improve your Rust code
      # - clippy
      # A formatter for Rust code, ensuring consistent style
      # - rustfmt
      # The language server for Rust, providing IDE features
      # - rust-analyzer
      # You can add any other rustup components you need here.

  # Example: Go Installer (via direct URL)
  # Uncomment if you develop with Go.
    # - name: go
    #   source: url
    ##  Update to latest desired version for macOS Intel
    ##  For Apple Silicon (ARM64), use: https://go.dev/dl/go1.24.5.darwin-arm64.pkg
    #   url: https://go.dev/dl/go1.24.5.darwin-amd64.pkg

  # Example: GitHub CLI (gh)
  # Uncomment if you use GitHub heavily from the command line.
  # - name: cli
  #   version: "2.74.0"
  #   source: github
  #   repo: cli/cli
  #   tag: v2.74.0
  #   rename_to: gh

  # - name: git-spellcheck
  #   version: 0.0.1
  #   source: github
  #   repo: kodelint/git-spellcheck
  #   tag: v0.0.1
  #   rename_to: git-spellcheck
  # - name: git-pr
  #   version: 0.1.0
  #   source: github
  #   repo: kodelint/git-pr
  #   tag: v0.1.0
  #   rename_to: git-pr
"#;

/// Default content for `settings.yaml`.
const SETTINGS_TEMPLATE: &str = r#"settings:
  macos: # Settings specifically for macOS
    - domain: NSGlobalDomain             # A common domain for global macOS settings
      key: AppleShowAllExtensions      # The specific setting key: show file extensions
      value: "true"                    # Set its value to true
      type: bool                       # This setting expects a boolean value
    - domain: com.apple.finder           # Settings specific to the macOS Finder application
      key: AppleShowAllFiles           # The specific setting key: show hidden files
      value: "true"                    # Set its value to true
      type: bool                       # This setting also expects a boolean value
"#;

/// Default content for `shellac.yaml`.
const SHELLRC_TEMPLATE: &str = r#"shellrc:
  shell: zsh # The type of shell being configured (e.g., zsh, bash, fish)
  raw_configs: # A list of lines that will be directly added to the shell's config file (e.g., ~/.zshrc)
    - export PATH=$HOME/bin:$PATH        # Add a custom 'bin' directory to the system PATH
    - export PATH=$HOME/.cargo/bin:$PATH # Add Rust's cargo binaries to the PATH
    # Uncomment the following if you install Go manually (e.g., via direct URL installer)
    # - export PATH=/usr/local/go/bin:$PATH # Add Go binaries to the PATH
    - eval "$(starship init zsh)"        # Initialize Starship prompt for a fancy shell prompt
    - export PYENV_ROOT="$HOME/.pyenv"   # Export pyenv root
    - [[ -d $PYENV_ROOT/bin ]] && export PATH="$PYENV_ROOT/bin:$PATH" # map it to the $PATH
    - eval "$(pyenv init - zsh)"         # Initialize pyenv
    - eval "$(pyenv virtualenv-init -)"  # enable pyenv-virtualenv in environment
aliases: # Custom command aliases for user convenience
  - name: cat                           # Replace `cat with `bat`
    value: bat                          # The command it expands to: change directory to a common dev folder
  - name: gocode                        # Another alias for Go development
    value: cd $HOME/go/src/             # Change directory to the Go source workspace
"#;

/// Default content for `fonts.yaml`.
const FONTS_TEMPLATE: &str = r#"fonts:
  - name: 0xProto     # The name of the font
    version: "2.304"  # A specific version of the font
    source: github    # Where to get it: GitHub releases
    repo: ryanoasis/nerd-fonts # The GitHub repo for Nerd Fonts
    tag: v3.4.0       # The specific release tag for this font version
"#;

/// Default content for `config.yaml`.
const CONFIG_TEMPLATE: &str = r#"tools: $HOME/.setup-devbox/configs/tools.yaml  # Tells devbox where to find the tools configuration
settings: $HOME/.setup-devbox/configs/settings.yaml # Tells devbox where to find the settings configuration
shellrc: $HOME/.setup-devbox/configs/shellrc.yaml   # Tells devbox where to find the shell configuration
fonts: $HOME/.setup-devbox/configs/fonts.yaml       # Tells devbox where to find the fonts configuration
"#;

/// Executes the `generate` command, creating default configuration files.
///
/// # Arguments
/// * `config_dir`: Optional custom directory for config files. Defaults to `~/.setup-devbox/configs/`.
/// * `_state_path`: Unused parameter for future state file path specification.
pub fn run(config_dir: Option<String>, _state_path: Option<String>) {
    log_debug!(
        "[Generate] Starting generation with config_dir: {}",
        config_dir.as_deref().unwrap_or("None")
    );

    // Resolve the base directory for config generation.
    let base_dir = config_dir.as_deref().unwrap_or("~/.setup-devbox/configs/");
    let base_dir = expand_tilde(base_dir);

    log_info!(
        "[Generate] Using config directory: {}",
        base_dir.to_string_lossy().green()
    );

    // Create the base configuration directory if it does not exist.
    if !base_dir.exists() {
        match fs::create_dir_all(&base_dir) {
            Ok(_) => log_info!(
                "[Generate] Created config directory {}",
                base_dir.to_string_lossy().green()
            ),
            Err(e) => {
                log_error!("[Generate] Failed to create config directory: {}", e);
                return;
            }
        }
    }

    // Generate individual configuration files from templates.
    generate_file(&base_dir, TOOLS_FILE, TOOLS_TEMPLATE);
    generate_file(&base_dir, SETTINGS_FILE, SETTINGS_TEMPLATE);
    generate_file(&base_dir, SHELLRC_FILE, SHELLRC_TEMPLATE);
    generate_file(&base_dir, FONTS_FILE, FONTS_TEMPLATE);

    // Generate the main `config.yaml` last, as it references the other files.
    generate_file(&base_dir, CONFIG_FILE, CONFIG_TEMPLATE);

    log_info!("[Generate] All requested config files processed.");
}

/// Helper function to create a single configuration file from a template.
/// Prevents overwriting existing files.
///
/// # Arguments
/// * `base_dir`: The target base directory.
/// * `filename`: The name of the file to create.
/// * `content`: The template content to write.
fn generate_file(base_dir: &Path, filename: &str, content: &str) {
    let file_path = base_dir.join(filename);

    // Skip file creation if it already exists to preserve user modifications.
    if file_path.exists() {
        log_info!(
            "[Generate] Skipping existing file {}. We don't want to overwrite your changes!",
            file_path.to_string_lossy().bright_yellow()
        );
        return;
    }

    log_info!(
        "[Generate] Creating new file {}",
        file_path.to_string_lossy().bright_green()
    );

    // Attempt to create and write content to the file.
    match fs::File::create(&file_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(content.as_bytes()) {
                log_error!(
                    "[Generate] Failed to write to {}: {}",
                    file_path.to_string_lossy().red(),
                    e
                );
            } else {
                log_info!(
                    "[Generate] Successfully wrote default content to {}",
                    file_path.to_string_lossy().bright_green()
                );
            }
        }
        Err(e) => {
            log_error!(
                "[Generate] Couldn't create file {}: {}",
                file_path.to_string_lossy().red(),
                e
            );
        }
    }
}

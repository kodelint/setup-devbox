// src/commands/generate.rs
// This file is all about setting up our initial configuration files.
// Think of it as the "kickstarter" for a new `setup-devbox` user, providing them with
// sensible default configuration files so they can hit the ground running.

// We're bringing in a helper function to correctly handle '~' (tilde) in paths,
// which is a common way users refer to their home directory.
use crate::libs::utilities::path_helpers::expand_tilde;
// Our custom logging macros to give us nicely formatted (and colored!) output
// for debugging, general information, and errors.
use crate::{log_debug, log_error, log_info};
// The 'colored' crate helps us make our console output look pretty and readable.
use colored::Colorize;
// 'std::fs' is our toolkit for interacting with the file system – creating directories, creating files, etc.
use std::fs;
// 'std::io::Write' is a trait that allows us to write data to files.
use std::io::Write;
// 'std::path::Path' is a powerful type for working with file paths in a robust way.
use std::path::Path;

// File Names for Our Configuration Templates
// These constants define the standard names for the configuration files
// that `setup-devbox` will generate. This keeps our file naming consistent.
const TOOLS_FILE: &str = "tools.yaml";      // Where users define the software tools they want.
const SETTINGS_FILE: &str = "settings.yaml"; // Where users define system-level settings (e.g., macOS defaults).
const SHELLRC_FILE: &str = "shellac.yaml";   // Where users customize their shell environment and aliases.
const FONTS_FILE: &str = "fonts.yaml";      // Where users list the custom fonts they want installed.
const CONFIG_FILE: &str = "config.yaml";    // The main configuration file that points to the others.

// Default Configuration Templates (YAML Content)
// These multi-line string constants hold the default YAML content for each
// configuration file. These are carefully crafted examples to show users
// how to structure their configurations and give them immediate working examples.

/// The default content for `tools.yaml`.
/// It includes examples for installing a GitHub CLI tool ('gh') and `rustup` via Homebrew.
/// This template is a great starting point for users to understand how to declare tools.
const TOOLS_TEMPLATE: &str = r#"tools:
  - name: cli            # The common name of the tool (e.g., 'gh' for GitHub CLI)
    version: "2.74.0"    # The specific version we recommend as a starting point
    source: github       # Where to get it from: GitHub releases
    repo: cli/cli        # The GitHub repository: owner/repo_name
    tag: v2.74.0         # The specific release tag to download
    rename_to: gh        # Rename the downloaded executable from 'cli' to 'gh' for convenience
  - name: rustup         # A popular tool for managing Rust toolchains
    source: brew         # Get it using the Homebrew package manager (macOS/Linux)
"#;

/// The default content for `settings.yaml`.
/// This template provides example macOS settings, such as showing file extensions
/// and hidden files in Finder, which are common quality-of-life improvements for developers.
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

/// The default content for `shellac.yaml`.
/// This template demonstrates how to add common `PATH` modifications for development tools
/// and how to set up command aliases for quick navigation.
const SHELLRC_TEMPLATE: &str = r#"shellrc:
  shell: zsh # The type of shell being configured (e.g., zsh, bash, fish)
  raw_configs: # A list of lines that will be directly added to the shell's config file (e.g., ~/.zshrc)
    - export PATH=$HOME/bin:$PATH        # Add a custom 'bin' directory to the system PATH
    - export PATH=$HOME/.cargo/bin:$PATH # Add Rust's cargo binaries to the PATH
    - export PATH=$HOME/go/bin:$PATH     # Add Go binaries to the PATH
    - eval "$(starship init zsh)"        # Initialize Starship prompt for a fancy shell prompt
aliases: # Custom command aliases for user convenience
  - name: code                           # The short alias name
    value: cd $HOME/Documents/github/    # The command it expands to: change directory to a common dev folder
  - name: gocode                         # Another alias for Go development
    value: cd $HOME/go/src/              # Change directory to the Go source workspace
"#;

/// The default content for `fonts.yaml`.
/// This template shows an example of how to declare a custom font for installation,
/// useful for developers who prefer specific coding fonts like Nerd Fonts.
const FONTS_TEMPLATE: &str = r#"fonts:
  - name: 0xProto     # The name of the font
    version: "2.304"  # A specific version of the font
    source: github    # Where to get it: GitHub releases
    repo: ryanoasis/nerd-fonts # The GitHub repo for Nerd Fonts
    tag: v3.4.0       # The specific release tag for this font version
"#;

/// This is the master configuration file template: `config.yaml`.
/// It acts as an index, pointing 'devbox' to the locations of all the other
/// specialized configuration files. This provides flexibility for users to
/// organize their config files as they see fit.
const CONFIG_TEMPLATE: &str = r#"tools: tools.yaml     # Tells devbox where to find the tools configuration
settings: settings.yaml # Tells devbox where to find the settings configuration
shellrc: shellac.yaml   # Tells devbox where to find the shell configuration
fonts: fonts.yaml     # Tells devbox where to find the fonts configuration
"#;

/// The main entry point for the `generate` command.
/// This function orchestrates the creation of all the default configuration files
/// in a specified (or default) directory. It's user-facing and helps them get started.
///
/// # Arguments
/// * `config_dir`: An `Option<String>` that allows the user to specify a custom directory
///                 for their configuration files. If `None`, a default location is used.
/// * `_state_path`: (Currently unused, hence the `_`) An `Option<String>` that might
///                  be used in the future to specify the path to the internal state file.
pub fn run(config_dir: Option<String>, _state_path: Option<String>) {
    // Log a detailed debug message about the starting parameters.
    log_debug!("[Generate] Starting generation with config_dir: {:?}", config_dir);

    // Determine the base directory where config files will be generated.
    // If the user provided a `config_dir`, use that. Otherwise, default to `~/.setup-devbox/configs/`.
    let base_dir = config_dir
        .as_deref() // Convert Option<String> to Option<&str>
        .unwrap_or("~/.setup-devbox/configs/"); // Default path if none provided

    // Expand the tilde (~) in the path to the actual home directory (e.g., "/Users/youruser/").
    let base_dir = expand_tilde(base_dir);

    // Inform the user about the chosen configuration directory.
    log_info!("[Generate] Using config directory: {:?}", base_dir);

    // Check if the base configuration directory already exists.
    if !base_dir.exists() {
        // If it doesn't exist, try to create it and all its parent directories.
        match fs::create_dir_all(&base_dir) {
            Ok(_) => log_info!("[Generate] Created config directory {:?}", base_dir),
            Err(e) => {
                // If directory creation fails, log an error and stop.
                log_error!("[Generate] Failed to create config directory: {}", e);
                return; // Exit the function early on failure.
            }
        }
    }

    // Now, let's generate each of the individual configuration files using their templates.
    // We call a helper function for each file to keep the code clean.
    generate_file(&base_dir, TOOLS_FILE, TOOLS_TEMPLATE);
    generate_file(&base_dir, SETTINGS_FILE, SETTINGS_TEMPLATE);
    generate_file(&base_dir, SHELLRC_FILE, SHELLRC_TEMPLATE);
    generate_file(&base_dir, FONTS_FILE, FONTS_TEMPLATE);

    // It's important to generate the 'config.yaml' file *last*.
    // This way, all the individual config files it points to are already in place.
    generate_file(&base_dir, CONFIG_FILE, CONFIG_TEMPLATE);

    // Finally, let the user know that all the requested config files have been handled.
    log_info!("[Generate] All requested config files processed.");
}

/// A helper function to create a single configuration file from a template.
/// It checks if the file already exists to avoid overwriting user changes.
///
/// # Arguments
/// * `base_dir`: The base directory where the file should be created.
/// * `filename`: The name of the file to create (e.g., "tools.yaml").
/// * `content`: The string content (YAML template) to write into the file.
fn generate_file(base_dir: &Path, filename: &str, content: &str) {
    // Construct the full path to the file (base_dir + filename).
    let file_path = base_dir.join(filename);

    // Before creating, let's check if the file already exists.
    // We don't want to accidentally erase a user's custom configuration!
    if file_path.exists() {
        // If it exists, we just log a message and skip creating it.
        log_info!("[Generate] Skipping existing file {:?}. We don't want to overwrite your changes!", file_path);
        return; // Exit this helper function.
    }

    // If the file doesn't exist, we'll proceed to create it.
    log_info!("[Generate] Creating new file {:?}", file_path);

    // Attempt to create the file.
    match fs::File::create(&file_path) {
        // If file creation is successful, we get a `File` handle.
        Ok(mut file) => {
            // Now, try to write the template content into the newly created file.
            if let Err(e) = file.write_all(content.as_bytes()) {
                // If writing fails (e.g., permissions issue), log an error.
                log_error!("[Generate] Oh no! Failed to write to {:?}: {}", file_path, e);
            } else {
                // Success! The file was written.
                log_info!("[Generate] Successfully wrote default content to {:?}", file_path);
            }
        }
        // If file creation itself fails (e.g., directory permissions), log an error.
        Err(e) => {
            log_error!("[Generate] Couldn't create file {:?}: {}", file_path, e);
        }
    }
}
